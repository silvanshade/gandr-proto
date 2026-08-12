//! **Embedding-based matching, with its convexity check.**
//!
//! The second of the three faces the crate boundary ruling names
//! (`spec:implementation/circuit-terms.md`,
//! `circuit-terms-question-12`).
//!
//! # What this module owns
//!
//! Matching a circuit pattern against a circuit target.
//!
//! The engines' existing one-sided matcher and two-sided unifier are written
//! against a pattern language whose consumer side is a linear spine. A circuit
//! pattern is neither a spine nor a tree: it has several roots, it may
//! reconverge, and it may have components with no wire between them. Matching
//! therefore stops being a structural recursion and becomes a **sub-diagram
//! embedding problem** (§"Matching, normalization, and the crate boundary").
//!
//! An [`Embedding`] is a monomorphism of diagrams: injective on hyperedges,
//! injective on wires, label-preserving, arity-preserving, and incidence- and
//! port-order-preserving. Injectivity on wires is not an extra demand — two
//! pattern wires incident to pattern generators cannot share an image without
//! giving that image wire in-degree or out-degree two, which the target's own
//! monogamy already forbids.
//!
//! # The search shape, and why one seed per component suffices
//!
//! The settled shape is wire-driven propagation with **one nondeterministic
//! seed per connected component of the pattern**, and monogamy is what makes it
//! sufficient rather than merely convenient. Once one pattern generator has an
//! image, each of its wires has one; and a wire has **at most one producer and
//! at most one consumer**, so a pattern wire's assignment forces the image of
//! the pattern generator on either side of it. Propagation from a single seed
//! therefore determines a whole connected component with no further choices,
//! and the only branching left is the choice of seed image per component.
//!
//! A component with no generators at all — a bare wire — is seeded on the
//! target's wires instead, which is the identity pattern's case.
//!
//! # Convexity: the one conjunct that does not decompose
//!
//! Convexity is a condition on the match **as a whole**: no directed path may
//! leave the image and return to it. It cannot be checked while the search
//! descends, so it is checked once per completed candidate, and a candidate it
//! refuses is kept as a [`ConvexityRefusal`] naming the wire the path escapes
//! on, the generator outside the image it runs through, and the wire it
//! re-enters on.
//!
//! Two properties of the check are load-bearing and are stated here because
//! getting either wrong is a soundness gap rather than a slowdown.
//!
//! - **The sweep reads the whole complement.** It walks forward from every
//!   image output through **every** generator of the target that is not in the
//!   image. It is not a test on the image alone, and it is not a test on a
//!   diagram with any edge removed.
//! - **A cut-open verdict never travels.** Cutting a diagram open — severing
//!   delay edges, or any other edge — is by construction *more permissive* than
//!   convexity on the whole: re-closure only ever adds edges, convexity is
//!   violated by the *existence* of a path, so convexity in the re-closed form
//!   implies convexity in the cut-open form and never the converse
//!   (`circuit-terms-spike-08`). This module therefore reports convexity of the
//!   target it was given and of no other, and a re-closed target is refused at
//!   [`Wiring::assemble`] as cyclic rather than inheriting the cut-open
//!   verdict. Matching a re-closed loop is `circuit-terms-rung-09`'s, through
//!   the delay's own path extension.
//!
//! # The guard this module matches behind
//!
//! Convexity is not left to taste here: `circuit-terms-question-15` rules the
//! guard, and `circuit-terms-spike-07` carries its accounting. Two applications
//! commute when their positions are incomparable, their cell pair has trivial
//! overlap, and each match image is still convex in the other's reduct — with
//! the third conjunct **discharged outright, never run**, on a store certified
//! left-connected over an acyclic target.
//!
//! This module supplies the **third conjunct's two routes**, and it supplies
//! the one the engine's own datum records as missing: the engine carries
//! `gandr_theory_computads::alphabet::ConvexityDischarge`, whose
//! `ReCheckRequired` arm names the per-pair convexity re-check and refuses the
//! shift rather than assuming it, because the re-check is not built
//! engine-side. Here it is built, as the sweep.
//!
//! The discharge is **computed, not trusted**. A warrant is not a datum this
//! module accepts from a caller; [`convexity_warrant`] derives it from the
//! pattern, and its content is an argument this crate can give in full:
//!
//! > Let the pattern be strongly connected — a directed path from every input
//! > port to every output port. Suppose a directed path leaves the image at an
//! > image output and returns at an image input.
//! >
//! > **An image input is the image of a declared pattern input port.** Take its
//! > preimage wire. If some pattern generator produced that wire, the generator
//! > is in the image and — an embedding preserving incidence and port order —
//! > it produces the image wire too, so the image wire's producer would be
//! > inside the image and the wire would not be an image input at all. So the
//! > preimage has no producer in the pattern; and [`Wiring::assemble`] refuses
//! > an open wire the interface does not declare, so it is a wire of
//! > `boundary().inputs` — which is what [`connectivity`] quantifies over.
//! > Dually, an image output is the image of a declared pattern output port.
//! > **Boundary honesty is therefore a premise of this argument, alongside
//! > monogamy and acyclicity**, and it is an invariant of [`Wiring`] for the
//! > same reason they are.
//! >
//! > Strong connectedness now gives a directed path from that input to that
//! > output *inside* the pattern, hence inside the image. The two paths compose
//! > to a directed cycle in the target — which the target cannot have, since
//! > [`Wiring::assemble`] refuses a cyclic diagram. So no such path exists and
//! > the match is convex, with no sweep run.
//! >
//! > Two degenerate shapes are covered rather than excluded. The condition
//! > holds
//! > vacuously when either boundary list is empty, and then the image has no
//! > image input to return to, or no image output to leave from, by the two
//! > readings above. And a pattern may be disconnected *as a graph* while still
//! > strongly connected in this sense, when one of its components has no open
//! > port at all; such a component contributes neither an image input nor an
//! > image output, so no escape can begin or end in it.
//!
//! That is the automatic-convexity theorem's argument at gandr's own rung
//! (§"The correspondence at gandr's own rung, at theorem grade", the thm 38
//! reading, whose proof likewise runs through acyclicity of the target). The
//! discharge is audited rather than asserted: [`embeddings_by_sweep`] runs the
//! sweep unconditionally, and the differential between the two routes is a
//! witnessed test rather than a claim.
//!
//! # What this module declines
//!
//! - **Firing, rewriting, and budgets.** Whether a matched cell may fire, and
//!   what happens when it does, are `gandr_theory_computads`'s — the alphabet's
//!   firing discipline and the crate's own rewriting and normalization loops.
//!   Nothing here builds a reduct; a fixture that needs one supplies it.
//! - **Overlap enumeration and completion.** Critical pairs, the multi-sum
//!   overlap families, and the budgeted completion loop stay engine-side over
//!   whatever alphabet the engine is given.
//! - **The downward dependency.** If completion ever consumes this matcher, it
//!   does so through a matcher seam supplied where the engine is instantiated —
//!   never by `theory-computads` depending on this crate. See the crate-root
//!   documentation for why the dependency direction makes that structural.
//!
//! # Status
//!
//! Built at `circuit-terms-rung-05`: embeddings for multi-root, reconvergent,
//! and disconnected patterns, the convexity sweep and its discharge, and the
//! differential against the one-sided matcher over the spine reading of
//! [`crate::interface::spine`]. The boundary complement a rewrite would need is
//! not here and is not this rung's.
//!
//! [`Wiring::assemble`]: crate::interface::Wiring::assemble

use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::interface::ComponentIndex;
use crate::interface::Edge;
use crate::interface::PartialBijection;
use crate::interface::Seam;
use crate::interface::Wire;
use crate::interface::Wiring;

/// How many search steps an embedding search may take.
///
/// A step is one pending assignment resolved — a generator paired with a
/// candidate, or a wire paired with its image. The budget is the honest form of
/// the search's cost: a pattern of free wires against a large target has many
/// candidate assignments, and declining is better than diverging.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MatchBudget(pub usize);

/// How many search steps a search consumed.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SearchSteps(pub usize);

/// How many embeddings a search produced.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MatchCount(pub usize);

/// The **warrant** an embedding's convexity conjunct was granted under.
///
/// The two arms are the guard's two routes for its third conjunct, and the
/// module documentation carries the argument that the first is sound.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ConvexityWarrant
{
    /// Discharged outright and never run: the pattern is strongly connected and
    /// the target is acyclic, so no match of it can fail to be convex.
    LeftConnectedOverAcyclicTarget,
    /// Swept: a directed reachability walk from every image output through
    /// every generator outside the image found no path back to an image input.
    SweptOverTheComplement,
}

/// Whether a pattern is **strongly connected** — a directed path from every
/// input port to every output port.
///
/// This is the property the convexity discharge rests on, and the negative arm
/// carries the unreached pair so a caller can see which port pair costs the
/// discharge rather than only that it was lost.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Connectivity
{
    /// Every input port reaches every output port.
    StronglyConnected,
    /// One input port does not reach one output port.
    Disconnected
    {
        /// The input port that fails to reach.
        from: Wire,
        /// The output port it does not reach.
        to: Wire,
    },
}

/// An **embedding** of a pattern diagram into a target diagram.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Embedding
{
    /// Pattern generator to target generator, addressed by pattern [`Edge`]
    /// position.
    image: Box<[Edge]>,
    /// Pattern wire to target wire — total on the pattern's wires, injective.
    wires: PartialBijection,
    /// The span-level seam datum: the interface's two halves, mapped.
    seam: Seam,
    /// The warrant the convexity conjunct was granted under.
    convexity: ConvexityWarrant,
}

impl Embedding
{
    /// The target generator the pattern generator at `edge` maps to.
    #[inline]
    #[must_use]
    pub fn image_of(
        &self,
        edge: Edge,
    ) -> Option<Edge>
    {
        self.image.get(edge.0).copied()
    }

    /// The image, in pattern generator order.
    #[inline]
    #[must_use]
    pub fn image(&self) -> &[Edge]
    {
        &self.image
    }

    /// The wire map — total on the pattern's wires.
    #[inline]
    #[must_use]
    pub fn wires(&self) -> &PartialBijection
    {
        &self.wires
    }

    /// The span-level seam datum.
    #[inline]
    #[must_use]
    pub fn seam(&self) -> &Seam
    {
        &self.seam
    }

    /// The warrant the convexity conjunct was granted under.
    #[inline]
    #[must_use]
    pub fn convexity(&self) -> ConvexityWarrant
    {
        self.convexity
    }
}

/// A candidate the convexity conjunct **refused**, with the path that refused
/// it.
///
/// Refusal is data, never a silent filter: a pattern that embeds structurally
/// and is not convex is exactly the published blocking shape, and a caller that
/// cannot see it cannot tell a pattern that does not fit from one that fits and
/// is illegal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConvexityRefusal
{
    /// The image the candidate would have had, in pattern generator order.
    image: Box<[Edge]>,
    /// The wire the offending path leaves the image on — an image output.
    escape: Wire,
    /// A generator outside the image the offending path runs through.
    through: Edge,
    /// The wire the offending path re-enters the image on — an image input.
    re_entry: Wire,
}

impl ConvexityRefusal
{
    /// The image the candidate would have had.
    #[inline]
    #[must_use]
    pub fn image(&self) -> &[Edge]
    {
        &self.image
    }

    /// The wire the offending path leaves the image on.
    #[inline]
    #[must_use]
    pub fn escape(&self) -> Wire
    {
        self.escape
    }

    /// A generator outside the image the offending path runs through.
    #[inline]
    #[must_use]
    pub fn through(&self) -> Edge
    {
        self.through
    }

    /// The wire the offending path re-enters the image on.
    #[inline]
    #[must_use]
    pub fn re_entry(&self) -> Wire
    {
        self.re_entry
    }
}

/// What an embedding search found: what it admitted and what it refused.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Matching
{
    /// The embeddings that cleared the convexity conjunct.
    admitted: Vec<Embedding>,
    /// The candidates the convexity conjunct refused.
    refused: Vec<ConvexityRefusal>,
}

impl Matching
{
    /// The embeddings that cleared the convexity conjunct.
    #[inline]
    #[must_use]
    pub fn admitted(&self) -> &[Embedding]
    {
        &self.admitted
    }

    /// The candidates the convexity conjunct refused.
    #[inline]
    #[must_use]
    pub fn refused(&self) -> &[ConvexityRefusal]
    {
        &self.refused
    }

    /// How many embeddings were admitted.
    #[inline]
    #[must_use]
    pub fn admitted_count(&self) -> MatchCount
    {
        MatchCount(self.admitted.len())
    }

    /// How many candidates were refused.
    #[inline]
    #[must_use]
    pub fn refused_count(&self) -> MatchCount
    {
        MatchCount(self.refused.len())
    }
}

/// Why an embedding search was declined outright.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchObstruction
{
    /// The search reached its budget before enumerating every candidate, so the
    /// result would have been a partial enumeration presented as a total one.
    BudgetExhausted
    {
        /// How many steps were taken before the budget ran out.
        consumed: SearchSteps,
    },
}

/// Whether `pattern` is strongly connected.
///
/// # Contract
/// - ensures: [`Connectivity::StronglyConnected`] exactly when every input port
///   reaches every output port by a directed path, which holds vacuously when
///   either list is empty; otherwise the first unreached pair, in port order.
/// - provides: the pattern-side half of the convexity discharge.
/// - panics: none.
/// - intension: the walk is an explicit frontier per input port, so it never
///   recurses on diagram size.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the decision is a reachability quantifier, and
///   its arms are separated by a spine (every input reaches the one output), by
///   a two-output generator whose second output no input-to-first-output path
///   covers, and by a disconnected pair.
/// - witness: `matching::tests::a_spine_pattern_is_strongly_connected`
/// - witness: `matching::tests::a_disconnected_pattern_is_not_strongly_connected`
#[inline]
#[must_use]
pub fn connectivity(pattern: &Wiring) -> Connectivity
{
    for from in pattern.boundary().inputs.iter().copied() {
        let reached = reachable_from(pattern, from);
        for to in pattern.boundary().outputs.iter().copied() {
            if !reached.contains(&to) {
                return Connectivity::Disconnected { from, to };
            }
        }
    }
    Connectivity::StronglyConnected
}

/// The convexity **warrant** `pattern` earns.
///
/// # Contract
/// - ensures: [`ConvexityWarrant::LeftConnectedOverAcyclicTarget`] exactly when
///   `pattern` is strongly connected; otherwise
///   [`ConvexityWarrant::SweptOverTheComplement`].
/// - provides: the guard's third conjunct as a route decision taken once per
///   pattern rather than a datum taken on a caller's word. The target side of
///   the warrant — acyclicity — is not asked for here because it is an
///   invariant of [`Wiring`] rather than a property to sample.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the warrant an embedding carries is validated
///   against the sweep's own verdict on the same inputs, which is what makes
///   the discharge auditable rather than asserted; the differential covers the
///   three shapes on which the discharge is granted for a reason other than
///   one-connected-piece-with-ports-at-both-ends, because those are where the
///   argument for it is most likely to be wrong — a component with no open
///   port, and vacuity on either boundary leg.
/// - witness: `matching::tests::the_discharge_and_the_sweep_agree_where_both_apply`
#[inline]
#[must_use]
pub fn convexity_warrant(pattern: &Wiring) -> ConvexityWarrant
{
    match connectivity(pattern) {
        | Connectivity::StronglyConnected => ConvexityWarrant::LeftConnectedOverAcyclicTarget,
        | Connectivity::Disconnected { .. } => ConvexityWarrant::SweptOverTheComplement,
    }
}

/// **Find every embedding** of `pattern` into `target`, behind the decided
/// guard.
///
/// # Contract
/// - requires: both diagrams are monogamous, boundary-honest and acyclic, which
///   [`Wiring::assemble`] has already established for any value of the type.
///   All three are premises of the discharge, not two.
/// - ensures: every admitted embedding is label-, arity-, incidence- and
///   port-order-preserving, injective on generators and on wires, and convex in
///   `target`; every structurally-complete candidate that is not convex appears
///   as a [`ConvexityRefusal`] rather than being dropped; the enumeration is
///   complete up to the budget.
/// - provides: the matching face of the crate boundary, with the convexity
///   conjunct decided by the route [`convexity_warrant`] selects — discharged
///   for a strongly connected pattern, swept otherwise.
/// - fails: [`MatchObstruction::BudgetExhausted`] when the search would have to
///   exceed `budget`, which is declined rather than truncated because a
///   truncated enumeration presented as a complete one is the error worth
///   avoiding.
/// - panics: none.
/// - intension: components are seeded in pattern generator order and candidates
///   in target generator order, so the enumeration is deterministic; the search
///   is an explicit backtracking stack, so it never recurses on pattern size.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L1 evidence — every admitted embedding is validated against
///   the two diagrams it claims to relate (labels, arities, incidence, and port
///   order re-checked from the wire map), and the convexity arms are separated
///   by the published blocking shape and by its non-blocking twin.
/// - witness: `matching::tests::a_multi_output_pattern_embeds`
/// - witness: `matching::tests::a_multi_root_pattern_embeds`
/// - witness: `matching::tests::a_reconvergent_pattern_embeds`
/// - witness: `matching::tests::a_disconnected_pattern_embeds_component_by_component`
/// - witness: `matching::tests::an_arity_mismatch_under_one_label_is_not_an_embedding`
/// - witness: `matching::tests::port_order_is_preserved_so_a_swapped_target_is_not_a_match`
/// - witness: `matching::tests::two_components_never_claim_one_generator`
/// - witness: `matching::tests::two_port_free_components_never_claim_one_generator`
/// - witness: `matching::tests::a_disconnected_pattern_does_not_match_a_wire_that_joins_it`
/// - witness: `matching::tests::a_cut_open_verdict_does_not_travel_to_the_re_closed_form`
/// - witness: `matching::tests::the_embedding_matcher_agrees_with_the_one_sided_matcher_on_the_spine`
/// - witness: `matching::tests::the_blocking_shape_is_refused_on_the_convexity_conjunct`
/// - witness: `matching::tests::the_sweep_starts_from_every_image_output`
/// - witness: `matching::tests::the_sweep_follows_a_path_through_more_than_one_outside_generator`
/// - witness: `matching::tests::the_same_image_is_admitted_without_the_blocking_generator`
/// - witness: `matching::tests::a_bare_wire_pattern_embeds_on_every_target_wire`
/// - witness: `matching::tests::the_empty_pattern_embeds_exactly_once`
/// - witness: `matching::tests::an_exhausted_budget_declines_rather_than_truncating`
#[inline]
pub fn embeddings(
    pattern: &Wiring,
    target: &Wiring,
    budget: MatchBudget,
) -> Result<Matching, MatchObstruction>
{
    let warrant = convexity_warrant(pattern);
    search(pattern, target, budget, warrant)
}

/// **Find every embedding, always sweeping** — the audit route.
///
/// # Contract
/// - ensures: the same admitted set as [`embeddings`] whenever the pattern is
///   strongly connected, with every embedding carrying
///   [`ConvexityWarrant::SweptOverTheComplement`] instead of the discharge.
/// - provides: the external oracle the discharge is checked against, so
///   "skipped rather than run" is a measured claim rather than a documented
///   one. It is never faster and is not the route a caller should take.
/// - fails: as [`embeddings`].
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L2 agreement — this is the reference the discharge route is
///   differentially compared against, over every fixture where the discharge
///   applies.
/// - witness: `matching::tests::the_discharge_and_the_sweep_agree_where_both_apply`
#[inline]
pub fn embeddings_by_sweep(
    pattern: &Wiring,
    target: &Wiring,
    budget: MatchBudget,
) -> Result<Matching, MatchObstruction>
{
    search(
        pattern,
        target,
        budget,
        ConvexityWarrant::SweptOverTheComplement,
    )
}

/// What a component of the pattern is seeded on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Seed
{
    /// A generator: the candidates are the target's generators.
    Generator(Edge),
    /// A wire incident to no generator: the candidates are the target's wires.
    Wire(Wire),
}

/// One pending assignment the propagation has still to resolve.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Pending
{
    /// A pattern generator paired with its candidate image.
    Generator(Edge, Edge),
    /// A pattern wire paired with its candidate image.
    Wire(Wire, Wire),
}

/// How a propagation ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Extension
{
    /// Every forced assignment was consistent.
    Consistent,
    /// Some forced assignment contradicted one already made, or no such
    /// assignment exists in the target.
    Clash,
    /// The budget ran out mid-propagation.
    Exhausted,
}

/// A partial assignment of the pattern into the target.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Assignment
{
    /// Pattern generator to target generator, by pattern position.
    generators: Vec<Option<Edge>>,
    /// The target generators already claimed, which is what makes the
    /// generator map injective.
    claimed: BTreeSet<Edge>,
    /// Pattern wire to target wire, injective by construction.
    wires: PartialBijection,
}

/// One choice point of the backtracking search.
#[derive(Clone, Debug)]
struct Frame
{
    /// Which component this frame chooses an image for.
    component: usize,
    /// The next candidate index to try for it.
    next: usize,
    /// The assignment as it stood before this component was seeded.
    state: Assignment,
}

/// The wires forward-reachable from `start`, `start` included.
///
/// # Contract
/// - ensures: the reflexive-transitive forward closure of `start` under
///   "consumed by a generator, produced by that generator".
/// - panics: none.
/// - intension: an explicit frontier, so the walk never recurses on diagram
///   size; the diagram's acyclicity is not relied on, because a visited set
///   bounds the walk regardless.
fn reachable_from(
    diagram: &Wiring,
    start: Wire,
) -> BTreeSet<Wire>
{
    let mut seen: BTreeSet<Wire> = BTreeSet::new();
    let mut frontier: Vec<Wire> = alloc::vec![start];
    while let Some(wire) = frontier.pop() {
        if !seen.insert(wire) {
            continue;
        }
        let Some(edge) = diagram.consumer_of(wire)
        else {
            continue;
        };
        let Some(generator) = diagram.generator(edge)
        else {
            continue;
        };
        for next in generator.targets.iter().copied() {
            frontier.push(next);
        }
    }
    seen
}

/// The pattern's connected components, as one seed each.
///
/// # Contract
/// - ensures: one [`Seed::Generator`] per connected component that holds a
///   generator, naming that component's lowest-positioned generator; one
///   [`Seed::Wire`] per wire incident to no generator; and nothing else.
/// - provides: the "one nondeterministic seed per connected component" the
///   settled search shape asks for.
/// - panics: none.
/// - intension: components are discovered in generator position order and their
///   seeds emitted in that order, so the enumeration below is deterministic.
///   The walk itself is [`Wiring::components`]'s, which promises exactly that
///   order; this function reads a representative off each component rather than
///   keeping a second walk that would have to agree with it.
fn seeds_of(pattern: &Wiring) -> Vec<Seed>
{
    let mut seeds: Vec<Seed> = Vec::new();
    let components = pattern.components();
    for index in 0 .. components.count().0 {
        let Some(members) = components.members(ComponentIndex(index))
        else {
            // Unreachable: `index` is below the component count, and
            // `Wiring::components` promises a row for every index below it.
            continue;
        };
        if let Some(first) = members.first().copied() {
            seeds.push(Seed::Generator(first));
        }
    }
    for index in 0 .. pattern.wire_count().0 {
        let wire = Wire(index);
        if pattern.producer_of(wire).is_none() && pattern.consumer_of(wire).is_none() {
            seeds.push(Seed::Wire(wire));
        }
    }
    seeds
}

/// Resolve one pending assignment and everything it forces.
///
/// # Contract
/// - ensures: [`Extension::Consistent`] leaves `state` extended with the seed
///   and with every assignment monogamy forces from it; [`Extension::Clash`]
///   may leave `state` partially extended, which is why the caller works on a
///   clone.
/// - provides: the deterministic half of the search — after the seed there are
///   no choices left inside a component.
/// - panics: none.
/// - intension: a worklist, so propagation never recurses on component size;
///   every resolved item costs one unit of `budget`.
fn extend(
    pattern: &Wiring,
    target: &Wiring,
    state: &mut Assignment,
    seed: Pending,
    budget: &mut MatchBudget,
) -> Extension
{
    let mut queue: Vec<Pending> = alloc::vec![seed];
    while let Some(item) = queue.pop() {
        let Some(remaining) = budget.0.checked_sub(1)
        else {
            return Extension::Exhausted;
        };
        budget.0 = remaining;
        match item {
            | Pending::Generator(source, image) => {
                let Some(slot) = state.generators.get(source.0)
                else {
                    return Extension::Clash;
                };
                if let Some(bound) = *slot {
                    if bound == image {
                        continue;
                    }
                    return Extension::Clash;
                }
                let Some(from) = pattern.generator(source)
                else {
                    return Extension::Clash;
                };
                let Some(onto) = target.generator(image)
                else {
                    return Extension::Clash;
                };
                if from.label != onto.label {
                    return Extension::Clash;
                }
                if from.sources.len() != onto.sources.len() {
                    return Extension::Clash;
                }
                if from.targets.len() != onto.targets.len() {
                    return Extension::Clash;
                }
                if !state.claimed.insert(image) {
                    return Extension::Clash;
                }
                if let Some(slot) = state.generators.get_mut(source.0) {
                    *slot = Some(image);
                }
                for (wire, onto_wire) in from
                    .sources
                    .iter()
                    .copied()
                    .zip(onto.sources.iter().copied())
                {
                    queue.push(Pending::Wire(wire, onto_wire));
                }
                for (wire, onto_wire) in from
                    .targets
                    .iter()
                    .copied()
                    .zip(onto.targets.iter().copied())
                {
                    queue.push(Pending::Wire(wire, onto_wire));
                }
            },
            | Pending::Wire(source, image) => {
                if state.wires.extend(source, image).is_err() {
                    return Extension::Clash;
                }
                if let Some(producer) = pattern.producer_of(source) {
                    let Some(onto) = target.producer_of(image)
                    else {
                        return Extension::Clash;
                    };
                    queue.push(Pending::Generator(producer, onto));
                }
                if let Some(consumer) = pattern.consumer_of(source) {
                    let Some(onto) = target.consumer_of(image)
                    else {
                        return Extension::Clash;
                    };
                    queue.push(Pending::Generator(consumer, onto));
                }
            },
        }
    }
    Extension::Consistent
}

/// The offending path a convexity sweep found, if it found one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Detour
{
    /// The image output the path leaves on.
    escape: Wire,
    /// The generator outside the image the path runs through.
    through: Edge,
    /// The image input the path returns on.
    re_entry: Wire,
}

/// Sweep `target` for a directed path that leaves `image` and returns to it.
///
/// # Contract
/// - requires: `image` is the set of target generators an embedding covers.
/// - ensures: `None` exactly when the image is convex — no directed path
///   between two of its generators leaves it — and otherwise a path's escape
///   wire, the generator outside the image it runs through, and its re-entry
///   wire.
/// - provides: the guard's third conjunct where the discharge does not apply.
/// - panics: none.
/// - intension: an explicit frontier per image output over **every** generator
///   outside the image; nothing is removed from the target before the walk, so
///   the verdict is convexity on the diagram as given and never on a cut-open
///   proxy for it. The walk's guard against stepping into the image is
///   unreachable and kept defensively: a wire the walk reaches through the
///   complement whose consumer is an image generator is a wire the image's own
///   inputs already hold, so it is reported before it can be stepped through —
///   no test separates that guard, deliberately.
fn sweep_for_escape(
    target: &Wiring,
    image: &BTreeSet<Edge>,
) -> Option<Detour>
{
    let mut entries: BTreeSet<Wire> = BTreeSet::new();
    let mut exits: Vec<Wire> = Vec::new();
    for edge in image {
        let Some(generator) = target.generator(*edge)
        else {
            continue;
        };
        for wire in generator.sources.iter().copied() {
            let inside = match target.producer_of(wire) {
                | Some(producer) => image.contains(&producer),
                | None => false,
            };
            if !inside {
                entries.insert(wire);
            }
        }
        for wire in generator.targets.iter().copied() {
            let inside = match target.consumer_of(wire) {
                | Some(consumer) => image.contains(&consumer),
                | None => false,
            };
            if !inside {
                exits.push(wire);
            }
        }
    }
    for start in exits {
        let mut seen: BTreeSet<Wire> = BTreeSet::new();
        let mut frontier: Vec<Wire> = alloc::vec![start];
        while let Some(wire) = frontier.pop() {
            if !seen.insert(wire) {
                continue;
            }
            let Some(edge) = target.consumer_of(wire)
            else {
                continue;
            };
            if image.contains(&edge) {
                continue;
            }
            let Some(generator) = target.generator(edge)
            else {
                continue;
            };
            for next in generator.targets.iter().copied() {
                if entries.contains(&next) {
                    return Some(Detour {
                        escape: start,
                        through: edge,
                        re_entry: next,
                    });
                }
                frontier.push(next);
            }
        }
    }
    None
}

/// The backtracking enumeration, with the convexity route already chosen.
///
/// # Contract
/// - ensures: as [`embeddings`], with `warrant` deciding whether the sweep runs
///   or is discharged.
/// - fails: [`MatchObstruction::BudgetExhausted`].
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
fn search(
    pattern: &Wiring,
    target: &Wiring,
    budget: MatchBudget,
    warrant: ConvexityWarrant,
) -> Result<Matching, MatchObstruction>
{
    let seeds = seeds_of(pattern);
    let empty = Assignment {
        generators: alloc::vec![None; pattern.edge_count().0],
        claimed: BTreeSet::new(),
        wires: PartialBijection::new(),
    };
    let mut remaining = budget;
    let mut completions: Vec<Assignment> = Vec::new();
    if seeds.is_empty() {
        completions.push(empty);
    }
    else {
        let mut frames: Vec<Frame> = alloc::vec![Frame {
            component: 0,
            next: 0,
            state: empty,
        }];
        while !frames.is_empty() {
            let depth = frames.len().saturating_sub(1);
            let Some(frame) = frames.get_mut(depth)
            else {
                break;
            };
            let component = frame.component;
            let candidate = frame.next;
            frame.next = frame.next.saturating_add(1);
            let base = frame.state.clone();
            let Some(seed) = seeds.get(component).copied()
            else {
                frames.pop();
                continue;
            };
            let pending = match seed {
                | Seed::Generator(edge) => {
                    if candidate >= target.edge_count().0 {
                        frames.pop();
                        continue;
                    }
                    Pending::Generator(edge, Edge(candidate))
                },
                | Seed::Wire(wire) => {
                    if candidate >= target.wire_count().0 {
                        frames.pop();
                        continue;
                    }
                    Pending::Wire(wire, Wire(candidate))
                },
            };
            let mut state = base;
            match extend(pattern, target, &mut state, pending, &mut remaining) {
                | Extension::Exhausted => {
                    let consumed = budget.0.saturating_sub(remaining.0);
                    return Err(MatchObstruction::BudgetExhausted {
                        consumed: SearchSteps(consumed),
                    });
                },
                | Extension::Clash => {},
                | Extension::Consistent => {
                    let next = component.saturating_add(1);
                    if next >= seeds.len() {
                        completions.push(state);
                    }
                    else {
                        frames.push(Frame {
                            component: next,
                            next: 0,
                            state,
                        });
                    }
                },
            }
        }
    }
    Ok(admit(pattern, target, completions, warrant))
}

/// Turn complete assignments into admitted embeddings and refusals.
///
/// # Contract
/// - requires: each assignment is structurally complete for `pattern`.
/// - ensures: an assignment whose generator map is not total is dropped (it is
///   not an embedding at all); one whose image is convex — by the discharge or
///   by the sweep — is admitted; one whose image is not is refused with the
///   offending path.
/// - panics: none.
/// - intension: the totality check is unreachable for the search above, because
///   seeding one generator per connected component and propagating along wires
///   assigns every generator of that component. It is kept as the check that
///   would catch a search which stopped doing so, rather than as an outcome the
///   present one can produce — so no test separates it, deliberately.
fn admit(
    pattern: &Wiring,
    target: &Wiring,
    completions: Vec<Assignment>,
    warrant: ConvexityWarrant,
) -> Matching
{
    let mut admitted: Vec<Embedding> = Vec::new();
    let mut refused: Vec<ConvexityRefusal> = Vec::new();
    for completion in completions {
        let mut image: Vec<Edge> = Vec::with_capacity(completion.generators.len());
        let mut total = true;
        for slot in &completion.generators {
            match *slot {
                | Some(edge) => image.push(edge),
                | None => total = false,
            }
        }
        if !total {
            continue;
        }
        let escape = match warrant {
            | ConvexityWarrant::LeftConnectedOverAcyclicTarget => None,
            | ConvexityWarrant::SweptOverTheComplement => {
                let covered: BTreeSet<Edge> = image.iter().copied().collect();
                sweep_for_escape(target, &covered)
            },
        };
        match escape {
            | Some(found) => refused.push(ConvexityRefusal {
                image: image.into_boxed_slice(),
                escape: found.escape,
                through: found.through,
                re_entry: found.re_entry,
            }),
            | None => {
                let seam = seam_of(pattern, &completion.wires);
                admitted.push(Embedding {
                    image: image.into_boxed_slice(),
                    wires: completion.wires,
                    seam,
                    convexity: warrant,
                });
            },
        }
    }
    Matching { admitted, refused }
}

/// The span-level seam datum of one embedding — the interface's two halves,
/// restricted from the wire map.
///
/// # Contract
/// - ensures: each half carries exactly the pattern's ports of that side that
///   the wire map assigns.
/// - panics: none.
fn seam_of(
    pattern: &Wiring,
    wires: &PartialBijection,
) -> Seam
{
    Seam {
        inputs: wires.restricted_to(&pattern.boundary().inputs),
        outputs: wires.restricted_to(&pattern.boundary().outputs),
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_computads::alphabet::CellAlphabet as _;
    use gandr_theory_computads::boundary::SubstitutionDecision;
    use gandr_theory_computads::pattern::CmdPat;
    use gandr_theory_computads::pattern::ConsPat;
    use gandr_theory_computads::pattern::ProdPat;
    use gandr_theory_computads::sequent::SequentAlphabet;
    use gandr_theory_computads::subst::Subst;

    use super::*;
    use crate::interface::Generator;
    use crate::interface::GeneratorLabel;
    use crate::interface::GeneratorSort;
    use crate::interface::Interface;
    use crate::interface::PairCount;
    use crate::interface::WireCount;
    use crate::interface::WiringObstruction;
    use crate::interface::spine::SpineReading;
    use crate::interface::spine::read_spine;

    /// A value-sorted generator label.
    fn value<Name>(name: Name) -> GeneratorLabel
    where
        Name: Into<Box<str>>,
    {
        GeneratorLabel::new(name, GeneratorSort::Value)
    }

    /// A fixture diagram, which the fragment's conditions must accept.
    fn diagram(
        wires: WireCount,
        generators: Vec<Generator>,
        boundary: Interface,
    ) -> Wiring
    {
        Wiring::assemble(wires, generators, boundary)
            .expect("a fixture diagram is monogamous and acyclic")
    }

    /// A pattern paired with the target it is matched against.
    struct Against<Diagram>
    {
        /// The pattern side.
        pattern: Diagram,
        /// The target side.
        target: Diagram,
    }

    /// A generous budget — every fixture here is small enough that exhausting
    /// it would be a defect rather than a cost.
    fn budget() -> MatchBudget
    {
        MatchBudget(10_000)
    }

    /// The images of every admitted embedding, in enumeration order.
    fn images(matching: &Matching) -> Vec<Vec<Edge>>
    {
        let mut images: Vec<Vec<Edge>> = Vec::new();
        for embedding in matching.admitted() {
            images.push(embedding.image().to_vec());
        }
        images
    }

    /// `f: (0) -> (1, 2)`, `g: (1) -> (3)`, `h: (2) -> (4)` — one generator
    /// emitting to two destinations, each read by a different generator.
    fn multi_output_pattern() -> Wiring
    {
        diagram(
            WireCount(5),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1), Wire(2)]),
                Generator::new(value("g"), [Wire(1)], [Wire(3)]),
                Generator::new(value("h"), [Wire(2)], [Wire(4)]),
            ],
            Interface::new([Wire(0)], [Wire(3), Wire(4)]),
        )
    }

    /// The multi-output pattern sitting inside a larger diagram, with a
    /// generator above it and a generator below one of its outputs.
    fn multi_output_target() -> Wiring
    {
        diagram(
            WireCount(7),
            alloc::vec![
                Generator::new(value("e"), [Wire(0)], [Wire(1)]),
                Generator::new(value("f"), [Wire(1)], [Wire(2), Wire(3)]),
                Generator::new(value("g"), [Wire(2)], [Wire(4)]),
                Generator::new(value("h"), [Wire(3)], [Wire(5)]),
                Generator::new(value("k"), [Wire(4)], [Wire(6)]),
            ],
            Interface::new([Wire(0)], [Wire(5), Wire(6)]),
        )
    }

    /// `f: (0) -> (2)`, `g: (1) -> (3)`, `h: (2, 3) -> (4)` — two roots, so the
    /// pattern is not a spine and has no single place a structural recursion
    /// could start from.
    fn multi_root_pattern() -> Wiring
    {
        diagram(
            WireCount(5),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(2)]),
                Generator::new(value("g"), [Wire(1)], [Wire(3)]),
                Generator::new(value("h"), [Wire(2), Wire(3)], [Wire(4)]),
            ],
            Interface::new([Wire(0), Wire(1)], [Wire(4)]),
        )
    }

    /// The multi-root pattern with a generator above each of its roots and one
    /// below its output.
    fn multi_root_target() -> Wiring
    {
        diagram(
            WireCount(8),
            alloc::vec![
                Generator::new(value("e0"), [Wire(0)], [Wire(1)]),
                Generator::new(value("e1"), [Wire(2)], [Wire(3)]),
                Generator::new(value("f"), [Wire(1)], [Wire(4)]),
                Generator::new(value("g"), [Wire(3)], [Wire(5)]),
                Generator::new(value("h"), [Wire(4), Wire(5)], [Wire(6)]),
                Generator::new(value("k"), [Wire(6)], [Wire(7)]),
            ],
            Interface::new([Wire(0), Wire(2)], [Wire(7)]),
        )
    }

    /// `f: (0) -> (1, 2)`, `g: (1, 2) -> (3)` — two paths out of one generator
    /// rejoining at another.
    fn reconvergent_pattern() -> Wiring
    {
        diagram(
            WireCount(4),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1), Wire(2)]),
                Generator::new(value("g"), [Wire(1), Wire(2)], [Wire(3)]),
            ],
            Interface::new([Wire(0)], [Wire(3)]),
        )
    }

    /// The reconvergent pattern inside a larger diagram.
    fn reconvergent_target() -> Wiring
    {
        diagram(
            WireCount(6),
            alloc::vec![
                Generator::new(value("e"), [Wire(0)], [Wire(1)]),
                Generator::new(value("f"), [Wire(1)], [Wire(2), Wire(3)]),
                Generator::new(value("g"), [Wire(2), Wire(3)], [Wire(4)]),
                Generator::new(value("k"), [Wire(4)], [Wire(5)]),
            ],
            Interface::new([Wire(0)], [Wire(5)]),
        )
    }

    /// `a: (0) -> (1)` beside `b: (2) -> (3)`, with no wire between them.
    fn disconnected_pattern() -> Wiring
    {
        diagram(
            WireCount(4),
            alloc::vec![
                Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                Generator::new(value("b"), [Wire(2)], [Wire(3)]),
            ],
            Interface::new([Wire(0), Wire(2)], [Wire(1), Wire(3)]),
        )
    }

    #[test]
    fn a_spine_pattern_is_strongly_connected()
    {
        let spine = diagram(
            WireCount(3),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], [Wire(2)]),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        );
        assert_eq!(
            Connectivity::StronglyConnected,
            connectivity(&spine),
            "the one input reaches the one output"
        );
        assert_eq!(
            ConvexityWarrant::LeftConnectedOverAcyclicTarget,
            convexity_warrant(&spine),
            "so its matches are convex without a sweep"
        );
        let bare = diagram(
            WireCount(1),
            Vec::new(),
            Interface::new([Wire(0)], [Wire(0)]),
        );
        assert_eq!(
            Connectivity::StronglyConnected,
            connectivity(&bare),
            "and a bare wire reaches itself, so the identity pattern keeps the discharge"
        );
    }

    #[test]
    fn an_arity_mismatch_under_one_label_is_not_an_embedding()
    {
        // One label at two arities is two generators. Both directions matter,
        // because a check on only one of them would let the other's port list
        // be silently truncated to the shorter of the two.
        let one_out = diagram(
            WireCount(2),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(1)])],
            Interface::new([Wire(0)], [Wire(1)]),
        );
        let two_out = diagram(
            WireCount(3),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(1), Wire(2)])],
            Interface::new([Wire(0)], [Wire(1), Wire(2)]),
        );
        let two_in = diagram(
            WireCount(3),
            alloc::vec![Generator::new(value("f"), [Wire(0), Wire(1)], [Wire(2)])],
            Interface::new([Wire(0), Wire(1)], [Wire(2)]),
        );
        assert_eq!(
            MatchCount(0),
            embeddings(&one_out, &two_out, budget())
                .expect("the search fits the budget")
                .admitted_count(),
            "a one-output pattern does not embed into a two-output generator of that name"
        );
        assert_eq!(
            MatchCount(0),
            embeddings(&two_out, &one_out, budget())
                .expect("the search fits the budget")
                .admitted_count(),
            "nor the other way round"
        );
        assert_eq!(
            MatchCount(0),
            embeddings(&one_out, &two_in, budget())
                .expect("the search fits the budget")
                .admitted_count(),
            "and the source side is checked as well as the target side"
        );
    }

    #[test]
    fn a_disconnected_pattern_is_not_strongly_connected()
    {
        assert_eq!(
            Connectivity::Disconnected {
                from: Wire(0),
                to: Wire(3)
            },
            connectivity(&disconnected_pattern()),
            "the first component's input reaches nothing in the second"
        );
        assert_eq!(
            ConvexityWarrant::SweptOverTheComplement,
            convexity_warrant(&disconnected_pattern()),
            "so the discharge is lost and the sweep is what carries"
        );
    }

    #[test]
    fn a_multi_output_pattern_embeds()
    {
        let matching = embeddings(&multi_output_pattern(), &multi_output_target(), budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(1),
            matching.admitted_count(),
            "the multi-output pattern occurs exactly once in the target"
        );
        let embedding = matching
            .admitted()
            .first()
            .expect("the admitted embedding is there");
        assert_eq!(
            alloc::vec![Edge(1), Edge(2), Edge(3)],
            embedding.image().to_vec(),
            "and it covers the target's f, g and h, in pattern order"
        );
        assert_eq!(
            Some(Wire(1)),
            embedding.wires().image_of(Wire(0)),
            "the pattern's input port lands on the wire below the generator above it"
        );
    }

    #[test]
    fn a_multi_root_pattern_embeds()
    {
        let matching = embeddings(&multi_root_pattern(), &multi_root_target(), budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(1),
            matching.admitted_count(),
            "two roots are seeded from one component and propagated, not recursed into"
        );
        let embedding = matching
            .admitted()
            .first()
            .expect("the admitted embedding is there");
        assert_eq!(
            alloc::vec![Edge(2), Edge(3), Edge(4)],
            embedding.image().to_vec(),
            "and the image is the pattern's occurrence, not the context around it"
        );
        assert_eq!(
            alloc::vec![(Wire(0), Wire(1)), (Wire(1), Wire(3))],
            embedding.seam().inputs.pairs(),
            "both roots' input ports land on the wires below the generators above them"
        );
    }

    #[test]
    fn a_reconvergent_pattern_embeds()
    {
        let matching = embeddings(&reconvergent_pattern(), &reconvergent_target(), budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(1),
            matching.admitted_count(),
            "reconvergence is matched, not refused"
        );
        let embedding = matching
            .admitted()
            .first()
            .expect("the admitted embedding is there");
        assert_eq!(
            alloc::vec![Edge(1), Edge(2)],
            embedding.image().to_vec(),
            "and the two rejoining wires are matched as one pair, not two"
        );
    }

    #[test]
    fn a_disconnected_pattern_embeds_component_by_component()
    {
        // Two components, two seeds: the first can land on either of the
        // target's two `a` generators, which is exactly the branching the
        // per-component seed exists to enumerate.
        let target = diagram(
            WireCount(6),
            alloc::vec![
                Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                Generator::new(value("a"), [Wire(2)], [Wire(3)]),
                Generator::new(value("b"), [Wire(4)], [Wire(5)]),
            ],
            Interface::new(alloc::vec![Wire(0), Wire(2), Wire(4)], alloc::vec![
                Wire(1),
                Wire(3),
                Wire(5)
            ]),
        );
        let matching = embeddings(&disconnected_pattern(), &target, budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(2),
            matching.admitted_count(),
            "one embedding per choice of image for the first component"
        );
        assert_eq!(
            alloc::vec![alloc::vec![Edge(0), Edge(2)], alloc::vec![Edge(1), Edge(2)]],
            images(&matching),
            "and the enumeration is deterministic, in target generator order"
        );
    }

    #[test]
    fn a_disconnected_pattern_does_not_match_a_wire_that_joins_it()
    {
        // The pattern asks for two generators with NO wire between them. A
        // target whose two generators share a wire would need the match to
        // identify two pattern wires, which a monomorphism may not do — and
        // which would give the shared wire in-degree or out-degree two anyway.
        let joined = diagram(
            WireCount(3),
            alloc::vec![
                Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                Generator::new(value("b"), [Wire(1)], [Wire(2)]),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        );
        let matching = embeddings(&disconnected_pattern(), &joined, budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(0),
            matching.admitted_count(),
            "identifying the two components' wires is not an embedding"
        );
        assert_eq!(
            MatchCount(0),
            matching.refused_count(),
            "and it is refused as no candidate at all, not as a convexity failure"
        );
    }

    #[test]
    fn two_components_never_claim_one_generator()
    {
        // Both components carry the label `a`, and the target holds one `a`.
        let pattern = diagram(
            WireCount(4),
            alloc::vec![
                Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                Generator::new(value("a"), [Wire(2)], [Wire(3)]),
            ],
            Interface::new([Wire(0), Wire(2)], [Wire(1), Wire(3)]),
        );
        let target = diagram(
            WireCount(2),
            alloc::vec![Generator::new(value("a"), [Wire(0)], [Wire(1)])],
            Interface::new([Wire(0)], [Wire(1)]),
        );
        let matching = embeddings(&pattern, &target, budget()).expect("the search fits the budget");
        assert_eq!(
            MatchCount(0),
            matching.admitted_count(),
            "an embedding is injective on generators, so one target generator serves one component"
        );
    }

    #[test]
    fn two_port_free_components_never_claim_one_generator()
    {
        // Injectivity on generators is implied by injectivity on wires wherever
        // a generator HAS a port: two pattern generators sharing an image would
        // identify their ports. A port-free generator is the one case where the
        // generator-level check carries alone, so it is the case that separates
        // it.
        let pattern = diagram(
            WireCount(0),
            alloc::vec![
                Generator::new(value("s"), Vec::new(), Vec::new()),
                Generator::new(value("s"), Vec::new(), Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        );
        let one = diagram(
            WireCount(0),
            alloc::vec![Generator::new(value("s"), Vec::new(), Vec::new())],
            Interface::new(Vec::new(), Vec::new()),
        );
        assert_eq!(
            MatchCount(0),
            embeddings(&pattern, &one, budget())
                .expect("the search fits the budget")
                .admitted_count(),
            "one target generator cannot serve two pattern generators"
        );
        let two = diagram(
            WireCount(0),
            alloc::vec![
                Generator::new(value("s"), Vec::new(), Vec::new()),
                Generator::new(value("s"), Vec::new(), Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        );
        assert_eq!(
            MatchCount(2),
            embeddings(&pattern, &two, budget())
                .expect("the search fits the budget")
                .admitted_count(),
            "and with two of them both orderings are embeddings"
        );
    }

    #[test]
    fn port_order_is_preserved_so_a_swapped_target_is_not_a_match()
    {
        // The target has the same generators and the same wires as the
        // reconvergent pattern's occurrence, with `g`'s two sources swapped.
        let swapped = diagram(
            WireCount(4),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1), Wire(2)]),
                Generator::new(value("g"), [Wire(2), Wire(1)], [Wire(3)]),
            ],
            Interface::new([Wire(0)], [Wire(3)]),
        );
        let matching = embeddings(&reconvergent_pattern(), &swapped, budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(0),
            matching.admitted_count(),
            "within-generator port order is carried, so the swap is a different diagram"
        );
    }

    #[test]
    fn the_blocking_shape_is_refused_on_the_convexity_conjunct()
    {
        // The published blocking shape: two applications on DISJOINT generator
        // sets that still interfere, because a directed path runs out of one
        // image and back into the other through a generator neither covers.
        // Disjointness of supports is not independence once matches must be
        // convex, which is why the guard's third conjunct exists at all.
        let blocked = diagram(
            WireCount(4),
            alloc::vec![
                Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                Generator::new(value("c"), [Wire(1)], [Wire(2)]),
                Generator::new(value("b"), [Wire(2)], [Wire(3)]),
            ],
            Interface::new([Wire(0)], [Wire(3)]),
        );
        let matching = embeddings(&disconnected_pattern(), &blocked, budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(0),
            matching.admitted_count(),
            "the candidate embeds structurally and is still not a legal match"
        );
        assert_eq!(
            MatchCount(1),
            matching.refused_count(),
            "and it is refused rather than silently dropped"
        );
        let refusal = matching.refused().first().expect("the refusal is recorded");
        assert_eq!(
            alloc::vec![Edge(0), Edge(2)],
            refusal.image().to_vec(),
            "the refusal names the image it would have had"
        );
        assert_eq!(
            Wire(1),
            refusal.escape(),
            "the wire the path leaves the image on"
        );
        assert_eq!(
            Edge(1),
            refusal.through(),
            "the generator outside the image it runs through"
        );
        assert_eq!(
            Wire(2),
            refusal.re_entry(),
            "and the wire it re-enters the image on"
        );
    }

    #[test]
    fn the_sweep_starts_from_every_image_output()
    {
        // The blocking shape again, with the target's generators listed in a
        // different order so the escape runs from the image's SECOND output
        // rather than its first: the image is {Edge(0), Edge(2)} and the exits
        // are read in image order, so `a`'s dead-end output is tried before
        // `b`'s escaping one. A sweep that stopped after one image output would
        // admit this candidate — and admitting a non-convex match is the
        // unsound direction, not the slow one.
        let blocked_late = diagram(
            WireCount(4),
            alloc::vec![
                Generator::new(value("a"), [Wire(2)], [Wire(3)]),
                Generator::new(value("c"), [Wire(1)], [Wire(2)]),
                Generator::new(value("b"), [Wire(0)], [Wire(1)]),
            ],
            Interface::new([Wire(0)], [Wire(3)]),
        );
        let matching = embeddings(&disconnected_pattern(), &blocked_late, budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(0),
            matching.admitted_count(),
            "the escape is found although it leaves from the later image output"
        );
        assert_eq!(
            MatchCount(1),
            matching.refused_count(),
            "and the candidate is refused rather than admitted"
        );
        let refusal = matching.refused().first().expect("the refusal is recorded");
        assert_eq!(
            Wire(1),
            refusal.escape(),
            "the escape wire is the second image output, not the first"
        );
        assert_eq!(
            Edge(1),
            refusal.through(),
            "running through the generator neither component covers"
        );
        assert_eq!(
            Wire(2),
            refusal.re_entry(),
            "and re-entering on the other component's input"
        );
    }

    #[test]
    fn the_sweep_follows_a_path_through_more_than_one_outside_generator()
    {
        // Two intervening generators instead of one. The intermediate wire is
        // not an image input, so a walk that only looked at the direct
        // successors of the escape wire's consumer would find nothing and admit
        // the candidate. Convexity is violated by the existence of a path of
        // any length, so the walk has to be transitive.
        let blocked_deep = diagram(
            WireCount(5),
            alloc::vec![
                Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                Generator::new(value("c0"), [Wire(1)], [Wire(2)]),
                Generator::new(value("c1"), [Wire(2)], [Wire(3)]),
                Generator::new(value("b"), [Wire(3)], [Wire(4)]),
            ],
            Interface::new([Wire(0)], [Wire(4)]),
        );
        let matching = embeddings(&disconnected_pattern(), &blocked_deep, budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(0),
            matching.admitted_count(),
            "a two-generator detour is still a detour"
        );
        let refusal = matching.refused().first().expect("the refusal is recorded");
        assert_eq!(
            Wire(1),
            refusal.escape(),
            "the path leaves on the first component's output"
        );
        assert_eq!(
            Edge(2),
            refusal.through(),
            "and the generator named is the LAST one outside the image it runs \
             through, which is what shows the walk was transitive"
        );
        assert_eq!(
            Wire(3),
            refusal.re_entry(),
            "re-entering on the second component's input"
        );
    }

    #[test]
    fn a_bare_wire_pattern_embeds_on_every_target_wire()
    {
        // The identity pattern's case, which the module documents as seeded on
        // the target's WIRES rather than on its generators. Every wire of the
        // target is one embedding and nothing else is, so the count pins both
        // completeness and the range: an image outside `wire_count` would be an
        // embedding into a wire the target does not have.
        let bare = diagram(
            WireCount(1),
            Vec::new(),
            Interface::new([Wire(0)], [Wire(0)]),
        );
        let target = multi_output_target();
        let matching = embeddings(&bare, &target, budget()).expect("the search fits the budget");
        assert_eq!(
            MatchCount(target.wire_count().0),
            matching.admitted_count(),
            "one embedding per target wire"
        );
        let mut landed: Vec<Wire> = Vec::new();
        for embedding in matching.admitted() {
            let image = embedding
                .wires()
                .image_of(Wire(0))
                .expect("the bare wire is mapped");
            assert!(
                image.0 < target.wire_count().0,
                "and never onto a wire the target does not declare"
            );
            assert!(
                embedding.image().is_empty(),
                "the generator image is empty, so convexity is vacuous"
            );
            landed.push(image);
        }
        assert_eq!(
            alloc::vec![
                Wire(0),
                Wire(1),
                Wire(2),
                Wire(3),
                Wire(4),
                Wire(5),
                Wire(6)
            ],
            landed,
            "in target wire order, which is the declared enumeration order"
        );
    }

    #[test]
    fn the_empty_pattern_embeds_exactly_once()
    {
        // No generators and no wires: the empty map is the one embedding, and
        // it is admitted rather than dropped for having nothing in it.
        let empty = diagram(
            WireCount(0),
            Vec::new(),
            Interface::new(Vec::new(), Vec::new()),
        );
        let matching = embeddings(&empty, &multi_output_target(), budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(1),
            matching.admitted_count(),
            "the empty pattern embeds once, by the empty map"
        );
        let embedding = matching
            .admitted()
            .first()
            .expect("the admitted embedding is there");
        assert!(embedding.image().is_empty(), "with an empty image");
        assert_eq!(
            PairCount(0),
            embedding.wires().pair_count(),
            "and an empty wire map"
        );
    }

    #[test]
    fn the_same_image_is_admitted_without_the_blocking_generator()
    {
        // The twin of the blocking fixture, differing in exactly one generator.
        // The verdict turns on a generator OUTSIDE the image, which is what
        // makes the check a sweep over the whole complement rather than a
        // property of the image: a test that looked only at the image, or at a
        // diagram with the complement cut away, would admit both.
        let unblocked = diagram(
            WireCount(4),
            alloc::vec![
                Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                Generator::new(value("b"), [Wire(2)], [Wire(3)]),
            ],
            Interface::new([Wire(0), Wire(2)], [Wire(1), Wire(3)]),
        );
        let matching = embeddings(&disconnected_pattern(), &unblocked, budget())
            .expect("the search fits the budget");
        assert_eq!(
            MatchCount(1),
            matching.admitted_count(),
            "with the intervening generator gone the same image is convex"
        );
        assert_eq!(
            MatchCount(0),
            matching.refused_count(),
            "and nothing is refused"
        );
        let embedding = matching
            .admitted()
            .first()
            .expect("the admitted embedding is there");
        assert_eq!(
            ConvexityWarrant::SweptOverTheComplement,
            embedding.convexity(),
            "the pattern is not strongly connected, so the sweep is what granted it"
        );
    }

    #[test]
    fn a_cut_open_verdict_does_not_travel_to_the_re_closed_form()
    {
        // The recorded two-cell witness: a body with one delayed back-edge,
        // cut open. `f` runs from the cut end `d⁻` to an internal wire, and `g`
        // takes the body input and that wire to the body output and the cut end
        // `d⁺`. Cut open, the match whose image is f together with g is convex
        // — there is no third generator for a path to leave through.
        let cut_open = diagram(
            WireCount(5),
            alloc::vec![
                Generator::new(value("f"), [Wire(1)], [Wire(2)]),
                Generator::new(value("g"), [Wire(0), Wire(2)], [Wire(3), Wire(4)]),
            ],
            Interface::new([Wire(0), Wire(1)], [Wire(3), Wire(4)]),
        );
        let pattern = diagram(
            WireCount(5),
            alloc::vec![
                Generator::new(value("f"), [Wire(1)], [Wire(2)]),
                Generator::new(value("g"), [Wire(0), Wire(2)], [Wire(3), Wire(4)]),
            ],
            Interface::new([Wire(0), Wire(1)], [Wire(3), Wire(4)]),
        );
        let matching =
            embeddings_by_sweep(&pattern, &cut_open, budget()).expect("the search fits the budget");
        assert_eq!(
            MatchCount(1),
            matching.admitted_count(),
            "the cut-open form admits the match and the sweep finds no escape"
        );

        // Re-closing the delay adds the edge from d⁺ back to d⁻. That edge runs
        // from an OUTPUT of the image to an INPUT of it, so the same image is
        // not convex there — and the cut-open verdict above says nothing about
        // it, because re-closure only ever adds edges and convexity is violated
        // by the existence of a path. The re-closed form is refused as a
        // matching target outright rather than inheriting the verdict.
        let re_closed = Wiring::assemble(
            WireCount(5),
            alloc::vec![
                Generator::new(value("f"), [Wire(1)], [Wire(2)]),
                Generator::new(value("g"), [Wire(0), Wire(2)], [Wire(3), Wire(4)]),
                Generator::new(value("delay"), [Wire(4)], [Wire(1)]),
            ],
            Interface::new([Wire(0)], [Wire(3)]),
        );
        let refusal = re_closed.expect_err("re-closing the delay leaves the acyclic fragment");
        assert_eq!(
            WiringObstruction::DirectedCycle { through: Edge(0) },
            refusal,
            "so the cut-open form is the only legal matching target, as ruled"
        );
    }

    #[test]
    fn the_discharge_and_the_sweep_agree_where_both_apply()
    {
        // The discharge is "skipped rather than run", which is only honest if
        // running it would have agreed. This is that measurement, over every
        // strongly connected fixture here, against the sweep as the external
        // oracle.
        let pairs = alloc::vec![
            Against {
                pattern: multi_output_pattern(),
                target: multi_output_target(),
            },
            Against {
                pattern: reconvergent_pattern(),
                target: reconvergent_target(),
            },
            Against {
                pattern: multi_output_pattern(),
                target: reconvergent_target(),
            },
            Against {
                pattern: reconvergent_pattern(),
                target: multi_output_target(),
            },
            Against {
                pattern: multi_root_pattern(),
                target: multi_root_target(),
            },
            // The three shapes on which the discharge is granted for a reason
            // OTHER than "the pattern is one connected piece with ports at both
            // ends" — which is where an argument about the discharge is most
            // likely to be wrong, and where the sweep is therefore worth the
            // most as an oracle.
            //
            // (i) Strongly connected by its declared boundary while being two
            // components as a graph: `s → t` has no open port at all, so it
            // contributes neither an image input nor an image output.
            Against {
                pattern: diagram(
                    WireCount(3),
                    alloc::vec![
                        Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                        Generator::new(value("s"), Vec::new(), [Wire(2)]),
                        Generator::new(value("t"), [Wire(2)], Vec::new()),
                    ],
                    Interface::new([Wire(0)], [Wire(1)]),
                ),
                target: diagram(
                    WireCount(5),
                    alloc::vec![
                        Generator::new(value("e"), [Wire(0)], [Wire(1)]),
                        Generator::new(value("a"), [Wire(1)], [Wire(2)]),
                        Generator::new(value("s"), Vec::new(), [Wire(3)]),
                        Generator::new(value("t"), [Wire(3)], Vec::new()),
                        Generator::new(value("k"), [Wire(2)], [Wire(4)]),
                    ],
                    Interface::new([Wire(0)], [Wire(4)]),
                ),
            },
            // (ii) Vacuous on the output side: a pattern with no open output has
            // no image output for a path to leave from.
            Against {
                pattern: diagram(
                    WireCount(1),
                    alloc::vec![Generator::new(value("t"), [Wire(0)], Vec::new())],
                    Interface::new([Wire(0)], Vec::new()),
                ),
                target: diagram(
                    WireCount(2),
                    alloc::vec![
                        Generator::new(value("e"), [Wire(0)], [Wire(1)]),
                        Generator::new(value("t"), [Wire(1)], Vec::new()),
                    ],
                    Interface::new([Wire(0)], Vec::new()),
                ),
            },
            // (iii) Vacuous on the input side: a pattern with no open input has
            // no image input for a path to return to.
            Against {
                pattern: diagram(
                    WireCount(1),
                    alloc::vec![Generator::new(value("s"), Vec::new(), [Wire(0)])],
                    Interface::new(Vec::new(), [Wire(0)]),
                ),
                target: diagram(
                    WireCount(2),
                    alloc::vec![
                        Generator::new(value("s"), Vec::new(), [Wire(0)]),
                        Generator::new(value("k"), [Wire(0)], [Wire(1)]),
                    ],
                    Interface::new(Vec::new(), [Wire(1)]),
                ),
            },
        ];
        let mut total: usize = 0;
        for pair in &pairs {
            let pattern = &pair.pattern;
            let target = &pair.target;
            assert_eq!(
                Connectivity::StronglyConnected,
                connectivity(pattern),
                "the discharge only applies to a strongly connected pattern"
            );
            let discharged =
                embeddings(pattern, target, budget()).expect("the search fits the budget");
            let swept = embeddings_by_sweep(pattern, target, budget())
                .expect("the audit search fits the budget");
            total = total.saturating_add(discharged.admitted_count().0);
            assert_eq!(
                images(&swept),
                images(&discharged),
                "the discharged route admits exactly what the sweep admits"
            );
            assert_eq!(
                MatchCount(0),
                swept.refused_count(),
                "and the sweep refuses nothing the discharge silently kept"
            );
            for embedding in discharged.admitted() {
                assert_eq!(
                    ConvexityWarrant::LeftConnectedOverAcyclicTarget,
                    embedding.convexity(),
                    "the discharged route records which warrant it was granted under"
                );
            }
            for embedding in swept.admitted() {
                assert_eq!(
                    ConvexityWarrant::SweptOverTheComplement,
                    embedding.convexity(),
                    "and the audit route records that it ran the sweep"
                );
            }
        }
        assert_eq!(
            6_usize, total,
            "the comparison is not vacuous: six of the pairs each contribute one \
             occurrence, and the two crossed pairs agree on finding none"
        );
    }

    #[test]
    fn an_embedding_carries_its_seam_as_a_pair_of_partial_bijections()
    {
        let matching = embeddings(&reconvergent_pattern(), &reconvergent_target(), budget())
            .expect("the search fits the budget");
        let embedding = matching
            .admitted()
            .first()
            .expect("the admitted embedding is there");
        assert_eq!(
            alloc::vec![(Wire(0), Wire(1))],
            embedding.seam().inputs.pairs(),
            "the input half maps the pattern's one input port"
        );
        assert_eq!(
            alloc::vec![(Wire(3), Wire(4))],
            embedding.seam().outputs.pairs(),
            "and the output half its one output port, kept apart from the input half"
        );
    }

    #[test]
    fn an_exhausted_budget_declines_rather_than_truncating()
    {
        let refusal = embeddings(
            &reconvergent_pattern(),
            &reconvergent_target(),
            MatchBudget(2),
        )
        .expect_err("two steps cannot enumerate this search");
        let MatchObstruction::BudgetExhausted { consumed } = refusal;
        assert_eq!(
            SearchSteps(2),
            consumed,
            "the decline names what it spent rather than returning a partial enumeration"
        );
    }

    /// The differential fragment's anchoring condition: a one-sided term match
    /// is exactly an embedding that sends the pattern's cut wire to the
    /// target's, so the two matchers decide the same question only there.
    fn anchored_decision(
        pattern: &SpineReading,
        target: &SpineReading,
    ) -> SubstitutionDecision
    {
        let matching = embeddings(pattern.wiring(), target.wiring(), budget())
            .expect("the search fits the budget");
        for embedding in matching.admitted() {
            if embedding.wires().image_of(pattern.cut()) == Some(target.cut()) {
                return SubstitutionDecision::from(true);
            }
        }
        SubstitutionDecision::from(false)
    }

    #[test]
    fn the_embedding_matcher_agrees_with_the_one_sided_matcher_on_the_spine()
    {
        let rows = alloc::vec![
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Succ", [ProdPat::meta("m")]),
                    ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
                    ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Succ", [ProdPat::meta("m")]),
                    ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::meta("m"),
                    ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
                    ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::meta("m"),
                    ConsPat::meta("alpha"),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
                    ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::meta("alpha"),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
                    ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(Polarity::Positive, ProdPat::meta("m"), ConsPat::Top),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::meta("m"),
                    ConsPat::frame("Succ", ConsPat::meta("alpha")),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::frame("Succ", ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::meta("m"),
                    ConsPat::frame("Succ", ConsPat::meta("alpha")),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Succ", [ProdPat::meta("m")]),
                    ConsPat::meta("alpha"),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::frame("Succ", ConsPat::Top),
                ),
            },
            // The last two rows are the ones that pin the label's SORT. A
            // return-side frame `K⁻(c)` and a nullary operation frame `f(; c)`
            // wear one name at one arity and are different generators; a reading
            // that labelled by name alone would match each against the other,
            // anchored, while the one-sided matcher refuses both.
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::meta("m"),
                    ConsPat::frame("Succ", ConsPat::meta("alpha")),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::op("Succ", [], ConsPat::Top),
                ),
            },
            Against {
                pattern: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::meta("m"),
                    ConsPat::op("Succ", [], ConsPat::meta("alpha")),
                ),
                target: CmdPat::cut(
                    Polarity::Positive,
                    ProdPat::ctor("Zero", []),
                    ConsPat::frame("Succ", ConsPat::Top),
                ),
            },
        ];
        let mut agreed: usize = 0;
        for row in &rows {
            let pattern = &row.pattern;
            let target = &row.target;
            let mut subst = Subst::default();
            let by_term = SequentAlphabet::match_cmd(pattern, target, &mut subst);
            let pattern = read_spine(pattern).expect("the fixture pattern is linear");
            let target = read_spine(target).expect("the fixture target is linear");
            let by_embedding = anchored_decision(&pattern, &target);
            assert_eq!(
                by_term, by_embedding,
                "the cut-anchored embedding decides what the one-sided matcher decides"
            );
            if bool::from(by_term) {
                agreed = agreed.saturating_add(1);
            }
        }
        assert_eq!(
            12_usize,
            rows.len(),
            "the table is the one this differential was written against"
        );
        assert_eq!(
            5_usize, agreed,
            "and it separates both verdicts rather than agreeing only on refusals"
        );
    }
}
