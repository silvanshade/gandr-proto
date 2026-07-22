//! The goals report (`A2-PLAN.md` §A2.2).
//!
//! Lists every hole with its origin span, the expected type at its
//! position, and the local typing context — the plan's "a goals report API
//! returns (byte range, expected type) for every hole". This is the v0 seed
//! of the A2.4 agent stream (D7: "the same report carries hole goals"); the
//! surface here is deliberately minimal — plain data, no serialization —
//! JSON arrives with A2.4's `Report`.
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

use gandr_core_checker::control::Control;
use gandr_core_checker::control::Dir;
use gandr_core_checker::ctx::Ctx;
use gandr_core_checker::machine;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::HoleId;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;

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
#[non_exhaustive]
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
/// - provides: the v0 goals report (the A2.4 agent-stream seed).
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
    item: &crate::lower::LoweredItem,
    base: &Ctx,
    goals: &mut GoalMap,
)
{
    let base_len = ContextLength(base.bindings().len());
    let Some(mut state) = initial_state(item, base)
    else {
        // `Term` is non-exhaustive upstream; nothing to observe for an
        // unknown sort.
        return;
    };
    loop {
        match *state.control() {
            | Control::DescendValue {
                value: Value::Hole(id),
                ref dir,
            } => {
                let expected = match *dir {
                    | Dir::Check(ref ty) => Some(Ty::Value(ty.clone())),
                    // `Dir` is non-exhaustive upstream; only checking mode
                    // carries a goal type.
                    | _ => None,
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
                    | _ => None,
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
        match machine::step(state) {
            | machine::Outcome::Step(next) => state = next,
            // Done, Error, and any future terminal outcome end observation.
            | _ => return,
        }
    }
}

/// Builds the initial machine [`State`](machine::State) for typing one
/// lowered item: against its recorded ascription when the sorts match,
/// otherwise in inference mode.
///
/// Shared by the goals report (pass 2 below) and the A2.4 diagnostics
/// surface ([`crate::diag`]) so both drive items through the machine
/// identically. Returns [`None`] for an item whose term sort is unknown
/// (`Term` is non-exhaustive upstream).
pub(crate) fn initial_state(
    item: &crate::lower::LoweredItem,
    base: &Ctx,
) -> Option<machine::State>
{
    match (&item.term, &item.ascription) {
        | (&Term::Value(ref value), &Some(Ty::Value(ref expected))) => Some(
            machine::State::new_value(base.clone(), value.clone(), Dir::Check(expected.clone())),
        ),
        | (&Term::Value(ref value), _) => Some(machine::State::new_value(
            base.clone(),
            value.clone(),
            Dir::Infer,
        )),
        | (&Term::Comp(ref comp), &Some(Ty::Comp(ref expected))) => Some(machine::State::new_comp(
            base.clone(),
            comp.clone(),
            Dir::Check(expected.clone()),
        )),
        | (&Term::Comp(ref comp), _) => Some(machine::State::new_comp(
            base.clone(),
            comp.clone(),
            Dir::Infer,
        )),
        | _ => None,
    }
}

/// Records one observed hole descend into its goal (if the static pass saw
/// it; holes belong to exactly one item, so a same-identifier observation
/// from another item is ignored).
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
        return;
    };
    if goal.item != item_index.0 {
        return;
    }
    goal.expected = expected;
    goal.ctx_local = Some(ctx.bindings().get(base_len.0 ..).unwrap_or(&[]).to_vec());
}
