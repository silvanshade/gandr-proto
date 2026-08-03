//! **Polarized match footprints** — a *prototype* independence test that sits
//! **beside** the decided shift guard and is consumed by nothing.
//!
//! # Status: prototype, and deliberately inert
//!
//! Nothing in this crate or downstream calls into this module.
//! [`crate::shift::derive_shift_equivalence`] remains the only licence for a
//! shift, with the three conjuncts it has always had. This module exists to
//! **measure** one question the template-games read left open: whether a
//! *polarized* independence test — rewritten versus matched-but-preserved,
//! read off the match image rather than declared on the alphabet — licenses a
//! strictly larger commuting class than the guard's incomparable-positions
//! conjunct. It answers by disagreeing with the guard on recorded fixtures, and
//! the disagreements are the deliverable; see the differential suite
//! (`footprint::tests` in the crate's integration target).
//!
//! # The test being prototyped
//!
//! The source's independence relation on transitions is **four** conditions
//! over a quadruple `(rd, wr, lock, mem)`, of which only two are plain
//! disjointness \[@mellies-stefanesco-2020-csl, §3.2.1\]:
//!
//! ```text
//! (rd(p) ∪ wr(p)) ∩ wr(p') = ∅        lock(p) ∩ lock(p') = ∅
//! (rd(p') ∪ wr(p')) ∩ wr(p) = ∅       mem(p) ∩ mem(p') = ∅
//! ```
//!
//! The load-bearing fact is that independence is **polarized**, not
//! disjointness: two transitions may share *read* locations freely, and only
//! write-versus-anything collides. `lock` has no counterpart in this carrier
//! and is not a gap to be filled; `mem` (allocation) has no counterpart at the
//! cell-application layer either. So the prototype implements the two `rd`/`wr`
//! conjuncts and nothing else.
//!
//! # What a footprint is here, and the three-way split
//!
//! A footprint is attached to a **transition** — one [`CellApp`] firing in one
//! term — never to a store and never to a cell. Every position of the term
//! covered by the redex (the match image: the redex root and everything under
//! it) lands in exactly one of three classes:
//!
//! - **written** — the node at that address does not survive the rewrite *as
//!   the same node*: it is absent afterwards, its children change, or its own
//!   content changes once the children are held equal;
//! - **read** — the node survives, and the cell's *firing* at its recorded
//!   position is sensitive to the content there (the matched-but-preserved
//!   part: the rule looked, and left it alone);
//! - **framed** — the node survives and the firing is insensitive to it: the
//!   hole instances the rule carries through without inspecting.
//!
//! Only `written` and `read` enter the footprint. **The frame is excluded on
//! purpose**, and that is the whole content of the frame rule read
//! structurally: a transition that neither reads nor writes a location does not
//! collide there, however deeply that location sits inside its match image.
//!
//! # Three modelling choices, each of which could have gone the other way
//!
//! 1. **The spine is not in the footprint.** Positions strictly above the redex
//!    are traversed to reach it and are structurally rebuilt by splicing, but
//!    the *nodes* are unchanged. Counting the spine as read (let alone written)
//!    would make every pair of applications in one term collide at the root,
//!    which is exactly the connectivity-closed reading the read ruled out as
//!    the wrong level: footprints in the source are **shallow**, and nothing
//!    takes a transitive closure.
//! 2. **The write test is node-local, not subtree-local.** A rewrite strictly
//!    below a node changes that node's *subterm* but not the node, so
//!    subtree-keyed writes would again collide at the root. Node-locality is
//!    decided by grafting: hold the immediate children equal and ask whether
//!    what is left agrees ([`NodeSurvival`]).
//! 3. **Addresses are absolute term positions.** The source's footprints range
//!    over machine addresses, which do not move; a term position does. A rule
//!    that relocates its frame therefore reports the vacated addresses as
//!    *written*, so the prototype refuses such a pair rather than licensing a
//!    commutation the recorded schedule cannot perform. This is conservative,
//!    not unsound — and it is the concrete shape of the debt an adoption would
//!    take on: a residual map on positions, which [`CellApp`] has no room for.
//!
//! # The instantiation obligation, and the gate it fires on
//!
//! The shapes here are **generic in the cell alphabet**: an address is
//! `A::Pos`, the alphabet's own position type, so a later carrier that
//! addresses a heap instantiates [`MatchFootprint`] and
//! [`FootprintIndependence`] rather than re-inventing them. Two alphabets
//! instantiate them today — the sequent alphabet, where the position
//! vocabulary degenerates to the redex root, and the toy alphabet the
//! differential suite runs on — and **neither addresses a store**, so the heap
//! side of the shape is interface only. That is a fact about what exists, not
//! a judgement about the shape.
//!
//! **The gate is one condition, and it is not this module's to discharge.**
//! The obligation fires when a `CellAlphabet` instance exists whose positions
//! address a partial store — the addressed, partial heap of
//! `docs/gandr/spec/implementation/proposed/separation-logic.md`,
//! `separation-logic-requirement-01` — and whose transitions still report read
//! against write as two sets rather than one (`separation-logic-requirement-07`
//! there, which asks the heap interface not to lose the polarization this
//! module computes). Until such an instance exists there is nothing to
//! instantiate against, and instantiating early would fix the three modelling
//! choices above at term positions for a carrier whose addresses do not behave
//! like term positions.
//!
//! **What the instantiation owes when it fires** is those three choices,
//! re-decided rather than inherited: whether the spine is in the footprint,
//! whether the write test stays node-local, and — the one with a name — the
//! residual map on positions that absolute addressing needs, since a heap
//! address does not move where a term position does.
//!
//! # Two components this module does not have, and why each is machinery
//!
//! **Allocation (the `mem` conjunct) has no event to read off.** Its
//! independence condition is plain disjointness of allocated addresses
//! \[@mellies-stefanesco-2020-csl, §3.2.1\], which needs a per-transition
//! freshness datum. gandr's candidate carrier is the internal-wire binder, and
//! it does not exist: the circuit surface's cycle-closing statement is a
//! parse-only grammar form whose back-edge rung is owed, and the circuit
//! derivation in `gandr-theory-levitation` refuses wiring cycles outright
//! rather than binding them. The gate is that binder landing with freshness
//! visible per transition
//! (`docs/gandr/spec/implementation/proposed/separation-logic.md`,
//! `separation-logic-requirement-06`). The freshness this crate *does* have is
//! a different thing and does not substitute: [`CellAlphabet::rename_apart`]
//! and [`CellAlphabet::skolemize`] mint fresh **metavariable** names for
//! unification hygiene between two cells, and neither records an address a
//! firing brought into or out of existence.
//!
//! **A permission carrier is deliberately absent, and the split here is not
//! one.** What this module computes is owned-against-framed **at one
//! transition**: the footprint is what a firing touches and the frame is what
//! it carries through. A permission divides ownership of one location among
//! **simultaneous holders**, which is a different axis, needs a partial
//! cancellative commutative monoid, and is decided by the mode and reference
//! calculus's shared-borrow question — not here
//! (`docs/gandr/spec/surface-language/proposed/modes-and-references.md`,
//! `mode-decision-05`;
//! `docs/gandr/spec/implementation/proposed/separation-logic.md`,
//! `separation-logic-requirement-02`). Inventing one ahead of that decision
//! would almost certainly re-spell the sealed grade semiring, which counts uses
//! along a run and does not divide ownership.
//!
//! One name in this crate reads like a counterexample and is not:
//! [`crate::boundary::FiringPermission`] says whether a cell's provenance
//! permits it to fire at a target term — an η-polarity admissibility flag, with
//! no relation to ownership or to a permission monoid.
//!
//! # Interaction with the metavariable-seam gap
//!
//! The overlap enumerator counts a **metavariable position** as a composition
//! seam ([`crate::overlap::overlaps_between`]; recorded as the independence
//! relation's own gap on the tracker's `cells_equal` tractability spike). Over
//! an alphabet whose every subterm is a command position, a cell whose
//! right-hand side exposes a hole therefore "overlaps" every cell, and the
//! guard's second conjunct refuses the pair.
//!
//! This prototype consults the enumerator **not at all**: it derives everything
//! from the instance. So part of any measured gap between the two tests is the
//! recorded enumerator defect and part is genuine polarization, and the two
//! must be reported separately or the measurement is worthless. The
//! differential suite separates them with a fixture whose *only* overlap is a
//! hole seam.
//!
//! # Cost
//!
//! Deliberately unoptimized: the footprint of one application costs a rewrite
//! per candidate probe per covered position, plus a quadratic scan for
//! immediate children. It is a measurement instrument, not a fast path.
//!
//! [`CellApp`]: crate::rewrite::CellApp

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::alphabet::CellAlphabet;
use crate::alphabet::PositionOrder;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::rewrite::CellApp;
use crate::rewrite::rewrite_at;
use crate::sequent::SequentAlphabet;

/// Whether the node at one address survives an application **as the same
/// node** — the write test, decided node-locally rather than on subterms.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum NodeSurvival
{
    /// The address still carries a node with the same immediate-child
    /// addresses and the same content once those children are held equal.
    Preserved,
    /// The address is gone, its children changed, or its own content changed.
    Replaced,
}

/// Whether an application's **firing** is sensitive to the content at an
/// address inside its match image — the read/frame test.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ContentSensitivity
{
    /// Some probe substituted at the address stops the cell firing at its
    /// recorded position: the rule looked there.
    Sensitive,
    /// No available probe stops it: the address is frame, carried through
    /// without being inspected.
    Insensitive,
}

/// The **polarized footprint** of one application in one term — the match
/// image split into rewritten, matched-but-preserved, and framed addresses.
///
/// The three vectors partition the addresses the redex covers, in the
/// alphabet's own position order, and are pairwise disjoint by construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchFootprint<A: CellAlphabet = SequentAlphabet>
{
    /// The position the application fired at (the match image's root).
    pub at: A::Pos,
    /// The addresses the rewrite destroys or replaces.
    pub written: Vec<A::Pos>,
    /// The addresses the rule matches and leaves standing.
    pub read: Vec<A::Pos>,
    /// The addresses the rule carries through without inspecting — the frame,
    /// which is **not** part of the footprint.
    pub framed: Vec<A::Pos>,
}

/// Why a footprint could not be read off a pair of a term and an application.
///
/// Refusal is data: a footprint is a property of a transition, so an
/// application that is not a transition of this term has none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FootprintObstruction<A: CellAlphabet = SequentAlphabet>
{
    /// The application names a cell the store does not hold.
    UnknownCell
    {
        /// The identifier that resolved to nothing.
        cell: CellId,
    },
    /// The application does not fire at its recorded position in this term, so
    /// there is no transition to attach a footprint to.
    StepDoesNotFire
    {
        /// The step that failed to fire.
        step: Box<CellApp<A>>,
    },
}

/// The **polarized independence verdict** for two footprints — the two
/// `rd`/`wr` conjuncts of \[@mellies-stefanesco-2020-csl, §3.2.1\], and nothing
/// else.
///
/// A read/read overlap is *not* a collision, which is the entire point: it is
/// the one place the source lets this carrier earn more than the guard's
/// incomparable-positions conjunct does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FootprintIndependence<A: CellAlphabet = SequentAlphabet>
{
    /// Neither write set meets the other's write or read set.
    Independent,
    /// Both applications rewrite the same address.
    WriteWrite
    {
        /// The address both of them write.
        position: A::Pos,
    },
    /// One application rewrites an address the other matches and preserves.
    ReadWrite
    {
        /// The address one writes and the other reads.
        position: A::Pos,
    },
}

/// Read the **polarized footprint** of `app` firing in `term`.
///
/// # Contract
/// - requires: `A::command_positions` enumerates the term's addresses without
///   repetition, and `A::position_order` decides prefix containment on them.
/// - ensures: `Ok(footprint)` whose three vectors partition exactly the
///   addresses `app.at` covers (itself and everything it encloses); `written`
///   holds the addresses whose node does not survive the rewrite, `read` the
///   surviving addresses whose content the firing depends on, and `framed` the
///   rest.
/// - provides: the transition-attached datum the polarized independence test
///   reads, derived from the instance and never from the alphabet or the
///   overlap enumerator.
/// - fails: [`FootprintObstruction`] — an unresolvable cell identifier, or an
///   application that does not fire at its recorded position.
/// - panics: none.
/// - intension: the three vectors come out in `A::command_positions` order, so
///   two runs over one input agree entrywise; probes are the term's own
///   distinct subterms, in that same order, so the read/frame split is a
///   function of the input alone.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L2 — the three classes own separate decision surfaces, each
///   separated by a fixture that moves exactly one address between them: a
///   ground redex whose whole image is written, a rule whose matched root
///   survives and lands in `read`, and a rule whose hole instance lands in
///   `framed` while the sibling it matched lands in `written`; the two refusals
///   are separated by an unstored identifier and a non-firing step; the
///   partition postcondition and the order projection are separated from all of
///   them by a sweep over the differential fixture family, which fails on any
///   address classified twice, left out of the match image, taken from above
///   it, or emitted out of enumeration order.
/// - witness: `footprint::tests::a_ground_redex_writes_its_whole_match_image`
/// - witness: `footprint::tests::a_preserved_root_is_read_and_its_hole_is_framed`
/// - witness: `footprint::tests::a_footprint_needs_a_transition_to_attach_to`
/// - witness: `footprint::tests::every_footprint_partitions_exactly_the_addresses_its_redex_covers`
#[inline]
pub fn match_footprint<A>(
    store: &CellStore<A>,
    term: &A::Cmd,
    app: &CellApp<A>,
) -> Result<MatchFootprint<A>, FootprintObstruction<A>>
where
    A: CellAlphabet,
{
    let Some(cell) = store.get(app.cell)
    else {
        return Err(FootprintObstruction::UnknownCell { cell: app.cell });
    };
    let Some(after) = rewrite_at(cell, term, &app.at)
    else {
        return Err(FootprintObstruction::StepDoesNotFire {
            step: Box::new(app.clone()),
        });
    };
    let before_positions = A::command_positions(term);
    let after_positions = A::command_positions(&after);
    let probes = distinct_subterms::<A>(term, &before_positions);
    let mut written = Vec::new();
    let mut read = Vec::new();
    let mut framed = Vec::new();
    for position in &before_positions {
        // Inside the match image: the redex root and everything it encloses.
        // Nothing above it enters the footprint — the spine is traversed, not
        // read, and counting it would collapse every pair at the root.
        if !matches!(
            A::position_order(&app.at, position),
            PositionOrder::Same | PositionOrder::Encloses
        ) {
            continue;
        }
        match node_survival::<A>(term, &after, &before_positions, &after_positions, position) {
            | NodeSurvival::Replaced => written.push(position.clone()),
            | NodeSurvival::Preserved => {
                match content_sensitivity::<A>(cell, term, &app.at, position, &probes) {
                    | ContentSensitivity::Sensitive => read.push(position.clone()),
                    | ContentSensitivity::Insensitive => framed.push(position.clone()),
                }
            },
        }
    }
    Ok(MatchFootprint {
        at: app.at.clone(),
        written,
        read,
        framed,
    })
}

/// Decide **polarized independence** of two footprints.
///
/// # Contract
/// - requires: the two footprints were read off the same term, so their
///   addresses are comparable.
/// - ensures: [`FootprintIndependence::Independent`] exactly when neither write
///   set meets the other's write set or read set; otherwise the variant naming
///   the first colliding address.
/// - provides: the commutation licence the polarized reading grants, which
///   permits read/read overlap and therefore differs from the shift guard on
///   both of the guard's first two conjuncts.
/// - panics: none.
/// - intension: collisions are searched in the left footprint's `written` order
///   first, then the right's, so a pair colliding several ways reports the
///   earliest address in that scan and the verdict is deterministic.
///
/// # Adequacy
/// - hypothesis: L2 — the three verdicts own separate decision surfaces: a
///   disjoint pair is independent, a pair rewriting one address collides
///   write/write, and a pair whose inner application rewrites the outer's
///   matched-and-preserved skeleton collides read/write; the read/read case is
///   separated from all three by a pair whose *only* shared address is read by
///   both, which is licensed; the scan-order projection is pinned by a pair
///   colliding at every address of one match image, which fixes the enumeration
///   order and that the reported address is the earliest in it — a self-paired
///   root collision cannot separate scan order from a canonical order, so full
///   separation is owed by a fixture whose earliest scanned collision is not
///   the canonical minimum.
/// - witness: `footprint::tests::two_root_rules_sharing_only_a_read_node_are_independent`
/// - witness: `footprint::tests::a_rule_that_destroys_a_node_another_only_reads_is_refused`
/// - witness: `footprint::tests::two_rules_rewriting_one_child_collide_on_writes`
/// - witness: `footprint::tests::a_pair_colliding_several_ways_reports_the_earliest_address_it_scans`
#[inline]
#[must_use]
pub fn footprint_independence<A>(
    left: &MatchFootprint<A>,
    right: &MatchFootprint<A>,
) -> FootprintIndependence<A>
where
    A: CellAlphabet,
{
    for position in &left.written {
        if right.written.contains(position) {
            return FootprintIndependence::WriteWrite {
                position: position.clone(),
            };
        }
        if right.read.contains(position) {
            return FootprintIndependence::ReadWrite {
                position: position.clone(),
            };
        }
    }
    for position in &right.written {
        if left.read.contains(position) {
            return FootprintIndependence::ReadWrite {
                position: position.clone(),
            };
        }
    }
    FootprintIndependence::Independent
}

/// The term's distinct subterms, in position order — the probe supply the
/// read/frame split perturbs addresses with.
///
/// # Contract
/// - ensures: one entry per distinct subterm addressed by `positions`, in that
///   order; an alphabet exposing few positions supplies few probes, which makes
///   the split coarser rather than wrong.
/// - panics: none.
#[inline]
fn distinct_subterms<A>(
    term: &A::Cmd,
    positions: &[A::Pos],
) -> Vec<A::Cmd>
where
    A: CellAlphabet,
{
    let mut out: Vec<A::Cmd> = Vec::new();
    for position in positions {
        let Some(subterm) = A::subterm_cmd_at(term, position)
        else {
            continue;
        };
        if !out.contains(&subterm) {
            out.push(subterm);
        }
    }
    out
}

/// The addresses immediately below `parent` among `positions`.
///
/// # Contract
/// - requires: `positions` is a term's complete address set, so "no address
///   strictly between" is decidable inside it.
/// - ensures: every address `parent` encloses with no enclosed intermediary, in
///   `positions` order.
/// - panics: none.
#[inline]
fn immediate_children<A>(
    positions: &[A::Pos],
    parent: &A::Pos,
) -> Vec<A::Pos>
where
    A: CellAlphabet,
{
    let mut out = Vec::new();
    for candidate in positions {
        if A::position_order(parent, candidate) != PositionOrder::Encloses {
            continue;
        }
        let nested = positions.iter().any(|between| {
            A::position_order(parent, between) == PositionOrder::Encloses
                && A::position_order(between, candidate) == PositionOrder::Encloses
        });
        if !nested {
            out.push(candidate.clone());
        }
    }
    out
}

/// Whether the node at `position` survives the rewrite as the same node.
///
/// The test is a **graft**: hold the immediate children equal to what the
/// rewrite produced and ask whether the rest of the node agrees. A rewrite
/// strictly below the address then leaves it [`NodeSurvival::Preserved`], which
/// is what keeps the spine and every ancestor out of the write set.
///
/// # Contract
/// - ensures: [`NodeSurvival::Preserved`] exactly when the address exists in
///   both terms, has the same immediate-child addresses in both, and the
///   before-term with those children replaced by the after-term's agrees with
///   the after-term at the address; otherwise [`NodeSurvival::Replaced`].
/// - panics: none.
#[inline]
fn node_survival<A>(
    before: &A::Cmd,
    after: &A::Cmd,
    before_positions: &[A::Pos],
    after_positions: &[A::Pos],
    position: &A::Pos,
) -> NodeSurvival
where
    A: CellAlphabet,
{
    if !after_positions.contains(position) {
        return NodeSurvival::Replaced;
    }
    let children = immediate_children::<A>(before_positions, position);
    // The two child address lists must denote the same set; order is not
    // compared, because an alphabet's enumeration order within one depth is
    // its own business.
    let after_children = immediate_children::<A>(after_positions, position);
    if children.len() != after_children.len()
        || !children.iter().all(|child| after_children.contains(child))
    {
        return NodeSurvival::Replaced;
    }
    let mut grafted = before.clone();
    for child in &children {
        let Some(replacement) = A::subterm_cmd_at(after, child)
        else {
            return NodeSurvival::Replaced;
        };
        let Some(next) = A::splice_cmd_at(&grafted, child, replacement)
        else {
            return NodeSurvival::Replaced;
        };
        grafted = next;
    }
    match (
        A::subterm_cmd_at(&grafted, position),
        A::subterm_cmd_at(after, position),
    ) {
        | (Some(left), Some(right)) if left == right => NodeSurvival::Preserved,
        | _ => NodeSurvival::Replaced,
    }
}

/// Whether the cell's firing at `at` depends on the content at `position`.
///
/// # Contract
/// - requires: `cell` fires at `at` in `term`, and `position` is covered by
///   `at`.
/// - ensures: [`ContentSensitivity::Sensitive`] when some probe differing from
///   the current content stops the cell firing at `at`;
///   [`ContentSensitivity::Insensitive`] when no supplied probe does.
/// - panics: none.
/// - intension: the probes are tried in supply order and the first refusal
///   decides, so the verdict is a function of the probe supply — an
///   under-approximation of "read" bounded by the probes available, never a
///   guess.
#[inline]
fn content_sensitivity<A>(
    cell: &Cell<A>,
    term: &A::Cmd,
    at: &A::Pos,
    position: &A::Pos,
    probes: &[A::Cmd],
) -> ContentSensitivity
where
    A: CellAlphabet,
{
    let Some(current) = A::subterm_cmd_at(term, position)
    else {
        return ContentSensitivity::Sensitive;
    };
    for probe in probes {
        if *probe == current {
            continue;
        }
        let Some(perturbed) = A::splice_cmd_at(term, position, probe.clone())
        else {
            continue;
        };
        if rewrite_at(cell, &perturbed, at).is_none() {
            return ContentSensitivity::Sensitive;
        }
    }
    ContentSensitivity::Insensitive
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;

    use super::*;
    use crate::pattern::CmdPat;
    use crate::pattern::ConsPat;
    use crate::pattern::Pos;
    use crate::pattern::ProdPat;
    use crate::pattern::Sym;
    use crate::sequent::frame_defining_cell;

    #[test]
    fn the_sequent_alphabet_gives_the_prototype_one_address_to_work_with()
    {
        // The read/frame split is only as fine as the alphabet's address
        // vocabulary. A sequent command pattern has exactly one command
        // position, so the whole match image is the root and the prototype
        // degenerates to "the redex is written" — no holes are visible to it,
        // because no hole sits at a command position. This is a limit of the
        // position vocabulary, not of the polarized reading, and it is the
        // reason the differential suite runs on the toy alphabet.
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let footprint = match_footprint(&store, &term, &CellApp {
            cell: frame,
            at: Pos::root(),
        })
        .expect("the frame-defining cell fires at the root");
        assert_eq!(
            alloc::vec![Pos::root()],
            footprint.written,
            "the only address there is is written"
        );
        assert!(
            footprint.read.is_empty() && footprint.framed.is_empty(),
            "and the alphabet exposes no address for either of the other two classes"
        );
    }

    #[test]
    fn a_footprint_needs_a_transition_to_attach_to()
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        // A term the frame-defining cell has no redex in.
        let term = CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top);
        let step = CellApp {
            cell: frame,
            at: Pos::root(),
        };
        assert_eq!(
            Err(FootprintObstruction::StepDoesNotFire {
                step: Box::new(step.clone()),
            }),
            match_footprint(&store, &term, &step),
            "a footprint is a property of a transition, so a non-transition has none"
        );
        let missing = CellId(41);
        assert_eq!(
            Err(FootprintObstruction::UnknownCell { cell: missing }),
            match_footprint(&store, &term, &CellApp {
                cell: missing,
                at: Pos::root(),
            }),
            "and an unresolvable identifier is refused before the term is read"
        );
    }
}
