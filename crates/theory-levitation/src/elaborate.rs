//! **Elaborating a circuit block** into the boundary language.
//!
//! A rewrite-sorted port becomes the interface pair it binds, and the block's
//! redex becomes the whiskered composite it makes inside the block's frames
//! (`spec:surface-language/circuit-cells.md`, sections "Sorting a
//! rewrite port" and "Frame and redex";
//! `spec:surface-language/higher-cells.md`, section "The boundary
//! language").
//!
//! # A rewrite-sorted port is sorted by the face, and binds an interface pair
//!
//! A port ranging over rewrites is sorted by the **2-cell face at the level's
//! own arrow** — `rule p : Nat ==> Nat` — and the sort is the boundary *sort*,
//! never the boundary *terms*. Elaborating that binder binds the triple
//! `(a, b, ρ : a ==> b)`: an [`InterfacePair`], the same shape a hole carries,
//! since a hole *is* its interface pair. The pinned form `rule p : x ==> x′`
//! stays available when the interface should name its endpoints, and then `a`
//! and `b` are the terms the declaration writes ([`PortFace`]).
//!
//! The rejected candidate is recorded with the adopted one: sorting a rewrite
//! port by the `Model` field's `Path(Nat, x, x′)` writes the interpretation
//! into the syntax and silently makes a directed rewrite invertible. Sorting by
//! the face keeps one spelling across the directed family.
//!
//! # Instantiating a port is one match and one binding
//!
//! `node : p(x) ==> (x′)` **unifies the source** — the port's source pattern
//! variables take the line's input wiring in declaration order, which is what
//! the boundary language's `r(t₁, …, tₙ)` already means — and **binds the
//! target**: an endpoint the source does not bind is the opaque endpoint the
//! wire itself supplies, so it becomes the line's output port. The result is
//! the [`CircuitRedex`] the boundary derivation already consumes
//! ([`InterfacePair::instantiate`]).
//!
//! # A block elaborates to a whiskered composite
//!
//! A redex applied inside a frame *is* the boundary language's whiskering
//! `f(t̄, ρ, ū)`, and congruence is the free coherence of parallel composition:
//! every whiskering is a degenerate two-sided congruence with one side. So
//! elaborating a block produces [`WhiskeredCell`] — a chain of frame
//! applications around one redex leaf — against the boundaries
//! [`derive_boundaries`] already fixes.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;

use crate::boundary::CircuitNodeBudget;
use crate::boundary::PortArgumentCount;
use crate::boundary::TermPositionIndex;
use crate::circuit::CircuitBody;
use crate::circuit::CircuitDerivationError;
use crate::circuit::CircuitNode;
use crate::circuit::CircuitRedex;
use crate::circuit::DerivedBoundaries;
use crate::circuit::FrameHead;
use crate::circuit::derive_boundaries;
use crate::code::Name;
use crate::rule::FreeTerm;

/// The **face** a rewrite-sorted port is declared at — what its declaration
/// writes between the two arrowheads.
///
/// The two forms are the ruled spellings: the sorted form writes the boundary
/// sort, the pinned form writes the endpoint terms. Pinning is all-or-nothing,
/// because one arrow cannot carry a sort on one side and a term on the other.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PortFace
{
    /// The **sorted** form `rule p : Nat ==> Nat` — the boundary sort on both
    /// sides, and no endpoint terms.
    Sorted(Name),
    /// The **pinned** form `rule p : x ==> x′` — the declaration names its own
    /// endpoints, and binding pinned rewrites in the parameter telescope is
    /// what lets a congruence cell's body shrink to its frame.
    Pinned
    {
        /// The declared source endpoint.
        source: FreeTerm,
        /// The declared target endpoint.
        target: FreeTerm,
    },
}

/// Which endpoint of a sorted port's interface pair a minted variable names.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum InterfaceRole
{
    /// The source endpoint `a` of `ρ : a ==> b`.
    Source,
    /// The target endpoint `b` of `ρ : a ==> b`.
    Target,
}

/// The variable a sorted port's interface pair carries at one endpoint.
///
/// The sorted form writes no endpoint term, so the endpoints are variables the
/// elaboration mints. They are spelled with mathematical angle brackets, which
/// the surface's identifier lexis cannot produce, so a minted endpoint can
/// never be captured by a body port of the same name. They are also transient:
/// [`InterfacePair::instantiate`] substitutes the source endpoint away and
/// replaces the target endpoint by the line's output port, so no minted name
/// reaches a [`CircuitRedex`] or a derived boundary.
fn interface_variable(
    port: &Name,
    role: InterfaceRole,
) -> Name
{
    let role = match role {
        | InterfaceRole::Source => "source",
        | InterfaceRole::Target => "target",
    };
    format!("{port}\u{27e8}{role}\u{27e9}").into()
}

/// A **rewrite-sorted port** of a circuit rule's parameter telescope — the
/// binder `rule p : Nat ==> Nat`, whose inhabitant is a rewrite.
///
/// A redex line applies this port by name; the port is *not* a signature
/// symbol, which is why the declaration table checks a redex head against the
/// telescope rather than against the datatype's constructors and operations.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RewritePort
{
    /// The port's name — the head a redex line applies.
    pub name: Name,
    /// The face the port is sorted by.
    pub face: PortFace,
}

impl RewritePort
{
    /// A port in the **sorted** form `rule name : sort ==> sort`.
    #[inline]
    #[must_use]
    pub fn sorted<N, S>(
        name: N,
        sort: S,
    ) -> Self
    where
        N: Into<Name>,
        S: Into<Name>,
    {
        Self {
            name: name.into(),
            face: PortFace::Sorted(sort.into()),
        }
    }

    /// A port in the **pinned** form `rule name : source ==> target`.
    #[inline]
    #[must_use]
    pub fn pinned<N>(
        name: N,
        source: FreeTerm,
        target: FreeTerm,
    ) -> Self
    where
        N: Into<Name>,
    {
        Self {
            name: name.into(),
            face: PortFace::Pinned { source, target },
        }
    }

    /// The **interface pair** this port binds — the triple `(a, b, ρ : a ==>
    /// b)`.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: the pinned form's pair is the two terms its declaration
    ///   writes; the sorted form's pair is two distinct minted endpoint
    ///   variables, because the boundary sort names no terms.
    /// - provides: the binding an instantiating redex line matches against.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 only — the two forms are the whole decision surface,
    ///   and they are separated by whether the pair carries the declaration's
    ///   own terms; the sorted arm additionally has to keep its two endpoints
    ///   apart, or the target would be bound by the source.
    /// - witness: `elaborate::tests::a_sorted_port_binds_two_distinct_endpoints`
    /// - witness: `elaborate::tests::a_pinned_port_binds_the_terms_it_writes`
    #[inline]
    #[must_use]
    pub fn interface(&self) -> InterfacePair
    {
        match self.face {
            | PortFace::Sorted(_) => InterfacePair::new(
                self.name.clone(),
                FreeTerm::Var(interface_variable(&self.name, InterfaceRole::Source)),
                FreeTerm::Var(interface_variable(&self.name, InterfaceRole::Target)),
            ),
            | PortFace::Pinned {
                ref source,
                ref target,
            } => InterfacePair::new(self.name.clone(), source.clone(), target.clone()),
        }
    }
}

/// The **interface pair** a rewrite binds at one place: the triple
/// `(a, b, ρ : a ==> b)`.
///
/// This is the shape a hole carries — a context is an object of `Cᵒᵖ × C`, a
/// pair `(X / Y)` for a hole admitting a process from `X` to `Y` — and it is
/// what a rewrite-sorted port elaborates to ([`RewritePort::interface`]) and
/// what the active cell of a whiskered composite is
/// ([`WhiskeredCell::Redex`]).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct InterfacePair
{
    /// The rewrite `ρ` the pair is the interface of.
    pub rewrite: Name,
    /// The source endpoint `a`.
    pub source: FreeTerm,
    /// The target endpoint `b`.
    pub target: FreeTerm,
}

impl InterfacePair
{
    /// The pair `(source, target, rewrite : source ==> target)`.
    #[inline]
    #[must_use]
    pub fn new<R>(
        rewrite: R,
        source: FreeTerm,
        target: FreeTerm,
    ) -> Self
    where
        R: Into<Name>,
    {
        Self {
            rewrite: rewrite.into(),
            source,
            target,
        }
    }

    /// **Instantiate** this interface pair at a redex line: unify the source
    /// against the line's input wiring, and bind the target to its output port.
    ///
    /// `node : p(x) ==> (x′)` supplies `args = [x]` and `out = x′`. The
    /// source's pattern variables take the arguments in declaration order —
    /// the boundary language's own reading of `r(t₁, …, tₙ)` — and the
    /// resulting substitution is applied to both endpoints. A target
    /// endpoint the source does not bind is the **opaque endpoint**: the
    /// declaration names no term for it, so the wire does, and the line's
    /// output port becomes the target.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: on success the redex's source is this pair's source under the
    ///   substitution binding its distinct source variables to `args` in
    ///   first-occurrence order; its target is this pair's target under that
    ///   same substitution when the source binds every target variable, and
    ///   `out` when the target is a lone variable the source does not bind.
    /// - provides: the [`CircuitRedex`] a body statement holds, so a
    ///   rewrite-sorted port and the boundary derivation meet at one type.
    /// - fails: [`PortInstantiationError::SourceArity`] when the line supplies
    ///   a number of arguments other than the source's distinct variable count;
    ///   [`PortInstantiationError::UnboundTargetEndpoint`] when the target
    ///   names endpoints the source does not bind and is not itself the lone
    ///   opaque endpoint the output port supplies.
    /// - panics: none.
    /// - intension: source variables are bound in **first-occurrence,
    ///   left-to-right** order, so a repeated source variable consumes one
    ///   argument rather than two and a linear source's arguments read in
    ///   surface order.
    ///
    /// # Errors
    /// See [`PortInstantiationError`].
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the two endpoint arms and the two declines are
    ///   separated pointwise: an unpinned pair leaves the target opaque and so
    ///   reaches `out`, a pinned pair whose target the source binds reaches the
    ///   substituted term, a short argument list reaches the arity decline, and
    ///   a target naming an endpoint neither the source nor the wire supplies
    ///   reaches the endpoint decline.
    /// - witness: `elaborate::tests::instantiating_a_port_unifies_the_source_and_binds_the_target`
    /// - witness: `elaborate::tests::instantiating_a_pinned_port_substitutes_into_its_target`
    /// - witness: `elaborate::tests::a_repeated_source_variable_consumes_one_argument`
    /// - witness: `elaborate::tests::an_instantiation_that_does_not_unify_declines`
    /// - witness: `elaborate::tests::a_target_endpoint_no_wire_supplies_declines`
    #[inline]
    pub fn instantiate<A, N>(
        &self,
        args: A,
        out: N,
    ) -> Result<CircuitRedex, PortInstantiationError>
    where
        A: Into<Box<[FreeTerm]>>,
        N: Into<Name>,
    {
        let args = args.into();
        let out = out.into();
        let vars = distinct_vars(&self.source);
        if vars.len() != args.len() {
            return Err(PortInstantiationError::SourceArity {
                rewrite: self.rewrite.clone(),
                expected: vars.len().into(),
                supplied: args.len().into(),
            });
        }
        let bindings: BTreeMap<Name, FreeTerm> = vars.into_iter().zip(args.into_vec()).collect();
        let source = substitute(&self.source, &bindings);
        let unbound: Vec<Name> = distinct_vars(&self.target)
            .into_iter()
            .filter(|name| !bindings.contains_key(name))
            .collect();
        let target = if unbound.is_empty() {
            substitute(&self.target, &bindings)
        }
        else if matches!(self.target, FreeTerm::Var(_)) {
            // The opaque endpoint: the declaration names no term for the
            // target, so the wire is the target.
            FreeTerm::Var(out.clone())
        }
        else {
            return Err(PortInstantiationError::UnboundTargetEndpoint {
                rewrite: self.rewrite.clone(),
                endpoints: unbound.into(),
            });
        };
        Ok(CircuitRedex::new(self.rewrite.clone(), source, target, out))
    }
}

/// Why a redex line does not instantiate the port it applies.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PortInstantiationError
{
    /// The line's input wiring does not match the port's source interface: the
    /// source has a different number of distinct pattern variables than the
    /// line supplies arguments.
    SourceArity
    {
        /// The applied rewrite.
        rewrite: Name,
        /// The source interface's distinct variable count.
        expected: PortArgumentCount,
        /// The number of arguments the line supplied.
        supplied: PortArgumentCount,
    },
    /// The declared target names endpoints the source does not bind, and the
    /// target is not the lone opaque endpoint the output port supplies — so
    /// there is no wire for those endpoints to come from. One redex line binds
    /// exactly one output port; a many-out node is the cell-alphabet question,
    /// which this route declines rather than omits.
    UnboundTargetEndpoint
    {
        /// The applied rewrite.
        rewrite: Name,
        /// The target endpoints the source does not bind, in first-occurrence
        /// order.
        endpoints: Box<[Name]>,
    },
}

/// A term of the **boundary language**.
///
/// This is the composite a circuit block's wiring makes out of its redex and
/// its frames (`spec:surface-language/higher-cells.md`, section "The
/// boundary language").
///
/// Three of the language's four constructions are built here. `here(t)` is the
/// identity rewrite, `r(t̄)` the rule instantiation — carried as the
/// [`InterfacePair`] the application sits between, which is `r` together with
/// the two boundary terms its arguments resolve to — and `f(t̄, ρ, ū)` the
/// whiskering. The fourth, `ρ then ρ′`, is what a body with more than one redex
/// occurrence needs, and it is what [`elaborate_body`] declines rather than
/// builds.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum WhiskeredCell
{
    /// `here(t)` — the identity (empty) rewrite at a term, which is what a body
    /// with no redex occurrence elaborates to.
    Here(FreeTerm),
    /// `r(t̄)` — the applied rewrite, carried as the interface pair it spans
    /// once the wiring has resolved both of its boundary terms.
    Redex(InterfacePair),
    /// `f(t̄, ρ, ū)` — whiskering: the composite `ρ` applied in one argument
    /// position of a constructor or operation, with the unchanged arguments on
    /// either side.
    Whisker
    {
        /// The frame's head and the alphabet it is declared in.
        head: FrameHead,
        /// The unchanged arguments left of the active position (`t̄`).
        before: Box<[FreeTerm]>,
        /// The composite at the active position (`ρ`).
        inner: Box<Self>,
        /// The unchanged arguments right of the active position (`ū`).
        after: Box<[FreeTerm]>,
    },
}

impl WhiskeredCell
{
    /// The composite's **active position** — the path of argument indices from
    /// the root of the derived boundary down to the redex.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: `None` for a composite whose innermost node is
    ///   [`WhiskeredCell::Here`], which has no active cell; otherwise the
    ///   argument index of each enclosing [`WhiskeredCell::Whisker`], outermost
    ///   first, so the empty path is a redex at the root.
    /// - provides: the position the shift-equivalence guard's incomparability
    ///   conjunct is asked about.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 only — the decision surface is the chain walk and the
    ///   identity case, separated by a two-level whisker (which pins that the
    ///   index is the count of unchanged arguments to the left, not a constant)
    ///   and by an identity composite.
    /// - witness: `elaborate::tests::a_nested_whisker_reports_its_argument_path`
    /// - witness: `elaborate::tests::an_identity_composite_has_no_active_position`
    #[inline]
    #[must_use]
    pub fn active_position(&self) -> Option<Vec<TermPositionIndex>>
    {
        let mut path = Vec::new();
        let mut cursor = self;
        loop {
            match *cursor {
                | Self::Here(_) => return None,
                | Self::Redex(_) => return Some(path),
                | Self::Whisker {
                    ref before,
                    ref inner,
                    ..
                } => {
                    path.push(before.len().into());
                    cursor = inner;
                },
            }
        }
    }
}

/// One redex **occurrence** in a block's derived boundary — the applied rewrite
/// and where the wiring plants it.
///
/// An occurrence is a position in the boundary term, not a body statement: a
/// wire consumed twice is unfolded twice, so one redex line reconverging into
/// two argument slots is two occurrences.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RedexOccurrence
{
    /// The applied rewrite.
    pub rewrite: Name,
    /// The path of argument indices from the boundary's root to this
    /// occurrence.
    pub position: Box<[TermPositionIndex]>,
}

impl RedexOccurrence
{
    /// An occurrence of `rewrite` at `position`.
    #[inline]
    #[must_use]
    pub fn new<R, P>(
        rewrite: R,
        position: P,
    ) -> Self
    where
        R: Into<Name>,
        P: Into<Box<[TermPositionIndex]>>,
    {
        Self {
            rewrite: rewrite.into(),
            position: position.into(),
        }
    }
}

/// Why a circuit block elaborates to no composite.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CircuitElaborationError
{
    /// The wiring derives no boundary pair, so there is nothing to whisker
    /// into.
    Derivation(CircuitDerivationError),
    /// The declared output port unfolds to **more than one** redex occurrence,
    /// so the composite is not a single whiskering.
    ///
    /// The occurrences are carried with their positions, because the position
    /// order is what says which composite is owed. Two occurrences at
    /// **incomparable** positions are horizontal composition, which is licensed
    /// exactly on disjoint positions where the two readings are shift-equal and
    /// never any earlier or any wider — the earned witness, not this
    /// elaboration, is what grants it. Two at **comparable** positions are
    /// sequential composition, which the boundary language spells `ρ then ρ′`.
    ManyRedexOccurrences
    {
        /// Every occurrence, in the order the source reading reaches them.
        occurrences: Box<[RedexOccurrence]>,
    },
    /// The occurrence walk reached a position the derived boundary does not
    /// address.
    ///
    /// The walk and the derivation are separate traversals of the same wiring,
    /// so a disagreement between them is reported rather than assumed away.
    PositionOffBoundary
    {
        /// The rewrite whose occurrence could not be located.
        rewrite: Name,
        /// The path that ran off the boundary term.
        position: Box<[TermPositionIndex]>,
    },
}

/// **Elaborate** a circuit block into the boundary language: the whiskered
/// composite of its redex inside its frames.
///
/// A redex applied inside a frame is the boundary language's whiskering
/// `f(t̄, ρ, ū)`, and congruence is the free coherence of parallel composition —
/// every whiskering is a degenerate two-sided congruence with one side. So a
/// block whose declared output port unfolds through frames to one redex
/// elaborates to that redex wrapped in one [`WhiskeredCell::Whisker`] per
/// enclosing application, and a block with no redex at all elaborates to
/// `here(t)` at the term its frames build.
///
/// # The position structure this composite exposes
///
/// **The composite exposes exactly one active position, and it is a path of
/// argument indices.** A whiskered composite is a chain of frame applications
/// ending in one redex leaf, so the redex occupies the path obtained by reading
/// each whisker's argument index from the root
/// ([`WhiskeredCell::active_position`]), and that path addresses the redex's
/// source in the derived source boundary and its target in the derived target
/// boundary. It is the only position at which the composite is not an identity.
/// A body whose declared output port unfolds to two or more redex occurrences
/// therefore has no single-position composite here: it is declined by
/// [`CircuitElaborationError::ManyRedexOccurrences`], which carries every
/// occurrence's rewrite name and position path. Two occurrences at
/// **incomparable** positions are the horizontal composite that is licensed
/// only against an earned shift-equivalence witness; two at **comparable**
/// positions are sequential composition, spelled `ρ then ρ′`. Reconvergence
/// produces two occurrences of the *same* redex, so a wire consumed twice is
/// two positions rather than one.
///
/// # Contract
/// - requires: none.
/// - ensures: a body with no redex occurrence elaborates to
///   [`WhiskeredCell::Here`] at its derived source boundary; a body with
///   exactly one elaborates to that occurrence's [`InterfacePair`] — its source
///   read from the derived source boundary and its target from the derived
///   target boundary — wrapped in one whisker per enclosing application, whose
///   unchanged arguments are that application's other arguments.
/// - provides: the boundary-language composite a circuit rule's filler denotes,
///   against the boundaries the sphere check already fixed.
/// - fails: [`CircuitElaborationError::Derivation`] when the wiring derives no
///   boundary pair; [`CircuitElaborationError::ManyRedexOccurrences`] when the
///   composite would hold more than one redex occurrence;
///   [`CircuitElaborationError::PositionOffBoundary`] when the occurrence walk
///   and the derivation disagree about the boundary's shape.
/// - panics: none.
/// - intension: the occurrence order is the source reading's own left-to-right
///   unfolding of the declared output port, so the declined report reads in
///   diagram order.
///
/// # Errors
/// See [`CircuitElaborationError`].
///
/// # Adequacy
/// - hypothesis: L3 — the three arms are separated pointwise: a frames-only
///   body reaches `here`, the single-redex congruence body reaches a whisker
///   whose unchanged argument is the frame's other operand, and the two-redex
///   `cong2` body reaches the decline with both positions; the nesting
///   direction is pinned by a two-level frame, where an outermost-first and an
///   innermost-first fold differ.
/// - witness: `elaborate::tests::a_single_redex_block_elaborates_to_a_whiskered_cell`
/// - witness: `elaborate::tests::a_redex_under_two_frames_whiskers_outermost_first`
/// - witness: `elaborate::tests::a_redex_at_the_root_needs_no_whisker`
/// - witness: `elaborate::tests::a_body_with_no_redex_is_the_identity_rewrite`
/// - witness: `elaborate::tests::two_disjoint_redexes_decline_with_incomparable_positions`
/// - witness: `elaborate::tests::a_reconvergent_redex_is_two_occurrences_of_one_rewrite`
/// - witness: `elaborate::tests::a_cyclic_wiring_elaborates_to_nothing`
#[inline]
pub fn elaborate_body(body: &CircuitBody) -> Result<WhiskeredCell, CircuitElaborationError>
{
    let derived = derive_boundaries(body).map_err(CircuitElaborationError::Derivation)?;
    let mut occurrences = redex_occurrences(body).map_err(CircuitElaborationError::Derivation)?;
    let Some(occurrence) = occurrences.pop()
    else {
        return Ok(WhiskeredCell::Here(derived.source));
    };
    if !occurrences.is_empty() {
        occurrences.push(occurrence);
        return Err(CircuitElaborationError::ManyRedexOccurrences {
            occurrences: occurrences.into(),
        });
    }
    whisker_along(&derived, &occurrence)
}

/// One frame level of a whisker chain under construction — the head and the
/// unchanged arguments on either side of the active slot.
struct WhiskerFrame
{
    /// The frame's head and the alphabet it is declared in.
    head: FrameHead,
    /// The unchanged arguments left of the active slot (`t̄`).
    before: Box<[FreeTerm]>,
    /// The unchanged arguments right of the active slot (`ū`).
    after: Box<[FreeTerm]>,
}

/// Build the whisker chain addressing `occurrence` in the derived boundary
/// pair.
///
/// # Contract
/// - requires: none.
/// - ensures: one [`WhiskeredCell::Whisker`] per application the position
///   descends through, outermost first, around a [`WhiskeredCell::Redex`] whose
///   pair is the two boundaries' subterms at that position.
/// - fails: [`CircuitElaborationError::PositionOffBoundary`] when either
///   boundary does not address the position.
/// - panics: none.
fn whisker_along(
    derived: &DerivedBoundaries,
    occurrence: &RedexOccurrence,
) -> Result<WhiskeredCell, CircuitElaborationError>
{
    let off_boundary = || CircuitElaborationError::PositionOffBoundary {
        rewrite: occurrence.rewrite.clone(),
        position: occurrence.position.clone(),
    };
    let mut frames: Vec<WhiskerFrame> = Vec::new();
    let mut source = &derived.source;
    let mut target = &derived.target;
    for &step in &occurrence.position {
        let step = usize::from(step);
        let (head, args) = match *source {
            | FreeTerm::Ctor(ref name, ref args) => (FrameHead::Ctor(name.clone()), args),
            | FreeTerm::Op(ref name, ref args) => (FrameHead::Op(name.clone()), args),
            | FreeTerm::Var(_) => return Err(off_boundary()),
        };
        let Some(child) = args.get(step)
        else {
            return Err(off_boundary());
        };
        frames.push(WhiskerFrame {
            head,
            before: args.iter().take(step).cloned().collect(),
            after: args.iter().skip(step.saturating_add(1)).cloned().collect(),
        });
        source = child;
        let (FreeTerm::Ctor(_, ref args) | FreeTerm::Op(_, ref args)) = *target
        else {
            return Err(off_boundary());
        };
        let Some(child) = args.get(step)
        else {
            return Err(off_boundary());
        };
        target = child;
    }
    let mut cell = WhiskeredCell::Redex(InterfacePair::new(
        occurrence.rewrite.clone(),
        source.clone(),
        target.clone(),
    ));
    for frame in frames.into_iter().rev() {
        cell = WhiskeredCell::Whisker {
            head: frame.head,
            before: frame.before,
            inner: Box::new(cell),
            after: frame.after,
        };
    }
    Ok(cell)
}

/// Enumerate the redex occurrences the declared output port unfolds to, each
/// with its position in the derived boundary.
///
/// # Contract
/// - requires: none.
/// - ensures: one entry per redex occurrence reached by the source reading's
///   unfolding of the declared output port, in left-to-right order, positioned
///   by the argument-index path the unfolding took; a wire consumed twice
///   contributes two entries.
/// - fails: [`CircuitDerivationError::CyclicWiring`] on a port reachable from
///   itself other than as a redex's own opaque endpoint — the same guard the
///   derivation runs, so the walk stays total on a wiring the derivation
///   refuses; [`CircuitDerivationError::NodeBudget`] on a wiring whose
///   unfolding passes [`CircuitNodeBudget::DEFAULT`], which is the same ceiling
///   the derivation runs under and is charged here too because this is a
///   *second* traversal of the same reconvergence.
/// - panics: none.
fn redex_occurrences(body: &CircuitBody) -> Result<Vec<RedexOccurrence>, CircuitDerivationError>
{
    /// One step of the occurrence worklist.
    enum Walk<'body>
    {
        /// Resolve a port through the node that binds it.
        Port(&'body Name),
        /// Walk a term, resolving its port leaves.
        Term(&'body FreeTerm),
        /// Enter an argument slot.
        Descend(TermPositionIndex),
        /// Leave an argument slot.
        Ascend,
        /// The port's subtree is walked: take it off the active path.
        Leave(&'body Name),
    }

    /// Queue every argument of an application, left to right, under its index.
    fn push_args<'body>(
        stack: &mut Vec<Walk<'body>>,
        args: &'body [FreeTerm],
    )
    {
        for (index, arg) in args.iter().enumerate().rev() {
            stack.push(Walk::Ascend);
            stack.push(Walk::Term(arg));
            stack.push(Walk::Descend(index.into()));
        }
    }

    let mut producers: BTreeMap<&Name, &CircuitNode> = BTreeMap::new();
    for node in &body.nodes {
        producers.entry(node.out()).or_insert(node);
    }

    let ceiling = usize::from(CircuitNodeBudget::DEFAULT);
    let mut visited: usize = 0;
    let mut path: BTreeSet<&Name> = BTreeSet::new();
    let mut position: Vec<TermPositionIndex> = Vec::new();
    let mut occurrences: Vec<RedexOccurrence> = Vec::new();
    let mut stack: Vec<Walk<'_>> = vec![Walk::Port(&body.out)];
    while let Some(step) = stack.pop() {
        // The same node-visit charge the derivation makes, for the same reason:
        // this walk is a second traversal of one wiring and inherits its
        // reconvergence cost, so it inherits its ceiling rather than running
        // unbounded beside a bounded one.
        if matches!(step, Walk::Port(_) | Walk::Term(_)) {
            visited = visited.saturating_add(1);
            if visited > ceiling {
                return Err(CircuitDerivationError::NodeBudget {
                    budget: CircuitNodeBudget::DEFAULT,
                });
            }
        }
        match step {
            | Walk::Port(port) => {
                let Some(node) = producers.get(port).copied()
                else {
                    // An interface port: a boundary variable, and no rewrite
                    // happens at it.
                    continue;
                };
                match *node {
                    | CircuitNode::Redex(ref redex) => {
                        occurrences.push(RedexOccurrence::new(
                            redex.rewrite.clone(),
                            position.clone(),
                        ));
                        if matches!(redex.source, FreeTerm::Var(ref name) if name == port) {
                            continue;
                        }
                        if !path.insert(port) {
                            return Err(CircuitDerivationError::CyclicWiring(port.clone()));
                        }
                        stack.push(Walk::Leave(port));
                        stack.push(Walk::Term(&redex.source));
                    },
                    | CircuitNode::Frame(ref frame) => {
                        if !path.insert(port) {
                            return Err(CircuitDerivationError::CyclicWiring(port.clone()));
                        }
                        stack.push(Walk::Leave(port));
                        push_args(&mut stack, &frame.args);
                    },
                }
            },
            | Walk::Term(term) => match *term {
                | FreeTerm::Var(ref name) => stack.push(Walk::Port(name)),
                | FreeTerm::Ctor(_, ref args) | FreeTerm::Op(_, ref args) => {
                    push_args(&mut stack, args);
                },
            },
            | Walk::Descend(index) => position.push(index),
            | Walk::Ascend => {
                position.pop();
            },
            | Walk::Leave(port) => {
                path.remove(port);
            },
        }
    }
    Ok(occurrences)
}

/// A term's distinct variable leaves, in first-occurrence left-to-right order.
///
/// # Contract
/// - requires: none.
/// - ensures: every variable leaf appears exactly once, ordered by its first
///   occurrence — the declaration order the boundary language's `r(t₁, …, tₙ)`
///   instantiates in.
/// - panics: none.
fn distinct_vars(term: &FreeTerm) -> Vec<Name>
{
    let mut distinct: Vec<Name> = Vec::new();
    for name in term.collect_vars() {
        if !distinct.contains(&name) {
            distinct.push(name);
        }
    }
    distinct
}

/// Apply a variable substitution to a term.
///
/// # Contract
/// - requires: none.
/// - ensures: every variable leaf bound by `bindings` is replaced by its term
///   and every other leaf is kept; applications keep their head, alphabet and
///   argument order.
/// - panics: none; the result stack is drained with saturating arithmetic, and
///   a term with no applications leaves exactly one result to take.
fn substitute(
    term: &FreeTerm,
    bindings: &BTreeMap<Name, FreeTerm>,
) -> FreeTerm
{
    /// The head an application is rebuilt under.
    enum AppShape<'term>
    {
        /// A constructor application.
        Ctor(&'term Name),
        /// An operation application.
        Op(&'term Name),
    }

    /// One step of the substitution worklist.
    enum Step<'term>
    {
        /// Rewrite a term's leaves.
        Term(&'term FreeTerm),
        /// Rebuild an application over the last `arity` results.
        Finish(AppShape<'term>, usize),
    }

    let mut results: Vec<FreeTerm> = Vec::new();
    let mut stack: Vec<Step<'_>> = vec![Step::Term(term)];
    while let Some(step) = stack.pop() {
        match step {
            | Step::Term(node) => match *node {
                | FreeTerm::Var(ref name) => results.push(
                    bindings
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| FreeTerm::Var(name.clone())),
                ),
                | FreeTerm::Ctor(ref name, ref args) => {
                    stack.push(Step::Finish(AppShape::Ctor(name), args.len()));
                    for arg in args.iter().rev() {
                        stack.push(Step::Term(arg));
                    }
                },
                | FreeTerm::Op(ref name, ref args) => {
                    stack.push(Step::Finish(AppShape::Op(name), args.len()));
                    for arg in args.iter().rev() {
                        stack.push(Step::Term(arg));
                    }
                },
            },
            | Step::Finish(shape, arity) => {
                let split = results.len().saturating_sub(arity);
                let args = results.split_off(split);
                results.push(match shape {
                    | AppShape::Ctor(name) => FreeTerm::Ctor(name.clone(), args.into()),
                    | AppShape::Op(name) => FreeTerm::Op(name.clone(), args.into()),
                });
            },
        }
    }
    results.pop().unwrap_or_else(|| term.clone())
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::circuit::CircuitFrame;

    /// `node : p(x) ==> (x′); node : add(x′, y) --> (z);` with `z` declared —
    /// the congruence body of the ruled block form with one of its two
    /// rewrite-sorted ports, which is the shape the whiskering construction
    /// `f(t̄, ρ, ū)` spells.
    fn one_redex_body() -> CircuitBody
    {
        CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::var("x\u{2032}"),
                    "x\u{2032}",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("x\u{2032}"), FreeTerm::var("y")],
                    "z",
                )),
            ],
            "z",
        )
    }

    #[test]
    fn a_sorted_port_binds_two_distinct_endpoints()
    {
        // `rule p : Nat ==> Nat` writes the boundary sort and no terms, so the
        // pair it binds is two endpoints the sort alone does not name.
        let pair = RewritePort::sorted("p", "Nat").interface();
        assert_eq!("p", pair.rewrite.to_string(), "the pair is the port's own");
        assert_ne!(
            pair.source, pair.target,
            "the two endpoints stay apart, or the source would bind the target"
        );
        assert_ne!(
            FreeTerm::var("p"),
            pair.source,
            "an endpoint is not the port itself"
        );
    }

    #[test]
    fn a_pinned_port_binds_the_terms_it_writes()
    {
        // `rule p : x ==> Succ(x)` names its own endpoints, so the pair is
        // those terms rather than minted variables.
        let pair = RewritePort::pinned(
            "p",
            FreeTerm::var("x"),
            FreeTerm::ctor("Succ", [FreeTerm::var("x")]),
        )
        .interface();
        assert_eq!(
            InterfacePair::new(
                "p",
                FreeTerm::var("x"),
                FreeTerm::ctor("Succ", [FreeTerm::var("x")])
            ),
            pair,
            "the pinned form's interface pair is what its declaration writes"
        );
    }

    #[test]
    fn instantiating_a_port_unifies_the_source_and_binds_the_target()
    {
        // `node : p(x) ==> (x′)` against `rule p : Nat ==> Nat`: the source
        // takes the line's input wiring, and the target — which the sort does
        // not name — is the output port, the opaque endpoint the derivation
        // reads as a leaf.
        let redex = RewritePort::sorted("p", "Nat")
            .interface()
            .instantiate([FreeTerm::var("x")], "x\u{2032}")
            .expect("a one-argument line matches a one-endpoint source");
        assert_eq!(
            CircuitRedex::new(
                "p",
                FreeTerm::var("x"),
                FreeTerm::var("x\u{2032}"),
                "x\u{2032}"
            ),
            redex,
            "the instantiation is the redex the boundary derivation consumes"
        );
    }

    #[test]
    fn instantiating_a_pinned_port_substitutes_into_its_target()
    {
        // `rule p : x ==> Succ(x)` fired at `a`: the source binds the target's
        // variable, so the target is a term rather than the output port.
        let redex = RewritePort::pinned(
            "p",
            FreeTerm::var("x"),
            FreeTerm::ctor("Succ", [FreeTerm::var("x")]),
        )
        .interface()
        .instantiate([FreeTerm::var("a")], "w")
        .expect("a one-argument line matches a one-variable source");
        assert_eq!(
            CircuitRedex::new(
                "p",
                FreeTerm::var("a"),
                FreeTerm::ctor("Succ", [FreeTerm::var("a")]),
                "w"
            ),
            redex,
            "the pinned target is instantiated, not replaced by the output port"
        );
    }

    #[test]
    fn a_repeated_source_variable_consumes_one_argument()
    {
        // `rule p : add(a, a) ==> a` has one distinct source variable, so the
        // line supplies one argument and both occurrences take it.
        let redex = RewritePort::pinned(
            "p",
            FreeTerm::op("add", [FreeTerm::var("a"), FreeTerm::var("a")]),
            FreeTerm::var("a"),
        )
        .interface()
        .instantiate([FreeTerm::var("u")], "z")
        .expect("one distinct source variable takes one argument");
        assert_eq!(
            CircuitRedex::new(
                "p",
                FreeTerm::op("add", [FreeTerm::var("u"), FreeTerm::var("u")]),
                FreeTerm::var("u"),
                "z"
            ),
            redex,
            "a repeated source variable is bound once and used twice"
        );
    }

    #[test]
    fn an_instantiation_that_does_not_unify_declines()
    {
        // Two arguments against a one-endpoint source: the line's input wiring
        // is not this port's interface, and the decline says so by count.
        let declined = RewritePort::sorted("p", "Nat")
            .interface()
            .instantiate([FreeTerm::var("x"), FreeTerm::var("y")], "z");
        assert_eq!(
            Err(PortInstantiationError::SourceArity {
                rewrite: "p".into(),
                expected: 1.into(),
                supplied: 2.into(),
            }),
            declined,
            "an instantiation that does not unify is a typed decline"
        );
    }

    #[test]
    fn a_target_endpoint_no_wire_supplies_declines()
    {
        // `rule p : x ==> Succ(y)`: `y` is neither bound by the source nor the
        // lone opaque endpoint the output port supplies, so no wire carries it.
        let declined = RewritePort::pinned(
            "p",
            FreeTerm::var("x"),
            FreeTerm::ctor("Succ", [FreeTerm::var("y")]),
        )
        .interface()
        .instantiate([FreeTerm::var("a")], "w");
        assert_eq!(
            Err(PortInstantiationError::UnboundTargetEndpoint {
                rewrite: "p".into(),
                endpoints: vec!["y".into()].into(),
            }),
            declined,
            "a target endpoint the wiring cannot supply is a typed decline"
        );
    }

    #[test]
    fn a_single_redex_block_elaborates_to_a_whiskered_cell()
    {
        let cell = elaborate_body(&one_redex_body()).expect("one redex whiskers into one frame");
        assert_eq!(
            WhiskeredCell::Whisker {
                head: FrameHead::Op("add".into()),
                before: Box::default(),
                inner: Box::new(WhiskeredCell::Redex(InterfacePair::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::var("x\u{2032}")
                ))),
                after: vec![FreeTerm::var("y")].into(),
            },
            cell,
            "a redex inside a frame is the whiskering `add(rho, y)`"
        );
        assert_eq!(
            Some(vec![0.into()]),
            cell.active_position(),
            "and its active position is the argument slot the redex sits in"
        );
    }

    #[test]
    fn a_redex_under_two_frames_whiskers_outermost_first()
    {
        // `node : p(n) ==> (m); node : Succ(m) --> (s); node : add(w, s) --> (z);`
        // — the redex sits in the second argument of `add`, inside the first
        // argument of `Succ`.
        let body = CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("n"),
                    FreeTerm::var("m"),
                    "m",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Ctor("Succ".into()),
                    [FreeTerm::var("m")],
                    "s",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("w"), FreeTerm::var("s")],
                    "z",
                )),
            ],
            "z",
        );
        let cell = elaborate_body(&body).expect("one redex whiskers into two frames");
        assert_eq!(
            WhiskeredCell::Whisker {
                head: FrameHead::Op("add".into()),
                before: vec![FreeTerm::var("w")].into(),
                inner: Box::new(WhiskeredCell::Whisker {
                    head: FrameHead::Ctor("Succ".into()),
                    before: Box::default(),
                    inner: Box::new(WhiskeredCell::Redex(InterfacePair::new(
                        "p",
                        FreeTerm::var("n"),
                        FreeTerm::var("m")
                    ))),
                    after: Box::default(),
                }),
                after: Box::default(),
            },
            cell,
            "the outermost frame is the outermost whisker"
        );
        assert_eq!(
            Some(vec![1.into(), 0.into()]),
            cell.active_position(),
            "the active position reads the argument slots root-first"
        );
    }

    #[test]
    fn a_redex_at_the_root_needs_no_whisker()
    {
        let body = CircuitBody::new(
            [CircuitNode::Redex(CircuitRedex::new(
                "p",
                FreeTerm::var("x"),
                FreeTerm::var("x\u{2032}"),
                "x\u{2032}",
            ))],
            "x\u{2032}",
        );
        let cell = elaborate_body(&body).expect("a bare redex is its own composite");
        assert_eq!(
            WhiskeredCell::Redex(InterfacePair::new(
                "p",
                FreeTerm::var("x"),
                FreeTerm::var("x\u{2032}")
            )),
            cell,
            "no frame encloses it, so no whisker wraps it"
        );
        assert_eq!(
            Some(Vec::new()),
            cell.active_position(),
            "the root is the empty argument path"
        );
    }

    #[test]
    fn a_body_with_no_redex_is_the_identity_rewrite()
    {
        // A body whose statements are all frames is a 1-cell definition; read
        // as a 2-cell it is the empty rewrite at the term it builds.
        let body = CircuitBody::new(
            [CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Ctor("Succ".into()),
                [FreeTerm::var("n")],
                "m",
            ))],
            "m",
        );
        let cell = elaborate_body(&body).expect("a frames-only wiring is acyclic");
        assert_eq!(
            WhiskeredCell::Here(FreeTerm::ctor("Succ", [FreeTerm::var("n")])),
            cell,
            "no redex means `here(t)` at the frame's own term"
        );
    }

    #[test]
    fn an_identity_composite_has_no_active_position()
    {
        assert_eq!(
            None,
            WhiskeredCell::Here(FreeTerm::var("z")).active_position(),
            "an identity composite is nowhere active"
        );
    }

    #[test]
    fn a_nested_whisker_reports_its_argument_path()
    {
        // Built directly, so the path is pinned to the count of unchanged
        // arguments left of the active slot at each level rather than to a
        // constant.
        let cell = WhiskeredCell::Whisker {
            head: FrameHead::Op("f".into()),
            before: vec![FreeTerm::var("a")].into(),
            inner: Box::new(WhiskeredCell::Whisker {
                head: FrameHead::Op("g".into()),
                before: vec![FreeTerm::var("b"), FreeTerm::var("c")].into(),
                inner: Box::new(WhiskeredCell::Redex(InterfacePair::new(
                    "p",
                    FreeTerm::var("s"),
                    FreeTerm::var("t"),
                ))),
                after: Box::default(),
            }),
            after: vec![FreeTerm::var("d")].into(),
        };
        assert_eq!(
            Some(vec![1.into(), 2.into()]),
            cell.active_position(),
            "each level contributes the index of its active argument slot"
        );
    }

    #[test]
    fn two_disjoint_redexes_decline_with_incomparable_positions()
    {
        // The ruled `cong2` body: two redexes sharing no port name, whiskered
        // into one frame. Their positions are incomparable, which is the
        // horizontal composite licensed only against an earned witness.
        let body = CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::var("x\u{2032}"),
                    "x\u{2032}",
                )),
                CircuitNode::Redex(CircuitRedex::new(
                    "q",
                    FreeTerm::var("y"),
                    FreeTerm::var("y\u{2032}"),
                    "y\u{2032}",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("x\u{2032}"), FreeTerm::var("y\u{2032}")],
                    "z",
                )),
            ],
            "z",
        );
        assert_eq!(
            Err(CircuitElaborationError::ManyRedexOccurrences {
                occurrences: vec![
                    RedexOccurrence::new("p", vec![0.into()]),
                    RedexOccurrence::new("q", vec![1.into()]),
                ]
                .into(),
            }),
            elaborate_body(&body),
            "two redex occurrences decline, carrying the positions the guard asks about"
        );
    }

    #[test]
    fn a_reconvergent_redex_is_two_occurrences_of_one_rewrite()
    {
        // One redex output feeding both arguments of one frame: the wire is
        // unfolded at each consumption, so the composite would hold the same
        // rewrite at two positions.
        let body = CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::ctor("Succ", [FreeTerm::var("x")]),
                    "w",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("w"), FreeTerm::var("w")],
                    "z",
                )),
            ],
            "z",
        );
        assert_eq!(
            Err(CircuitElaborationError::ManyRedexOccurrences {
                occurrences: vec![
                    RedexOccurrence::new("p", vec![0.into()]),
                    RedexOccurrence::new("p", vec![1.into()]),
                ]
                .into(),
            }),
            elaborate_body(&body),
            "a shared wire is two occurrences of one rewrite, not one position"
        );
    }

    #[test]
    fn a_cyclic_wiring_elaborates_to_nothing()
    {
        let body = CircuitBody::new(
            [
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("f".into()),
                    [FreeTerm::var("b")],
                    "a",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("g".into()),
                    [FreeTerm::var("a")],
                    "b",
                )),
            ],
            "a",
        );
        assert_eq!(
            Err(CircuitElaborationError::Derivation(
                CircuitDerivationError::CyclicWiring("a".into())
            )),
            elaborate_body(&body),
            "no boundary pair means no composite, and the decline names the port"
        );
    }
}
