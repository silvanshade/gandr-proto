//! **Tracelet 3-cell certificates** — replayable derivations that certify a
//! fused cell equals its two-step composite, generic over the
//! [`CellAlphabet`] (metatheory roadmap spike S1).
//!
//! Spec: `proposal-sequent-kernel.md` §7.3 (the `Tracelet` struct); VDC
//! addendum §A (replay-equivalence as identity, ADR-69).
//!
//! A [`Tracelet`] records a **peak** (the overlap superposition) and two
//! rewrite **paths** from it that both reach a common `joins_at`. It is
//! **replayed, not trusted** (ADR-69): [`Tracelet::replay`] skolemizes the
//! peak's metavariables to fresh constants, re-executes each recorded step by
//! ground rewriting, and checks both paths reach the skolemized join.
//! Replay-equivalence ([`replay_equivalent`]) — same boundary, both replay — is
//! the **identity criterion** the addendum promotes tracelet grafting to (the
//! coherence that makes the future `compose_*` operations associative/unital).
//! Certificate identity (replay-equivalence) stays strictly separate from
//! type-level term identity (structural [`Eq`]) — no conflation anywhere in
//! the abstraction.
//!
//! - A **confluence** tracelet ([`confluence_tracelet`]) joins the two reducts
//!   of a Knuth–Bendix critical pair (§7.3.3).
//! - A **composition** tracelet ([`derive_fused`]) certifies a fused cell: its
//!   `path_a` is the two-step `[left, right]` derivation, its `path_b` the
//!   single fused step, and `joins_at` their common result — the fused≡two-step
//!   contract as a replayable object (§7.2, §7.3.4).

use alloc::vec::Vec;

use crate::alphabet::CellAlphabet;
use crate::boundary::NormalizationBudget;
use crate::boundary::TraceletEquivalence;
use crate::boundary::TraceletReplay;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::overlap::Overlap;
use crate::rewrite::CellApp;
use crate::rewrite::normalize;
use crate::rewrite::rewrite_at;
use crate::sequent::SequentAlphabet;

/// A **3-cell certificate** — a peak and two replayable paths joining at a
/// common term (`proposal-sequent-kernel.md` §7.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tracelet<A: CellAlphabet = SequentAlphabet>
{
    /// The overlap the tracelet certifies.
    pub overlap: Overlap<A>,
    /// The first path from the peak to `joins_at`.
    pub path_a: Vec<CellApp<A>>,
    /// The second path from the peak to `joins_at`.
    pub path_b: Vec<CellApp<A>>,
    /// The common term both paths reach.
    pub joins_at: A::Cmd,
}

impl<A: CellAlphabet> Tracelet<A>
{
    /// **Replay** the certificate: re-execute both paths from the peak and
    /// check they reach `joins_at` (ADR-69 — replayed, not trusted).
    ///
    /// # Contract
    /// - ensures: `true` iff, after skolemizing the peak's metavariables to
    ///   fresh constants, running `path_a` and `path_b` (ground rewriting each
    ///   recorded step) both succeed and reach the skolemized `joins_at`. A
    ///   step that no longer fires, or a path that lands elsewhere, yields
    ///   `false`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn replay(
        &self,
        store: &CellStore<A>,
    ) -> TraceletReplay
    {
        replay_from_peak(
            store,
            &self.overlap.peak,
            &self.joins_at,
            &self.path_a,
            &self.path_b,
        )
    }
}

/// Replay **two recorded paths from one peak** against one join — the
/// certificate-shaped check both [`Tracelet::replay`] and the shift-equivalence
/// witness ([`crate::shift::ShiftEquivalence::replay`]) are the two readings
/// of.
///
/// It is shared rather than duplicated because the object is the same in both
/// places: a peak, two derivations out of it, and a term they must both reach.
/// What differs is where the boundary comes from — an [`Overlap`] superposition
/// for a tracelet, a pair of adjacent applications for a shift.
///
/// # Contract
/// - ensures: positive iff, after skolemizing `peak` and `joins_at` to fresh
///   constants, running each path by ground rewriting succeeds and lands on the
///   skolemized `joins_at`; a step that no longer fires, a stale cell id, or a
///   path landing elsewhere yields a negative.
/// - provides: the "replayed, not trusted" discipline (ADR-69) for any two-path
///   boundary.
/// - panics: none.
#[inline]
pub(crate) fn replay_from_peak<A>(
    store: &CellStore<A>,
    peak: &A::Cmd,
    joins_at: &A::Cmd,
    path_a: &[CellApp<A>],
    path_b: &[CellApp<A>],
) -> TraceletReplay
where
    A: CellAlphabet,
{
    let peak = A::skolemize(peak);
    let target = A::skolemize(joins_at);
    let ran_a = run_path(store, peak.clone(), path_a);
    let ran_b = run_path(store, peak, path_b);
    TraceletReplay::from(
        matches!(ran_a, Some(ref t) if *t == target)
            && matches!(ran_b, Some(ref t) if *t == target),
    )
}

/// Whether two tracelets are **replay-equivalent** — **the definition** of when
/// two certificates are the same transformation (ADR-69 D1; VDC addendum §A).
///
/// The engine's discipline was always "certificates are replayed, not trusted";
/// ADR-69 promotes that posture to the *identity criterion*: two tracelets
/// denote the same 3-cell iff they share a boundary (peak and `joins_at`) and
/// each replays. Equality is therefore **proof-irrelevant up to replay** — two
/// structurally distinct derivations (different `path_a` / `path_b`) of one
/// boundary are one transformation, which is exactly what makes derivation
/// grafting associative and unital in the ADR-68 reading and lets the
/// [`crate::compose`] operations treat the composite as canonical. The finer
/// derived [`PartialEq`] on [`Tracelet`] compares whole structures; this
/// coarser relation is the certificate quotient.
///
/// # Contract
/// - ensures: `true` iff the two tracelets share a peak and a `joins_at` and
///   both [`Tracelet::replay`] successfully — the boundary agrees and each is a
///   valid derivation, so they denote the same transformation (proof-irrelevant
///   up to replay); it ignores the recorded paths beyond that they each replay.
/// - panics: none.
#[inline]
#[must_use]
pub fn replay_equivalent<A>(
    a: &Tracelet<A>,
    b: &Tracelet<A>,
    store: &CellStore<A>,
) -> TraceletEquivalence
where
    A: CellAlphabet,
{
    TraceletEquivalence::from(
        a.overlap.peak == b.overlap.peak
            && a.joins_at == b.joins_at
            && bool::from(a.replay(store))
            && bool::from(b.replay(store)),
    )
}

/// Build a **confluence** tracelet from a joinable critical pair, or `None`
/// when the reducts do not join within `budget` (`proposal-sequent-kernel.md`
/// §7.3.3).
///
/// # Contract
/// - requires: `overlap.kind == OverlapKind::Confluence`.
/// - ensures: `Some(tracelet)` when both reducts normalize (within `budget`) to
///   the same term — `path_a = [left] ++ normalize(left_reduct)`, `path_b =
///   [right] ++ normalize(right_reduct)`, `joins_at` the common normal form;
///   `None` when a reduct is stale, a normalization exhausts the budget, or the
///   normal forms differ (a genuine, non-joinable divergence).
/// - panics: none.
#[inline]
#[must_use]
pub fn confluence_tracelet<A>(
    overlap: &Overlap<A>,
    store: &CellStore<A>,
    budget: NormalizationBudget,
) -> Option<Tracelet<A>>
where
    A: CellAlphabet,
{
    let left_reduct = overlap.left_reduct(store)?;
    let right_reduct = overlap.right_reduct(store)?;
    let norm_a = normalize(store, &left_reduct, budget);
    let norm_b = normalize(store, &right_reduct, budget);
    if norm_a.exhausted || norm_b.exhausted || norm_a.normal != norm_b.normal {
        return None;
    }
    let mut path_a = alloc::vec![CellApp {
        cell: overlap.left,
        at: A::root_position(),
    }];
    path_a.extend(norm_a.path);
    let mut path_b = alloc::vec![CellApp {
        cell: overlap.right,
        at: A::root_position(),
    }];
    path_b.extend(norm_b.path);
    Some(Tracelet {
        overlap: overlap.clone(),
        path_a,
        path_b,
        joins_at: norm_a.normal,
    })
}

/// Derive the **fused cell** of a composition overlap and its certifying
/// tracelet, inserting the fused cell into `store`
/// (`proposal-sequent-kernel.md` §7.2, §7.3.4).
///
/// # Contract
/// - requires: `overlap.kind == OverlapKind::Composition`.
/// - ensures: `Some((fused_id, tracelet))` where the fused cell is `peak ~>
///   composite` (the derived provenance, an invertible certificate), `path_a =
///   [left@root, right@seam]` is the two-step derivation and `path_b =
///   [fused@root]` the single fused step, both reaching the composite. `None`
///   if the composite cannot be formed.
/// - panics: none.
#[inline]
pub fn derive_fused<A>(
    overlap: &Overlap<A>,
    store: &mut CellStore<A>,
) -> Option<(CellId, Tracelet<A>)>
where
    A: CellAlphabet,
{
    let composite = overlap.composite(store)?;
    let fused = Cell::new(
        overlap.peak.clone(),
        composite.clone(),
        A::derived_orientation(),
        A::derived_provenance(),
    );
    let fused_id = store.insert(fused);
    let tracelet = Tracelet {
        overlap: overlap.clone(),
        path_a: alloc::vec![
            CellApp {
                cell: overlap.left,
                at: A::root_position(),
            },
            CellApp {
                cell: overlap.right,
                at: overlap.seam.clone(),
            },
        ],
        path_b: alloc::vec![CellApp {
            cell: fused_id,
            at: A::root_position(),
        }],
        joins_at: composite,
    };
    Some((fused_id, tracelet))
}

/// Run a recorded path from `start`, firing each step by ground rewriting.
///
/// # Contract
/// - ensures: `Some(term)` when every step's cell fires at its recorded
///   position in sequence; `None` when any step fails to fire (a stale id or a
///   no-longer-present redex).
/// - panics: none.
#[inline]
fn run_path<A>(
    store: &CellStore<A>,
    start: A::Cmd,
    path: &[CellApp<A>],
) -> Option<A::Cmd>
where
    A: CellAlphabet,
{
    let mut current = start;
    for step in path {
        let cell = store.get(step.cell)?;
        current = rewrite_at(cell, &current, &step.at)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;

    use super::*;
    use crate::overlap::OverlapKind;
    use crate::overlap::enumerate_overlaps;
    use crate::pattern::CmdPat;
    use crate::pattern::ConsPat;
    use crate::pattern::ProdPat;
    use crate::pattern::Sym;
    use crate::sequent::CellProvenance;
    use crate::sequent::Orientation;
    use crate::sequent::frame_defining_cell;

    #[test]
    fn a_fused_cell_certificate_replays()
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let overlaps = enumerate_overlaps(&store);
        let composition = overlaps
            .into_iter()
            .find(|o| o.kind == OverlapKind::Composition && o.left == frame && o.right == add)
            .expect("the composition overlap exists");
        let (_fused, tracelet) =
            derive_fused(&composition, &mut store).expect("fused cell derived");
        assert!(
            bool::from(tracelet.replay(&store)),
            "the fused≡two-step certificate replays"
        );
        assert_eq!(2, tracelet.path_a.len(), "two-step path");
        assert_eq!(1, tracelet.path_b.len(), "single fused step");
    }

    #[test]
    fn a_certificate_is_replay_equivalent_to_itself()
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let composition = enumerate_overlaps(&store)
            .into_iter()
            .find(|o| o.kind == OverlapKind::Composition && o.left == frame && o.right == add)
            .expect("the composition overlap exists");
        let (_fused, tracelet) =
            derive_fused(&composition, &mut store).expect("fused cell derived");
        assert!(
            bool::from(replay_equivalent(&tracelet, &tracelet, &store)),
            "a valid certificate is replay-equivalent to itself (ADR-69 identity)"
        );
    }

    #[test]
    fn distinct_derivations_of_one_boundary_are_replay_equivalent()
    {
        // ADR-69 D1: identity is replay-equivalence, not structural equality.
        // `derive_fused` gives a tracelet whose `path_b` is the single fused step
        // and `path_a` the two-step. A SECOND tracelet over the SAME boundary
        // whose `path_b` is *also* the two-step is a structurally distinct
        // derivation of the same transformation — proof-irrelevant up to replay.
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let composition = enumerate_overlaps(&store)
            .into_iter()
            .find(|o| o.kind == OverlapKind::Composition && o.left == frame && o.right == add)
            .expect("the composition overlap exists");
        let (_fused, fused_derivation) =
            derive_fused(&composition, &mut store).expect("fused cell derived");
        let two_step_derivation = Tracelet {
            overlap: fused_derivation.overlap.clone(),
            path_a: fused_derivation.path_a.clone(),
            path_b: fused_derivation.path_a.clone(),
            joins_at: fused_derivation.joins_at.clone(),
        };
        assert_ne!(
            fused_derivation, two_step_derivation,
            "the two tracelets differ structurally (single fused step vs two-step path_b)"
        );
        assert!(
            bool::from(replay_equivalent(
                &fused_derivation,
                &two_step_derivation,
                &store
            )),
            "distinct derivations of one boundary are the same certificate (replay identity)"
        );
    }

    #[test]
    fn a_derivation_that_misses_its_boundary_is_not_self_equivalent()
    {
        // Replay identity is falsifiable: a tracelet whose recorded paths do not
        // reach its `joins_at` fails replay, so it is not even equivalent to
        // itself — the criterion is a real check, not a rubber stamp.
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let composition = enumerate_overlaps(&store)
            .into_iter()
            .find(|o| o.kind == OverlapKind::Composition && o.left == frame && o.right == add)
            .expect("the composition overlap exists");
        let (_fused, tracelet) =
            derive_fused(&composition, &mut store).expect("fused cell derived");
        let mut broken = tracelet;
        // Retarget the join to a term the recorded paths never reach.
        broken.joins_at = CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top);
        assert!(
            !bool::from(broken.replay(&store)),
            "the retargeted certificate no longer replays"
        );
        assert!(
            !bool::from(replay_equivalent(&broken, &broken, &store)),
            "a non-replaying tracelet is not replay-equivalent, even to itself"
        );
    }

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
}
