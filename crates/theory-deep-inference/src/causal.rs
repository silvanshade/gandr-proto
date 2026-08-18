//! The **finite event partial order** of a recorded derivation — the causal
//! structure the tracelet normal form quotients by, surfaced as data rather
//! than computed and thrown away.
//!
//! A derivation that has survived unit elimination is a finite sequence of
//! **events**: one cell application each, in the order they were recorded. Two
//! events are *dependent* when the crate's single independence relation
//! ([`crate::shift`]'s guard) refuses to commute them, and the **causal order**
//! is the transitive closure of "earlier in the recording and dependent". That
//! order is a strict partial order on a finite set — the Mazurkiewicz
//! dependence order of the trace, and the object the canonical schedule is a
//! linear extension of.
//!
//! # Why this is a type and not a local variable
//!
//! [`crate::normal_form`] needs one linear extension of this order and nothing
//! else, so it used to compute the depths inline and flatten them into the
//! schedule. Flattening loses the parallel structure: a schedule says which
//! order to fire in, and cannot say which steps could have fired *together*.
//! Every consumer of that structure — a parallel replay plan, a critical-path
//! cost, a rendering of a derivation as a poset rather than a list — has to
//! rebuild the whole relation to get it back.
//!
//! So the order is built once, as [`EventOrder`], and the normal form's
//! schedule is one of its projections ([`EventOrder::canonical_order`]) beside
//! the layer grouping ([`EventOrder::layers`]).
//!
//! # What this module does **not** decide
//!
//! It does not decide certificate identity, and it is not an equality.
//! Everything here reads the *recorded steps* of one derivation, and
//! [`gandr_theory_coherent_resolutions::tracelet::replay_equivalent`] — the
//! semantic oracle — reads only the boundary and whether the paths replay. A
//! relation that reads a derivation's steps therefore separates certificates
//! the oracle identifies, and no amount of structure here changes that. The
//! event order is a **presentation** of one derivation, and two presentations
//! of one certificate may differ.
//!
//! # The independence relation is asked, never restated
//!
//! [`step_independence_with_support`] delegates to
//! [`crate::shift::check_shift_guard_with_support`] and reads **any** refusal
//! as dependence. That is the conservative direction:
//! refusing to commute keeps the recorded order, which is always a valid
//! derivation. It also means the crate has exactly one independence relation,
//! so this module cannot drift from the shift quotient it is the causal
//! structure of.
//!
//! The relation is **symmetric**, which everything below depends on and which
//! is a fact about the guard rather than an assumption here — its first
//! conjunct answers
//! [`gandr_theory_cell_complexes::alphabet::PositionOrder::Incomparable`] for a
//! pair exactly when it does for the swap, its second asks the overlap
//! enumerator in both ordered directions, and its third does not read the pair
//! at all.
//!
//! # The premise no conjunct checks
//!
//! Independence is sound only if the alphabet's term algebra is **local** — a
//! rewrite at one position leaves every incomparable position alone
//! ([`CellAlphabet::splice_cmd_at`]'s own `- ensures:`). Two applications can
//! satisfy every conjunct of the guard honestly and still fail to commute if
//! that clause is broken, because locality is neither a position nor a cell
//! content and the guard reads only those two. Nothing in this module can see
//! that failure; what catches it is [`crate::normal_form::normalize`] replaying
//! the schedule this order induces.
//!
//! [`CellAlphabet::splice_cmd_at`]: gandr_theory_cell_complexes::alphabet::CellAlphabet::splice_cmd_at

use alloc::vec::Vec;

use gandr_theory_cell_complexes::alphabet::CellAlphabet;
use gandr_theory_cell_complexes::alphabet::ConvexityDischarge;
use gandr_theory_cell_complexes::boundary::CausalDepth;
use gandr_theory_cell_complexes::boundary::EventConcurrency;
use gandr_theory_cell_complexes::boundary::EventCount;
use gandr_theory_cell_complexes::boundary::EventDependence;
use gandr_theory_cell_complexes::boundary::EventIndex;
use gandr_theory_cell_complexes::boundary::EventPrecedence;
use gandr_theory_cell_complexes::boundary::SchedulePosition;
use gandr_theory_cell_complexes::boundary::StepIndependence;
use gandr_theory_cell_complexes::boundary::TranspositionCount;
use gandr_theory_cell_complexes::cell::CellStore;
use gandr_theory_cell_complexes::sequent::SequentAlphabet;
use gandr_theory_coherent_resolutions::overlap::OverlapSupport;
use gandr_theory_coherent_resolutions::rewrite::CellApp;

use crate::normal_form::CausalPast;
use crate::normal_form::PrimId;
use crate::normal_form::causal_past_address;
use crate::shift::check_shift_guard_with_support;

/// One **event** of a recorded derivation — a step that moved the term,
/// together with the content address of the primitive it applies.
///
/// It is nominally distinct from a [`CellApp`] because the two are read
/// differently: a [`CellApp`] is a step of a path, which may be a unit and
/// whose place is the recorded one, while an event is a node of a partial
/// order, never a unit and placed by the order rather than by the recording.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DerivationEvent<A: CellAlphabet = SequentAlphabet>
{
    /// The step this event fires, as recorded.
    step: CellApp<A>,
    /// The content address of the primitive it applies.
    address: PrimId,
}

impl<A: CellAlphabet> DerivationEvent<A>
{
    /// An event from a surviving step and its content address.
    #[inline]
    #[must_use]
    pub const fn new(
        step: CellApp<A>,
        address: PrimId,
    ) -> Self
    {
        Self { step, address }
    }

    /// The step this event fires.
    #[inline]
    #[must_use]
    pub const fn step(&self) -> &CellApp<A>
    {
        &self.step
    }

    /// The content address of the primitive this event applies.
    #[inline]
    #[must_use]
    pub const fn address(&self) -> PrimId
    {
        self.address
    }
}

/// The **finite event partial order** of one recorded derivation.
///
/// The events are held in recorded order; the order proper is the transitive
/// closure of the direct dependence edges, and [`EventOrder::depth`] is the
/// layering induced by it.
///
/// A value of this type is **a presentation of one derivation**, not a
/// certificate: it says nothing about the boundary and cannot say whether the
/// derivation replays. [`crate::normal_form::ReplayWitness`] is what carries
/// both.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOrder<A: CellAlphabet = SequentAlphabet>
{
    /// The events, in recorded order.
    events: Vec<DerivationEvent<A>>,
    /// For each event, the strictly earlier events it depends on directly.
    dependences: Vec<Vec<EventIndex>>,
    /// For each event, its layer in the dependence order.
    depths: Vec<CausalDepth>,
    /// For each event, its intrinsic sort key.
    keys: Vec<EventKey>,
    /// The warrant the independence relation's convexity conjunct was decided
    /// under, carried rather than recomputed.
    convexity: ConvexityDischarge,
}

/// The **intrinsic key** of an event — the content-derived value that orders
/// two events sharing a causal depth.
///
/// It is the primitive's content address refined by a digest of the event's
/// labeled causal past, compared in that order. The refinement matters because
/// the address alone is not injective: [`core::hash::Hash`] never promises
/// injectivity, so an alphabet may legally give two applications at different
/// sites one address, and where those two sit over different causes the past
/// digest separates them.
///
/// **Neither component reads arrival order or a store-local index.** The
/// address digests the resolved cell's *content* and the position, never the
/// [`gandr_theory_cell_complexes::cell::CellId`] the cell was interned under;
/// the past digest folds addresses over the **sorted** predecessor digests, so
/// it cannot see which linear extension was recorded. So the key is a function
/// of the labeled causal order, which is what makes the canonical order one
/// too.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventKey
{
    /// The content address of the primitive the event applies.
    address: PrimId,
    /// The digest of the event's labeled causal past.
    past: CausalPast,
}

impl EventKey
{
    /// The content address of the primitive the event applies.
    #[inline]
    #[must_use]
    pub const fn address(&self) -> PrimId
    {
        self.address
    }

    /// The digest of the event's labeled causal past.
    #[inline]
    #[must_use]
    pub const fn past(&self) -> CausalPast
    {
        self.past
    }
}

/// Two distinct events that **tie** on the canonical sort key.
///
/// A tie means the canonical order is not determined by the labeled causal
/// order: the sort would fall back on its own stability and so on the arrival
/// order, which would make the normal form depend on which sequentialization
/// happened to be recorded. Refusing is the conservative direction, and it is
/// the same direction [`crate::normal_form`] takes for a content-address
/// collision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct KeyCollision
{
    /// The earlier of the two tying events, in recorded order.
    pub earlier: EventIndex,
    /// The later of them.
    pub later: EventIndex,
    /// The depth both sit at.
    pub depth: CausalDepth,
    /// The key both carry.
    pub key: EventKey,
}

/// Why an **exchange** between two sequentializations was refused.
///
/// Refusal is data, never a panic and never a silent reordering.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExchangeObstruction
{
    /// The target order is not a rearrangement of the source order — they
    /// differ in length, or one holds an event the other does not.
    NotARearrangement,
    /// **The exchange kill signal.** Reaching the target order requires
    /// transposing a pair the independence relation calls dependent.
    ///
    /// Two sequentializations related only by such a swap are not in one trace
    /// class, so identifying them would be unsound. A caller asking for the
    /// canonical order and receiving this has a canonical key that is not a
    /// linear extension of the causal order.
    DependentTransposition
    {
        /// The event that sits earlier in the sequence being transposed.
        earlier: EventIndex,
        /// The event that sits later in it.
        later: EventIndex,
    },
}

/// One **licensed adjacent transposition** of a sequentialization — the swap of
/// the events at `position` and `position + 1`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Transposition
{
    /// The lower of the two positions swapped.
    position: SchedulePosition,
}

impl Transposition
{
    /// The lower of the two positions this transposition swaps.
    #[inline]
    #[must_use]
    pub const fn position(&self) -> SchedulePosition
    {
        self.position
    }
}

/// The **exchange witness** carrying one sequentialization of a derivation to
/// another: the adjacent transpositions that do it, each one licensed.
///
/// It is the evidence behind "these two orders are the same trace". Every
/// transposition it holds was checked against the crate's single independence
/// relation at construction, so replaying the witness is a rearrangement no
/// dependence edge objects to — which is exactly what shift equivalence is.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ExchangeWitness
{
    /// The transpositions, in the order they are performed.
    transpositions: Vec<Transposition>,
}

impl ExchangeWitness
{
    /// The transpositions, in the order they are performed.
    #[inline]
    #[must_use]
    pub fn transpositions(&self) -> &[Transposition]
    {
        &self.transpositions
    }

    /// How many transpositions this witness performs.
    #[inline]
    #[must_use]
    pub fn transposition_count(&self) -> TranspositionCount
    {
        TranspositionCount::from(self.transpositions.len())
    }

    /// Perform the witness's transpositions on `order`.
    ///
    /// This is what makes the witness checkable rather than merely recorded: a
    /// caller replays it and compares, instead of trusting that it describes
    /// the rearrangement it claims.
    ///
    /// # Contract
    /// - requires: `order` is the sequentialization the witness was built from.
    /// - ensures: `Some(target)` — the source order with every transposition
    ///   applied in turn — for a witness whose positions are all in range.
    /// - provides: the rearrangement the witness describes, computed rather
    ///   than asserted.
    /// - fails: `None` when a transposition names a position `order` does not
    ///   hold, which [`EventOrder::exchange_between`] cannot produce for the
    ///   order it was given.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the result is validated against the input
    ///   rather than a predicted answer: the fixtures apply the witness to the
    ///   recorded order and compare with the canonical order the same
    ///   [`EventOrder`] computed independently.
    /// - witness: `causal::tests::the_exchange_witness_carries_the_recorded_order_to_the_canonical_one`
    /// - witness: `normal_form::tests::an_exchange_witness_replays_to_its_target_order`
    #[inline]
    #[must_use]
    pub fn apply(
        &self,
        order: &[EventIndex],
    ) -> Option<Vec<EventIndex>>
    {
        let mut current = order.to_vec();
        for transposition in &self.transpositions {
            let below = usize::from(transposition.position);
            let above = below.checked_add(1_usize)?;
            let lower = current.get(below).copied()?;
            let upper = current.get(above).copied()?;
            let slot = current.get_mut(below)?;
            *slot = upper;
            let slot = current.get_mut(above)?;
            *slot = lower;
        }
        Some(current)
    }
}

impl<A: CellAlphabet> EventOrder<A>
{
    /// Build the event order of a derivation's surviving steps.
    ///
    /// # Contract
    /// - requires: `events` are the steps of one recorded derivation that moved
    ///   the term, in recorded order, addressed against `store`.
    /// - ensures: event `i` depends directly on every strictly earlier event
    ///   the independence relation refuses to commute it with, and takes the
    ///   depth `1 + max` over those (zero when there are none).
    /// - provides: the causal structure `equiv_S` quotients by, decided through
    ///   the crate's single independence relation rather than a second copy of
    ///   it.
    /// - panics: none.
    /// - intension: quadratic in the number of events — one constant-time
    ///   support lookup per ordered pair after one store-wide cache build.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise, and the depth recurrence needs **three**
    ///   layers to bite. It has two residues — "maximum over every earlier
    ///   dependence" versus the nearest or the first one, and the `1 +` on the
    ///   prior depth — and a two-layer derivation separates neither, because
    ///   with one dependence apiece every variant agrees. A further residue: a
    ///   depth read off the earlier step's *position index* instead of its
    ///   depth is still a valid topological layering, so it survives every
    ///   fixture whose dependence order is a single chain. It is separated by
    ///   shift-invariance across two recorded orders of two interleaved chains,
    ///   where the index-based variant gives one trace class two schedules.
    /// - witness: `normal_form::tests::the_dependence_edges_are_the_pairs_the_guard_refuses`
    /// - witness: `normal_form::tests::a_three_layer_derivation_gives_three_layers`
    /// - witness: `normal_form::tests::a_layered_derivation_keeps_its_dependent_step_last`
    /// - witness: `normal_form::tests::a_three_layer_derivation_orders_each_layer_by_content_address`
    /// - witness: `normal_form::tests::two_interleaved_dependence_chains_layer_by_depth_and_not_by_position`
    #[inline]
    #[must_use]
    pub fn of_events(
        store: &CellStore<A>,
        events: Vec<DerivationEvent<A>>,
        convexity: ConvexityDischarge,
    ) -> Self
    {
        let support = OverlapSupport::from_store(store);
        let mut dependences: Vec<Vec<EventIndex>> = Vec::with_capacity(events.len());
        let mut depths: Vec<CausalDepth> = Vec::with_capacity(events.len());
        let mut keys: Vec<EventKey> = Vec::with_capacity(events.len());
        for (index, current) in events.iter().enumerate() {
            let mut edges: Vec<EventIndex> = Vec::new();
            let mut depth = 0_usize;
            for (earlier, prior) in events.iter().enumerate().take(index) {
                if bool::from(step_independence_with_support(
                    store,
                    &prior.step,
                    &current.step,
                    convexity,
                    &support,
                )) {
                    continue;
                }
                edges.push(EventIndex::from(earlier));
                let prior_depth = depths.get(earlier).copied().unwrap_or_default();
                depth = depth.max(usize::from(prior_depth).saturating_add(1_usize));
            }
            let mut inherited: Vec<CausalPast> = Vec::with_capacity(edges.len());
            for earlier in &edges {
                let past = keys
                    .get(usize::from(*earlier))
                    .map(EventKey::past)
                    .unwrap_or_default();
                inherited.push(past);
            }
            keys.push(EventKey {
                address: current.address,
                past: causal_past_address(current.address, &inherited),
            });
            dependences.push(edges);
            depths.push(CausalDepth::from(depth));
        }
        Self {
            events,
            dependences,
            depths,
            keys,
            convexity,
        }
    }

    /// The events, in recorded order.
    #[inline]
    #[must_use]
    pub fn events(&self) -> &[DerivationEvent<A>]
    {
        &self.events
    }

    /// How many events the derivation has, after unit elimination.
    #[inline]
    #[must_use]
    pub fn event_count(&self) -> EventCount
    {
        EventCount::from(self.events.len())
    }

    /// The event at `at`, or `None` when the index names no event.
    #[inline]
    #[must_use]
    pub fn event(
        &self,
        at: EventIndex,
    ) -> Option<&DerivationEvent<A>>
    {
        self.events.get(usize::from(at))
    }

    /// The layer of the event at `at`, or `None` when the index names no event.
    #[inline]
    #[must_use]
    pub fn depth(
        &self,
        at: EventIndex,
    ) -> Option<CausalDepth>
    {
        self.depths.get(usize::from(at)).copied()
    }

    /// The intrinsic sort key of the event at `at`, or `None` when the index
    /// names no event.
    #[inline]
    #[must_use]
    pub fn key(
        &self,
        at: EventIndex,
    ) -> Option<EventKey>
    {
        self.keys.get(usize::from(at)).copied()
    }

    /// Refuse this order if two distinct events **tie** on the canonical sort
    /// key.
    ///
    /// # Contract
    /// - ensures: `Ok(())` exactly when `(depth, key)` is a strict total order
    ///   on the events, which is what makes [`EventOrder::canonical_order`] a
    ///   function of the labeled causal order rather than of the recorded one.
    /// - provides: the **enforcement** of the injectivity the canonical order
    ///   claims. Every [`EventOrder`] reachable from outside this crate has
    ///   passed it, because both public constructors
    ///   ([`crate::normal_form::event_order`] and
    ///   [`crate::normal_form::normalize_certified`]) call it before returning.
    /// - fails: [`KeyCollision`], naming both events, the depth they share, and
    ///   the key they share.
    /// - panics: none.
    /// - intension: one pass over the canonical order, because a tie in a
    ///   sorted sequence is necessarily between neighbours.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the decision surface is the neighbour
    ///   comparison, separated by an order whose keys are distinct, one whose
    ///   two events share a key at one depth, and one whose two events share a
    ///   key at **different** depths (which is not a tie and must pass, since
    ///   that is exactly the graded-multiplicity case every repeated primitive
    ///   produces). The tying inputs are built by hand rather than derived:
    ///   over both shipped alphabets a shared address forces a shared position,
    ///   which forces dependence and so distinct depths, so no derivation over
    ///   them can reach this refusal — that is a fact about those alphabets and
    ///   not evidence the check is redundant, and it is why the witnesses
    ///   assemble the order directly.
    /// - witness: `causal::tests::an_order_whose_keys_are_distinct_is_accepted`
    /// - witness: `causal::tests::two_events_tying_on_depth_and_key_are_refused`
    /// - witness: `causal::tests::a_repeated_primitive_at_two_depths_is_not_a_tie`
    #[inline]
    pub(crate) fn refuse_key_collisions(&self) -> Result<(), KeyCollision>
    {
        let order = self.canonical_order();
        let mut held: Option<(EventIndex, CausalDepth, EventKey)> = None;
        for index in order {
            let depth = self.depth(index).unwrap_or_default();
            let Some(key) = self.key(index)
            else {
                continue;
            };
            if let Some(previous) = held
                && previous.1 == depth
                && previous.2 == key
            {
                return Err(KeyCollision {
                    earlier: previous.0.min(index),
                    later: previous.0.max(index),
                    depth,
                    key,
                });
            }
            held = Some((index, depth, key));
        }
        Ok(())
    }

    /// The warrant this order's independence relation was decided under.
    #[inline]
    #[must_use]
    pub const fn convexity(&self) -> ConvexityDischarge
    {
        self.convexity
    }

    /// Whether `later` depends **directly** on `earlier`.
    ///
    /// Direct dependence is not transitive: `x` may depend on `y` and `y` on
    /// `z` with `x` and `z` independent, because independence is a relation on
    /// pairs of steps and nothing forces it to compose. The transitive relation
    /// is [`EventOrder::precedes`].
    ///
    /// # Contract
    /// - ensures: positive exactly when `earlier` is strictly earlier in the
    ///   recording than `later` and the independence relation refuses the pair.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the decision surface is membership in one
    ///   event's edge list, separated by a dependent pair, an independent pair,
    ///   the same pair asked in the opposite direction (negative, because the
    ///   edges point backwards), and an out-of-range index.
    /// - witness: `normal_form::tests::the_dependence_edges_are_the_pairs_the_guard_refuses`
    /// - witness: `causal::tests::an_out_of_range_index_depends_on_nothing`
    #[inline]
    #[must_use]
    pub fn depends_directly(
        &self,
        later: EventIndex,
        earlier: EventIndex,
    ) -> EventDependence
    {
        let Some(edges) = self.dependences.get(usize::from(later))
        else {
            return EventDependence::from(false);
        };
        EventDependence::from(edges.contains(&earlier))
    }

    /// Whether two events are licensed to commute.
    ///
    /// This is the symmetric complement of direct dependence, and it is the
    /// relation an exchange consults: an adjacent transposition is licensed
    /// exactly when the pair it swaps is independent. An event is **never**
    /// independent of itself.
    ///
    /// # Contract
    /// - ensures: positive exactly when the two indices name distinct events
    ///   the independence relation licenses; the answer does not depend on the
    ///   argument order. An index naming no event answers **negative**, which
    ///   is the conservative direction — refusing to commute keeps the recorded
    ///   order, and reading an absent event as commuting freely would license a
    ///   transposition nothing decided.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the decision surface is the complement plus
    ///   the reflexive case plus the range guard, separated by an independent
    ///   pair asked in both directions, a dependent pair, one event against
    ///   itself, and an index outside the order.
    /// - witness: `causal::tests::an_event_is_never_independent_of_itself`
    /// - witness: `causal::tests::an_out_of_range_index_depends_on_nothing`
    /// - witness: `normal_form::tests::independence_is_symmetric_and_irreflexive`
    #[inline]
    #[must_use]
    pub fn independent(
        &self,
        left: EventIndex,
        right: EventIndex,
    ) -> StepIndependence
    {
        if left == right || self.event(left).is_none() || self.event(right).is_none() {
            return StepIndependence::from(false);
        }
        let (earlier, later) = if usize::from(left) < usize::from(right) {
            (left, right)
        }
        else {
            (right, left)
        };
        StepIndependence::from(!bool::from(self.depends_directly(later, earlier)))
    }

    /// Whether `earlier` **causally precedes** `later` — the strict partial
    /// order proper.
    ///
    /// # Contract
    /// - ensures: positive exactly when `later` is reachable from `earlier`
    ///   along one or more direct dependence edges. The relation is
    ///   irreflexive, asymmetric, and transitive, because every edge points
    ///   from a strictly later recorded index to a strictly earlier one.
    /// - provides: the finite partial order the canonical schedule is a linear
    ///   extension of.
    /// - panics: none.
    /// - intension: an iterative reachability sweep over an explicit worklist,
    ///   bounded by the number of edges; no recursion, so derivation length
    ///   cannot reach the stack.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the decision surface is reachability rather
    ///   than adjacency, separated by a pair joined only through an intervening
    ///   event (positive here, negative at [`EventOrder::depends_directly`]),
    ///   by the same pair asked backwards, and by an event against itself. The
    ///   order laws themselves are asserted as **properties** rather than
    ///   pointwise, over generated derivations: a relation that got
    ///   transitivity or asymmetry wrong on some shape a fixture does not have
    ///   would pass every pointwise test and fail those.
    /// - witness: `causal::tests::precedence_is_the_transitive_closure_of_dependence`
    /// - witness: `normal_form::tests::causal_precedence_is_a_strict_partial_order`
    #[inline]
    #[must_use]
    pub fn precedes(
        &self,
        earlier: EventIndex,
        later: EventIndex,
    ) -> EventPrecedence
    {
        if earlier == later {
            return EventPrecedence::from(false);
        }
        let Some(seeds) = self.dependences.get(usize::from(later))
        else {
            return EventPrecedence::from(false);
        };
        let mut visited: Vec<bool> = alloc::vec![false; self.events.len()];
        let mut frontier: Vec<EventIndex> = seeds.clone();
        while let Some(index) = frontier.pop() {
            if index == earlier {
                return EventPrecedence::from(true);
            }
            let Some(seen) = visited.get_mut(usize::from(index))
            else {
                continue;
            };
            if *seen {
                continue;
            }
            *seen = true;
            if let Some(next) = self.dependences.get(usize::from(index)) {
                frontier.extend_from_slice(next);
            }
        }
        EventPrecedence::from(false)
    }

    /// Whether two distinct events are **causally unordered** — neither
    /// precedes the other.
    ///
    /// Concurrency is coarser than independence: two events may be causally
    /// unordered while being directly dependent on nothing in common, and two
    /// *independent* events are always concurrent, but the converse fails
    /// wherever a dependence edge is mediated. An event is never concurrent
    /// with itself.
    ///
    /// # Contract
    /// - ensures: positive exactly when the two indices name distinct events
    ///   with no precedence either way; the answer does not depend on the
    ///   argument order. An index naming no event answers **negative**, for the
    ///   reason [`EventOrder::independent`] gives: an absent event is not a
    ///   thing that could have fired alongside anything.
    /// - provides: the "could have fired together" relation a parallel replay
    ///   plan reads, which the flattened schedule cannot express.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the decision surface is the conjunction of
    ///   two precedence questions plus the reflexive case, separated by a
    ///   dependent pair, an independent pair, and one event against itself. The
    ///   load-bearing property is that events sharing a depth are pairwise
    ///   concurrent, which is a theorem about the recurrence rather than a
    ///   fixture's accident, so it is asserted over generated derivations.
    /// - witness: `causal::tests::an_event_is_never_concurrent_with_itself`
    /// - witness: `causal::tests::an_out_of_range_index_depends_on_nothing`
    /// - witness: `normal_form::tests::events_sharing_a_layer_are_pairwise_concurrent`
    #[inline]
    #[must_use]
    pub fn concurrent(
        &self,
        left: EventIndex,
        right: EventIndex,
    ) -> EventConcurrency
    {
        if left == right || self.event(left).is_none() || self.event(right).is_none() {
            return EventConcurrency::from(false);
        }
        let forwards = bool::from(self.precedes(left, right));
        let backwards = bool::from(self.precedes(right, left));
        EventConcurrency::from(!forwards && !backwards)
    }

    /// The events in **recorded** order.
    #[inline]
    #[must_use]
    pub fn recorded_order(&self) -> Vec<EventIndex>
    {
        (0 .. self.events.len()).map(EventIndex::from).collect()
    }

    /// The events in **canonical** order — the causal layering, flattened.
    ///
    /// # The binding invariant
    ///
    /// Owner ruling, 2026-08-16, on the arc's decision queue
    /// (`gandr-5lf.18-question-01`): **causal layering — earliest causal
    /// position — is the canonical schedule, and union-find component
    /// pre-grouping is declined.** The invariant that ruling binds this
    /// function to has four clauses, and every one of them is checked rather
    /// than asserted:
    ///
    /// 1. The order is the **lexicographic** sort by `(causal depth, intrinsic
    ///    event key)`.
    /// 2. **Depth** is the length of the longest dependence chain strictly
    ///    below the event under the transitive closure of dependence. The
    ///    recurrence at [`EventOrder::of_events`] computes it over the *direct*
    ///    edges, which is the same number: taking the transitive closure of a
    ///    finite acyclic relation adds only shortcuts, and a shortcut is never
    ///    longer than the chain it skips.
    /// 3. The **key** is content-derived, total, injective over the events of
    ///    one derivation, and independent of arrival order and of store-local
    ///    indices. [`EventKey`] carries the first, the third and the fourth by
    ///    construction; injectivity is not free and is **enforced** at
    ///    [`EventOrder::refuse_key_collisions`], which every public constructor
    ///    of an [`EventOrder`] runs before returning one.
    /// 4. The result therefore depends **only on the labeled causal partial
    ///    order**, and that is checked by exchange witnesses over adjacent
    ///    independent pairs: transposing such a pair in the recorded derivation
    ///    leaves this order unchanged.
    ///
    /// The declined alternative is recorded rather than forgotten, because it
    /// stays cheap to reverse. Union-find over the dependence relation yields
    /// components, and concatenating whole components in content-key order is
    /// also a canonical form — it is sound and cheap, and it differs from this
    /// one by delaying a whole component behind another instead of putting
    /// every event at the earliest layer its causes allow. Switching to it
    /// would replace the depth-and-key sort here and nothing else; the
    /// soundness contract does not move, because the canonical schedule is
    /// replayed and verified before any normal form is returned either way.
    /// What makes the switch grow more expensive over time is a consumer that
    /// reads [`EventOrder::layers`] as parallel batches, which
    /// component-major order does not expose.
    ///
    /// # Contract
    /// - ensures: the recorded order sorted by `(depth, key)`. The sort is
    ///   stable, and its stability is **unobservable**: an order that reaches
    ///   this function has passed [`EventOrder::refuse_key_collisions`], so no
    ///   two events tie and the key is a strict total order.
    /// - provides: the linear extension of the causal order that
    ///   [`crate::normal_form::TraceletNf`]'s schedule is built from — a
    ///   canonical representative of the shift class, determined by the labeled
    ///   causal order alone.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the key's components are separated
    ///   independently: the depth by a layered derivation whose dependent step
    ///   must stay last, the address by a layer with two occupants whose
    ///   declared ascending order is observable in the result, and the causal
    ///   past by two events sharing an address over different causes, which the
    ///   address alone cannot separate. The two structural claims are asserted
    ///   as **properties** over generated derivations rather than argued: that
    ///   the depth agrees with the longest chain under the transitive closure,
    ///   and that transposing any adjacent independent pair leaves this order
    ///   fixed.
    /// - witness: `normal_form::tests::a_three_layer_derivation_orders_each_layer_by_content_address`
    /// - witness: `normal_form::tests::the_canonical_key_never_ties`
    /// - witness: `normal_form::tests::the_depth_is_the_longest_chain_strictly_below`
    /// - witness: `normal_form::tests::every_adjacent_independent_transposition_leaves_the_canonical_order_fixed`
    /// - witness: `normal_form::tests::the_canonical_order_is_the_same_in_two_differently_ordered_stores`
    /// - witness: `causal::tests::the_causal_past_separates_two_events_sharing_an_address`
    #[inline]
    #[must_use]
    pub fn canonical_order(&self) -> Vec<EventIndex>
    {
        let mut order = self.recorded_order();
        order.sort_by(|left, right| {
            let left_depth = self.depth(*left).unwrap_or_default();
            let right_depth = self.depth(*right).unwrap_or_default();
            left_depth
                .cmp(&right_depth)
                .then_with(|| self.key(*left).cmp(&self.key(*right)))
        });
        order
    }

    /// The **layers** of the causal order — the events grouped by depth, each
    /// layer in ascending address order.
    ///
    /// This is the projection a flattened schedule cannot supply: every event
    /// in one layer is concurrent with every other in it, so a layer is a batch
    /// that could fire together, and the number of layers is the derivation's
    /// causal critical path.
    ///
    /// # Contract
    /// - ensures: a partition of every event into consecutive groups of equal
    ///   depth, in ascending depth order, whose concatenation is
    ///   [`EventOrder::canonical_order`].
    /// - provides: the parallel batches a replay plan schedules and the
    ///   critical-path length a cost model reads.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the decision surface is where one group
    ///   ends and the next begins, separated by a three-layer derivation with a
    ///   two-occupant first layer and singleton layers after it, so a grouping
    ///   that split on every event or never split is caught by the layer count
    ///   as well as by the contents. That the concatenation is the canonical
    ///   order, and that each layer is an antichain, are asserted as properties
    ///   over generated derivations — a grouping can be wrong on a shape no
    ///   fixture has.
    /// - witness: `normal_form::tests::a_three_layer_derivation_gives_three_layers`
    /// - witness: `normal_form::tests::the_layers_concatenate_to_the_canonical_order`
    /// - witness: `normal_form::tests::events_sharing_a_layer_are_pairwise_concurrent`
    #[inline]
    #[must_use]
    pub fn layers(&self) -> Vec<Vec<EventIndex>>
    {
        let mut layers: Vec<Vec<EventIndex>> = Vec::new();
        let mut current: Vec<EventIndex> = Vec::new();
        let mut held: Option<CausalDepth> = None;
        for index in self.canonical_order() {
            let depth = self.depth(index).unwrap_or_default();
            if held != Some(depth) {
                if !current.is_empty() {
                    layers.push(core::mem::take(&mut current));
                }
                held = Some(depth);
            }
            current.push(index);
        }
        if !current.is_empty() {
            layers.push(current);
        }
        layers
    }

    /// The **exchange witness** carrying `from` to `to`, or the refusal that
    /// says they are not one trace.
    ///
    /// This is the decision procedure for "are these two sequentializations of
    /// one derivation shift-equivalent", answered with evidence rather than
    /// with a verdict: the returned witness is a list of adjacent
    /// transpositions, each of which was checked against the independence
    /// relation before it was recorded.
    ///
    /// # Contract
    /// - requires: `from` and `to` name events of **this** order.
    /// - ensures: `Ok(witness)` whose transpositions, applied in turn to
    ///   `from`, give `to`, and each of which swaps an independent pair.
    /// - provides: shift equivalence of two sequentializations, decided and
    ///   witnessed. A positive answer means the two orders lie in one trace
    ///   class, so a derivation recorded in either fires the same primitives.
    /// - fails: [`ExchangeObstruction::NotARearrangement`] when `to` is not a
    ///   permutation of `from`, and
    ///   [`ExchangeObstruction::DependentTransposition`] — the exchange kill
    ///   signal — when reaching `to` would require swapping a dependent pair.
    /// - panics: none.
    /// - intension: a selection pass that brings each wanted event down to its
    ///   target position by adjacent swaps, so the witness holds exactly the
    ///   inversions between the two orders and its length is quadratic in the
    ///   number of events in the worst case.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the witness is validated against the input
    ///   rather than a predicted answer, by applying it and comparing with the
    ///   target order. Both `- fails:` modes own a decision surface separated
    ///   by a fixture that triggers only it: a target holding an event the
    ///   source does not for the first, and a target that inverts a dependent
    ///   pair for the second. The load-bearing claim is that the canonical
    ///   order is always reachable — that the canonical key is a linear
    ///   extension of the causal order — which is asserted over generated
    ///   derivations rather than at a fixture, because a key that inverted a
    ///   dependent pair on some shape would pass every hand-written case.
    /// - witness: `causal::tests::the_exchange_witness_carries_the_recorded_order_to_the_canonical_one`
    /// - witness: `causal::tests::a_target_that_is_not_a_rearrangement_is_refused`
    /// - witness: `causal::tests::a_target_inverting_a_dependent_pair_is_refused`
    /// - witness: `normal_form::tests::an_independent_pair_is_reordered_by_licensed_transpositions`
    /// - witness: `normal_form::tests::a_containment_dependent_pair_refuses_its_transposition`
    /// - witness: `normal_form::tests::an_exchange_witness_replays_to_its_target_order`
    /// - witness: `normal_form::tests::the_canonical_order_is_always_reachable_by_licensed_transpositions`
    #[inline]
    pub fn exchange_between(
        &self,
        from: &[EventIndex],
        to: &[EventIndex],
    ) -> Result<ExchangeWitness, ExchangeObstruction>
    {
        if from.len() != to.len() {
            return Err(ExchangeObstruction::NotARearrangement);
        }
        let mut current: Vec<EventIndex> = from.to_vec();
        let mut transpositions: Vec<Transposition> = Vec::new();
        for (target, wanted) in to.iter().enumerate() {
            let mut found: Option<usize> = None;
            for (offset, held) in current.iter().enumerate().skip(target) {
                if held == wanted {
                    found = Some(offset);
                    break;
                }
            }
            let Some(mut position) = found
            else {
                return Err(ExchangeObstruction::NotARearrangement);
            };
            while position > target {
                let Some(below) = position.checked_sub(1_usize)
                else {
                    return Err(ExchangeObstruction::NotARearrangement);
                };
                let Some(upper) = current.get(position).copied()
                else {
                    return Err(ExchangeObstruction::NotARearrangement);
                };
                let Some(lower) = current.get(below).copied()
                else {
                    return Err(ExchangeObstruction::NotARearrangement);
                };
                if !bool::from(self.independent(lower, upper)) {
                    return Err(ExchangeObstruction::DependentTransposition {
                        earlier: lower,
                        later: upper,
                    });
                }
                let Some(slot) = current.get_mut(below)
                else {
                    return Err(ExchangeObstruction::NotARearrangement);
                };
                *slot = upper;
                let Some(slot) = current.get_mut(position)
                else {
                    return Err(ExchangeObstruction::NotARearrangement);
                };
                *slot = lower;
                transpositions.push(Transposition {
                    position: SchedulePosition::from(below),
                });
                position = below;
            }
        }
        Ok(ExchangeWitness { transpositions })
    }

    /// The **exchange witness** carrying the recorded order to the canonical
    /// one.
    ///
    /// # Contract
    /// - ensures: `Ok(witness)` whose transpositions carry
    ///   [`EventOrder::recorded_order`] to [`EventOrder::canonical_order`].
    /// - provides: the evidence that canonicalization stays inside the trace
    ///   class — that the normal form's schedule is a rearrangement the
    ///   independence relation licenses, rather than one asserted to be.
    /// - fails: [`ExchangeObstruction::DependentTransposition`] when the
    ///   canonical key is not a linear extension of the causal order. That
    ///   cannot happen while depths come from the recurrence at
    ///   [`EventOrder::of_events`] — a dependent pair has strictly increasing
    ///   depth, so the key never inverts it — and the arm is a **tripwire for a
    ///   future canonical key** rather than a reachable failure. It has no
    ///   witness through this entry point, which is recorded here rather than
    ///   left implicit; the arm itself is witnessed through
    ///   [`EventOrder::exchange_between`], where a caller supplies the target
    ///   order.
    /// - panics: none.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the witness is applied to the recorded order
    ///   and compared with the canonical order; the refusal arm is unreachable
    ///   here by the depth argument above and is separated at
    ///   [`EventOrder::exchange_between`] instead.
    /// - witness: `causal::tests::the_exchange_witness_carries_the_recorded_order_to_the_canonical_one`
    /// - witness: `normal_form::tests::the_canonical_order_is_always_reachable_by_licensed_transpositions`
    #[inline]
    pub fn exchange_to_canonical(&self) -> Result<ExchangeWitness, ExchangeObstruction>
    {
        self.exchange_between(&self.recorded_order(), &self.canonical_order())
    }
}

/// Whether two recorded steps are licensed to commute.
///
/// The question is delegated to the crate's single shift guard
/// ([`check_shift_guard_with_support`]) rather than restated, and **any**
/// refusal — a comparable position, a genuine overlap, an undischarged
/// convexity conjunct, or an unresolvable identifier — is read as dependence.
/// That direction is the conservative one: refusing to commute keeps the
/// recorded order, which is always a valid derivation.
///
/// # Contract
/// - ensures: a positive answer exactly when the guard's three conjuncts hold
///   of the pair under `convexity`; the answer does not depend on which of the
///   two steps is passed as `left`.
/// - provides: the independence relation the causal order is built from.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the relation is not restated here, so its
///   conjuncts are witnessed at [`check_shift_guard_with_support`]'s own suite;
///   what this wrapper adds is the direction of the collapse, separated by an
///   overlapping pair at incomparable positions whose two orders demonstrably
///   reach one term and which the quotient still refuses. The third conjunct's
///   contribution reaches this wrapper only over an alphabet that withholds the
///   warrant, which is a missing input rather than a missing assertion.
///   Symmetry is **not** separated by any fixture and is not claimed to be:
///   swapping the arguments is an equivalent mutation, because the guard is
///   symmetric. The relation's soundness is conditional on a premise no
///   conjunct checks and this function cannot see — that the alphabet's term
///   algebra is **local**. The only thing standing between an alphabet that
///   breaks it and a wrong identification is
///   [`crate::normal_form::normalize`]'s replay of the canonical schedule, and
///   that has a witness.
/// - witness: `normal_form::tests::an_overlapping_pair_keeps_its_recorded_order`
/// - witness: `normal_form::tests::a_layered_derivation_keeps_its_dependent_step_last`
/// - witness: `normal_form::tests::a_withheld_convexity_warrant_empties_the_shift_quotient`
/// - witness: `normal_form::tests::a_non_local_term_algebra_trips_the_kill_signal_at_the_join`
#[must_use]
fn step_independence_with_support<A>(
    store: &CellStore<A>,
    left: &CellApp<A>,
    right: &CellApp<A>,
    convexity: ConvexityDischarge,
    support: &OverlapSupport,
) -> StepIndependence
where
    A: CellAlphabet,
{
    StepIndependence::from(
        check_shift_guard_with_support(store, left, right, convexity, support).is_ok(),
    )
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_cell_complexes::cell::Cell;
    use gandr_theory_cell_complexes::cell::CellId;
    use gandr_theory_cell_complexes::pattern::CmdPat;
    use gandr_theory_cell_complexes::pattern::ConsPat;
    use gandr_theory_cell_complexes::pattern::Pos;
    use gandr_theory_cell_complexes::pattern::ProdPat;
    use gandr_theory_cell_complexes::sequent::CellProvenance;
    use gandr_theory_cell_complexes::sequent::Orientation;
    use gandr_theory_coherent_resolutions::rewrite::CellApp;

    use super::*;
    use crate::normal_form::event_order;
    use crate::normal_form::prim_address;

    /// (add-S): ⟨Succ(m) | add(n; α)⟩ ~> ⟨m | add(n; Succ⁻(α))⟩.
    fn add_s() -> Cell
    {
        let lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
        );
        let rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("m"),
            ConsPat::op(
                "add",
                [ProdPat::meta("n")],
                ConsPat::frame("Succ", ConsPat::meta("alpha")),
            ),
        );
        Cell::new(
            lhs,
            rhs,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// A store holding `add-S`, and the two-step derivation
    /// `⟨Succ(Succ(Zero)) | add(Zero; ⊤)⟩ ~>* ⟨Zero | add(Zero;
    /// Succ⁻(Succ⁻(⊤)))⟩`.
    ///
    /// A sequent term has exactly one command position, so the two steps are at
    /// the **same** position and are therefore dependent: this fixture is a
    /// two-event chain, which is what the sequent-side unit tests need.
    fn chain_fixture() -> (CellStore, CmdPat, [CellApp; 2])
    {
        let mut store = CellStore::new();
        let add = store.insert(add_s());
        let peak = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])])]),
            ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
        );
        let steps = [
            CellApp {
                cell: add,
                at: Pos::root(),
            },
            CellApp {
                cell: add,
                at: Pos::root(),
            },
        ];
        (store, peak, steps)
    }

    #[test]
    fn precedence_is_the_transitive_closure_of_dependence()
    {
        let (store, peak, steps) = chain_fixture();
        let order = event_order(&store, &peak, &steps).expect("the chain replays");
        let first = EventIndex::from(0_usize);
        let second = EventIndex::from(1_usize);
        assert!(
            bool::from(order.depends_directly(second, first)),
            "the two steps share the one command position, so they are dependent"
        );
        assert!(
            bool::from(order.precedes(first, second)),
            "and dependence gives precedence"
        );
        assert!(
            !bool::from(order.precedes(second, first)),
            "precedence is asymmetric: the edges only point backwards"
        );
        assert!(
            !bool::from(order.depends_directly(first, second)),
            "and direct dependence is recorded in one direction only"
        );
    }

    #[test]
    fn an_event_is_never_independent_of_itself()
    {
        let (store, peak, steps) = chain_fixture();
        let order = event_order(&store, &peak, &steps).expect("the chain replays");
        let first = EventIndex::from(0_usize);
        assert!(
            !bool::from(order.independent(first, first)),
            "a step is always dependent on itself: its position order with itself is Same"
        );
    }

    #[test]
    fn an_event_is_never_concurrent_with_itself()
    {
        let (store, peak, steps) = chain_fixture();
        let order = event_order(&store, &peak, &steps).expect("the chain replays");
        let first = EventIndex::from(0_usize);
        assert!(
            !bool::from(order.concurrent(first, first)),
            "concurrency is irreflexive"
        );
    }

    #[test]
    fn an_out_of_range_index_depends_on_nothing()
    {
        let (store, peak, steps) = chain_fixture();
        let order = event_order(&store, &peak, &steps).expect("the chain replays");
        let beyond = EventIndex::from(9_usize);
        assert!(
            order.event(beyond).is_none(),
            "the index names no event, so there is nothing to read"
        );
        assert!(order.depth(beyond).is_none(), "and no depth either");
        assert!(
            !bool::from(order.depends_directly(beyond, EventIndex::from(0_usize))),
            "an index outside the order depends on nothing rather than panicking"
        );
        assert!(
            !bool::from(order.precedes(EventIndex::from(0_usize), beyond)),
            "and precedes nothing"
        );
        // THE CONSERVATIVE DIRECTION, which the two symmetric relations get
        // wrong if they are written as plain complements: an absent event
        // reached through `depends_directly` alone reads as depending on
        // nothing, which would make it INDEPENDENT of everything and
        // CONCURRENT with everything. Refusing is the safe answer, and it is
        // the one the contracts state.
        assert!(
            !bool::from(order.independent(beyond, EventIndex::from(0_usize))),
            "an index outside the order commutes with nothing"
        );
        assert!(
            !bool::from(order.concurrent(beyond, EventIndex::from(0_usize))),
            "and is concurrent with nothing"
        );
    }

    #[test]
    fn a_dependent_chain_is_its_own_canonical_order()
    {
        let (store, peak, steps) = chain_fixture();
        let order = event_order(&store, &peak, &steps).expect("the chain replays");
        assert_eq!(
            order.recorded_order(),
            order.canonical_order(),
            "a totally ordered derivation has one sequentialization"
        );
        assert_eq!(
            2,
            order.layers().len(),
            "and every event sits in a layer of its own"
        );
    }

    #[test]
    fn the_exchange_witness_carries_the_recorded_order_to_the_canonical_one()
    {
        let (store, peak, steps) = chain_fixture();
        let order = event_order(&store, &peak, &steps).expect("the chain replays");
        let witness = order
            .exchange_to_canonical()
            .expect("the canonical key is a linear extension of the causal order");
        assert_eq!(
            TranspositionCount::from(0_usize),
            witness.transposition_count(),
            "a chain is already canonical, so no transposition is licensed or needed"
        );
        assert_eq!(
            Some(order.canonical_order()),
            witness.apply(&order.recorded_order()),
            "and applying the witness reproduces the canonical order"
        );
    }

    #[test]
    fn a_target_that_is_not_a_rearrangement_is_refused()
    {
        let (store, peak, steps) = chain_fixture();
        let order = event_order(&store, &peak, &steps).expect("the chain replays");
        let recorded = order.recorded_order();
        let short = order.exchange_between(&recorded, &[EventIndex::from(0_usize)]);
        assert_eq!(
            Err(ExchangeObstruction::NotARearrangement),
            short,
            "a target of a different length is not a rearrangement"
        );
        let foreign = order.exchange_between(&recorded, &[
            EventIndex::from(0_usize),
            EventIndex::from(7_usize),
        ]);
        assert_eq!(
            Err(ExchangeObstruction::NotARearrangement),
            foreign,
            "and neither is a target naming an event the source does not hold"
        );
    }

    /// An event order assembled directly from depths and keys.
    ///
    /// The refusal it exercises cannot be reached through a derivation over
    /// either shipped alphabet: an equal address forces an equal position,
    /// which forces `PositionOrder::Same`, which forces dependence and so
    /// strictly different depths. That is a fact about those alphabets rather
    /// than evidence the check is redundant — the moment an alphabet's
    /// `Hash` maps two sites to one address, the tie is reachable — so the
    /// witnesses assemble the order instead of deriving it.
    fn assembled_order(entries: &[(CellApp, CausalDepth, EventKey)]) -> EventOrder
    {
        EventOrder {
            events: entries
                .iter()
                .map(|entry| DerivationEvent::new(entry.0.clone(), entry.2.address()))
                .collect(),
            dependences: entries.iter().map(|_entry| Vec::new()).collect(),
            depths: entries.iter().map(|entry| entry.1).collect(),
            keys: entries.iter().map(|entry| entry.2).collect(),
            convexity: ConvexityDischarge::LeftConnectedOverAcyclicTarget,
        }
    }

    /// Two distinct content addresses, taken over two distinct cells.
    fn two_addresses() -> (CellStore, PrimId, PrimId)
    {
        let mut store = CellStore::new();
        let first = store.insert(add_s());
        let second = store.insert(Cell::new(
            CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
            CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        let left = store.get(first).expect("the first cell is stored");
        let right = store.get(second).expect("the second cell is stored");
        let addresses = (
            prim_address(left, &Pos::root()),
            prim_address(right, &Pos::root()),
        );
        assert_ne!(
            addresses.0, addresses.1,
            "the two cells differ in content, so their addresses differ"
        );
        (store, addresses.0, addresses.1)
    }

    #[test]
    fn an_order_whose_keys_are_distinct_is_accepted()
    {
        let (_store, left, right) = two_addresses();
        let step = CellApp {
            cell: CellId(0_usize),
            at: Pos::root(),
        };
        let order = assembled_order(&[
            (step.clone(), CausalDepth::from(0_usize), EventKey {
                address: left,
                past: causal_past_address(left, &[]),
            }),
            (step, CausalDepth::from(0_usize), EventKey {
                address: right,
                past: causal_past_address(right, &[]),
            }),
        ]);
        assert_eq!(
            Ok(()),
            order.refuse_key_collisions(),
            "two events at one depth with different keys are totally ordered"
        );
    }

    #[test]
    fn two_events_tying_on_depth_and_key_are_refused()
    {
        // THE INJECTIVITY ENFORCEMENT. Two events at one depth carrying one
        // key: the sort would fall back on its own stability and so on the
        // arrival order, which would make the canonical order a property of
        // the presentation rather than of the trace. It is refused instead.
        let (_store, address, _other) = two_addresses();
        let step = CellApp {
            cell: CellId(0_usize),
            at: Pos::root(),
        };
        let key = EventKey {
            address,
            past: causal_past_address(address, &[]),
        };
        let order = assembled_order(&[
            (step.clone(), CausalDepth::from(1_usize), key),
            (step, CausalDepth::from(1_usize), key),
        ]);
        assert_eq!(
            Err(KeyCollision {
                earlier: EventIndex::from(0_usize),
                later: EventIndex::from(1_usize),
                depth: CausalDepth::from(1_usize),
                key,
            }),
            order.refuse_key_collisions(),
            "the refusal names both events, the shared depth, and the shared key"
        );
    }

    #[test]
    fn a_repeated_primitive_at_two_depths_is_not_a_tie()
    {
        // The graded-multiplicity case, which must NOT be refused: a repeated
        // primitive shares its address by design, and what separates the two
        // occurrences is the depth. A check written on the key alone would
        // reject every derivation that fires one primitive twice.
        let (_store, address, _other) = two_addresses();
        let step = CellApp {
            cell: CellId(0_usize),
            at: Pos::root(),
        };
        let key = EventKey {
            address,
            past: causal_past_address(address, &[]),
        };
        let order = assembled_order(&[
            (step.clone(), CausalDepth::from(0_usize), key),
            (step, CausalDepth::from(1_usize), key),
        ]);
        assert_eq!(
            Ok(()),
            order.refuse_key_collisions(),
            "one primitive at two depths is two events the depth already separates"
        );
    }

    #[test]
    fn the_causal_past_separates_two_events_sharing_an_address()
    {
        // WHY THE KEY IS NOT THE ADDRESS ALONE. Two events at one depth with
        // one address — which an alphabet may produce legally, since
        // `core::hash::Hash` promises no injectivity — are separated by what
        // they sit over. Drop the past component and this pair ties.
        let (_store, address, other) = two_addresses();
        let step = CellApp {
            cell: CellId(0_usize),
            at: Pos::root(),
        };
        let rootless = EventKey {
            address,
            past: causal_past_address(address, &[]),
        };
        let caused = EventKey {
            address,
            past: causal_past_address(address, &[causal_past_address(other, &[])]),
        };
        assert_ne!(
            rootless.past(),
            caused.past(),
            "one address over two different causal pasts gives two keys"
        );
        assert_eq!(
            rootless.address(),
            caused.address(),
            "and the address component alone cannot tell them apart"
        );
        let order = assembled_order(&[
            (step.clone(), CausalDepth::from(0_usize), rootless),
            (step, CausalDepth::from(0_usize), caused),
        ]);
        assert_eq!(
            Ok(()),
            order.refuse_key_collisions(),
            "so the pair is totally ordered rather than refused"
        );
    }

    #[test]
    fn a_target_inverting_a_dependent_pair_is_refused()
    {
        // THE EXCHANGE KILL SIGNAL, raised. The two steps of the chain are
        // dependent, so the reversed order is a different trace and the swap
        // that would reach it is refused rather than performed.
        let (store, peak, steps) = chain_fixture();
        let order = event_order(&store, &peak, &steps).expect("the chain replays");
        let first = EventIndex::from(0_usize);
        let second = EventIndex::from(1_usize);
        let reversed = order.exchange_between(&order.recorded_order(), &[second, first]);
        assert_eq!(
            Err(ExchangeObstruction::DependentTransposition {
                earlier: first,
                later: second,
            }),
            reversed,
            "reaching the reversed order needs a transposition of a dependent pair"
        );
    }
}
