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

use gandr_theory_cell_complexes::alphabet::CellAlphabet;
use gandr_theory_cell_complexes::boundary::NormalizationBudget;
use gandr_theory_cell_complexes::boundary::TraceletEquivalence;
use gandr_theory_cell_complexes::boundary::TraceletReplay;
use gandr_theory_cell_complexes::cell::Cell;
use gandr_theory_cell_complexes::cell::CellId;
use gandr_theory_cell_complexes::cell::CellStore;
use gandr_theory_cell_complexes::sequent::SequentAlphabet;

use crate::memo::ReplayMemo;
use crate::memo::StepOutcome;
use crate::memo::resolve_step;
use crate::overlap::Overlap;
use crate::rewrite::CellApp;
use crate::rewrite::normalize;
use crate::rewrite::rewrite_at;

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

/// One successfully executed application in an observable replay path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayStep<A: CellAlphabet = SequentAlphabet>
{
    /// The recorded application that fired.
    pub application: CellApp<A>,
    /// The command produced by that application.
    pub result: A::Cmd,
}

/// The terminal observation of one replayed path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayPathOutcome<A: CellAlphabet = SequentAlphabet>
{
    /// Every recorded application fired, producing this command.
    Reached(A::Cmd),
    /// Replay stopped at the first application that could not be executed.
    Stuck
    {
        /// The first unknown or inapplicable recorded step.
        application: CellApp<A>,
    },
}

/// The observable execution of one recorded path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayPath<A: CellAlphabet = SequentAlphabet>
{
    /// The skolemized command from which replay started.
    pub started_at: A::Cmd,
    /// Every application that fired, in execution order.
    pub steps: Vec<ReplayStep<A>>,
    /// The completed command or the first replay obstruction.
    pub outcome: ReplayPathOutcome<A>,
}

/// The observable execution of both paths in a tracelet certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayTrace<A: CellAlphabet = SequentAlphabet>
{
    /// The first path's execution.
    pub path_a: ReplayPath<A>,
    /// The second path's execution.
    pub path_b: ReplayPath<A>,
    /// The skolemized join both completed paths must reach.
    pub joins_at: A::Cmd,
}

impl<A: CellAlphabet> ReplayTrace<A>
{
    /// Whether both observed paths completed at the recorded join.
    ///
    /// # Contract
    /// - ensures: positive iff both path outcomes are
    ///   [`ReplayPathOutcome::Reached`] at `joins_at`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — a completed two-path replay and a
    ///   store-permuted replay stuck on its first positional cell distinguish
    ///   the positive and negative verdicts exactly.
    /// - witness: `differential::tests::replay_is_pure_over_a_fixed_certificate_and_store`
    /// - witness: `differential::tests::store_permutation_is_not_an_indexed_certificate_invariant`
    #[inline]
    #[must_use]
    pub fn verdict(&self) -> TraceletReplay
    {
        TraceletReplay::from(
            matches!(self.path_a.outcome, ReplayPathOutcome::Reached(ref term) if *term == self.joins_at)
                && matches!(self.path_b.outcome, ReplayPathOutcome::Reached(ref term) if *term == self.joins_at),
        )
    }
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
    /// - ensures: resolves every recorded [`CellId`] as an insertion-order
    ///   index into `store`; it never retargets a step by structural cell
    ///   content. Store clones and append-only extensions preserve that
    ///   assignment, while permutations may change the replay.
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

    /// Replay the certificate and retain each successfully executed step.
    ///
    /// This is the observable companion to [`Tracelet::replay`]. The ordinary
    /// verdict path retains no step vectors; callers that need evidence can pay
    /// for the two exact-capacity vectors and compare their emitted traces.
    ///
    /// # Contract
    /// - ensures: starts both paths at the skolemized peak; records every
    ///   successful application and its result in path order; stops each path
    ///   at its first unknown cell or inapplicable step; records the skolemized
    ///   `joins_at` against which [`ReplayTrace::verdict`] judges completion.
    /// - ensures: resolves recorded [`CellId`]s by their insertion-order index
    ///   in `store`, so the emitted trace makes any permuted binding
    ///   observable.
    /// - provides: an observable replay step list without changing
    ///   indexed-store certificate identity.
    /// - panics: none.
    /// - intension: allocates one step vector per path with capacity equal to
    ///   that recorded path's length; [`Tracelet::replay`] retains neither.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence + L3 pointwise — repeated and cloned-store
    ///   replays expose equal traces, append-only extension preserves the
    ///   trace, and a permutation that rebinds a positional cell id exposes the
    ///   exact first stuck application.
    /// - witness: `differential::tests::replay_is_pure_over_a_fixed_certificate_and_store`
    /// - witness: `differential::tests::append_only_store_extension_preserves_replay_trace`
    /// - witness: `differential::tests::store_permutation_is_not_an_indexed_certificate_invariant`
    #[inline]
    #[must_use]
    pub fn replay_trace(
        &self,
        store: &CellStore<A>,
    ) -> ReplayTrace<A>
    {
        replay_trace_from_peak(
            store,
            &self.overlap.peak,
            &self.joins_at,
            &self.path_a,
            &self.path_b,
        )
    }

    /// **Replay the certificate through a memo**, reusing the outcome of any
    /// step whose support this memo already answered.
    ///
    /// This is the opt-in companion to [`Tracelet::replay`], which is unchanged
    /// and consults nothing. The memo is threaded in by the caller and may be
    /// shared across replays of different certificates against different
    /// stores: reuse follows the support, not the certificate.
    ///
    /// # Contract
    /// - ensures: the same verdict as [`Tracelet::replay`] for every
    ///   `(certificate, store)` pair, whatever the memo already holds — the
    ///   memo changes what is computed, never what is answered.
    /// - ensures: each step is answered from `memo` when its full support (the
    ///   resolved cell's content, the position, the input term) was already
    ///   memoized, and executed and recorded otherwise; a step naming a cell
    ///   the store does not hold is stuck without consulting the memo, since an
    ///   unresolved identifier has no content to key on.
    /// - provides: reuse across repeated replays, across append-only store
    ///   growth, and across certificates that share steps, measured by
    ///   [`ReplayMemo::steps_executed`] against [`ReplayMemo::steps_reused`].
    /// - panics: none.
    /// - intension: reuse is keyed by resolved cell content rather than by
    ///   [`CellId`], so appending to the store preserves every memoized key
    ///   while permuting it misses wholesale instead of answering for content
    ///   the engine would not have fired.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — repeated replay, replay across a
    ///   `derive_fused` append, and the whole composition corpus separate the
    ///   reuse path from the execution path while pinning both verdicts equal,
    ///   and a poisoned memo entry makes them differ.
    /// - witness: `replay_memo::tests::replaying_one_tracelet_twice_reuses_every_step`
    /// - witness: `replay_memo::tests::a_derive_fused_append_reuses_the_unaffected_steps`
    /// - witness: `replay_memo::tests::the_composition_corpus_agrees_with_and_without_the_memo`
    /// - witness: `tracelet::tests::a_poisoned_memo_entry_makes_the_memoized_verdict_disagree`
    #[inline]
    #[must_use]
    pub fn replay_memoized(
        &self,
        store: &CellStore<A>,
        memo: &mut ReplayMemo<A>,
    ) -> TraceletReplay
    {
        replay_memoized_from_peak(
            store,
            memo,
            &self.overlap.peak,
            &self.joins_at,
            &self.path_a,
            &self.path_b,
        )
    }
}

/// Replay **two recorded paths from one peak** against one join.
///
/// This is the certificate-shaped check that [`Tracelet::replay`] and the
/// shift-equivalence witness one crate above are the two readings of.
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
pub fn replay_from_peak<A>(
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
    let ran_a = run_path(store, peak.clone(), path_a, None);
    let ran_b = run_path(store, peak, path_b, None);
    TraceletReplay::from(
        matches!(ran_a, ReplayPathOutcome::Reached(ref term) if *term == target)
            && matches!(ran_b, ReplayPathOutcome::Reached(ref term) if *term == target),
    )
}

/// Replay **two recorded paths from one peak** through a memo.
///
/// This is [`replay_from_peak`] with the per-step engine call routed through
/// `memo`. The skolemization, the path order, and the join comparison are the
/// same; only where each step's outcome comes from differs.
///
/// # Contract
/// - ensures: the same verdict as [`replay_from_peak`] on the same arguments,
///   for every memo state reachable through [`ReplayMemo::resolve`].
/// - ensures: both paths start at the same skolemized peak, so a step shared
///   between them is memoized once and reused on the second.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 agreement — every rung of the memo differential compares
///   this route's verdict against the non-memoized route on the same inputs.
/// - witness: `replay_memo::tests::replaying_one_tracelet_twice_reuses_every_step`
/// - witness: `replay_memo::tests::the_composition_corpus_agrees_with_and_without_the_memo`
#[inline]
pub fn replay_memoized_from_peak<A>(
    store: &CellStore<A>,
    memo: &mut ReplayMemo<A>,
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
    let ran_a = run_path_memoized(store, memo, peak.clone(), path_a);
    let ran_b = run_path_memoized(store, memo, peak, path_b);
    TraceletReplay::from(
        matches!(ran_a, ReplayPathOutcome::Reached(ref term) if *term == target)
            && matches!(ran_b, ReplayPathOutcome::Reached(ref term) if *term == target),
    )
}

/// Whether two tracelets are **replay-equivalent**, with both replays routed
/// through a memo.
///
/// The relation is [`replay_equivalent`]'s; only the replays are memoized. It
/// exists because the two tracelets of an equivalence question usually share
/// steps, so the second replay is often answered entirely from the first's
/// entries.
///
/// # Contract
/// - ensures: the same answer as [`replay_equivalent`] on the same arguments,
///   for every memo state.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 agreement — the composition corpus compares this against
///   [`replay_equivalent`] for every ordered pair it generates.
/// - witness: `replay_memo::tests::the_composition_corpus_agrees_with_and_without_the_memo`
#[inline]
#[must_use]
pub fn replay_equivalent_memoized<A>(
    a: &Tracelet<A>,
    b: &Tracelet<A>,
    store: &CellStore<A>,
    memo: &mut ReplayMemo<A>,
) -> TraceletEquivalence
where
    A: CellAlphabet,
{
    if a.overlap.peak != b.overlap.peak || a.joins_at != b.joins_at {
        return TraceletEquivalence::from(false);
    }
    let ran_a = a.replay_memoized(store, memo);
    let ran_b = b.replay_memoized(store, memo);
    TraceletEquivalence::from(bool::from(ran_a) && bool::from(ran_b))
}

/// Run a recorded path from `start`, answering each step through `memo`.
///
/// # Contract
/// - ensures: the same outcome as [`run_path`] with no step retention, for
///   every memo state.
/// - ensures: a step naming a cell absent from `store`, and a step the resolved
///   cell refuses, both stop the path at that application.
/// - panics: none.
#[inline]
fn run_path_memoized<A>(
    store: &CellStore<A>,
    memo: &mut ReplayMemo<A>,
    start: A::Cmd,
    path: &[CellApp<A>],
) -> ReplayPathOutcome<A>
where
    A: CellAlphabet,
{
    let mut current = start;
    for step in path {
        let outcome = resolve_step(store, memo, step.cell, &step.at, &current);
        let Some(StepOutcome::Fired(result)) = outcome
        else {
            return ReplayPathOutcome::Stuck {
                application: step.clone(),
            };
        };
        current = result;
    }
    ReplayPathOutcome::Reached(current)
}

/// Replay two paths while retaining their successful steps.
///
/// # Contract
/// - ensures: returns the same verdict as [`replay_from_peak`] through
///   [`ReplayTrace::verdict`], with both path executions exposed.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 agreement — every differential replay row compares this
///   evidence-bearing route's verdict with the non-tracing route.
/// - witness: `differential::tests::replay_is_pure_over_a_fixed_certificate_and_store`
/// - witness: `differential::tests::append_only_store_extension_preserves_replay_trace`
/// - witness: `differential::tests::store_permutation_is_not_an_indexed_certificate_invariant`
#[inline]
fn replay_trace_from_peak<A>(
    store: &CellStore<A>,
    peak: &A::Cmd,
    joins_at: &A::Cmd,
    path_a: &[CellApp<A>],
    path_b: &[CellApp<A>],
) -> ReplayTrace<A>
where
    A: CellAlphabet,
{
    let peak = A::skolemize(peak);
    let joins_at = A::skolemize(joins_at);
    ReplayTrace {
        path_a: trace_path(store, peak.clone(), path_a),
        path_b: trace_path(store, peak, path_b),
        joins_at,
    }
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
/// `gandr_theory_decomposition_spaces::compose` operations treat the composite
/// as canonical. The finer derived [`PartialEq`] on [`Tracelet`] compares whole
/// structures; this coarser relation is the certificate quotient.
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

/// Run a recorded path from `start`, optionally retaining successful steps.
///
/// # Contract
/// - ensures: [`ReplayPathOutcome::Reached`] contains the final command when
///   every step fires in sequence; [`ReplayPathOutcome::Stuck`] identifies the
///   first stale cell id or no-longer-present redex. When `steps` is present,
///   appends exactly the successfully fired prefix in execution order.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the fusion fixture exercises the complete path;
///   its permuted indexed store exercises the first-step obstruction; exact
///   emitted traces distinguish both outcomes.
/// - witness: `differential::tests::replay_is_pure_over_a_fixed_certificate_and_store`
/// - witness: `differential::tests::store_permutation_is_not_an_indexed_certificate_invariant`
#[inline]
fn run_path<A>(
    store: &CellStore<A>,
    start: A::Cmd,
    path: &[CellApp<A>],
    mut steps: Option<&mut Vec<ReplayStep<A>>>,
) -> ReplayPathOutcome<A>
where
    A: CellAlphabet,
{
    let mut current = start;
    for step in path {
        let Some(cell) = store.get(step.cell)
        else {
            return ReplayPathOutcome::Stuck {
                application: step.clone(),
            };
        };
        let Some(result) = rewrite_at(cell, &current, &step.at)
        else {
            return ReplayPathOutcome::Stuck {
                application: step.clone(),
            };
        };
        if let Some(steps) = steps.as_deref_mut() {
            steps.push(ReplayStep {
                application: step.clone(),
                result: result.clone(),
            });
        }
        current = result;
    }
    ReplayPathOutcome::Reached(current)
}

/// Replay one path and retain its successful execution prefix.
///
/// # Contract
/// - ensures: `started_at` is `start`; `steps` and `outcome` are the observable
///   projection of [`run_path`] over the same inputs.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 agreement — trace verdicts are compared with the
///   non-tracing replay result over complete, append-extended, and permuted
///   stores.
/// - witness: `differential::tests::replay_is_pure_over_a_fixed_certificate_and_store`
/// - witness: `differential::tests::append_only_store_extension_preserves_replay_trace`
/// - witness: `differential::tests::store_permutation_is_not_an_indexed_certificate_invariant`
#[inline]
fn trace_path<A>(
    store: &CellStore<A>,
    start: A::Cmd,
    path: &[CellApp<A>],
) -> ReplayPath<A>
where
    A: CellAlphabet,
{
    let started_at = start.clone();
    let mut steps = Vec::with_capacity(path.len());
    let outcome = run_path(store, start, path, Some(&mut steps));
    ReplayPath {
        started_at,
        steps,
        outcome,
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_cell_complexes::pattern::CmdPat;
    use gandr_theory_cell_complexes::pattern::ConsPat;
    use gandr_theory_cell_complexes::pattern::ProdPat;
    use gandr_theory_cell_complexes::pattern::Sym;
    use gandr_theory_cell_complexes::sequent::CellProvenance;
    use gandr_theory_cell_complexes::sequent::Orientation;
    use gandr_theory_cell_complexes::sequent::frame_defining_cell;

    use super::*;
    use crate::memo::MemoPoisonOutcome;
    use crate::memo::StepSupport;
    use crate::overlap::OverlapKind;
    use crate::overlap::enumerate_overlaps;

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

    /// The frame ∘ (add-S) composition store, the identifier its (add-S) cell
    /// took, and the fused≡two-step certificate over it.
    fn fused_fixture() -> (CellStore, CellId, Tracelet)
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
        (store, add, tracelet)
    }

    /// The memoized supports whose recorded outcome fired, with their outcomes.
    fn fired_supports(memo: &ReplayMemo) -> Vec<StepSupport>
    {
        memo.entries()
            .filter(|entry| matches!(*entry.1, StepOutcome::Fired(_)))
            .map(|entry| entry.0.clone())
            .collect()
    }

    #[test]
    fn a_poisoned_memo_entry_makes_the_memoized_verdict_disagree()
    {
        // The differential's teeth. Every rung asserts that the memoized verdict
        // equals the fresh verdict; that assertion is only evidence if a wrong
        // memo could have broken it. Corrupting one recorded step to a refusal
        // must make the memoized route disagree with the engine — for *every*
        // recorded step, so no rung passes on an entry the verdict cannot see.
        let (store, _add, tracelet) = fused_fixture();
        let honest = bool::from(tracelet.replay(&store));
        assert!(honest, "the fused≡two-step certificate replays honestly");

        let mut recording = ReplayMemo::new();
        let _warm = tracelet.replay_memoized(&store, &mut recording);
        let supports = fired_supports(&recording);
        assert_eq!(
            3,
            supports.len(),
            "the two-step path and the fused step record three distinct supports"
        );

        for support in &supports {
            let mut poisoned = recording.clone();
            assert_eq!(
                MemoPoisonOutcome::Poisoned,
                poisoned.poison(support, StepOutcome::Refused),
                "the recorded support is the one poisoned"
            );
            assert!(
                !bool::from(tracelet.replay_memoized(&store, &mut poisoned)),
                "a memo that refuses a step the engine fires flips the memoized verdict"
            );
        }
    }

    #[test]
    fn a_poisoned_refusal_licenses_a_step_the_engine_refuses()
    {
        // The dangerous direction: a memo that reports a firing where the engine
        // refuses. A wrong refusal is conservative — it loses reuse and fails
        // loudly; a wrong firing admits a derivation that never happened, so the
        // differential has to catch this one to be worth anything.
        let (store, add, tracelet) = fused_fixture();
        let refusing = Tracelet {
            overlap: tracelet.overlap.clone(),
            // (add-S) does not fire at the root of the skolemized peak, whose
            // consumer is still wrapped in the Succ⁻ frame.
            path_a: alloc::vec![CellApp {
                cell: add,
                at: <SequentAlphabet as CellAlphabet>::root_position(),
            }],
            path_b: tracelet.path_b.clone(),
            joins_at: tracelet.joins_at,
        };
        assert!(
            !bool::from(refusing.replay(&store)),
            "the engine refuses the recorded first step, so the certificate fails replay"
        );

        let mut memo = ReplayMemo::new();
        assert!(
            !bool::from(refusing.replay_memoized(&store, &mut memo)),
            "an honest memo reproduces the refusal"
        );
        let refused = memo
            .entries()
            .find(|entry| matches!(*entry.1, StepOutcome::Refused))
            .map(|entry| entry.0.clone())
            .expect("the refused step is memoized alongside the ones that fired");

        let reached = <SequentAlphabet as CellAlphabet>::skolemize(&refusing.joins_at);
        assert_eq!(
            MemoPoisonOutcome::Poisoned,
            memo.poison(&refused, StepOutcome::Fired(reached)),
            "the refusal is the entry poisoned"
        );
        assert!(
            bool::from(refusing.replay_memoized(&store, &mut memo)),
            "a memo that fires a step the engine refuses forges a positive verdict"
        );
        assert!(
            !bool::from(refusing.replay(&store)),
            "the engine's own verdict is unchanged, so the differential separates them"
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
