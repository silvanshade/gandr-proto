//! **Two-mode certificate composition** and the **acyclicity gate** (ADR-69
//! D3).
//!
//! Spec: the VDC reflection design's §4.1, §4.3; VDC addendum §A. Generic
//! over the [`CellAlphabet`] (the executed meta-spike-01).
//!
//! Chaining derived transformations (certificates) has two costs, and the
//! categorical line proves they behave differently:
//!
//! - **invertible / coherence lane** ([`compose_invertible`]): both
//!   certificates live in the groupoid fragment (completion-emitted joinability
//!   certificates), where dinaturals *always* compose (LLV Thm 4.5).
//!   Composition is **unconditional** — no gate.
//! - **directed lane** ([`compose_directed`]): oriented certificates are
//!   dinaturality-shaped, and dinaturals compose only under **loop-freeness**
//!   (LLV Thm 5.3's no-full-cut, the Danos–Regnier proof-net pedigree of §4.3).
//!   The gate builds the **variable-flow graph** across the composed seam,
//!   preserving *both* face-internal directions for [`SeamRole::Both`] holes,
//!   and [`cycle_witness`] it **once**: a cycle DECLINES the composition,
//!   carrying the cycle as the diagnostic (the completion-budget posture —
//!   decline-with-report, never divergence or panic).
//!
//! Both modes glue the certificates **sequentially**: `a`'s certified output
//! (`a.joins_at`) is `b`'s certified input (`b.overlap.peak`), and the
//! composite certificate replays as `a`'s recorded derivation followed by `b`'s
//! (the identity that keeps grafting associative/unital is replay-equivalence,
//! [`crate::tracelet::replay_equivalent`], ADR-69 D1).
//!
//! # The graph seam
//!
//! Nodes are `(CellId, hole)` — the metavariable holes of the cells the two
//! certificates fire, restricted to those the **seam variables** (the holes of
//! `a.joins_at`) touch. Edges carry the face-internal flow across the seam,
//! read through [`CellAlphabet::hole_flow`]:
//!
//! - a [`SeamRole::Forward`] hole flows forward, `a`-side → `b`-side;
//! - a [`SeamRole::Backward`] hole flows backward, `b`-side → `a`-side;
//! - a [`SeamRole::Both`] hole (the sequent `Mixed` — a name at both
//!   polarities, which `μ`/`μ̃` and cocase create) flows **both** ways, so a
//!   shared `Both` seam hole closes a loop.
//!
//! The criterion is a **sufficient** loop-freeness check (conservative by
//! design: the reversal trigger of ADR-69 is "refine the criterion, never
//! remove the gate"). A single cell is never gated — with no seam there are no
//! edges, so cell *application* (as opposed to certificate *composition*) is
//! untouched.
//!
//! # The verdict is not a certificate invariant
//!
//! **[`compose_directed`]'s verdict is a function of the two certificates'
//! recorded cell *support*, the hole names of `a.joins_at`, and the store — and
//! of nothing else.** Positions, step order and repetition never reach it:
//! [`participating_cells`] deduplicates, and no position is read anywhere in
//! the build. Two derivations recording the same cells are one input.
//!
//! **A cell support is not certificate data.** Certificate identity is
//! [`crate::tracelet::replay_equivalent`] — a peak, a join, and two replays —
//! which forgets the recorded derivation entirely. So two presentations of one
//! certificate can carry different supports, and **the verdict can differ
//! between them**. That is measured rather than feared:
//! `composition::tests::the_acyclicity_verdict_is_not_invariant_under_certificate_identity`
//! composes one certificate's two-step presentation and its fused presentation
//! against one partner, gets `Err` and `Ok`, and replays the composite the
//! `Ok` admitted.
//!
//! **The exact class is cell-support equality _at a fixed boundary_**, and the
//! restriction is not decoration: the build reads
//! [`participating_cells`] on **both** sides *and* the seam holes of
//! `a.joins_at`, so equal supports alone do not fix the verdict. The two
//! coincide only when the comparison is between presentations of one
//! certificate, where replay-equivalence holds the boundary fixed by its own
//! definition — which is the setting this was measured in.
//!
//! **That relation is _incomparable_ with replay-equivalence, not coarser than
//! it.** Neither implies the other, and both failures are exhibited in the
//! fixtures: `the_acyclicity_verdict_is_not_invariant_under_certificate_identity`
//! shows two replay-equivalent certificates with different supports, and a
//! presentation with a repeated step has one certificate's support without
//! replaying at all. Reading the relations as a chain with cell support at the
//! coarse end contradicts the finding above — were replay-equivalence to imply
//! cell-support equality, the verdict *would* be a certificate invariant.
//!
//! **What the non-invariance can cost is availability, never soundness**, and
//! the asymmetry is structural rather than lucky. The `Ok` branch returns the
//! graft, whose boundary is `a`'s peak and `b`'s join — both of them data
//! replay-equivalence compares — so **the composite is a certificate invariant
//! even where the verdict is not**, and two admitted presentations compose to
//! one certificate. That is measured too, against a partner that admits both:
//! `composition::tests::the_composite_is_a_certificate_invariant_even_where_the_verdict_is_not`.
//! The gate is a sufficient check by construction; what a presentation changes
//! is *how* conservative it is on that instance.
//!
//! **So the operation is well defined on the cell-support-at-a-fixed-boundary
//! quotient and not on the replay quotient, and the implementation carries the
//! recorded derivation rather than quotienting it away.** The two alternatives
//! are worse and were considered: *refusing* a composition whose presentations
//! disagree would decline the compositions the lane exists to admit, and
//! *canonicalizing* has nothing to canonicalize to, because a
//! replay-equivalence class has no canonical derivation — the tracelet normal
//! form canonicalizes **within** a cell support (it preserves the primitive
//! multiset) and so cannot bridge two supports.
//!
//! **One consequence for anyone reading a verdict as a property of a
//! certificate: it is not one.** A decline is a fact about the derivation in
//! hand. Re-deriving the same boundary another way may compose where this one
//! declined, and that is a legitimate response to an obstruction rather than a
//! contradiction.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use gandr_theory_graphs::CycleWitness;
use gandr_theory_graphs::EdgeSource;
use gandr_theory_graphs::NodeCount;
use gandr_theory_graphs::NodeId;
use gandr_theory_graphs::cycle_witness;

use crate::alphabet::CellAlphabet;
use crate::alphabet::SeamRole;
use crate::boundary::VarianceFlowRole;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::sequent::SequentAlphabet;
use crate::tracelet::Tracelet;

/// A **variable-flow cycle** obstructing directed certificate composition
/// (`proposal-vdc-reflection.md` §4.3; ADR-69 D3).
///
/// The `cycle` is the closed walk of `(CellId, hole)` nodes whose seam flow
/// loops — the decline diagnostic. A [`SeamRole::Both`] seam hole (shared
/// across the composed seam) is the canonical cause: it flows both ways, so it
/// closes a loop the directed lane cannot admit (over groupoids it could —
/// [`compose_invertible`]).
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionObstruction<A: CellAlphabet = SequentAlphabet>
{
    /// The closed walk of `(cell, hole)` nodes the seam flow loops through.
    pub cycle: Vec<(CellId, A::Var)>,
}

/// **Compose two invertible certificates** — the coherence lane, unconditional
/// (LLV Thm 4.5: over groupoids, dinaturals always compose).
///
/// The composite certifies `a.overlap.peak ~> b.joins_at` by replaying `a`'s
/// derivation then `b`'s; it reuses `a`'s overlap (whose peak is the
/// composite's input boundary) and concatenates the recorded paths. No
/// acyclicity gate: the invertible flag (the metadata's `invertible` — in the
/// sequent alphabet [`crate::sequent::CellMeta::invertible`]) is the caller's
/// warrant that both certificates live in the groupoid fragment.
///
/// # Contract
/// - requires: `a.joins_at == b.overlap.peak` (`a`'s certified output is `b`'s
///   certified input — the sequential seam), and both certificates are
///   invertible joinability certificates (the coherence lane); under the seam
///   precondition the result replays (paths concatenate positionally, since
///   `a`'s derivation lands exactly on `b`'s peak).
/// - ensures: a tracelet whose `path_a` / `path_b` are `a`'s followed by `b`'s
///   and whose `joins_at` is `b.joins_at`.
/// - panics: none.
///
/// # Certificate identity
/// **Unaffected by the directed lane's non-invariance, and checked rather than
/// inherited.** This function never consults a recorded cell; it grafts, and
/// the graft's boundary is `a`'s peak and `b`'s join, which are exactly what
/// [`crate::tracelet::replay_equivalent`] compares. So two presentations of one
/// certificate compose to two presentations of one certificate, and the
/// operation **descends to the replay quotient** — which is what the directed
/// lane's verdict does not do (see the module header).
///
/// # Adequacy
/// - hypothesis: L1 evidence — the linear ground chain fixture composes two
///   real fused certificates and the composite replays over the store; the
///   paths are the concatenation.
/// - witness: `composition::tests::invertible_composition_of_a_ground_chain_replays`
/// - witness: `composition::tests::invertible_composition_is_well_defined_on_the_replay_quotient`
#[inline]
#[must_use]
pub fn compose_invertible<A>(
    a: &Tracelet<A>,
    b: &Tracelet<A>,
) -> Tracelet<A>
where
    A: CellAlphabet,
{
    graft(a, b)
}

/// **Compose two directed certificates**, gated by variable-flow acyclicity
/// across the composed seam (ADR-69 D3; the decline-with-report posture).
///
/// Builds the seam variable-flow graph (nodes `(CellId, hole)`, edges the
/// face-internal flow with both directions for [`SeamRole::Both`] holes),
/// calls [`cycle_witness`] **once**, and either declines
/// (a validated closed walk mapped back to `(CellId, hole)`) or composes
/// (the sequential graft, as [`compose_invertible`]).
///
/// # Contract
/// - requires: `a.joins_at == b.overlap.peak` (the sequential seam) for the
///   `Ok` composite to replay; the `Err` path (a cycle) needs no seam agreement
///   — the loop is read from the cells' variance alone.
/// - ensures: `Ok(tracelet)` (the graft) when the seam variable-flow graph is
///   acyclic; `Err(obstruction)` carrying the closed `(cell, hole)` cycle when
///   a shared seam hole (canonically `Both`) loops the flow. Never diverges or
///   panics — the gate is a bounded static check.
/// - panics: none.
///
/// # Errors
/// Returns [`CompositionObstruction`] when the composed seam variable-flow
/// graph contains a directed cycle.
///
/// # Certificate identity
/// **The verdict is not a certificate invariant** — it reads the recorded cell
/// support, which [`crate::tracelet::replay_equivalent`] forgets. The module
/// header states what that costs and why it is carried rather than repaired
/// here; the short form is that a decline is a fact about the derivation in
/// hand, the `Ok` branch's composite is an invariant even so, and the operation
/// is well defined on the cell-support-at-a-fixed-boundary quotient and not on
/// the replay quotient.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the mixed-variance cycle fixture drives this
///   function to `Err` and validates the returned cycle is a closed walk of
///   `(CellId, hole)` nodes over the participating cells with the `Mixed` hole
///   `r`; the linear ground chain drives it to `Ok` and the composite replays.
/// - witness: `composition::tests::directed_composition_declines_a_mixed_variance_cycle`
/// - witness: `composition::tests::directed_composition_of_a_ground_chain_replays`
/// - witness: `composition::tests::the_acyclicity_verdict_reads_the_recorded_cell_support_and_nothing_finer`
/// - witness: `composition::tests::the_acyclicity_verdict_is_not_invariant_under_certificate_identity`
/// - witness: `composition::tests::the_composite_is_a_certificate_invariant_even_where_the_verdict_is_not`
#[inline]
pub fn compose_directed<A>(
    a: &Tracelet<A>,
    b: &Tracelet<A>,
    store: &CellStore<A>,
) -> Result<Tracelet<A>, CompositionObstruction<A>>
where
    A: CellAlphabet,
{
    let graph = VarFlowGraph::build(a, b, store);
    // One call; a well-formed dense graph never surfaces a validation error, so
    // an (unreachable) boundary failure is read as "no cycle found".
    match cycle_witness(&graph).ok().flatten() {
        | Some(witness) => Err(graph.obstruction(&witness)),
        | None => Ok(graft(a, b)),
    }
}

/// Graft `b`'s derivation onto `a`'s: the sequential composite certificate.
///
/// # Contract
/// - requires: `a.joins_at == b.overlap.peak` for the result to replay.
/// - ensures: a tracelet reusing `a.overlap`, with each path the concatenation
///   of `a`'s and `b`'s, joining at `b.joins_at`.
/// - panics: none.
#[inline]
fn graft<A>(
    a: &Tracelet<A>,
    b: &Tracelet<A>,
) -> Tracelet<A>
where
    A: CellAlphabet,
{
    let mut path_a = a.path_a.clone();
    path_a.extend(b.path_a.iter().cloned());
    let mut path_b = a.path_b.clone();
    path_b.extend(b.path_b.iter().cloned());
    Tracelet {
        overlap: a.overlap.clone(),
        path_a,
        path_b,
        joins_at: b.joins_at.clone(),
    }
}

/// The dense **variable-flow graph** of a composed seam — an
/// [`EdgeSource`] over `(CellId, hole)` nodes.
struct VarFlowGraph<A: CellAlphabet>
{
    /// The nodes, densely indexed; dense id `i` is node `nodes[i]`.
    nodes: Vec<(CellId, A::Var)>,
    /// Outgoing successor dense ids per node, indexed by dense id.
    adjacency: Vec<Vec<NodeId>>,
}

impl<A: CellAlphabet> VarFlowGraph<A>
{
    /// Build the seam variable-flow graph for composing `a` then `b`.
    ///
    /// # Contract
    /// - ensures: a dense graph whose nodes are the `(cell, hole)` endpoints
    ///   the seam variables (the holes of `a.joins_at`) touch across `a`'s and
    ///   `b`'s participating cells, and whose edges are the face-internal flow
    ///   — `a`→`b` for a forward role, `b`→`a` for a backward role, both for a
    ///   [`SeamRole::Both`] role; every edge target is a valid node index.
    /// - panics: none.
    #[inline]
    fn build(
        a: &Tracelet<A>,
        b: &Tracelet<A>,
        store: &CellStore<A>,
    ) -> Self
    {
        let a_cells = participating_cells(a);
        let b_cells = participating_cells(b);
        let mut builder = GraphBuilder::default();
        for hole in seam_holes::<A>(&a.joins_at) {
            let a_endpoints = endpoints(&a_cells, &hole, store);
            let b_endpoints = endpoints(&b_cells, &hole, store);
            if a_endpoints.is_empty() || b_endpoints.is_empty() {
                // Not shared across the seam — no cross flow.
                continue;
            }
            let forward = a_endpoints
                .iter()
                .chain(&b_endpoints)
                .any(|&(_, _, role)| bool::from(forward_role(role)));
            let backward = a_endpoints
                .iter()
                .chain(&b_endpoints)
                .any(|&(_, _, role)| bool::from(backward_role(role)));
            for a_endpoint in &a_endpoints {
                for b_endpoint in &b_endpoints {
                    let node_a = builder.intern(a_endpoint.0, &a_endpoint.1);
                    let node_b = builder.intern(b_endpoint.0, &b_endpoint.1);
                    if forward {
                        builder.edge(node_a, node_b);
                    }
                    if backward {
                        builder.edge(node_b, node_a);
                    }
                }
            }
        }
        builder.finish()
    }

    /// Map a validated cycle witness back to the semantic `(CellId, hole)`
    /// obstruction.
    ///
    /// # Contract
    /// - ensures: the witness node walk (its closing duplicate dropped) mapped
    ///   through [`VarFlowGraph::nodes`]; ids outside the node range are
    ///   skipped (a defensive guard — a well-formed witness stays in range).
    /// - panics: none.
    #[inline]
    fn obstruction(
        &self,
        witness: &CycleWitness,
    ) -> CompositionObstruction<A>
    {
        // Drop the trailing node that closes the walk (`first == last`).
        let walk = witness
            .nodes
            .split_last()
            .map_or(witness.nodes.as_slice(), |(_, rest)| rest);
        let mut cycle = Vec::with_capacity(walk.len());
        for &node in walk {
            if let Some(entry) = usize::try_from(u32::from(node))
                .ok()
                .and_then(|index| self.nodes.get(index))
            {
                cycle.push(entry.clone());
            }
        }
        CompositionObstruction { cycle }
    }
}

impl<A: CellAlphabet> EdgeSource for VarFlowGraph<A>
{
    type Successors<'successors>
        = core::iter::Copied<core::slice::Iter<'successors, NodeId>>
    where
        Self: 'successors;

    #[inline]
    fn node_count(&self) -> NodeCount
    {
        NodeCount::from(u32::try_from(self.nodes.len()).unwrap_or(u32::MAX))
    }

    #[inline]
    fn successors(
        &self,
        node: NodeId,
    ) -> Self::Successors<'_>
    {
        let empty: &[NodeId] = &[];
        usize::try_from(u32::from(node))
            .ok()
            .and_then(|index| self.adjacency.get(index))
            .map_or(empty, Vec::as_slice)
            .iter()
            .copied()
    }
}

/// Accumulates the dense node table and edge set of a [`VarFlowGraph`].
struct GraphBuilder<A: CellAlphabet>
{
    /// The interned nodes, in allocation order (dense id order).
    nodes: Vec<(CellId, A::Var)>,
    /// The dense id assigned to each `(cell, hole)` key.
    index: BTreeMap<(CellId, A::Hole), NodeId>,
    /// Outgoing successor dense ids per node, indexed by dense id.
    adjacency: Vec<Vec<NodeId>>,
}

impl<A: CellAlphabet> Default for GraphBuilder<A>
{
    #[inline]
    fn default() -> Self
    {
        Self {
            nodes: Vec::new(),
            index: BTreeMap::new(),
            adjacency: Vec::new(),
        }
    }
}

impl<A: CellAlphabet> GraphBuilder<A>
{
    /// The dense id for `(cell, hole)`, allocating one on first sight.
    #[inline]
    fn intern(
        &mut self,
        cell: CellId,
        var: &A::Var,
    ) -> NodeId
    {
        let key = (cell, A::hole_of(var));
        if let Some(&id) = self.index.get(&key) {
            return id;
        }
        let id = NodeId::from(u32::try_from(self.nodes.len()).unwrap_or(u32::MAX));
        self.nodes.push((cell, var.clone()));
        self.adjacency.push(Vec::new());
        self.index.insert(key, id);
        id
    }

    /// Add a directed edge, deduplicating parallel edges.
    #[inline]
    fn edge(
        &mut self,
        from: NodeId,
        to: NodeId,
    )
    {
        if let Some(row) = usize::try_from(u32::from(from))
            .ok()
            .and_then(|index| self.adjacency.get_mut(index))
            && !row.contains(&to)
        {
            row.push(to);
        }
    }

    /// The finished graph.
    #[inline]
    fn finish(self) -> VarFlowGraph<A>
    {
        VarFlowGraph {
            nodes: self.nodes,
            adjacency: self.adjacency,
        }
    }
}

/// Whether a flow role contributes the **forward** direction.
#[inline]
fn forward_role(role: SeamRole) -> VarianceFlowRole
{
    VarianceFlowRole::from(matches!(role, SeamRole::Forward | SeamRole::Both))
}

/// Whether a flow role contributes the **backward** direction.
#[inline]
fn backward_role(role: SeamRole) -> VarianceFlowRole
{
    VarianceFlowRole::from(matches!(role, SeamRole::Backward | SeamRole::Both))
}

/// The distinct cells a certificate fires, in first-appearance order over
/// `path_a` then `path_b` (deterministic).
///
/// # Contract
/// - ensures: each [`CellId`] in `path_a`/`path_b` once, in first-appearance
///   order.
/// - panics: none.
#[inline]
fn participating_cells<A>(tracelet: &Tracelet<A>) -> Vec<CellId>
where
    A: CellAlphabet,
{
    let mut cells = Vec::new();
    for step in tracelet.path_a.iter().chain(&tracelet.path_b) {
        if !cells.contains(&step.cell) {
            cells.push(step.cell);
        }
    }
    cells
}

/// The distinct hole identities of a term (the seam variables), in
/// first-occurrence order.
///
/// # Contract
/// - ensures: one entry per distinct [`CellAlphabet::Hole`] of `cmd`'s
///   metavariables, preserving first occurrence.
/// - panics: none.
#[inline]
fn seam_holes<A>(cmd: &A::Cmd) -> Vec<A::Hole>
where
    A: CellAlphabet,
{
    let mut holes: Vec<A::Hole> = Vec::new();
    for var in A::metavariables(cmd) {
        let hole = A::hole_of(&var);
        if !holes.contains(&hole) {
            holes.push(hole);
        }
    }
    holes
}

/// The `(cell, hole, role)` endpoints among `cells` whose derived metadata
/// carries `hole`.
///
/// # Contract
/// - ensures: one endpoint per `(cell, matching hole)`, reading the live
///   metadata of each present cell through [`CellAlphabet::hole_flow`]; a stale
///   id contributes nothing.
/// - panics: none.
#[inline]
fn endpoints<A>(
    cells: &[CellId],
    hole: &A::Hole,
    store: &CellStore<A>,
) -> Vec<(CellId, A::Var, SeamRole)>
where
    A: CellAlphabet,
{
    let mut out = Vec::new();
    for &cell in cells {
        let Some(entry) = store.get(cell)
        else {
            continue;
        };
        for (var, role) in A::hole_flow(&entry.meta, hole) {
            out.push((cell, var, role));
        }
    }
    out
}
