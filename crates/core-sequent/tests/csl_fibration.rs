//! The **CSL fibration property suite** over the nominal memo-cell heap — the
//! independent, unscheduled residual of
//! `spec:implementation/roadmap.md`, run against
//! `crates/core-sequent/src/store.rs` (the L machine's two-region store,
//! `proposal-sequent-kernel.md` §4.2; ADR-50 call-by-need).
//!
//! # The reading
//!
//! Read the memo-cell heap as a partial commutative monoid: a heap is a
//! finite map from nominal cell addresses to memo states, composition is
//! disjoint union (always defined — addresses are unique and append-only),
//! and the unit is the empty heap. Specifications are predicates over heap
//! fragments, fibred over the monoid; separating conjunction `P * Q` holds of
//! a heap when it splits into disjoint fragments satisfying `P` and `Q`.
//! Under this reading a store operation **right-lifts** along a framing
//! `h = h₁ * h₂` when, its footprint lying inside `h₁`, running it on the
//! composite yields `h₁' * h₂` with the frame `h₂` preserved pointwise — the
//! frame-rule direction of the lifting. The **four right-lifting conditions**
//! are the heap's contract under this reading. They are not spelled out
//! anywhere in the tree (the roadmap names them only); the suite pins them as
//! the following precise invariants, each reconstructed from the heap's own
//! contract in separation-logic shape:
//!
//! - **RL1 — frame preservation (disjoint-cell locality of forcing).** For
//!   every decomposition `c ↦ s * F` with `c` a nominal memo-cell address and
//!   `F` an arbitrary disjoint frame, and for every step of the force protocol
//!   on `c` (probe entry `Unforced → InProgress`, pure write-back `InProgress →
//!   Forced(v)`, probe decline `InProgress → Unforced`), the step lifts along
//!   `F` to `c ↦ s' * F`: every framed cell is bit-identical before and after —
//!   same state discriminant, same cached allocation.
//! - **RL2 — nominal identity (freshness and alias coherence).** The points of
//!   the separating conjunction are nominal addresses: allocation is fresh,
//!   dense, and append-only (an address is handed out once, never reused, never
//!   merged; the heap never shrinks), and sharing is exactly handle-sharing —
//!   every clone of a cell's handle denotes that same cell, so a mutation
//!   through any clone is observable at the nominal address and through every
//!   sibling clone, while cells of distinct allocations never alias.
//! - **RL3 — black-hole discipline under re-entry.** A force probe opens the
//!   black hole (`Unforced → InProgress`); while it is open, every re-entrant
//!   force — through the same handle or any shared clone — observes
//!   `InProgress` and, by the machine's protocol, runs inline writing nothing;
//!   the probe's resolution is written exactly once, by the opening probe
//!   alone, to `Forced(v)` on a pure terminal or back to `Unforced` on a
//!   decline; and under machine-legal traces `Forced` is absorbing — no
//!   protocol step has `Forced` as its source state.
//! - **RL4 — write-back purity (the cache is the exact probe allocation).** The
//!   pure-spine write-back caches the identical allocation the force probe
//!   returned — pointer-identical, no copy, no re-derivation; every later read
//!   of a `Forced` cell returns a clone of that same allocation
//!   (continuation-independence realized as allocation identity); a declined
//!   probe leaves no cache residue; and force state is invisible to value
//!   equality (a cell is a derived cache, never part of a value's meaning).
//!
//! # Provenance
//!
//! The roadmap entry is the only in-tree occurrence of the phrase "four
//! right-lifting conditions"; the neighbouring `PLAN.html` line ("the crDC
//! fibrational axioms staged as a property suite over the cell store") names
//! a *different* suite — the completion-engine crDC axioms, already landed as
//! `theory-virtual-doctrines/tests/crdc.rs`. No source in the tree spells the
//! four conditions out, so each condition above is **reconstructed from the
//! heap's contract**: the `store` module docs (nominal identity; per-cell
//! interior mutability; pure-spine-only write-back), `machine::LMachine::force`
//! (the black-hole probe, the re-entrant inline fall-back, the
//! `Rc::clone(&whnf)` write-back), and the `Cell` equality contract (a cache
//! is not meaning) — stated in the separation-logic shape the roadmap entry
//! presupposes.
//!
//! # Scope: the nominal half only
//!
//! The content-addressed half of the heap covers only the immutable value
//! graph; its entries are keyed by content and never mutated, so separating
//! conjunction degenerates there — there are no writes to frame (RL1), the
//! "address" *is* the content, not a nominal point (RL2), there is no force
//! protocol and no black hole (RL3), and a content entry is its own cache
//! (RL4). The nominal memo-cell half is the heap's only mutable fragment, so
//! the four conditions are live exactly here, and the suite is scoped to it.
//!
//! # Boundary (the permissive substrate)
//!
//! The store is a *permissive* substrate: transition legality (for example,
//! never clearing a `Forced` cell) is enforced by the machine's force
//! protocol, not by the `Cell` API. The suite therefore generates
//! machine-legal operation traces and measures the heap against them; the
//! CSL reading is of the heap *as the L machine uses it*. A property failure
//! here is a finding about the heap, never patched over.
//!
//! # Organization
//!
//! One deterministic interpreter ([`run`]) replays a generated operation
//! sequence against a real [`Store`], recording the full heap image before
//! and after every step plus the step's outcome. Each of the four properties
//! then folds the trace independently, asserting exactly its condition.

/// CSL fibration property tests.
#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;
    use alloc::vec::Vec;

    use gandr_core_sequent::LValue;
    use gandr_core_sequent::il::Lit;
    use gandr_core_sequent::store::Cell;
    use gandr_core_sequent::store::CellId;
    use gandr_core_sequent::store::MemoState;
    use gandr_core_sequent::store::Store;
    use proptest::prelude::*;

    /// The property case-count wrapper (the primitive-boundary discipline
    /// applied to the suite's own configuration).
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct Cases(u32);

    /// The generator's cell-space width — how many distinct slots generated
    /// operations may target before they wrap onto live cells.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct CellSpace(usize);

    /// A generator-side slot: the index of the tracked cell an operation
    /// targets (wrapped into the allocated range at interpretation time).
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct CellSlot(usize);

    /// A live cell index — a slot after wrapping into the allocated range.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct LiveCell(usize);

    /// A write-back value selector: which `LValue` allocation a
    /// `ResolvePure` payload builds.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct ValuePick(u8);

    /// The suite's proptest configuration: the default profile with the case
    /// count pinned unless the caller overrides it through `PROPTEST_CASES`
    /// (the `theory-virtual-doctrines` crDC-suite posture).
    fn suite_config(cases: Cases) -> ProptestConfig
    {
        let mut config = ProptestConfig::default();
        if std::env::var_os("PROPTEST_CASES").is_none() {
            config.cases = cases.0;
        }
        config
    }

    /// One machine-protocol step against the tracked heap, generated as data
    /// so a failing case replays exactly and proptest shrinking operates on
    /// the operation list itself.
    #[derive(Clone, Debug)]
    enum HeapOp
    {
        /// Allocate a fresh cell (the heap's append-only growth).
        Alloc,
        /// Share: clone one live handle of the cell — the `Rc`-clone sharing
        /// path, a second copy of one thunk value.
        Share(CellSlot),
        /// A machine force-entry on the cell (mirroring
        /// `machine::LMachine::force`'s entry): read the state; a `Forced`
        /// cache is a hit (no write), `InProgress` is the re-entrant inline
        /// fall-back (no write), `Unforced` opens the probe (mark the black
        /// hole).
        Force(CellSlot),
        /// The pure-spine write-back resolving an open probe: cache the exact
        /// allocation the probe returned (the payload selects which `LValue`
        /// allocation to build).
        ResolvePure(CellSlot, ValuePick),
        /// The decline path resolving an open probe: clear the black hole
        /// back to unforced and run inline (no cache residue).
        ResolveDecline(CellSlot),
        /// A re-entrant observation mid-probe: read the state through a
        /// shared handle, write nothing.
        Observe(CellSlot),
    }

    /// The state discriminant of a memo cell (the shape half of a
    /// [`CellSnapshot`]; `MemoState` itself carries no equality).
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum StateShape
    {
        /// Not yet forced.
        Unforced,
        /// The open black hole.
        InProgress,
        /// Forced, with a cached write-back.
        Forced,
    }

    /// A cached write-back allocation, compared by **pointer identity** —
    /// the observational-transparency witness: the machine reuses the exact
    /// allocation the probe returned, never a re-derivation or a copy.
    #[repr(transparent)]
    #[derive(Clone, Debug)]
    struct CachePtr(Rc<LValue>);

    impl PartialEq for CachePtr
    {
        /// Two cache pointers are equal exactly when they name the same
        /// allocation.
        fn eq(
            &self,
            other: &Self,
        ) -> bool
        {
            Rc::ptr_eq(&self.0, &other.0)
        }
    }

    impl Eq for CachePtr
    {
    }

    /// A point-in-time image of one cell: the state discriminant plus, for a
    /// forced cell, the cached allocation itself (so pointer identity is
    /// checkable across steps).
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct CellSnapshot
    {
        /// The state discriminant.
        shape: StateShape,
        /// The cached write-back allocation, present exactly when `shape` is
        /// `Forced`.
        cached: Option<CachePtr>,
    }

    /// One cell as seen from both sides of the nominal boundary: the store
    /// registry's view at the address, and the view through every live
    /// handle clone.
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct CellViews
    {
        /// The image read at the nominal address (`None` would mean an
        /// allocated cell unreadable at its address — itself a finding).
        at_address: Option<CellSnapshot>,
        /// The image read through each live handle clone, in handle order.
        through_handles: Vec<CellSnapshot>,
    }

    /// Snap a `MemoState` into its comparable image.
    fn snapshot_of(state: MemoState) -> CellSnapshot
    {
        match state {
            | MemoState::Unforced => CellSnapshot {
                shape: StateShape::Unforced,
                cached: None,
            },
            | MemoState::InProgress => CellSnapshot {
                shape: StateShape::InProgress,
                cached: None,
            },
            | MemoState::Forced(value) => CellSnapshot {
                shape: StateShape::Forced,
                cached: Some(CachePtr(value)),
            },
        }
    }

    /// One tracked cell: its nominal address plus every live handle clone
    /// (the allocation handle first, shared clones after).
    #[derive(Clone, Debug)]
    struct TrackedCell
    {
        /// The nominal address the store registered.
        id: CellId,
        /// Every live handle clone, allocation handle first.
        handles: Vec<Cell>,
    }

    /// The recorded outcome of one interpreted step — what the machine
    /// protocol observed or did — retained so each property folds the trace
    /// independently.
    #[derive(Clone, Debug)]
    enum Outcome
    {
        /// The allocation returned this nominal address.
        Allocated(CellId),
        /// A handle was cloned (the new clone joined the tracked handles).
        Shared,
        /// The force hit a cache: the exact allocation the machine returns.
        CacheHit(CachePtr),
        /// The force observed the black hole: the re-entrant inline
        /// fall-back (no write).
        ReentrantInline,
        /// The force opened a probe: the black hole is now marked.
        ProbeOpened,
        /// The write-back cached this exact allocation.
        WriteBack(CachePtr),
        /// The probe declined and cleared the black hole.
        Declined,
        /// An observation read this snapshot (no write).
        Observed(CellSnapshot),
        /// The operation did not apply (a slot against an empty heap, or a
        /// resolution step against a cell with no open probe): no write.
        Skipped,
    }

    /// One interpreted step: the operation, its outcome, the live cell it
    /// acted on, and the full heap image before and after (so the frame
    /// checks need no re-derivation).
    #[derive(Clone, Debug)]
    struct Step
    {
        /// The generated operation.
        op: HeapOp,
        /// What the step observed or did.
        outcome: Outcome,
        /// The live cell the step acted on (`None` for `Alloc` and for steps
        /// skipped against an empty heap).
        resolved: Option<LiveCell>,
        /// Every tracked cell's views before the step.
        before: Vec<CellViews>,
        /// Every tracked cell's views after the step.
        after: Vec<CellViews>,
    }

    /// The full run: the final store, the tracked cells, and the recorded
    /// steps.
    #[derive(Clone, Debug)]
    struct Trace
    {
        /// The store after the final step.
        store: Store,
        /// Every allocated cell, in allocation (that is, address) order.
        cells: Vec<TrackedCell>,
        /// The recorded steps, in order.
        steps: Vec<Step>,
    }

    /// Resolve a generated slot to a live tracked cell, wrapping the slot
    /// into the allocated range (checked remainder — no bare arithmetic);
    /// `None` while the heap is empty.
    fn live(
        cells: &mut [TrackedCell],
        slot: CellSlot,
    ) -> Option<(LiveCell, &mut TrackedCell)>
    {
        let index = slot.0.checked_rem(cells.len())?;
        let tracked = cells.get_mut(index)?;
        Some((LiveCell(index), tracked))
    }

    /// The write-back value a `ResolvePure` payload selects (a fresh
    /// allocation every time, so pointer identity has teeth).
    fn probe_value(pick: ValuePick) -> Rc<LValue>
    {
        Rc::new(LValue::Lit(Lit::Int(i64::from(pick.0))))
    }

    /// Snapshot every tracked cell, from both sides of the nominal boundary.
    fn snapshot_heap(
        store: &Store,
        cells: &[TrackedCell],
    ) -> Vec<CellViews>
    {
        cells
            .iter()
            .map(|tracked| CellViews {
                at_address: store.cell_state(tracked.id).map(snapshot_of),
                through_handles: tracked
                    .handles
                    .iter()
                    .map(|handle| snapshot_of(handle.state()))
                    .collect(),
            })
            .collect()
    }

    /// Apply one operation to the store and tracked cells, following the
    /// machine's force protocol exactly; record the outcome and the live
    /// cell acted on. Mutations go through the *last* tracked handle (so the
    /// shared-clone path, not only the allocation handle, is exercised);
    /// reads observe through the first.
    fn interpret(
        store: &mut Store,
        cells: &mut Vec<TrackedCell>,
        op: &HeapOp,
    ) -> (Outcome, Option<LiveCell>)
    {
        match *op {
            | HeapOp::Alloc => {
                let (id, cell) = store.alloc_cell();
                cells.push(TrackedCell {
                    id,
                    handles: vec![cell],
                });
                (Outcome::Allocated(id), None)
            },
            | HeapOp::Share(slot) => match live(cells, slot) {
                | Some((index, tracked)) => {
                    let base = tracked
                        .handles
                        .first()
                        .expect("a tracked cell always has its allocation handle")
                        .clone();
                    tracked.handles.push(base);
                    (Outcome::Shared, Some(index))
                },
                | None => (Outcome::Skipped, None),
            },
            | HeapOp::Force(slot) => match live(cells, slot) {
                | Some((index, tracked)) => {
                    let handle = tracked
                        .handles
                        .last()
                        .expect("a tracked cell always has its allocation handle");
                    let outcome = match handle.state() {
                        | MemoState::Forced(value) => Outcome::CacheHit(CachePtr(value)),
                        | MemoState::InProgress => Outcome::ReentrantInline,
                        | MemoState::Unforced => {
                            handle.mark_in_progress();
                            Outcome::ProbeOpened
                        },
                    };
                    (outcome, Some(index))
                },
                | None => (Outcome::Skipped, None),
            },
            | HeapOp::ResolvePure(slot, pick) => match live(cells, slot) {
                | Some((index, tracked)) => {
                    let handle = tracked
                        .handles
                        .last()
                        .expect("a tracked cell always has its allocation handle");
                    if matches!(handle.state(), MemoState::InProgress) {
                        let value = probe_value(pick);
                        handle.set_forced(Rc::clone(&value));
                        (Outcome::WriteBack(CachePtr(value)), Some(index))
                    }
                    else {
                        (Outcome::Skipped, Some(index))
                    }
                },
                | None => (Outcome::Skipped, None),
            },
            | HeapOp::ResolveDecline(slot) => match live(cells, slot) {
                | Some((index, tracked)) => {
                    let handle = tracked
                        .handles
                        .last()
                        .expect("a tracked cell always has its allocation handle");
                    if matches!(handle.state(), MemoState::InProgress) {
                        handle.clear();
                        (Outcome::Declined, Some(index))
                    }
                    else {
                        (Outcome::Skipped, Some(index))
                    }
                },
                | None => (Outcome::Skipped, None),
            },
            | HeapOp::Observe(slot) => match live(cells, slot) {
                | Some((index, tracked)) => {
                    let handle = tracked
                        .handles
                        .first()
                        .expect("a tracked cell always has its allocation handle");
                    (Outcome::Observed(snapshot_of(handle.state())), Some(index))
                },
                | None => (Outcome::Skipped, None),
            },
        }
    }

    /// Replay a generated operation sequence against a real store, recording
    /// the full trace.
    fn run(ops: &[HeapOp]) -> Trace
    {
        let mut store = Store::new();
        let mut cells: Vec<TrackedCell> = Vec::new();
        let mut steps: Vec<Step> = Vec::new();
        for op in ops {
            let before = snapshot_heap(&store, &cells);
            let (outcome, resolved) = interpret(&mut store, &mut cells, op);
            let after = snapshot_heap(&store, &cells);
            steps.push(Step {
                op: op.clone(),
                outcome,
                resolved,
                before,
                after,
            });
        }
        Trace {
            store,
            cells,
            steps,
        }
    }

    /// A single operation over a cell space of the given width.
    fn arb_heap_op(space: CellSpace) -> BoxedStrategy<HeapOp>
    {
        let slot = (0 .. space.0).prop_map(CellSlot);
        prop_oneof![
            1 => Just(HeapOp::Alloc),
            3 => slot.clone().prop_map(HeapOp::Share),
            4 => slot.clone().prop_map(HeapOp::Force),
            3 => (slot.clone(), any::<u8>())
                .prop_map(|(slot, pick)| HeapOp::ResolvePure(slot, ValuePick(pick))),
            2 => slot.clone().prop_map(HeapOp::ResolveDecline),
            3 => slot.prop_map(HeapOp::Observe),
        ]
        .boxed()
    }

    /// An operation sequence: a cell-space width, then up to 64 operations
    /// over it (sharing-, force-, and resolution-heavy).
    fn arb_heap_ops() -> BoxedStrategy<Vec<HeapOp>>
    {
        (1_usize ..= 8)
            .prop_flat_map(|width| prop::collection::vec(arb_heap_op(CellSpace(width)), 1 ..= 64))
            .boxed()
    }

    /// The live cell a step acted on, for folding the trace.
    fn subject(step: &Step) -> Option<LiveCell>
    {
        step.resolved
    }

    proptest! {
        #![proptest_config(suite_config(Cases(1024)))]

        /// **RL1 — frame preservation (disjoint-cell locality of forcing).**
        /// For every decomposition `c ↦ s * F` and every step of the force
        /// protocol on `c`, the step lifts along `F` to `c ↦ s' * F`: every
        /// framed cell is bit-identical before and after — same state
        /// discriminant, same cached allocation.
        ///
        /// Provenance: the frame rule / locality of separation logic
        /// (O'Hearn–Reynolds–Yang), reconstructed for the memo-cell heap from
        /// the store's contract — per-cell interior mutability, with a
        /// mutation only ever through one cell's own handle.
        ///
        /// The content-addressed half of the heap is out of scope: its
        /// entries are immutable, so there are no writes for a frame to
        /// survive and the condition degenerates to vacuity; the nominal
        /// memo-cell half is the heap's only mutable fragment, so frame
        /// preservation is live exactly here.
        #[test]
        fn frame_preservation_under_forcing(ops in arb_heap_ops())
        {
            let trace = run(&ops);
            for step in &trace.steps {
                for (index, before_views) in step.before.iter().enumerate() {
                    if subject(step).is_some_and(|live| live.0 == index) {
                        continue;
                    }
                    let after_views = step
                        .after
                        .get(index)
                        .expect("the append-only heap keeps every framed cell");
                    prop_assert_eq!(
                        &before_views.at_address,
                        &after_views.at_address,
                        "frame cell {} changed under a step on another cell ({:?})",
                        index,
                        step.op,
                    );
                }
            }
        }

        /// **RL2 — nominal identity (freshness and alias coherence).**
        /// Allocation is fresh, dense, and append-only; sharing is exactly
        /// handle-sharing — a mutation through any clone is observable at the
        /// nominal address and through every sibling clone, while cells of
        /// distinct allocations never alias.
        ///
        /// Provenance: the `store` module docs — "Cell identity is nominal:
        /// a cell is never deduplicated or merged … two thunks are the same
        /// cell exactly when they were allocated as one and copied (the
        /// shared handle)" — the well-definedness of the star decomposition
        /// itself (heaps split at addresses, never inside a cell).
        ///
        /// The content-addressed half of the heap is out of scope: there the
        /// "address" is the content hash, not a nominal point — identity is
        /// *structural* (two equal values are one entry), so nominal
        /// freshness and alias coherence have no counterpart; the condition
        /// is live exactly on the nominal memo-cell half.
        #[test]
        fn nominal_identity_freshness_and_alias_coherence(ops in arb_heap_ops())
        {
            let trace = run(&ops);
            // Freshness, density, append-only extent.
            for (index, tracked) in trace.cells.iter().enumerate() {
                prop_assert_eq!(
                    usize::from(tracked.id),
                    index,
                    "cell addresses are dense and fresh (the append-only arena)",
                );
            }
            prop_assert_eq!(
                usize::from(trace.store.heap_len()),
                trace.cells.len(),
                "every allocation is registered exactly once",
            );
            for step in &trace.steps {
                if let &Outcome::Allocated(id) = &step.outcome {
                    prop_assert_eq!(
                        usize::from(id),
                        step.before.len(),
                        "an allocation returned a non-dense or reused address",
                    );
                    prop_assert_eq!(
                        step.after.len(),
                        step.before.len().saturating_add(1),
                        "an allocation changed the heap's extent by more than one",
                    );
                } else {
                    prop_assert_eq!(
                        step.after.len(),
                        step.before.len(),
                        "a non-allocation step changed the heap's extent",
                    );
                }
                // Alias coherence: the address view equals every handle view.
                for (index, after_views) in step.after.iter().enumerate() {
                    let Some(address_view) = after_views.at_address.as_ref() else {
                        return Err(TestCaseError::fail(format!(
                            "cell {index} is registered but unreadable at its nominal address"
                        )));
                    };
                    prop_assert!(
                        !after_views.through_handles.is_empty(),
                        "cell {index} lost every handle"
                    );
                    for handle_view in &after_views.through_handles {
                        prop_assert_eq!(
                            address_view,
                            handle_view,
                            "cell {}: a shared handle diverged from the nominal address",
                            index,
                        );
                    }
                }
                // Separability: a step on cell `i` is invisible through the
                // handles of every `j ≠ i` (distinct allocations never alias).
                for (index, before_views) in step.before.iter().enumerate() {
                    if subject(step).is_some_and(|live| live.0 == index) {
                        continue;
                    }
                    let after_views = step
                        .after
                        .get(index)
                        .expect("the append-only heap keeps every framed cell");
                    prop_assert_eq!(
                        &before_views.through_handles,
                        &after_views.through_handles,
                        "cell {} observed a mutation targeted at another cell ({:?})",
                        index,
                        step.op,
                    );
                }
            }
        }

        /// **RL3 — black-hole discipline under re-entry.** While a probe is
        /// open, every re-entrant force observes `InProgress` and writes
        /// nothing; the resolution is written exactly once, by the opening
        /// probe alone, to `Forced(v)` or back to `Unforced`; and under
        /// machine-legal traces `Forced` is absorbing.
        ///
        /// Provenance: ADR-50 call-by-need black-holing carried over from the
        /// CEK — `machine::LMachine::force` marks the black hole across the
        /// probe and a re-entrant force observes it and falls back to running
        /// inline — reconstructed as a transition-system invariant (the only
        /// legal target-cell transitions are `Unforced → InProgress`,
        /// `InProgress → Forced`, `InProgress → Unforced`, and equality).
        ///
        /// The content-addressed half of the heap is out of scope: with no
        /// force protocol there is no black hole to discipline — re-entry
        /// cannot arise over immutable content; the condition is live exactly
        /// on the nominal memo-cell half, the heap's only force-bearing
        /// fragment.
        #[test]
        fn black_hole_discipline_under_reentry(ops in arb_heap_ops())
        {
            let trace = run(&ops);
            for step in &trace.steps {
                let Some(LiveCell(index)) = subject(step) else {
                    continue;
                };
                let (Some(before_views), Some(after_views)) =
                    (step.before.get(index), step.after.get(index))
                else {
                    continue;
                };
                let (Some(before), Some(after)) = (
                    before_views.at_address.as_ref(),
                    after_views.at_address.as_ref(),
                )
                else {
                    return Err(TestCaseError::fail(format!(
                        "cell {index} unreadable at its nominal address"
                    )));
                };
                // Transition legality (the machine-legal fragment only).
                let changed = before != after;
                let legal = matches!(
                    (before.shape, after.shape),
                    (StateShape::Unforced, StateShape::InProgress)
                        | (StateShape::InProgress, StateShape::Forced | StateShape::Unforced)
                ) || !changed;
                prop_assert!(
                    legal,
                    "illegal transition {:?} → {:?} by {:?}",
                    before.shape,
                    after.shape,
                    step.op,
                );
                // Forced is absorbing: no step leaves or rewrites a forced
                // cell.
                if matches!(before.shape, StateShape::Forced) {
                    prop_assert_eq!(
                        before,
                        after,
                        "a forced cell left its forced state or lost its cache ({:?})",
                        step.op,
                    );
                }
                // Outcome ↔ transition agreement: the resolution belongs to
                // the opening probe alone.
                if changed {
                    match (before.shape, after.shape) {
                        | (StateShape::Unforced, StateShape::InProgress) => prop_assert!(
                            matches!(step.outcome, Outcome::ProbeOpened),
                            "a black hole opened without a probe entry",
                        ),
                        | (StateShape::InProgress, StateShape::Forced) => prop_assert!(
                            matches!(step.outcome, Outcome::WriteBack(_)),
                            "a write-back landed outside the pure-spine resolution",
                        ),
                        | (StateShape::InProgress, StateShape::Unforced) => prop_assert!(
                            matches!(step.outcome, Outcome::Declined),
                            "a black hole cleared outside the decline path",
                        ),
                        | _ => {},
                    }
                }
                // Re-entrant discipline: observing the black hole is a pure
                // read (the inline fall-back writes nothing).
                if matches!(step.outcome, Outcome::ReentrantInline) {
                    prop_assert_eq!(
                        &before.shape,
                        &StateShape::InProgress,
                        "the re-entrant inline fall-back fired off the black hole",
                    );
                    prop_assert_eq!(
                        before, after,
                        "a re-entrant force wrote the cell mid-probe",
                    );
                }
                // An observation reports the state faithfully.
                if let Outcome::Observed(ref observed) = step.outcome {
                    prop_assert_eq!(
                        before, observed,
                        "an observation misreported the cell state",
                    );
                }
                // Read-only outcomes never write the target.
                if matches!(
                    step.outcome,
                    Outcome::CacheHit(_)
                        | Outcome::Observed(_)
                        | Outcome::Shared
                        | Outcome::Skipped
                ) {
                    prop_assert_eq!(
                        before, after,
                        "a read-only step wrote the cell ({:?})",
                        step.op,
                    );
                }
            }
        }

        /// **RL4 — write-back purity (the cache is the exact probe
        /// allocation).** The pure-spine write-back caches the identical
        /// allocation the probe returned; every later read of a `Forced`
        /// cell returns a clone of that same allocation; a declined probe
        /// leaves no residue; and force state is invisible to value
        /// equality.
        ///
        /// Provenance: the `store` module docs' "pure-spine-only write-back"
        /// and `machine::LMachine::force` — `cell.set_forced(Rc::clone(&whnf))`
        /// on success, a cache hit returning the cached allocation unchanged —
        /// plus the `Cell` equality contract ("a cell is a derived cache,
        /// not part of a value's meaning").
        ///
        /// The content-addressed half of the heap is out of scope: a content
        /// entry *is* its own cache — identity of content is identity of
        /// value, so there is no write-back whose purity could be at issue;
        /// the condition is live exactly on the nominal memo-cell half, where
        /// a cache is derived and must not become meaning.
        #[test]
        fn write_back_purity_caches_the_exact_probe_allocation(ops in arb_heap_ops())
        {
            let trace = run(&ops);
            for step in &trace.steps {
                let Some(LiveCell(index)) = subject(step) else {
                    continue;
                };
                let (Some(before_views), Some(after_views)) =
                    (step.before.get(index), step.after.get(index))
                else {
                    continue;
                };
                match step.outcome {
                    | Outcome::WriteBack(ref ptr) => {
                        let cached = after_views
                            .at_address
                            .as_ref()
                            .and_then(|view| view.cached.as_ref());
                        prop_assert_eq!(
                            cached,
                            Some(ptr),
                            "the write-back copied or re-derived the probe allocation",
                        );
                    },
                    | Outcome::CacheHit(ref ptr) => {
                        let cached = before_views
                            .at_address
                            .as_ref()
                            .and_then(|view| view.cached.as_ref());
                        prop_assert_eq!(
                            cached,
                            Some(ptr),
                            "a cache hit returned something other than the cached allocation",
                        );
                    },
                    | Outcome::Declined => {
                        let after = after_views
                            .at_address
                            .as_ref()
                            .expect("a declined cell stays registered");
                        prop_assert_eq!(
                            &after.shape,
                            &StateShape::Unforced,
                            "a declined probe did not restore the unforced state",
                        );
                        prop_assert!(
                            after.cached.is_none(),
                            "a declined probe left cache residue",
                        );
                    },
                    | _ => {},
                }
            }
            // Cache stability: once forced, a cell's cached allocation never
            // changes across the whole trace.
            for index in 0..trace.cells.len() {
                let mut cached: Option<CachePtr> = None;
                for step in &trace.steps {
                    let view = step
                        .after
                        .get(index)
                        .and_then(|views| views.at_address.as_ref())
                        .and_then(|views| views.cached.as_ref());
                    if let Some(ptr) = view {
                        if let Some(existing) = cached.as_ref() {
                            prop_assert_eq!(
                                existing,
                                ptr,
                                "cell {}: the forced cache allocation changed",
                                index,
                            );
                        } else {
                            cached = Some(ptr.clone());
                        }
                    }
                }
            }
            // A cache is not meaning: equality is blind to identity and
            // force state, so any two handles compare equal — even handles
            // of distinct cells.
            for (left_index, left) in trace.cells.iter().enumerate() {
                for (right_index, right) in trace.cells.iter().enumerate() {
                    if left_index != right_index {
                        let left_handle = left
                            .handles
                            .first()
                            .expect("a tracked cell always has its allocation handle");
                        let right_handle = right
                            .handles
                            .first()
                            .expect("a tracked cell always has its allocation handle");
                        prop_assert!(
                            left_handle == right_handle,
                            "cell equality became sensitive to identity or force state",
                        );
                    }
                }
            }
        }
    }
}
