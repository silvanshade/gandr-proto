//! The conflict-free, two-colour causal web over one tracelet leg.
//!
//! A [`CausalWeb`] is a read-only analysis of an existing [`EventOrder`]. Its
//! green relation is the order's transitive precedence; its white relation is
//! the complement on distinct vertices, the independence relation of the one
//! recorded run. A tracelet is one run, so no conflict colour is representable
//! here. The web is an analysis surface only: it is not an evaluator, a
//! certificate format, or a wire representation.

use alloc::boxed::Box;
use alloc::vec::Vec;

use gandr_theory_cell_complexes::alphabet::CellAlphabet;
use gandr_theory_cell_complexes::boundary::EventIndex;

use crate::causal::EventKey;
use crate::causal::EventOrder;

/// A coordinate into a [`CausalWeb`]'s canonical event list.
///
/// This is a web coordinate, not a second event identity. The event identity
/// remains the [`EventKey`] held by the web, and the source order remains the
/// [`EventOrder`] from which the web was built.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WebVertex(usize);

impl From<usize> for WebVertex
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<WebVertex> for usize
{
    #[inline]
    fn from(value: WebVertex) -> Self
    {
        value.0
    }
}

/// Number of vertices in a causal web.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WebVertexCount(usize);

impl From<usize> for WebVertexCount
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<WebVertexCount> for usize
{
    #[inline]
    fn from(value: WebVertexCount) -> Self
    {
        value.0
    }
}

/// Number of licensed slice-chain weakenings in a refinement witness.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SliceStepCount(usize);

impl From<usize> for SliceStepCount
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<SliceStepCount> for usize
{
    #[inline]
    fn from(value: SliceStepCount) -> Self
    {
        value.0
    }
}

/// Whether two web vertices carry a green directed edge or the white
/// independence relation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WebRelation
{
    /// `left` causally precedes `right`.
    Precedes,
    /// `right` causally precedes `left`.
    Follows,
    /// Neither vertex precedes the other in this conflict-free run.
    Independent,
    /// At least one coordinate names no web vertex.
    Missing,
}

/// The transitive precedence relation of a causal web.
///
/// The rows are indexed by canonical [`WebVertex`] coordinates. Independence
/// is deliberately not stored as a second matrix: it is the white non-edge
/// complement of this relation on distinct, valid vertices.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependenceBits
{
    /// A square, canonical-coordinate matrix of transitive precedence bits.
    rows: Box<[Box<[bool]>]>,
}

impl DependenceBits
{
    /// Whether the relation contains the directed edge `earlier → later`.
    #[inline]
    #[must_use]
    pub fn contains(
        &self,
        earlier: WebVertex,
        later: WebVertex,
    ) -> WebPrecedence
    {
        let Some(row) = self.rows.get(usize::from(earlier))
        else {
            return WebPrecedence::from(false);
        };
        WebPrecedence::from(row.get(usize::from(later)).copied().unwrap_or(false))
    }

    /// The number of coordinates represented by the relation.
    #[inline]
    #[must_use]
    pub fn vertex_count(&self) -> WebVertexCount
    {
        WebVertexCount::from(self.rows.len())
    }

    /// Check that the relation is square for the supplied vertex count.
    ///
    /// # Contract
    /// - requires: `count` is the candidate number of web vertices.
    /// - ensures: returns `true` exactly when every row and column has that
    ///   count.
    /// - provides: a structural validity check for public web comparisons.
    /// - panics: none.
    fn has_shape(
        &self,
        count: WebVertexCount,
    ) -> WebShapeValid
    {
        let count = usize::from(count);
        WebShapeValid::from(
            self.rows.len() == count && self.rows.iter().all(|row| row.len() == count),
        )
    }

    /// Materialize the order's precedence relation in canonical coordinates.
    ///
    /// # Contract
    /// - requires: `canonical` contains event indices from `order`.
    /// - ensures: each matrix bit agrees with [`EventOrder::precedes`].
    /// - provides: the green relation for a [`CausalWeb`].
    /// - panics: none.
    fn from_order<A>(
        order: &EventOrder<A>,
        canonical: &[EventIndex],
    ) -> Self
    where
        A: CellAlphabet,
    {
        let count = canonical.len();
        let mut rows: Vec<Box<[bool]>> = Vec::with_capacity(count);
        for left in canonical {
            let mut row = Vec::with_capacity(count);
            for right in canonical {
                row.push(bool::from(order.precedes(*left, *right)));
            }
            rows.push(row.into_boxed_slice());
        }
        Self {
            rows: rows.into_boxed_slice(),
        }
    }
}

/// The result of asking whether one web vertex precedes another.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WebPrecedence(bool);

impl From<bool> for WebPrecedence
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<WebPrecedence> for bool
{
    #[inline]
    fn from(value: WebPrecedence) -> Self
    {
        value.0
    }
}

/// The result of asking whether two web vertices are independent.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WebIndependence(bool);

impl From<bool> for WebIndependence
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<WebIndependence> for bool
{
    #[inline]
    fn from(value: WebIndependence) -> Self
    {
        value.0
    }
}

/// Whether a relation matrix has the shape required by its event list.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WebShapeValid
{
    /// Whether the event list and matrix shapes agree.
    value: bool,
}
impl From<bool> for WebShapeValid
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self { value }
    }
}

impl From<WebShapeValid> for bool
{
    #[inline]
    fn from(value: WebShapeValid) -> Self
    {
        value.value
    }
}

/// A causal web over the canonical events of one recorded tracelet leg.
///
/// The public fields match the design surface: event labels are canonical
/// [`EventKey`]s and `precedes` is the transitive green relation. White
/// independence is obtained through [`Self::relation`] or
/// [`Self::independent`]; there is no conflict relation because one tracelet
/// is one conflict-free run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CausalWeb
{
    /// One vertex per event, ordered by the source order's canonical schedule.
    pub events: Box<[EventKey]>,
    /// The transitive precedence relation over [`Self::events`].
    pub precedes: DependenceBits,
}

impl CausalWeb
{
    /// The number of canonical event vertices.
    #[inline]
    #[must_use]
    pub fn vertex_count(&self) -> WebVertexCount
    {
        WebVertexCount::from(self.events.len())
    }

    /// The canonical event key at `vertex`, when the coordinate is in range.
    #[inline]
    #[must_use]
    pub fn event(
        &self,
        vertex: WebVertex,
    ) -> Option<&EventKey>
    {
        self.events.get(usize::from(vertex))
    }

    /// Read the two-colour relation between two web vertices.
    #[inline]
    #[must_use]
    pub fn relation(
        &self,
        left: WebVertex,
        right: WebVertex,
    ) -> WebRelation
    {
        if self.event(left).is_none() || self.event(right).is_none() || left == right {
            return WebRelation::Missing;
        }
        if bool::from(self.precedes.contains(left, right)) {
            return WebRelation::Precedes;
        }
        if bool::from(self.precedes.contains(right, left)) {
            return WebRelation::Follows;
        }
        WebRelation::Independent
    }

    /// Whether two distinct vertices are in the white independence relation.
    #[inline]
    #[must_use]
    pub fn independent(
        &self,
        left: WebVertex,
        right: WebVertex,
    ) -> WebIndependence
    {
        WebIndependence::from(self.relation(left, right) == WebRelation::Independent)
    }

    /// Check that the public event list and relation matrix have equal shape.
    ///
    /// # Contract
    /// - ensures: returns `true` exactly for a square relation matching the
    ///   event-list length.
    /// - provides: the structural guard used before refinement.
    /// - panics: none.
    fn has_valid_shape(&self) -> WebShapeValid
    {
        self.precedes.has_shape(self.vertex_count())
    }
}

/// Build a causal web from an existing tracelet leg event order.
///
/// The event order is the only source of event identity and pairwise
/// independence. This constructor computes no second dependence relation; it
/// only changes coordinates to the canonical event list and materializes the
/// already-decided transitive precedence relation.
///
/// # Contract
/// - requires: `order` is an [`EventOrder`] returned by the deep-inference
///   causal or normal-form constructors.
/// - ensures: the web contains exactly the order's canonical event keys, and
///   `precedes` agrees with [`EventOrder::precedes`] for every event pair.
/// - provides: the conflict-free two-colour web used by the refinement seam.
/// - panics: none.
/// - intension: quadratic in the number of events, matching the finite web
///   surface rather than introducing a second event-order algorithm.
///
/// # Adequacy
/// - hypothesis: L3 pointwise over an independent pair, a dependent pair, and a
///   one-event boundary order; the differential tracelet suite exercises the
///   same event-order source over generated multi-layer fixtures.
/// - witness: `causal_web::tests::a_tracelet_fixture_builds_the_two_colour_web`
/// - witness: `causal_web::tests::a_dependent_pair_is_a_green_edge`
/// - witness: `causal_web::tests::a_single_event_is_the_boundary_web`
#[inline]
#[must_use]
pub fn causal_web<A>(order: &EventOrder<A>) -> CausalWeb
where
    A: CellAlphabet,
{
    let canonical = order.canonical_order();
    let mut events = Vec::with_capacity(canonical.len());
    for index in &canonical {
        if let Some(key) = order.key(*index) {
            events.push(key);
        }
    }
    CausalWeb {
        events: events.into_boxed_slice(),
        precedes: DependenceBits::from_order(order, &canonical),
    }
}

/// One licensed independence-to-order weakening in a slice chain.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SliceStep
{
    /// The vertex that is earlier after the weakening.
    pub earlier: WebVertex,
    /// The vertex that is later after the weakening.
    pub later: WebVertex,
}

/// Evidence that a finer web is obtained from a coarser web by slice-chain
/// weakenings only.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceChain
{
    /// The ordered licensed weakenings, stored without a second event graph.
    steps: Box<[SliceStep]>,
}

impl SliceChain
{
    /// The licensed independence-to-order steps, in canonical pair order.
    #[inline]
    #[must_use]
    pub fn steps(&self) -> &[SliceStep]
    {
        &self.steps
    }

    /// The number of licensed steps in the witness.
    #[inline]
    #[must_use]
    pub fn step_count(&self) -> SliceStepCount
    {
        SliceStepCount::from(self.steps.len())
    }
}

/// The first pair that prevents a same-vertex-set slice-chain refinement.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RefinementCounterexample
{
    /// The pair's earlier coordinate in the canonical web list.
    pub earlier: WebVertex,
    /// The pair's later coordinate in the canonical web list.
    pub later: WebVertex,
    /// The relation required by the coarser web.
    pub required: WebRelation,
    /// The relation supplied by the finer web.
    pub observed: WebRelation,
}

/// The named frontier where this spike refuses to decide a general graph
/// simulation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HomomorphismFrontier
{
    /// A same-cardinality web needs an event correspondence not provided by
    /// the canonical event labels.
    EdgeStrengtheningSimulation,
    /// A different-cardinality web would require the open h↓ homomorphism
    /// rule, whose cut elimination is not established.
    OpenHDownHomomorphism,
    /// The public fields do not describe a square precedence relation.
    MalformedWeb,
}

/// The result of the licensed causal-web refinement seam.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefinementVerdict
{
    /// The finer web adds only precedence to pairs independent in the coarser
    /// web, with a witness for every added relation.
    Refines
    {
        /// The slice-chain witness.
        witness: SliceChain,
    },
    /// The same labelled vertices disagree with a precedence obligation.
    DoesNotRefine
    {
        /// The first separating pair in canonical order.
        witness: RefinementCounterexample,
    },
    /// The requested comparison leaves the identity-map slice fragment and
    /// would require an undecided graph homomorphism.
    Refused
    {
        /// The named obstruction at the frontier.
        obstruction: HomomorphismFrontier,
    },
}

/// Decide the licensed slice-chain refinement fragment.
///
/// `finer` may replace any white independence relation of `coarser` by one
/// green directed precedence edge. Every precedence obligation already held
/// by `coarser` must remain oriented the same way. The event labels and their
/// cardinality must agree, because changing the vertex correspondence is the
/// refused edge-strengthening simulation / open h↓ homomorphism fragment.
///
/// # Contract
/// - ensures: returns [`RefinementVerdict::Refines`] exactly for the identity
///   correspondence whose only changes are independence-to-order weakenings;
///   returns a separating witness for a lost precedence; and returns a named
///   refusal rather than guessing for a correspondence outside that fragment.
/// - panics: none.
/// - intension: quadratic in the shared vertex count, with no replay or
///   evaluation-path work.
///
/// # Adequacy
/// - hypothesis: L3 pointwise over an empty boundary chain, one licensed added
///   edge, one lost edge, malformed shape, equal-cardinality label mismatch,
///   and a cardinality mismatch.
/// - witness: `causal_web::tests::an_equal_web_is_the_empty_slice_chain`
/// - witness: `causal_web::tests::an_independence_to_order_change_is_licensed`
/// - witness: `causal_web::tests::a_lost_precedence_is_a_negative_witness`
/// - witness: `causal_web::tests::a_malformed_web_refuses_structural_comparison`
/// - witness: `causal_web::tests::a_label_mismatch_refuses_edge_strengthening_simulation`
/// - witness: `causal_web::tests::a_cardinality_mismatch_refuses_open_h_down`
#[inline]
#[must_use]
pub fn refines(
    finer: &CausalWeb,
    coarser: &CausalWeb,
) -> RefinementVerdict
{
    if !bool::from(finer.has_valid_shape()) || !bool::from(coarser.has_valid_shape()) {
        return RefinementVerdict::Refused {
            obstruction: HomomorphismFrontier::MalformedWeb,
        };
    }
    if finer.events.len() != coarser.events.len() {
        return RefinementVerdict::Refused {
            obstruction: HomomorphismFrontier::OpenHDownHomomorphism,
        };
    }
    if finer.events != coarser.events {
        return RefinementVerdict::Refused {
            obstruction: HomomorphismFrontier::EdgeStrengtheningSimulation,
        };
    }

    let count = finer.events.len();
    let mut steps = Vec::new();
    for left in 0 .. count {
        for right in left.saturating_add(1) .. count {
            let left = WebVertex::from(left);
            let right = WebVertex::from(right);
            let required = coarser.relation(left, right);
            let observed = finer.relation(left, right);
            match required {
                | WebRelation::Precedes => {
                    if observed != WebRelation::Precedes {
                        return RefinementVerdict::DoesNotRefine {
                            witness: RefinementCounterexample {
                                earlier: left,
                                later: right,
                                required,
                                observed,
                            },
                        };
                    }
                },
                | WebRelation::Follows => {
                    if observed != WebRelation::Follows {
                        return RefinementVerdict::DoesNotRefine {
                            witness: RefinementCounterexample {
                                earlier: right,
                                later: left,
                                required,
                                observed,
                            },
                        };
                    }
                },
                | WebRelation::Independent => match observed {
                    | WebRelation::Precedes => steps.push(SliceStep {
                        earlier: left,
                        later: right,
                    }),
                    | WebRelation::Follows => steps.push(SliceStep {
                        earlier: right,
                        later: left,
                    }),
                    | WebRelation::Independent => {},
                    | WebRelation::Missing => {
                        return RefinementVerdict::DoesNotRefine {
                            witness: RefinementCounterexample {
                                earlier: left,
                                later: right,
                                required,
                                observed,
                            },
                        };
                    },
                },
                | WebRelation::Missing => {
                    return RefinementVerdict::Refused {
                        obstruction: HomomorphismFrontier::MalformedWeb,
                    };
                },
            }
        }
    }
    RefinementVerdict::Refines {
        witness: SliceChain {
            steps: steps.into_boxed_slice(),
        },
    }
}

#[cfg(test)]
mod tests
{
    use alloc::vec;
    use alloc::vec::Vec;

    use gandr_theory_cell_complexes::Cell;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyAlphabet;
    use gandr_theory_cell_complexes_tools::toy::ToyNameRef;
    use gandr_theory_cell_complexes_tools::toy::ToyPos;
    use gandr_theory_cell_complexes_tools::toy::toy_cell;
    use gandr_theory_coherent_resolutions::CellApp;

    use super::*;
    use crate::event_order;

    fn at<Steps>(steps: Steps) -> ToyPos
    where
        Steps: IntoIterator<Item = usize>,
    {
        ToyPos(steps.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    fn root() -> ToyPos
    {
        ToyPos(Vec::new().into_boxed_slice())
    }

    fn two_event_order(cell: Cell<ToyAlphabet>) -> EventOrder<ToyAlphabet>
    {
        let mut store = CellStore::new();
        let cell = store.insert(cell);
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero));
        let path = vec![CellApp { cell, at: at([0]) }, CellApp { cell, at: at([1]) }];
        event_order(&store, &peak, &path).expect("the two-event fixture replays")
    }

    fn one_event_order(cell: Cell<ToyAlphabet>) -> EventOrder<ToyAlphabet>
    {
        let mut store = CellStore::new();
        let cell = store.insert(cell);
        let peak = Toy::succ(Toy::Zero);
        let path = vec![CellApp { cell, at: root() }];
        event_order(&store, &peak, &path).expect("the one-event fixture replays")
    }
    fn dependent_order() -> EventOrder<ToyAlphabet>
    {
        let mut store = CellStore::new();
        let cell = store.insert(toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::var(ToyNameRef("x")),
        ));
        let peak = Toy::add(Toy::Zero, Toy::add(Toy::Zero, Toy::Zero));
        let path = vec![CellApp { cell, at: root() }, CellApp { cell, at: root() }];
        event_order(&store, &peak, &path).expect("the dependent two-event fixture replays")
    }

    fn f_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::Zero), Toy::Zero)
    }

    fn alternate_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::Zero), Toy::succ(Toy::succ(Toy::Zero)))
    }

    #[test]
    fn a_tracelet_fixture_builds_the_two_colour_web()
    {
        let order = two_event_order(f_cell());
        let web = causal_web(&order);
        let left = WebVertex::from(0_usize);
        let right = WebVertex::from(1_usize);
        assert_eq!(WebVertexCount::from(2_usize), web.vertex_count());
        assert_eq!(WebRelation::Independent, web.relation(left, right));
        assert_eq!(WebRelation::Independent, web.relation(right, left));
        assert_eq!(WebRelation::Missing, web.relation(left, left));
        assert_eq!(
            WebRelation::Missing,
            web.relation(left, WebVertex::from(2_usize))
        );
        assert_eq!(2_usize, web.events.len());
    }

    #[test]
    fn a_dependent_pair_is_a_green_edge()
    {
        let order = dependent_order();
        let web = causal_web(&order);
        let left = WebVertex::from(0_usize);
        let right = WebVertex::from(1_usize);
        assert_eq!(WebVertexCount::from(2_usize), web.vertex_count());
        assert_eq!(WebRelation::Precedes, web.relation(left, right));
        assert_eq!(WebRelation::Follows, web.relation(right, left));
    }
    #[test]
    fn a_single_event_is_the_boundary_web()
    {
        let order = one_event_order(f_cell());
        let web = causal_web(&order);
        let key = order.key(order.canonical_order()[0]);
        assert_eq!(key.as_ref(), web.event(WebVertex::from(0_usize)));
    }

    #[test]
    fn an_equal_web_is_the_empty_slice_chain()
    {
        let web = causal_web(&two_event_order(f_cell()));
        let RefinementVerdict::Refines { witness } = refines(&web, &web)
        else {
            panic!("an identical web is the boundary refinement");
        };
        assert_eq!(SliceStepCount::from(0_usize), witness.step_count());
    }

    #[test]
    fn an_independence_to_order_change_is_licensed()
    {
        let coarser = causal_web(&two_event_order(f_cell()));
        let mut precedes = coarser.precedes.clone();
        precedes.rows[0][1] = true;
        let finer = CausalWeb {
            events: coarser.events.clone(),
            precedes,
        };
        let RefinementVerdict::Refines { witness } = refines(&finer, &coarser)
        else {
            panic!("an independence-to-order change is the sl fragment");
        };
        assert_eq!(SliceStepCount::from(1_usize), witness.step_count());
        assert_eq!(
            Some(&SliceStep {
                earlier: WebVertex::from(0_usize),
                later: WebVertex::from(1_usize),
            }),
            witness.steps().first(),
        );
    }

    #[test]
    fn a_lost_precedence_is_a_negative_witness()
    {
        let independent = causal_web(&two_event_order(f_cell()));
        let mut coarser_precedes = independent.precedes.clone();
        coarser_precedes.rows[0][1] = true;
        let coarser = CausalWeb {
            events: independent.events.clone(),
            precedes: coarser_precedes,
        };
        let RefinementVerdict::DoesNotRefine { witness } = refines(&independent, &coarser)
        else {
            panic!("a lost precedence must remain a negative verdict");
        };
        assert_eq!(WebVertex::from(0_usize), witness.earlier);
        assert_eq!(WebVertex::from(1_usize), witness.later);
        assert_eq!(WebRelation::Precedes, witness.required);
        assert_eq!(WebRelation::Independent, witness.observed);
    }

    #[test]
    fn a_label_mismatch_refuses_edge_strengthening_simulation()
    {
        let coarser = causal_web(&two_event_order(f_cell()));
        let finer = causal_web(&two_event_order(alternate_cell()));
        assert_eq!(
            RefinementVerdict::Refused {
                obstruction: HomomorphismFrontier::EdgeStrengtheningSimulation,
            },
            refines(&finer, &coarser),
        );
    }

    #[test]
    fn a_cardinality_mismatch_refuses_open_h_down()
    {
        let finer = causal_web(&one_event_order(f_cell()));
        let coarser = causal_web(&two_event_order(f_cell()));
        assert_eq!(
            RefinementVerdict::Refused {
                obstruction: HomomorphismFrontier::OpenHDownHomomorphism,
            },
            refines(&finer, &coarser),
        );
    }
    #[test]
    fn a_malformed_web_refuses_structural_comparison()
    {
        let source = causal_web(&two_event_order(f_cell()));
        let malformed = CausalWeb {
            events: Vec::new().into_boxed_slice(),
            precedes: source.precedes.clone(),
        };
        assert_eq!(
            RefinementVerdict::Refused {
                obstruction: HomomorphismFrontier::MalformedWeb,
            },
            refines(&malformed, &source),
        );
    }
}
