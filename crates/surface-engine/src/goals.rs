//! The goals report.
//!
//! Lists every hole with its origin span, the expected type at its
//! position, and the local typing context — "a goals report API returns
//! (byte range, expected type) for every hole". This is the seed of the
//! hole-goal surface the versioned [`crate::diag::Report`] carries; the
//! surface here is deliberately minimal — plain data, no serialization —
//! JSON arrives with the report envelope.
//!
//! # How goals are collected
//!
//! Two passes, merged by hole identifier:
//!
//! 1. **Static** — walk the [`OriginMap`]'s compatibility path readback and
//!    resolve every recorded path against its item's term; paths landing on a
//!    [`Value::Hole`] / [`Comp::Hole`] yield a goal skeleton: identifier, item,
//!    path, byte range, and the lowerer's [`HoleNote`].
//! 2. **Dynamic** — drive the typing **machine** (not the recursive checker:
//!    the machine exposes its state, and its heap-allocated frame stack is
//!    robust on generated/deep input) step by step over each item; at every
//!    state about to descend into a hole, record the direction's expected type
//!    and `Γ` at that point. Items are typed against their recorded ascription
//!    when its sort matches the term's sort, in inference mode otherwise.
//!
//! A hole the machine never reaches — because typing failed earlier, or the
//! enclosing item never types — keeps `expected = None` and
//! `ctx_local = None`: "where derivable", honestly. A hole reached in
//! inference mode has `expected = None` too (its type is `Unknown` by rule
//! Hole⇑; there is no goal type to serve).
//!
//! [SPECULATIVE DECISION] `ctx_local` reports only the bindings *beyond*
//! the caller-provided base context (the prelude is noise at every goal);
//! the base is implied. Bindings are outermost-to-innermost, shadowing
//! unresolved — consumers must take the last binding per name.

use alloc::collections::BTreeMap;
use core::ops::Deref;
use core::ops::Range;

use gandr_core_checker::judgements::control::Control;
use gandr_core_checker::judgements::control::Dir;
use gandr_core_incremental::region::Item;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::HoleId;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

use crate::boundary::ContextLength;
use crate::boundary::ItemIndex;
use crate::lower::Lowered;
use crate::origin::HoleNote;
use crate::origin::OriginMap;
use crate::origin::OriginPath;
use crate::origin::TermRef;
use crate::origin::resolve;

/// Static goal skeletons keyed by their hole identity.
#[repr(transparent)]
struct GoalMap(BTreeMap<HoleId, Goal>);

/// One hole identity at the dynamic-observation boundary.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct GoalHoleId(HoleId);

impl From<HoleId> for GoalHoleId
{
    #[inline]
    fn from(value: HoleId) -> Self
    {
        Self(value)
    }
}

/// Per-item indication that static lowering found a hole.
#[repr(transparent)]
pub(crate) struct GoalItemFlags(pub Vec<bool>);

impl Deref for GoalItemFlags
{
    type Target = [bool];

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

/// One hole's goal: where it is, what was elided, and — where derivable —
/// what type is expected there and under which local context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Goal
{
    /// The hole's identifier (unique within one [`Lowered`]).
    pub hole: HoleId,
    /// The index of the item containing the hole.
    pub item: usize,
    /// The hole's compatibility term path (item index followed by child
    /// indices; see [`crate::origin`]).
    pub path: OriginPath,
    /// The elided region's byte range in the source.
    pub byte_range: Range<usize>,
    /// What was elided (always present on lowerer-minted holes).
    pub note: Option<HoleNote>,
    /// The expected type at the hole, when the machine reached it in
    /// checking mode; [`None`] for inference-position holes and for holes
    /// typing never reached.
    pub expected: Option<Ty>,
    /// The bindings introduced *beyond* the base context by the time the
    /// hole is typed (outermost first), when the machine reached it;
    /// [`None`] for holes typing never reached.
    pub ctx_local: Option<Vec<(String, ValueType)>>,
}

/// Computes the goals report for a lowered file: one [`Goal`] per hole, in
/// (item, path) order, typed under `base` (e.g. [`crate::prelude_ctx`]).
///
/// # Contract
/// - requires: `base` is the typing context the items were lowered against
///   (e.g. `prelude_ctx`), so reported `ctx_local` excludes it.
/// - ensures: returns one `Goal` per hole in `lowered`, in (item, path) order;
///   each carries its span and note, plus the expected type and local `Γ` where
///   the machine reached the hole in checking mode (else `None`).
/// - provides: the goals report the versioned envelope carries.
/// - panics: none.
#[inline]
#[must_use]
pub fn goals_report(
    lowered: &Lowered,
    base: &Ctx,
) -> Vec<Goal>
{
    let mut goals = collect_static(&lowered.origin, lowered);
    for (item_index, item) in lowered.items.iter().enumerate() {
        observe_item(item_index.into(), item, base, &mut goals);
    }
    finish_goals(goals)
}

/// Computes a goals report where each item is typed under its own context.
///
/// # Contract
/// - requires: `bases` contains the context that was current immediately before
///   each corresponding item in `lowered`; missing contexts leave that item's
///   holes with static-only information.
/// - ensures: returns one `Goal` per hole in `lowered`, in (item, path) order.
/// - panics: none.
#[inline]
#[must_use]
pub(crate) fn goals_report_with_contexts(
    lowered: &Lowered,
    bases: &[Ctx],
) -> Vec<Goal>
{
    let mut goals = collect_static(&lowered.origin, lowered);
    for (item_index, item) in lowered.items.iter().enumerate() {
        let Some(base) = bases.get(item_index)
        else {
            continue;
        };
        observe_item(item_index.into(), item, base, &mut goals);
    }
    finish_goals(goals)
}

/// Returns which lowered items contain at least one hole goal.
///
/// # Contract
/// - ensures: returns one flag per lowered item; `true` means the static origin
///   pass found a value or computation hole in that item.
/// - panics: none.
#[inline]
#[must_use]
pub(crate) fn goal_item_flags(lowered: &Lowered) -> GoalItemFlags
{
    let mut flags = vec![false; lowered.items.len()];
    for goal in collect_static(&lowered.origin, lowered).0.into_values() {
        if let Some(flag) = flags.get_mut(goal.item) {
            *flag = true;
        }
    }
    GoalItemFlags(flags)
}

/// Pass 1: goal skeletons from the origin map (see the module doc).
fn collect_static(
    origin: &OriginMap,
    lowered: &Lowered,
) -> GoalMap
{
    let mut goals = BTreeMap::new();
    for (path, _id, entry) in origin.iter_paths() {
        let Some((&item_component, term_path)) = path.split_first()
        else {
            continue;
        };
        let item_index = usize::try_from(item_component).unwrap_or(usize::MAX);
        let Some(item) = lowered.items.get(item_index)
        else {
            continue;
        };
        let Some(TermRef::Value(&Value::Hole(hole)) | TermRef::Comp(&Comp::Hole(hole))) =
            resolve(&item.term, term_path)
        else {
            continue;
        };
        let _previous = goals.insert(hole, Goal {
            hole,
            item: item_index,
            path: path.clone(),
            byte_range: entry.byte_range.0.clone(),
            note: entry.note.clone(),
            expected: None,
            ctx_local: None,
        });
    }
    GoalMap(goals)
}

/// Sorts collected goals into the public report order.
fn finish_goals(goals: GoalMap) -> Vec<Goal>
{
    let mut report: Vec<Goal> = goals.0.into_values().collect();
    report.sort_by(|lhs, rhs| (lhs.item, &lhs.path).cmp(&(rhs.item, &rhs.path)));
    report
}

/// Pass 2: drive the machine over one item, recording expected type and
/// local `Γ` at every hole `Descend` (see the module doc).
fn observe_item(
    item_index: ItemIndex,
    item: &Item,
    base: &Ctx,
    goals: &mut GoalMap,
)
{
    let base_len = ContextLength(base.bindings().len());
    let mut state = initial_state(item, base);
    loop {
        match *state.control() {
            | Control::DescendValue {
                value: Value::Hole(id),
                ref dir,
            } => {
                let expected = match *dir {
                    | Dir::Check(ref ty) => Some(Ty::Value(ty.clone())),
                    // Only checking mode carries a goal type.
                    | Dir::Infer => None,
                };
                record(
                    goals,
                    item_index,
                    id.into(),
                    expected,
                    state.ctx(),
                    base_len,
                );
            },
            | Control::DescendComp {
                comp: Comp::Hole(id),
                ref dir,
            } => {
                let expected = match *dir {
                    | Dir::Check(ref ty) => Some(Ty::Comp(ty.clone())),
                    // As above: only checking mode carries a goal type.
                    | Dir::Infer => None,
                };
                record(
                    goals,
                    item_index,
                    id.into(),
                    expected,
                    state.ctx(),
                    base_len,
                );
            },
            | _ => {},
        }
        match gandr_core_machine::step(state) {
            | gandr_core_machine::Outcome::Step(next) => state = next,
            // Done and Error end observation.
            | gandr_core_machine::Outcome::Done(_) | gandr_core_machine::Outcome::Error { .. } => {
                return;
            },
        }
    }
}

/// Builds the initial machine [`State`](gandr_core_machine::State) for typing
/// one lowered item: against its recorded ascription when the sorts match,
/// otherwise in inference mode.
///
/// Shared by the goals report (pass 2 below) and the diagnostics surface
/// ([`crate::diag`]) so both drive items through the machine
/// identically. The dispatch is total over `Term`'s two sorts (its upstream
/// growth point is retired; an added sort is a compile-visible change here).
pub(crate) fn initial_state(
    item: &Item,
    base: &Ctx,
) -> gandr_core_machine::State
{
    match (&item.term, &item.ascription) {
        | (&Term::Value(ref value), &Some(Ty::Value(ref expected))) => {
            gandr_core_machine::State::new_value(
                base.clone(),
                value.clone(),
                Dir::Check(expected.clone()),
            )
        },
        | (&Term::Value(ref value), _) => {
            gandr_core_machine::State::new_value(base.clone(), value.clone(), Dir::Infer)
        },
        | (&Term::Comp(ref comp), &Some(Ty::Comp(ref expected))) => {
            gandr_core_machine::State::new_comp(
                base.clone(),
                comp.clone(),
                Dir::Check(expected.clone()),
            )
        },
        | (&Term::Comp(ref comp), _) => {
            gandr_core_machine::State::new_comp(base.clone(), comp.clone(), Dir::Infer)
        },
    }
}

/// Records one observed hole descend into its goal (if the static pass saw
/// it; holes belong to exactly one item, so a same-identifier observation
/// from another item is ignored).
///
/// # The first early return is noncontractual, and silence is the wrong shape
///
/// `gandr-7ej3`. The `goals.0.get_mut` miss guards against a hole the machine
/// descends into that [`collect_static`] never registered. **That is an engine
/// fact, not a user one**, and it is narrower than it looks: all **eight**
/// `fresh_hole` call sites in the lowerer pair the minted hole with an
/// `OriginNode` — four recovery and user-hole constructors, the dangling-
/// signature item, the two module-member repairs, and the stuck-pattern mint —
/// so every hole carries an origin entry by construction. The branch can
/// therefore fire only when an origin path fails to *resolve* to its hole in
/// the lowered term — origin and term structure having diverged.
///
/// No caller may rely on the omission, so the branch is noncontractual and
/// owes no exerciser. But returning silently is the posture
/// `buildout-standing-02` refuses: it records the engine's own gap as **a
/// shorter goals report**, telling the reader there are fewer unfinished holes
/// than there are, which degrades a claim about what the author wrote. The
/// invariant is one the reader should **refute** rather than one the producer
/// asserts, so this branch carries a debug assertion naming the hole and the
/// item while the release path keeps the safe return.
///
/// The second early return is a different animal and stays as it is: two items
/// in one lowering share a hole-id space, and ignoring a foreign item's
/// same-identifier observation is the correct behaviour rather than a guard
/// against an impossible state.
///
/// The `goals.rs` floor moves to its measured value with this change — the
/// policy target genuinely moved when the declared-data arm made those holes
/// collectable — and the `lower.rs` floor follows the lines that left it for
/// `lower/pattern.rs`.
fn record(
    goals: &mut GoalMap,
    item_index: ItemIndex,
    hole: GoalHoleId,
    expected: Option<Ty>,
    ctx: &Ctx,
    base_len: ContextLength,
)
{
    let Some(goal) = goals.0.get_mut(&hole.0)
    else {
        // Unreachable by construction; see the note above. Refuted in test
        // builds rather than asserted by staying quiet, because the release
        // path's silence would record an engine gap as a shorter goals report.
        debug_assert!(
            false,
            "goals: machine descended into hole {:?} in item {} that the static pass did not \
             collect; origin and term structure have diverged",
            hole.0, item_index.0
        );
        return;
    };
    if goal.item != item_index.0 {
        return;
    }
    goal.expected = expected;
    goal.ctx_local = Some(ctx.bindings().get(base_len.0 ..).unwrap_or(&[]).to_vec());
}

#[cfg(test)]
mod tests
{
    use gandr_core_incremental::footprint::footprint_of;

    use super::goal_item_flags;
    use crate::lower::lower_source_total;

    const RECOVERY_FIXTURES: [(&str, &str); 2] = [
        (
            "incomplete-input",
            include_str!("../tests/fixtures/current/incomplete-input.gandr"),
        ),
        (
            "parser-recovery",
            include_str!("../tests/fixtures/current/parser-recovery.gandr"),
        ),
    ];

    /// The goals pass and checkpoint engine agree on the per-item hole
    /// predicate over both established recovery fixtures.
    #[test]
    fn goal_flags_match_checkpoint_footprints_for_recovery_fixtures()
    {
        for (name, source) in RECOVERY_FIXTURES {
            let lowered =
                lower_source_total(source.into()).expect("total lowering must accept the fixture");
            let footprint_flags: Vec<bool> = lowered
                .items
                .iter()
                .map(|item| footprint_of(item).has_hole)
                .collect();
            assert_eq!(
                &*goal_item_flags(&lowered),
                footprint_flags.as_slice(),
                "goal and checkpoint hole predicates diverged for {name}"
            );
        }
    }
}
