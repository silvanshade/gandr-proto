//! Edit-action reconstruction: a localized structured diff of the lowered CBPV
//! core (`A2`; `incremental-pipeline.md` §"pipeline-decision-02", the Porter
//! disposition; `edit-action work`).
//!
//! # The impedance gate this closes
//!
//! Porter et al.'s *Incremental Bidirectional Typing via Order Maintenance*
//! (arXiv:2504.08946) and Prinz et al.'s *Pantograph* (POPL 2025) both consume
//! a **structured edit-action** rather than raw text: Porter incrementally
//! re-types a marked program under a structural action calculus and leaves the
//! diff → action step out of scope; Pantograph accepts only structured
//! one-hole-context edits (not text diffs), and so always knows *which* child
//! an edit touched. (The corpus carries the verified harvest of both — Porter's
//! order-maintenance intervals and binding pointers, Pantograph's typed
//! error-boundary — in `incremental-pipeline.md` §"pipeline-decision-02" and
//! §"pipeline-decision-04"; the per-paper claims live there, not duplicated
//! here.) gandr's A2 front end, by contrast, produces a melder CST (the
//! merkle-hashed `gandr-surface-syntax` arena) and a re-lowered core term. This
//! module is the **translation layer** between the two: it reconstructs a
//! localized edit-action script over the lowered core from the before/after
//! lowerings, plus a byte-range **localizer** that maps a [`SourceEdit`]'s old
//! extent to the smallest enclosing core term (the *edit locus*).
//!
//! # Coverage with fallback (be honest)
//!
//! The papers fence this seam off because it is **partial**: a post-edit
//! snapshot cannot always tell which child changed, and arbitrary text
//! rearrangements (Pantograph §7's while-block ⇄ HTML) do not all map to
//! localized structured edits. So the contract here is **coverage with
//! fallback**, not total coverage:
//!
//! - **Soundness is total.** [`apply`]ing a [`diff`] to the old program always
//!   reproduces the new program (up to hole identifiers — typing ignores them;
//!   `syntax::HoleId`). This holds *regardless* of how well the diff localized:
//!   the worst case is a single coarse [`Action::Replace`] of a whole subtree
//!   (or an item alignment that deletes-and-reinserts rather than recognizing a
//!   move), which is exactly the "re-lower-then-diff" residual the bead names —
//!   expressed as an action, not a separate bail-out path.
//! - **Localization is partial.** Where the two terms share structure (the
//!   common case: an edit inside one definition), the diff descends to the
//!   changed node and emits a pin-point action ([`Action::SetInt`],
//!   [`Action::Rebind`], [`Action::FillHole`], …). Where they do not — a node
//!   whose constructor changed, or two unrelated items the name-alignment
//!   happened to pair — it emits a coarse subtree/item replacement. The quality
//!   axis is "how localized", never "is it correct".
//!
//! # What is and is not reconstructed
//!
//! [`apply`] operates on the **term forest** ([`LoweredItem`]s — names,
//! ascriptions, and core terms), *not* on the [`OriginMap`]: byte ranges and
//! CST identity are a function of lowering, and reconstructing them from an
//! action script is the order-maintenance-over-CST resync problem
//! (`CST-resynchronization work`, Pterodactyl relative positioning),
//! deliberately out of scope here. The [`localize`] half uses the existing
//! [`OriginMap`] byte-range nesting as its positioning substrate: a byte->node
//! *stabbing* query answered in **O(depth)** by descending the nesting (the
//! ranges nest, so the smallest enclosing entry is reached by stepping into the
//! containing child at each level — never scanning the whole map,
//! `stable-origin work`). The `gandr-theory-orders` `Interval` containment is a
//! *different* query — O(1) node->node *ancestry*, the query the future
//! dirty-frontier engine (A2.3) consumes, not a drop-in speed-up for this
//! byte->node lookup. Rebasing the [`OriginMap`] onto order-maintenance keys,
//! so a whitespace reparse leaves positions invariant, is the OM-over-CST
//! resync (`CST-resynchronization work`).
//!
//! # Index conventions
//!
//! Path-addressed actions and the item-metadata / [`Action::DeleteItem`]
//! actions are anchored in the **old** program — they describe how to transform
//! old material — so their item index is an index into the *old* item list.
//! [`Action::InsertItem`] introduces material with no old anchor and so carries
//! a **new**-list index. A path's first component is that item index; the
//! remaining components are core term-child indices in the same order as
//! [`crate::origin::resolve`], so an action's path resolves directly in the
//! [`OriginMap`] (the byte range an agent surfaces for it).

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::rc::Rc;

use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::grade::Grade;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::OpClause;
use gandr_core_checker::syntax::Side;
use gandr_core_checker::syntax::SplitMotive;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;

use crate::boundary::AlignmentOffset;
use crate::boundary::DefinitionName;
#[cfg(test)]
use crate::boundary::ItemCount;
use crate::boundary::ItemIndex;
use crate::boundary::MatchDecision;
use crate::boundary::OptionalDefinitionName;
use crate::boundary::OriginEntryCount;
use crate::boundary::OriginPathComponent;
#[cfg(test)]
use crate::boundary::PipelineSource;
use crate::boundary::RebuiltValueCount;
use crate::boundary::SourceLength;
use crate::boundary::SourceOffset;
#[cfg(test)]
use crate::boundary::TreeDepth;
use crate::lower::Lowered;
use crate::lower::LoweredItem;
use crate::origin::OriginEntry;
use crate::origin::OriginMap;
use crate::origin::OriginPath;

/// Which binder of a node a [`Action::Rebind`] renames.
///
/// `Abs` and `Bind` have one binder ([`Self::Sole`]); `Case`, `Split`, and
/// `ListCase` have two ([`Self::Fst`] = the `inj1` arm / first component /
/// list-case `head`, [`Self::Snd`] = the `inj2` arm / second component /
/// list-case `tail`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinderSlot
{
    /// The sole binder of an `Abs` or `Bind`.
    Sole,
    /// The first-arm (`inj1`) / first-component binder of a `Case` / `Split`.
    Fst,
    /// The second-arm (`inj2`) / second-component binder of a `Case` /
    /// `Split`.
    Snd,
}

/// Which type position a [`Action::SetAnnotation`] retypes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnnSlot
{
    /// The ascription of a `Value::Annot` (always present).
    Value,
    /// The optional binder annotation of a `Comp::Abs` (`None` ⇄ `Some` are
    /// both valid edits).
    AbsBinder,
}

/// A replacement subtree of either sort, carried by the coarse actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Subtree
{
    /// A value subtree (installed at a value-position path).
    Value(
        /// The value.
        Value,
    ),
    /// A computation subtree (installed at a computation-position path).
    Comp(
        /// The computation.
        Comp,
    ),
}

/// One localized structured edit-action (the Porter-style action calculus,
/// extended for gandr's CBPV / grade / hole constructs).
///
/// The item-list actions ([`Self::InsertItem`], [`Self::DeleteItem`],
/// [`Self::SetItemAscription`]) edit the top-level
/// definition list; the remaining, path-addressed actions edit one core term
/// (see the module doc's index conventions). The three *coarse* actions
/// ([`Self::Replace`], [`Self::FillHole`], [`Self::EraseToHole`]) all install a
/// whole new subtree — [`apply`] treats them identically; they are kept
/// distinct because the distinction is exactly the signal a consumer (the A2.4
/// agent stream) wants: a hole *filled*, a term *erased to a hole*, or a
/// constructor *replaced*.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action
{
    /// A new top-level item, positioned by its **new**-list index.
    InsertItem
    {
        /// The new item's index in the new item list.
        at: usize,
        /// The item to introduce.
        item: LoweredItem,
    },
    /// A top-level item removed, by its **old**-list index.
    DeleteItem
    {
        /// The removed item's index in the old item list.
        at: usize,
    },
    /// An item's recorded ascription changed, by its **old**-list index.
    SetItemAscription
    {
        /// The kept item's index in the old item list.
        at: usize,
        /// The old ascription.
        from: Option<Ty>,
        /// The new ascription.
        to: Option<Ty>,
    },
    /// A subtree whose constructor changed: replaced wholesale (the coarse
    /// residual — the most localized action available when the node does not
    /// align).
    Replace
    {
        /// The path of the replaced node.
        path: OriginPath,
        /// The new subtree.
        to: Subtree,
    },
    /// A hole filled with a concrete subtree (`?` ⇒ a term) — the editor's
    /// primary action.
    FillHole
    {
        /// The path of the hole.
        path: OriginPath,
        /// The concrete subtree the hole became.
        to: Subtree,
    },
    /// A concrete subtree erased to a hole (a term ⇒ `?`).
    EraseToHole
    {
        /// The path of the erased subtree.
        path: OriginPath,
        /// The new hole subtree (carrying the new hole identifier).
        to: Subtree,
    },
    /// An integer literal changed in place.
    SetInt
    {
        /// The path of the literal.
        path: OriginPath,
        /// The old value.
        from: i64,
        /// The new value.
        to: i64,
    },
    /// A variable occurrence renamed in place.
    SetVar
    {
        /// The path of the variable.
        path: OriginPath,
        /// The old name.
        from: String,
        /// The new name.
        to: String,
    },
    /// A thunk's usage grade changed in place.
    SetGrade
    {
        /// The path of the `Thunk`.
        path: OriginPath,
        /// The old grade.
        from: Grade,
        /// The new grade.
        to: Grade,
    },
    /// An injection's / projection's side flipped in place.
    SetSide
    {
        /// The path of the `Inj` / `Prj`.
        path: OriginPath,
        /// The old side.
        from: Side,
        /// The new side.
        to: Side,
    },
    /// A binder renamed in place (the binder is an attribute, not a child, so
    /// this composes with edits *inside* the node's body).
    Rebind
    {
        /// The path of the binding node (`Abs` / `Bind` / `Case` / `Split`).
        path: OriginPath,
        /// Which binder of the node.
        slot: BinderSlot,
        /// The old binder name.
        from: String,
        /// The new binder name.
        to: String,
    },
    /// An ascription (`Value::Annot`) or binder annotation (`Comp::Abs`)
    /// changed in place; composes with edits inside the annotated term.
    SetAnnotation
    {
        /// The path of the annotated node.
        path: OriginPath,
        /// Which annotation position.
        slot: AnnSlot,
        /// The old type (`None` only for an absent `Abs` binder annotation).
        from: Option<ValueType>,
        /// The new type (`None` only for an absent `Abs` binder annotation).
        to: Option<ValueType>,
    },
}

impl Action
{
    /// The action's path, for the path-addressed variants; [`None`] for the
    /// item-list variants.
    #[inline]
    #[must_use]
    pub fn path(&self) -> Option<&OriginPath>
    {
        match *self {
            | Self::Replace { ref path, .. }
            | Self::FillHole { ref path, .. }
            | Self::EraseToHole { ref path, .. }
            | Self::SetInt { ref path, .. }
            | Self::SetVar { ref path, .. }
            | Self::SetGrade { ref path, .. }
            | Self::SetSide { ref path, .. }
            | Self::Rebind { ref path, .. }
            | Self::SetAnnotation { ref path, .. } => Some(path),
            | Self::InsertItem { .. }
            | Self::DeleteItem { .. }
            | Self::SetItemAscription { .. } => None,
        }
    }
}

/// A reconstructed edit-action script transforming an old lowered program into
/// a new one (the order of `actions` is informative only; [`apply`] buckets
/// them).
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EditScript
{
    /// The actions, in the order the diff emitted them (item-list edits, then
    /// per-matched-item metadata and term edits in old-list order).
    pub actions: Vec<Action>,
}

impl EditScript
{
    /// A deterministic, line-oriented rendering for golden tests: one action
    /// per line, in `actions` order.
    #[inline]
    #[must_use]
    pub fn snapshot(&self) -> String
    {
        let mut lines: Vec<String> = Vec::new();
        for action in &self.actions {
            lines.push(format!("{action:?}\n"));
        }
        lines.concat()
    }
}

/// Reconstructs the edit-action script transforming `old` into `new` (the
/// whole-program convenience over [`diff_items`]).
///
/// # Contract
/// - ensures: returns an [`EditScript`] for which `apply(&old.items, &script)`
///   equals `new.items` up to hole identifiers (see [`apply`]).
/// - provides: a localized structured diff of the two lowerings, falling back
///   to coarse subtree/item replacement where they do not align.
/// - panics: none.
#[inline]
#[must_use]
pub fn diff(
    old: &Lowered,
    new: &Lowered,
) -> EditScript
{
    diff_items(&old.items, &new.items)
}

/// Reconstructs the edit-action script over two item lists (the origin-free
/// core of [`diff`]).
///
/// Items are aligned by name (a longest-common-subsequence over the items'
/// `name` keys, so unmatched old items become [`Action::DeleteItem`] and
/// unmatched new items [`Action::InsertItem`]); each matched pair is diffed
/// structurally. The alignment is sound for any inputs — a mis-pairing only
/// yields coarser actions, never a wrong [`apply`] result.
///
/// # Contract
/// - ensures: returns an [`EditScript`] for which `apply(old, &script)` equals
///   `new` up to hole identifiers (see [`apply`]).
/// - provides: the localized structured diff; coarse replacement is the
///   documented fallback, not a separate path.
/// - panics: none.
#[inline]
#[must_use]
pub fn diff_items(
    old: &[LoweredItem],
    new: &[LoweredItem],
) -> EditScript
{
    let old_keys: Vec<OptionalDefinitionName<'_>> =
        old.iter().map(|item| item.name.as_deref().into()).collect();
    let new_keys: Vec<OptionalDefinitionName<'_>> =
        new.iter().map(|item| item.name.as_deref().into()).collect();
    let matches = align(&old_keys, &new_keys);
    let matched_old: BTreeSet<ItemIndex> =
        matches.iter().map(|&(old_index, _)| old_index).collect();
    let matched_new: BTreeSet<ItemIndex> =
        matches.iter().map(|&(_, new_index)| new_index).collect();

    let mut actions: Vec<Action> = Vec::new();

    // Deletions: old items with no new counterpart (old-list indices).
    for old_index in 0 .. old.len() {
        if !matched_old.contains(&old_index.into()) {
            actions.push(Action::DeleteItem { at: old_index });
        }
    }
    // Insertions: new items with no old counterpart (new-list indices).
    for (new_index, item) in new.iter().enumerate() {
        if !matched_new.contains(&new_index.into()) {
            actions.push(Action::InsertItem {
                at: new_index,
                item: item.clone(),
            });
        }
    }
    // Matched pairs: metadata and term edits, anchored at the old index.
    for &(old_index, new_index) in &matches {
        let (old_index, new_index) = (usize::from(old_index), usize::from(new_index));
        let (Some(old_item), Some(new_item)) = (old.get(old_index), new.get(new_index))
        else {
            // Unreachable: `align` only returns in-range index pairs. The guard
            // keeps the surface total.
            continue;
        };
        // `align` pairs only equal-name keys, so a matched pair's names are
        // always equal — a *rename* is reconstructed as delete-and-reinsert,
        // not an in-place name edit (so there is no item-rename action). Only
        // the ascription can differ within a matched pair.
        if old_item.ascription != new_item.ascription {
            actions.push(Action::SetItemAscription {
                at: old_index,
                from: old_item.ascription.clone(),
                to: new_item.ascription.clone(),
            });
        }
        let prefix: OriginPath = vec![u32::try_from(old_index).unwrap_or(u32::MAX)].into();
        diff_term(&prefix, &old_item.term, &new_item.term, &mut actions);
    }
    EditScript { actions }
}

// --- Item alignment ----------------------------------------------------------

/// Aligns two key sequences by a longest common subsequence, returning the
/// matched `(old_index, new_index)` pairs in increasing order.
///
/// Keys are item names (`None` for anonymous items, which therefore match each
/// other in order). The alignment is sound for any inputs; mis-pairings only
/// coarsen the diff.
fn align(
    old: &[OptionalDefinitionName<'_>],
    new: &[OptionalDefinitionName<'_>],
) -> Vec<(ItemIndex, ItemIndex)>
{
    let rows = old.len();
    let cols = new.len();
    // `table[i][j]` = LCS length of `old[i..]` and `new[j..]`, stored row-major
    // over `(rows + 1) * (cols + 1)`.
    let stride = cols.saturating_add(1);
    let cells = rows.saturating_add(1).saturating_mul(stride);
    let mut table: Vec<usize> = vec![0; cells];
    let index = |row: ItemIndex, col: ItemIndex| {
        AlignmentOffset::from(
            usize::from(row)
                .saturating_mul(stride)
                .saturating_add(usize::from(col)),
        )
    };

    for row in (0 .. rows).rev() {
        for col in (0 .. cols).rev() {
            let value = if old.get(row) == new.get(col) {
                table
                    .get(usize::from(index(
                        row.saturating_add(1).into(),
                        col.saturating_add(1).into(),
                    )))
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(1)
            }
            else {
                let down = table
                    .get(usize::from(index(row.saturating_add(1).into(), col.into())))
                    .copied()
                    .unwrap_or(0);
                let right = table
                    .get(usize::from(index(row.into(), col.saturating_add(1).into())))
                    .copied()
                    .unwrap_or(0);
                down.max(right)
            };
            if let Some(slot) = table.get_mut(usize::from(index(row.into(), col.into()))) {
                *slot = value;
            }
        }
    }

    let mut matches: Vec<(ItemIndex, ItemIndex)> = Vec::new();
    let mut row = 0_usize;
    let mut col = 0_usize;
    while row < rows && col < cols {
        if old.get(row) == new.get(col) {
            matches.push((row.into(), col.into()));
            row = row.saturating_add(1);
            col = col.saturating_add(1);
        }
        else {
            let down = table
                .get(usize::from(index(row.saturating_add(1).into(), col.into())))
                .copied()
                .unwrap_or(0);
            let right = table
                .get(usize::from(index(row.into(), col.saturating_add(1).into())))
                .copied()
                .unwrap_or(0);
            if down >= right {
                row = row.saturating_add(1);
            }
            else {
                col = col.saturating_add(1);
            }
        }
    }
    matches
}

/// Diffs two terms of (possibly differing) sort at `prefix`.
fn diff_term(
    prefix: &OriginPath,
    old: &Term,
    new: &Term,
    out: &mut Vec<Action>,
)
{
    match (old, new) {
        | (&Term::Value(ref old_value), &Term::Value(ref new_value)) => {
            diff_nodes(
                DiffTask::Value {
                    path: prefix.to_owned(),
                    old: old_value,
                    new: new_value,
                },
                out,
            );
        },
        | (&Term::Comp(ref old_comp), &Term::Comp(ref new_comp)) => {
            diff_nodes(
                DiffTask::Comp {
                    path: prefix.to_owned(),
                    old: old_comp,
                    new: new_comp,
                },
                out,
            );
        },
        // Sort changed at the root: replace wholesale with whichever sort the
        // new term has.
        | (_, &Term::Value(ref new_value)) => out.push(Action::Replace {
            path: prefix.to_vec().into(),
            to: Subtree::Value(new_value.clone()),
        }),
        | (_, &Term::Comp(ref new_comp)) => out.push(Action::Replace {
            path: prefix.to_vec().into(),
            to: Subtree::Comp(new_comp.clone()),
        }),
    }
}

/// Pushes a [`Action::SetAnnotation`] for an `Abs` binder annotation that
/// changed (including `None` ⇄ `Some`).
fn diff_abs_annotation(
    prefix: &OriginPath,
    old_ann: Option<&Rc<ValueType>>,
    new_ann: Option<&Rc<ValueType>>,
    out: &mut Vec<Action>,
)
{
    let old_ty = old_ann.map(|ty| (**ty).clone());
    let new_ty = new_ann.map(|ty| (**ty).clone());
    if old_ty != new_ty {
        out.push(Action::SetAnnotation {
            path: prefix.to_vec().into(),
            slot: AnnSlot::AbsBinder,
            from: old_ty,
            to: new_ty,
        });
    }
}

/// One pending structural comparison in the iterative differ.
enum DiffTask<'term>
{
    /// Compare two value nodes at `path`.
    Value
    {
        /// Origin path of the compared node.
        path: OriginPath,
        /// Node from the previous program.
        old: &'term Value,
        /// Node from the current program.
        new: &'term Value,
    },
    /// Compare two computation nodes at `path`.
    Comp
    {
        /// Origin path of the compared node.
        path: OriginPath,
        /// Node from the previous program.
        old: &'term Comp,
        /// Node from the current program.
        new: &'term Comp,
    },
}

/// Drains structural comparisons without using the Rust call stack.
fn diff_nodes(
    initial: DiffTask<'_>,
    out: &mut Vec<Action>,
)
{
    let mut pending = vec![initial];
    while let Some(task) = pending.pop() {
        match task {
            | DiffTask::Value { path, old, new } => {
                diff_value_step(&path, old, new, &mut pending, out);
            },
            | DiffTask::Comp { path, old, new } => {
                diff_comp_step(&path, old, new, &mut pending, out);
            },
        }
    }
}

/// Diffs two values at `prefix` (see the module doc; holes compare equal
/// regardless of identifier).
fn diff_value_step<'term>(
    prefix: &OriginPath,
    old: &'term Value,
    new: &'term Value,
    pending: &mut Vec<DiffTask<'term>>,
    out: &mut Vec<Action>,
)
{
    match (old, new) {
        // Holes are equal up to their identifier (typing ignores it); units
        // carry no payload.
        | (&Value::Hole(_), &Value::Hole(_)) | (&Value::Unit, &Value::Unit) => {},
        | (&Value::Hole(_), _) => out.push(Action::FillHole {
            path: prefix.to_vec().into(),
            to: Subtree::Value(new.clone()),
        }),
        | (_, &Value::Hole(_)) => out.push(Action::EraseToHole {
            path: prefix.to_vec().into(),
            to: Subtree::Value(new.clone()),
        }),
        | (&Value::Var(ref old_name), &Value::Var(ref new_name)) => {
            if old_name != new_name {
                out.push(Action::SetVar {
                    path: prefix.to_vec().into(),
                    from: old_name.clone(),
                    to: new_name.clone(),
                });
            }
        },
        | (&Value::Int(old_int), &Value::Int(new_int)) => {
            if old_int != new_int {
                out.push(Action::SetInt {
                    path: prefix.to_vec().into(),
                    from: old_int,
                    to: new_int,
                });
            }
        },
        | (&Value::Pair(ref old_fst, ref old_snd), &Value::Pair(ref new_fst, ref new_snd)) => {
            pending.push(DiffTask::Value {
                path: child_path(prefix, 1.into()),
                old: old_snd,
                new: new_snd,
            });
            pending.push(DiffTask::Value {
                path: child_path(prefix, 0.into()),
                old: old_fst,
                new: new_fst,
            });
        },
        // A list literal (list-former design): when the lengths match, the elements align
        // 1:1 and the diff descends structurally into each (the `Pair`
        // discipline, n-ary). A length change does not align — the localized
        // diff cannot yet express a list-element insert/delete (a residual
        // mirroring the deferred string `SetStr` / numeric `SetNum`) — so it
        // replaces wholesale.
        | (&Value::List(ref old_elements), &Value::List(ref new_elements)) => {
            if old_elements.len() == new_elements.len() {
                for (index, (old_element, new_element)) in old_elements
                    .iter()
                    .zip(new_elements.iter())
                    .enumerate()
                    .rev()
                {
                    let child = u32::try_from(index).unwrap_or(u32::MAX);
                    pending.push(DiffTask::Value {
                        path: child_path(prefix, child.into()),
                        old: old_element,
                        new: new_element,
                    });
                }
            }
            else {
                out.push(Action::Replace {
                    path: prefix.to_vec().into(),
                    to: Subtree::Value(new.clone()),
                });
            }
        },
        // A record literal (record-former design): when the label sets match, the fields align
        // 1:1 by label (in canonical sorted order) and the diff descends
        // structurally into each (the `Pair` / `List` discipline, n-ary and
        // label-keyed). A label-set change does not align — the localized diff
        // cannot yet express a record field insert/delete (a residual mirroring
        // the deferred list `SetList`) — so it replaces wholesale. Critically,
        // this arm exists so a record self-diff descends to nothing instead of
        // hitting the `_` arm's unconditional `Replace` (the self-diff-empty
        // invariant — the latent bug class the `Str` rung fixed).
        | (&Value::Record(ref old_fields), &Value::Record(ref new_fields)) => {
            if old_fields.keys().eq(new_fields.keys()) {
                for (index, (old_field, new_field)) in old_fields
                    .values()
                    .zip(new_fields.values())
                    .enumerate()
                    .rev()
                {
                    let child = u32::try_from(index).unwrap_or(u32::MAX);
                    pending.push(DiffTask::Value {
                        path: child_path(prefix, child.into()),
                        old: old_field,
                        new: new_field,
                    });
                }
            }
            else {
                out.push(Action::Replace {
                    path: prefix.to_vec().into(),
                    to: Subtree::Value(new.clone()),
                });
            }
        },
        | (&Value::Inj(old_side, ref old_payload), &Value::Inj(new_side, ref new_payload)) => {
            if old_side != new_side {
                out.push(Action::SetSide {
                    path: prefix.to_vec().into(),
                    from: old_side,
                    to: new_side,
                });
            }
            pending.push(DiffTask::Value {
                path: child_path(prefix, 0.into()),
                old: old_payload,
                new: new_payload,
            });
        },
        | (&Value::Annot(ref old_inner, ref old_ty), &Value::Annot(ref new_inner, ref new_ty)) => {
            if old_ty != new_ty {
                out.push(Action::SetAnnotation {
                    path: prefix.to_vec().into(),
                    slot: AnnSlot::Value,
                    from: Some((**old_ty).clone()),
                    to: Some((**new_ty).clone()),
                });
            }
            pending.push(DiffTask::Value {
                path: child_path(prefix, 0.into()),
                old: old_inner,
                new: new_inner,
            });
        },
        | (&Value::Thunk(old_grade, ref old_body), &Value::Thunk(new_grade, ref new_body)) => {
            if old_grade != new_grade {
                out.push(Action::SetGrade {
                    path: prefix.to_vec().into(),
                    from: old_grade,
                    to: new_grade,
                });
            }
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 0.into()),
                old: old_body,
                new: new_body,
            });
        },
        // A string literal (string-literal design), a typed numeric literal (numeric-literal
        // design), and a reified stack are all opaque to the diff: an unchanged one emits
        // no action — so a self-diff over such a term stays empty — and a changed
        // one is a wholesale replacement, compared by exact structure (the whole
        // node, *not* a per-field comparison — the latent self-diff bug the Str
        // rung fixed). Granular sub-edits are deferred (a string `SetStr` and a
        // numeric `SetNum` mirroring `SetInt`, `scalar-edit refinement`; descent into a
        // stack, `deep edit descent`).
        | (&Value::Str(_), &Value::Str(_))
        | (&Value::Num(_), &Value::Num(_))
        | (&Value::Stk(_), &Value::Stk(_)) => {
            if old != new {
                out.push(Action::Replace {
                    path: prefix.to_vec().into(),
                    to: Subtree::Value(new.clone()),
                });
            }
        },
        // Different concrete constructors: replace wholesale.
        | _ => out.push(Action::Replace {
            path: prefix.to_vec().into(),
            to: Subtree::Value(new.clone()),
        }),
    }
}

/// Pushes a [`Action::Rebind`] when a binder name changed.
fn rebind_if_changed(
    prefix: &OriginPath,
    slot: BinderSlot,
    old_name: DefinitionName<'_>,
    new_name: DefinitionName<'_>,
    out: &mut Vec<Action>,
)
{
    if old_name.as_ref() != new_name.as_ref() {
        out.push(Action::Rebind {
            path: prefix.to_owned(),
            slot,
            from: old_name.as_ref().to_owned(),
            to: new_name.as_ref().to_owned(),
        });
    }
}

/// The actions targeting exactly `path`.
fn actions_at<'plan>(
    plan: &'plan BTreeMap<OriginPath, Vec<&'plan Action>>,
    path: &OriginPath,
) -> &'plan [&'plan Action]
{
    plan.get(path).map_or(&[], Vec::as_slice)
}

/// One pending node or assembly frame in the iterative edit rebuilder.
enum RebuildTask<'term>
{
    /// Rebuild a value node.
    Value
    {
        /// Value node to rebuild.
        value: &'term Value,
        /// Origin path the plan is consulted at.
        path: OriginPath,
    },
    /// Rebuild a computation node.
    Comp
    {
        /// Computation node to rebuild.
        comp: &'term Comp,
        /// Origin path the plan is consulted at.
        path: OriginPath,
    },
    /// Assemble a parent after its children have been rebuilt.
    Build(RebuildFrame),
}

/// Constructor metadata retained while child nodes are rebuilt.
enum RebuildFrame
{
    /// Rebuild a value pair.
    Pair,
    /// Rebuild a sum injection on the given side.
    Inj(Side),
    /// Rebuild a type ascription with the ascribed type.
    Annot(ValueType),
    /// Rebuild a thunk with its grade.
    Thunk(Grade),
    /// Rebuild a list from this many rebuilt elements.
    List(usize),
    /// Rebuild a record with these field labels in order.
    Record(Vec<String>),
    /// Rebuild a lambda with its binder and optional parameter ascription.
    Abs(String, Option<Rc<ValueType>>),
    /// Rebuild an application.
    App,
    /// Rebuild a returner.
    Ret,
    /// Rebuild a monadic bind with its binder.
    Bind(String),
    /// Rebuild a force.
    Force,
    /// Rebuild a data/coproduct case with the constructor and binder names.
    Case(String, String),
    /// Rebuild a pair split.
    Split
    {
        /// First component binder.
        fst_name: String,
        /// Second component binder.
        snd_name: String,
        /// Optional split motive ascription.
        motive: Option<Box<SplitMotive>>,
    },
    /// Rebuild a list case.
    ListCase
    {
        /// Head binder.
        head: String,
        /// Tail binder.
        tail: String,
    },
    /// Rebuild a with (copair).
    With,
    /// Rebuild a projection on the given side.
    Prj(Side),
    /// Rebuild a record projection on this label.
    RecordProj(String),
    /// Rebuild a duplication.
    Dup,
    /// Rebuild a drop.
    Drop,
    /// Rebuild an effect perform with its signature and operation.
    Perform(Box<EffectSig>, String),
    /// Rebuild an effect handle.
    Handle
    {
        /// Handled effect signature.
        sig: Box<EffectSig>,
        /// Return-clause binder.
        ret_name: String,
        /// Operation clauses as `(operation, parameter, continuation)` names.
        clauses: Vec<(String, String, String)>,
    },
    /// Rebuild a resume.
    Resume,
    /// Rebuild a reset.
    Reset,
    /// Rebuild a shift with its continuation binder.
    Shift(String),
}

/// One rebuilt node on the post-order result stack.
enum Rebuilt
{
    /// A rebuilt value node.
    Value(Value),
    /// A rebuilt computation node.
    Comp(Comp),
}

/// Rebuilds one value or computation tree with explicit pending/result stacks.
fn rebuild_node(
    initial: RebuildTask<'_>,
    plan: &BTreeMap<OriginPath, Vec<&Action>>,
) -> Option<Rebuilt>
{
    let mut pending = vec![initial];
    let mut rebuilt = Vec::new();
    while let Some(task) = pending.pop() {
        match task {
            | RebuildTask::Value { value, path } => {
                let here = actions_at(plan, &path);
                if let Some(installed) = terminal_value(here) {
                    rebuilt.push(Rebuilt::Value(installed));
                    continue;
                }
                match *value {
                    | Value::Pair(ref fst, ref snd) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Pair));
                        pending.push(RebuildTask::Value {
                            value: snd,
                            path: child_path(&path, 1.into()),
                        });
                        pending.push(RebuildTask::Value {
                            value: fst,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Value::Inj(side, ref payload) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Inj(
                            attr_side(here).unwrap_or(side),
                        )));
                        pending.push(RebuildTask::Value {
                            value: payload,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Value::Annot(ref inner, ref ty) => {
                        let ty = attr_value_annotation(here).unwrap_or_else(|| (**ty).clone());
                        pending.push(RebuildTask::Build(RebuildFrame::Annot(ty)));
                        pending.push(RebuildTask::Value {
                            value: inner,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Value::Thunk(grade, ref body) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Thunk(
                            attr_grade(here).unwrap_or(grade),
                        )));
                        pending.push(RebuildTask::Comp {
                            comp: body,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Value::List(ref elements) => {
                        pending.push(RebuildTask::Build(RebuildFrame::List(elements.len())));
                        for (index, element) in elements.iter().enumerate().rev() {
                            let child = u32::try_from(index).unwrap_or(u32::MAX);
                            pending.push(RebuildTask::Value {
                                value: element,
                                path: child_path(&path, child.into()),
                            });
                        }
                    },
                    | Value::Record(ref fields) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Record(
                            fields.keys().cloned().collect(),
                        )));
                        for (index, field) in fields.values().enumerate().rev() {
                            let child = u32::try_from(index).unwrap_or(u32::MAX);
                            pending.push(RebuildTask::Value {
                                value: field,
                                path: child_path(&path, child.into()),
                            });
                        }
                    },
                    | _ => rebuilt.push(Rebuilt::Value(value.clone())),
                }
            },
            | RebuildTask::Comp { comp, path } => {
                let here = actions_at(plan, &path);
                if let Some(installed) = terminal_comp(here) {
                    rebuilt.push(Rebuilt::Comp(installed));
                    continue;
                }
                match *comp {
                    | Comp::Abs(ref name, ref ann, ref body) => {
                        let name =
                            attr_rebind(here, BinderSlot::Sole).unwrap_or_else(|| name.clone());
                        let ann = resolve_abs_annotation(here, ann.as_ref());
                        pending.push(RebuildTask::Build(RebuildFrame::Abs(name, ann)));
                        pending.push(RebuildTask::Comp {
                            comp: body,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::App(ref head, ref arg) => {
                        pending.push(RebuildTask::Build(RebuildFrame::App));
                        pending.push(RebuildTask::Value {
                            value: arg,
                            path: child_path(&path, 1.into()),
                        });
                        pending.push(RebuildTask::Comp {
                            comp: head,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Ret(ref value) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Ret));
                        pending.push(RebuildTask::Value {
                            value,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Bind(ref bound, ref name, ref cont) => {
                        let name =
                            attr_rebind(here, BinderSlot::Sole).unwrap_or_else(|| name.clone());
                        pending.push(RebuildTask::Build(RebuildFrame::Bind(name)));
                        pending.push(RebuildTask::Comp {
                            comp: cont,
                            path: child_path(&path, 1.into()),
                        });
                        pending.push(RebuildTask::Comp {
                            comp: bound,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Force(ref value) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Force));
                        pending.push(RebuildTask::Value {
                            value,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Case(
                        ref scrut,
                        (ref left_name, ref left),
                        (ref right_name, ref right),
                    ) => {
                        let left_name =
                            attr_rebind(here, BinderSlot::Fst).unwrap_or_else(|| left_name.clone());
                        let right_name = attr_rebind(here, BinderSlot::Snd)
                            .unwrap_or_else(|| right_name.clone());
                        pending.push(RebuildTask::Build(RebuildFrame::Case(
                            left_name, right_name,
                        )));
                        pending.push(RebuildTask::Comp {
                            comp: right,
                            path: child_path(&path, 2.into()),
                        });
                        pending.push(RebuildTask::Comp {
                            comp: left,
                            path: child_path(&path, 1.into()),
                        });
                        pending.push(RebuildTask::Value {
                            value: scrut,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Split {
                        ref scrut,
                        ref fst_name,
                        ref snd_name,
                        ref motive,
                        ref body,
                    } => {
                        let fst_name =
                            attr_rebind(here, BinderSlot::Fst).unwrap_or_else(|| fst_name.clone());
                        let snd_name =
                            attr_rebind(here, BinderSlot::Snd).unwrap_or_else(|| snd_name.clone());
                        pending.push(RebuildTask::Build(RebuildFrame::Split {
                            fst_name,
                            snd_name,
                            motive: motive.clone(),
                        }));
                        pending.push(RebuildTask::Comp {
                            comp: body,
                            path: child_path(&path, 1.into()),
                        });
                        pending.push(RebuildTask::Value {
                            value: scrut,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::ListCase {
                        ref scrut,
                        ref nil,
                        ref head,
                        ref tail,
                        ref cons,
                    } => {
                        let head =
                            attr_rebind(here, BinderSlot::Fst).unwrap_or_else(|| head.clone());
                        let tail =
                            attr_rebind(here, BinderSlot::Snd).unwrap_or_else(|| tail.clone());
                        pending.push(RebuildTask::Build(RebuildFrame::ListCase { head, tail }));
                        pending.push(RebuildTask::Comp {
                            comp: cons,
                            path: child_path(&path, 2.into()),
                        });
                        pending.push(RebuildTask::Comp {
                            comp: nil,
                            path: child_path(&path, 1.into()),
                        });
                        pending.push(RebuildTask::Value {
                            value: scrut,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::With(ref fst, ref snd) => {
                        pending.push(RebuildTask::Build(RebuildFrame::With));
                        pending.push(RebuildTask::Comp {
                            comp: snd,
                            path: child_path(&path, 1.into()),
                        });
                        pending.push(RebuildTask::Comp {
                            comp: fst,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Prj(side, ref target) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Prj(
                            attr_side(here).unwrap_or(side),
                        )));
                        pending.push(RebuildTask::Comp {
                            comp: target,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::RecordProj {
                        ref record,
                        ref label,
                    } => {
                        pending.push(RebuildTask::Build(RebuildFrame::RecordProj(label.clone())));
                        pending.push(RebuildTask::Value {
                            value: record,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Dup(ref value) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Dup));
                        pending.push(RebuildTask::Value {
                            value,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Drop(ref value) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Drop));
                        pending.push(RebuildTask::Value {
                            value,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Perform(ref sig, ref op, ref arg) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Perform(
                            sig.clone(),
                            op.clone(),
                        )));
                        pending.push(RebuildTask::Value {
                            value: arg,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Handle {
                        ref sig,
                        ref scrutinee,
                        ref ret,
                        ref ops,
                    } => {
                        pending.push(RebuildTask::Build(RebuildFrame::Handle {
                            sig: sig.clone(),
                            ret_name: ret.0.clone(),
                            clauses: ops
                                .iter()
                                .map(|clause| {
                                    (
                                        clause.op.clone(),
                                        clause.payload.clone(),
                                        clause.resume.clone(),
                                    )
                                })
                                .collect(),
                        }));
                        for (index, clause) in ops.iter().enumerate().rev() {
                            pending.push(RebuildTask::Comp {
                                comp: &clause.body,
                                path: child_path(&path, handle_clause_child(index.into())),
                            });
                        }
                        pending.push(RebuildTask::Comp {
                            comp: &ret.1,
                            path: child_path(&path, 1.into()),
                        });
                        pending.push(RebuildTask::Comp {
                            comp: scrutinee,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Resume(ref stack, ref body) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Resume));
                        pending.push(RebuildTask::Comp {
                            comp: body,
                            path: child_path(&path, 1.into()),
                        });
                        pending.push(RebuildTask::Value {
                            value: stack,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Reset(ref body) => {
                        pending.push(RebuildTask::Build(RebuildFrame::Reset));
                        pending.push(RebuildTask::Comp {
                            comp: body,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | Comp::Shift(ref binder, ref body) => {
                        let binder =
                            attr_rebind(here, BinderSlot::Sole).unwrap_or_else(|| binder.clone());
                        pending.push(RebuildTask::Build(RebuildFrame::Shift(binder)));
                        pending.push(RebuildTask::Comp {
                            comp: body,
                            path: child_path(&path, 0.into()),
                        });
                    },
                    | _ => rebuilt.push(Rebuilt::Comp(comp.clone())),
                }
            },
            | RebuildTask::Build(frame) => assemble_rebuilt(frame, &mut rebuilt),
        }
    }
    rebuilt.pop()
}

/// Assembles one rebuilt parent from the post-order result stack.
fn assemble_rebuilt(
    frame: RebuildFrame,
    rebuilt: &mut Vec<Rebuilt>,
)
{
    match frame {
        | RebuildFrame::Pair => {
            let (Some(snd), Some(fst)) = (pop_value(rebuilt), pop_value(rebuilt))
            else {
                return;
            };
            rebuilt.push(Rebuilt::Value(Value::Pair(Rc::new(fst), Rc::new(snd))));
        },
        | RebuildFrame::Inj(side) => {
            let Some(payload) = pop_value(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Value(Value::Inj(side, Rc::new(payload))));
        },
        | RebuildFrame::Annot(ty) => {
            let Some(inner) = pop_value(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Value(Value::annot(inner, ty)));
        },
        | RebuildFrame::Thunk(grade) => {
            let Some(body) = pop_comp(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Value(Value::Thunk(grade, Rc::new(body))));
        },
        | RebuildFrame::List(count) => {
            let Some(elements) = pop_values(rebuilt, count.into())
            else {
                return;
            };
            rebuilt.push(Rebuilt::Value(Value::List(
                elements.into_iter().map(Rc::new).collect(),
            )));
        },
        | RebuildFrame::Record(labels) => {
            let Some(values) = pop_values(rebuilt, labels.len().into())
            else {
                return;
            };
            rebuilt.push(Rebuilt::Value(Value::Record(
                labels
                    .into_iter()
                    .zip(values)
                    .map(|(label, value)| (label, Rc::new(value)))
                    .collect(),
            )));
        },
        | RebuildFrame::Abs(name, ann) => {
            let Some(body) = pop_comp(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Abs(name, ann, Rc::new(body))));
        },
        | RebuildFrame::App => {
            let (Some(arg), Some(head)) = (pop_value(rebuilt), pop_comp(rebuilt))
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::App(Rc::new(head), Rc::new(arg))));
        },
        | RebuildFrame::Ret => {
            let Some(value) = pop_value(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Ret(Rc::new(value))));
        },
        | RebuildFrame::Bind(name) => {
            let (Some(cont), Some(bound)) = (pop_comp(rebuilt), pop_comp(rebuilt))
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Bind(
                Rc::new(bound),
                name,
                Rc::new(cont),
            )));
        },
        | RebuildFrame::Force => {
            let Some(value) = pop_value(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Force(Rc::new(value))));
        },
        | RebuildFrame::Case(left_name, right_name) => {
            let (Some(right), Some(left), Some(scrut)) =
                (pop_comp(rebuilt), pop_comp(rebuilt), pop_value(rebuilt))
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Case(
                Rc::new(scrut),
                (left_name, Rc::new(left)),
                (right_name, Rc::new(right)),
            )));
        },
        | RebuildFrame::Split {
            fst_name,
            snd_name,
            motive,
        } => {
            let (Some(body), Some(scrut)) = (pop_comp(rebuilt), pop_value(rebuilt))
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Split {
                scrut: Rc::new(scrut),
                fst_name,
                snd_name,
                motive,
                body: Rc::new(body),
            }));
        },
        | RebuildFrame::ListCase { head, tail } => {
            let (Some(cons), Some(nil), Some(scrut)) =
                (pop_comp(rebuilt), pop_comp(rebuilt), pop_value(rebuilt))
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::ListCase {
                scrut: Rc::new(scrut),
                nil: Rc::new(nil),
                head,
                tail,
                cons: Rc::new(cons),
            }));
        },
        | RebuildFrame::With => {
            let (Some(snd), Some(fst)) = (pop_comp(rebuilt), pop_comp(rebuilt))
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::With(Rc::new(fst), Rc::new(snd))));
        },
        | RebuildFrame::Prj(side) => {
            let Some(target) = pop_comp(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Prj(side, Rc::new(target))));
        },
        | RebuildFrame::RecordProj(label) => {
            let Some(record) = pop_value(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::RecordProj {
                record: Rc::new(record),
                label,
            }));
        },
        | RebuildFrame::Dup => {
            let Some(value) = pop_value(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Dup(Rc::new(value))));
        },
        | RebuildFrame::Drop => {
            let Some(value) = pop_value(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Drop(Rc::new(value))));
        },
        | RebuildFrame::Perform(sig, op) => {
            let Some(arg) = pop_value(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Perform(sig, op, Rc::new(arg))));
        },
        | RebuildFrame::Handle {
            sig,
            ret_name,
            clauses,
        } => {
            let mut bodies = Vec::with_capacity(clauses.len());
            for _ in 0 .. clauses.len() {
                let Some(body) = pop_comp(rebuilt)
                else {
                    return;
                };
                bodies.push(body);
            }
            bodies.reverse();
            let (Some(ret_body), Some(scrutinee)) = (pop_comp(rebuilt), pop_comp(rebuilt))
            else {
                return;
            };
            let ops = clauses
                .into_iter()
                .zip(bodies)
                .map(|((op, payload, resume), body)| OpClause::new(&op, &payload, &resume, body))
                .collect();
            rebuilt.push(Rebuilt::Comp(Comp::Handle {
                sig,
                scrutinee: Rc::new(scrutinee),
                ret: (ret_name, Rc::new(ret_body)),
                ops,
            }));
        },
        | RebuildFrame::Resume => {
            let (Some(body), Some(stack)) = (pop_comp(rebuilt), pop_value(rebuilt))
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Resume(Rc::new(stack), Rc::new(body))));
        },
        | RebuildFrame::Reset => {
            let Some(body) = pop_comp(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Reset(Rc::new(body))));
        },
        | RebuildFrame::Shift(binder) => {
            let Some(body) = pop_comp(rebuilt)
            else {
                return;
            };
            rebuilt.push(Rebuilt::Comp(Comp::Shift(binder, Rc::new(body))));
        },
    }
}

/// Pops one rebuilt value without disturbing a mismatched result.
fn pop_value(rebuilt: &mut Vec<Rebuilt>) -> Option<Value>
{
    match rebuilt.pop() {
        | Some(Rebuilt::Value(value)) => Some(value),
        | Some(other) => {
            rebuilt.push(other);
            None
        },
        | None => None,
    }
}

/// Pops one rebuilt computation without disturbing a mismatched result.
fn pop_comp(rebuilt: &mut Vec<Rebuilt>) -> Option<Comp>
{
    match rebuilt.pop() {
        | Some(Rebuilt::Comp(comp)) => Some(comp),
        | Some(other) => {
            rebuilt.push(other);
            None
        },
        | None => None,
    }
}

/// Pops `count` rebuilt values and restores source order.
fn pop_values(
    rebuilt: &mut Vec<Rebuilt>,
    count: RebuiltValueCount,
) -> Option<Vec<Value>>
{
    let count = usize::from(count);
    let mut values = Vec::with_capacity(count);
    for _ in 0 .. count {
        let value = pop_value(rebuilt)?;
        values.push(value);
    }
    values.reverse();
    Some(values)
}

/// Rebuilds a value at `path`.
fn rebuild_value(
    value: &Value,
    path: &OriginPath,
    plan: &BTreeMap<OriginPath, Vec<&Action>>,
) -> Value
{
    match rebuild_node(
        RebuildTask::Value {
            value,
            path: path.to_owned(),
        },
        plan,
    ) {
        | Some(Rebuilt::Value(rebuilt)) => rebuilt,
        | _ => value.clone(),
    }
}

/// Resolves an `Abs` binder annotation under the actions at this path: the new
/// annotation a [`Action::SetAnnotation`] (`AnnSlot::AbsBinder`) installs
/// (possibly `None`), or `old` when no such action is present.
fn resolve_abs_annotation(
    here: &[&Action],
    old: Option<&Rc<ValueType>>,
) -> Option<Rc<ValueType>>
{
    for &action in here {
        if let Action::SetAnnotation {
            slot: AnnSlot::AbsBinder,
            ref to,
            ..
        } = *action
        {
            return to.clone().map(Rc::new);
        }
    }
    old.cloned()
}

/// The child path `prefix` extended by component `index`.
fn child_path(
    prefix: &OriginPath,
    index: OriginPathComponent,
) -> OriginPath
{
    let mut path = prefix.to_owned();
    path.push(index.0);
    path
}

/// Diffs two computations at `prefix`.
fn diff_comp_step<'term>(
    prefix: &OriginPath,
    old: &'term Comp,
    new: &'term Comp,
    pending: &mut Vec<DiffTask<'term>>,
    out: &mut Vec<Action>,
)
{
    match (old, new) {
        | (&Comp::Hole(_), &Comp::Hole(_)) => {},
        | (&Comp::Hole(_), _) => out.push(Action::FillHole {
            path: prefix.to_vec().into(),
            to: Subtree::Comp(new.clone()),
        }),
        | (_, &Comp::Hole(_)) => out.push(Action::EraseToHole {
            path: prefix.to_vec().into(),
            to: Subtree::Comp(new.clone()),
        }),
        | (
            &Comp::Abs(ref old_name, ref old_ann, ref old_body),
            &Comp::Abs(ref new_name, ref new_ann, ref new_body),
        ) => {
            rebind_if_changed(
                prefix,
                BinderSlot::Sole,
                old_name.into(),
                new_name.into(),
                out,
            );
            diff_abs_annotation(prefix, old_ann.as_ref(), new_ann.as_ref(), out);
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 0.into()),
                old: old_body,
                new: new_body,
            });
        },
        | (&Comp::App(ref old_head, ref old_arg), &Comp::App(ref new_head, ref new_arg)) => {
            pending.push(DiffTask::Value {
                path: child_path(prefix, 1.into()),
                old: old_arg,
                new: new_arg,
            });
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 0.into()),
                old: old_head,
                new: new_head,
            });
        },
        // `Ret`/`Force` and the grade structural ops `dup`/`drop` each have a
        // single value child at index 0.
        | (&Comp::Ret(ref old_value), &Comp::Ret(ref new_value))
        | (&Comp::Force(ref old_value), &Comp::Force(ref new_value))
        | (&Comp::Dup(ref old_value), &Comp::Dup(ref new_value))
        | (&Comp::Drop(ref old_value), &Comp::Drop(ref new_value)) => {
            pending.push(DiffTask::Value {
                path: child_path(prefix, 0.into()),
                old: old_value,
                new: new_value,
            });
        },
        | (
            &Comp::Bind(ref old_bound, ref old_name, ref old_cont),
            &Comp::Bind(ref new_bound, ref new_name, ref new_cont),
        ) => {
            rebind_if_changed(
                prefix,
                BinderSlot::Sole,
                old_name.into(),
                new_name.into(),
                out,
            );
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 1.into()),
                old: old_cont,
                new: new_cont,
            });
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 0.into()),
                old: old_bound,
                new: new_bound,
            });
        },
        | (
            &Comp::Case(
                ref old_scrut,
                (ref old_left_name, ref old_left_body),
                (ref old_right_name, ref old_right_body),
            ),
            &Comp::Case(
                ref new_scrut,
                (ref new_left_name, ref new_left_body),
                (ref new_right_name, ref new_right_body),
            ),
        ) => {
            rebind_if_changed(
                prefix,
                BinderSlot::Fst,
                old_left_name.into(),
                new_left_name.into(),
                out,
            );
            rebind_if_changed(
                prefix,
                BinderSlot::Snd,
                old_right_name.into(),
                new_right_name.into(),
                out,
            );
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 2.into()),
                old: old_right_body,
                new: new_right_body,
            });
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 1.into()),
                old: old_left_body,
                new: new_left_body,
            });
            pending.push(DiffTask::Value {
                path: child_path(prefix, 0.into()),
                old: old_scrut,
                new: new_scrut,
            });
        },
        // A split's binders are the `Fst`/`Snd` attribute slots; the scrutinee
        // (0) and body (1) are the term children. The motive is a computation
        // *type* (dependent-split design) — an untyped-by-`edit` attribute carried verbatim
        // through `rebuild`, not diffed (as `Abs`'s binder annotation is not).
        | (
            &Comp::Split {
                scrut: ref old_scrut,
                fst_name: ref old_first,
                snd_name: ref old_second,
                body: ref old_body,
                ..
            },
            &Comp::Split {
                scrut: ref new_scrut,
                fst_name: ref new_first,
                snd_name: ref new_second,
                body: ref new_body,
                ..
            },
        ) => {
            rebind_if_changed(
                prefix,
                BinderSlot::Fst,
                old_first.into(),
                new_first.into(),
                out,
            );
            rebind_if_changed(
                prefix,
                BinderSlot::Snd,
                old_second.into(),
                new_second.into(),
                out,
            );
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 1.into()),
                old: old_body,
                new: new_body,
            });
            pending.push(DiffTask::Value {
                path: child_path(prefix, 0.into()),
                old: old_scrut,
                new: new_scrut,
            });
        },
        // A list-case (list-former design): the `head`/`tail` binders are attributes
        // (`Fst`/`Snd`), the scrutinee (0), `nil` body (1), and `cons` body (2)
        // are children — the `origin::resolve` order.
        | (
            &Comp::ListCase {
                scrut: ref old_scrut,
                nil: ref old_nil,
                head: ref old_head,
                tail: ref old_tail,
                cons: ref old_cons,
            },
            &Comp::ListCase {
                scrut: ref new_scrut,
                nil: ref new_nil,
                head: ref new_head,
                tail: ref new_tail,
                cons: ref new_cons,
            },
        ) => {
            rebind_if_changed(
                prefix,
                BinderSlot::Fst,
                old_head.into(),
                new_head.into(),
                out,
            );
            rebind_if_changed(
                prefix,
                BinderSlot::Snd,
                old_tail.into(),
                new_tail.into(),
                out,
            );
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 2.into()),
                old: old_cons,
                new: new_cons,
            });
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 1.into()),
                old: old_nil,
                new: new_nil,
            });
            pending.push(DiffTask::Value {
                path: child_path(prefix, 0.into()),
                old: old_scrut,
                new: new_scrut,
            });
        },
        | (&Comp::With(ref old_fst, ref old_snd), &Comp::With(ref new_fst, ref new_snd)) => {
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 1.into()),
                old: old_snd,
                new: new_snd,
            });
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 0.into()),
                old: old_fst,
                new: new_fst,
            });
        },
        | (&Comp::Prj(old_side, ref old_target), &Comp::Prj(new_side, ref new_target)) => {
            if old_side != new_side {
                out.push(Action::SetSide {
                    path: prefix.to_vec().into(),
                    from: old_side,
                    to: new_side,
                });
            }
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 0.into()),
                old: old_target,
                new: new_target,
            });
        },
        // A record projection `record.ℓ` (record-former design D4): localize into the record
        // value child (0) when the projected label is stable; a changed label is
        // a wholesale replacement (there is no per-label edit action, as
        // `Perform` has none for its op name). A self-diff descends to nothing.
        | (
            &Comp::RecordProj {
                record: ref old_record,
                label: ref old_label,
            },
            &Comp::RecordProj {
                record: ref new_record,
                label: ref new_label,
            },
        ) => {
            if old_label == new_label {
                pending.push(DiffTask::Value {
                    path: child_path(prefix, 0.into()),
                    old: old_record,
                    new: new_record,
                });
            }
            else {
                out.push(Action::Replace {
                    path: prefix.to_vec().into(),
                    to: Subtree::Comp(new.clone()),
                });
            }
        },
        // An effect operation `perform op v`: localize into the payload `v` only
        // when the operation identity (signature + op name) is stable; a changed
        // operation is a wholesale replacement.
        | (
            &Comp::Perform(ref old_sig, ref old_op, ref old_arg),
            &Comp::Perform(ref new_sig, ref new_op, ref new_arg),
        ) => {
            if old_sig == new_sig && old_op == new_op {
                pending.push(DiffTask::Value {
                    path: child_path(prefix, 0.into()),
                    old: old_arg,
                    new: new_arg,
                });
            }
            else {
                out.push(Action::Replace {
                    path: prefix.to_vec().into(),
                    to: Subtree::Comp(new.clone()),
                });
            }
        },
        // A deep handler: localize into the scrutinee (0), return body (1), and
        // aligned clause bodies (2..) only when the handler skeleton — the
        // signature, the return binder, and each clause's op, payload, and resume
        // binders — is identical; any skeleton change is a wholesale
        // replacement, because `rebuild_comp` reconstructs every handler binder
        // verbatim from the old skeleton (there is no per-binder action for a
        // handler), so a finer diff could not reinstate a changed binder.
        | (
            &Comp::Handle {
                sig: ref old_sig,
                scrutinee: ref old_scrut,
                ret: ref old_ret,
                ops: ref old_ops,
            },
            &Comp::Handle {
                sig: ref new_sig,
                scrutinee: ref new_scrut,
                ret: ref new_ret,
                ops: ref new_ops,
            },
        ) => {
            if handle_skeleton_eq(old_sig, old_ret, old_ops, new_sig, new_ret, new_ops).into() {
                for (index, (old_clause, new_clause)) in
                    old_ops.iter().zip(new_ops.iter()).enumerate().rev()
                {
                    pending.push(DiffTask::Comp {
                        path: child_path(prefix, handle_clause_child(index.into())),
                        old: &old_clause.body,
                        new: &new_clause.body,
                    });
                }
                pending.push(DiffTask::Comp {
                    path: child_path(prefix, 1.into()),
                    old: &old_ret.1,
                    new: &new_ret.1,
                });
                pending.push(DiffTask::Comp {
                    path: child_path(prefix, 0.into()),
                    old: old_scrut,
                    new: new_scrut,
                });
            }
            else {
                out.push(Action::Replace {
                    path: prefix.to_vec().into(),
                    to: Subtree::Comp(new.clone()),
                });
            }
        },
        // `resume v t`: a reified-stack value child (0) fed a computation (1).
        | (
            &Comp::Resume(ref old_stack, ref old_body),
            &Comp::Resume(ref new_stack, ref new_body),
        ) => {
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 1.into()),
                old: old_body,
                new: new_body,
            });
            pending.push(DiffTask::Value {
                path: child_path(prefix, 0.into()),
                old: old_stack,
                new: new_stack,
            });
        },
        // `reset t`: a single delimited computation child.
        | (&Comp::Reset(ref old_body), &Comp::Reset(ref new_body)) => {
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 0.into()),
                old: old_body,
                new: new_body,
            });
        },
        // `shift k. t`: a continuation binder (an attribute, like `Abs`) over a
        // single body child.
        | (
            &Comp::Shift(ref old_binder, ref old_body),
            &Comp::Shift(ref new_binder, ref new_body),
        ) => {
            rebind_if_changed(
                prefix,
                BinderSlot::Sole,
                old_binder.into(),
                new_binder.into(),
                out,
            );
            pending.push(DiffTask::Comp {
                path: child_path(prefix, 0.into()),
                old: old_body,
                new: new_body,
            });
        },
        // Different concrete constructors: replace wholesale. (`Value::Stk`
        // reified stacks are opaque to the value diff for the same reason —
        // they are machine-constructed, not a surface form; descent into a
        // reified stack is deferred, `deep edit descent`.)
        | _ => out.push(Action::Replace {
            path: prefix.to_vec().into(),
            to: Subtree::Comp(new.clone()),
        }),
    }
}

/// Whether two handlers share an identical binder/operation **skeleton** — the
/// condition under which a `Handle` edit localizes into its bodies rather than
/// replacing wholesale.
///
/// Compares the signature, the return binder, and each clause's
/// op/payload/resume binders in order; the clause **bodies** are deliberately
/// *not* compared — they are exactly what the localized diff descends into.
fn handle_skeleton_eq(
    old_sig: &EffectSig,
    old_ret: &(String, Rc<Comp>),
    old_ops: &[OpClause],
    new_sig: &EffectSig,
    new_ret: &(String, Rc<Comp>),
    new_ops: &[OpClause],
) -> MatchDecision
{
    (old_sig == new_sig
        && old_ret.0 == new_ret.0
        && old_ops.len() == new_ops.len()
        && old_ops
            .iter()
            .zip(new_ops.iter())
            .all(|(old_clause, new_clause)| {
                old_clause.op == new_clause.op
                    && old_clause.payload == new_clause.payload
                    && old_clause.resume == new_clause.resume
            }))
    .into()
}

/// The term-child index of the `index`-th `Handle` operation-clause body.
///
/// Clause bodies occupy children `2..` — after the scrutinee (child 0) and the
/// return body (child 1) — the convention [`diff_comp_step`], [`rebuild_comp`],
/// and `origin::step_comp` share. The index is bounded by [`u32`]; a clause
/// count past `u32::MAX - 2` saturates to `u32::MAX`. That is unreachable for
/// any constructible handler (it needs billions of operation clauses);
/// `diff_comp_step` and `rebuild_comp` route through this one helper so they
/// stay mutually consistent, though were the saturation boundary ever reached
/// distinct clauses could alias to the same child index.
fn handle_clause_child(index: ItemIndex) -> OriginPathComponent
{
    u32::try_from(index.0)
        .ok()
        .and_then(|clause| clause.checked_add(2))
        .unwrap_or(u32::MAX)
        .into()
}

// --- Application -------------------------------------------------------------

/// Applies an edit-action `script` to the old item list, reconstructing the
/// new item list.
///
/// This is the diff's adjoint and the soundness oracle: for any `old` and
/// `new`, `apply(old, &diff_items(old, new))` equals `new` **up to hole
/// identifiers** (typing ignores them, so the diff treats two holes at the same
/// position as equal and leaves the old identifier in place). The [`OriginMap`]
/// is *not* reconstructed — only the term forest (names, ascriptions, terms);
/// re-deriving byte ranges is `origin-reconstruction work`.
///
/// # Contract
/// - requires: `script` was produced by [`diff`] / [`diff_items`] from `old`
///   (its old-anchored indices and paths address `old`).
/// - ensures: returns the reconstructed new item list; equals the `new` the
///   script was diffed against, up to hole identifiers.
/// - provides: the soundness adjoint that closes the apply-action ≡ re-lower
///   property.
/// - panics: none — an out-of-range index or path is skipped, keeping the
///   surface total.
#[inline]
#[must_use]
pub fn apply(
    old: &[LoweredItem],
    script: &EditScript,
) -> Vec<LoweredItem>
{
    let mut deleted: BTreeSet<usize> = BTreeSet::new();
    let mut ascription_changes: BTreeMap<usize, Option<Ty>> = BTreeMap::new();
    let mut term_actions: BTreeMap<usize, Vec<&Action>> = BTreeMap::new();
    let mut inserts: Vec<(usize, &LoweredItem)> = Vec::new();

    for action in &script.actions {
        match *action {
            | Action::DeleteItem { at } => {
                let _existed = deleted.insert(at);
            },
            | Action::InsertItem { at, ref item } => inserts.push((at, item)),
            | Action::SetItemAscription { at, ref to, .. } => {
                let _previous = ascription_changes.insert(at, to.clone());
            },
            | _ => {
                if let Some(&old_index) = action.path().and_then(|path| path.first())
                    && let Ok(index) = usize::try_from(old_index)
                {
                    term_actions.entry(index).or_default().push(action);
                }
            },
        }
    }

    // Patch the kept items (old order, deleted ones dropped).
    let mut kept: Vec<LoweredItem> = Vec::new();
    for (old_index, item) in old.iter().enumerate() {
        if deleted.contains(&old_index) {
            continue;
        }
        let mut rebuilt = item.clone();
        if let Some(ascription) = ascription_changes.get(&old_index) {
            rebuilt.ascription.clone_from(ascription);
        }
        if let Some(actions) = term_actions.get(&old_index) {
            rebuilt.term = rebuild_term(&item.term, &plan_of(actions));
        }
        kept.push(rebuilt);
    }

    // Splice in the insertions at their final new-list indices (ascending, so
    // each target index is correct once the earlier inserts are placed).
    inserts.sort_by_key(|&(at, _)| at);
    for (at, item) in inserts {
        let position = at.min(kept.len());
        kept.insert(position, item.clone());
    }
    kept
}

/// Rebuilds a term under a per-path action plan (see [`apply`]).
///
/// The item **root** is the one position whose sort can flip (`Value` ⇄
/// `Comp`): an item's term may be of either sort, so a root `Replace` /
/// `FillHole` / `EraseToHole` can carry the *other* sort's [`Subtree`] (e.g.
/// `def f = 1;` ⇄ `def f = g(1);`). Every non-root child slot is sort-fixed by
/// its parent constructor, so the diff only ever flips the root. A root
/// terminal is therefore installed by the carried subtree's sort *before*
/// dispatching on the old term's sort; same-sort root terminals are reproduced
/// identically by this path, so it composes with the recursive rebuild below.
fn rebuild_term(
    term: &Term,
    plan: &BTreeMap<OriginPath, Vec<&Action>>,
) -> Term
{
    let root_path = OriginPath::default();
    let root = actions_at(plan, &root_path);
    if let Some(comp) = terminal_comp(root) {
        return Term::Comp(comp);
    }
    if let Some(value) = terminal_value(root) {
        return Term::Value(value);
    }
    match *term {
        | Term::Value(ref value) => Term::Value(rebuild_value(value, &root_path, plan)),
        | Term::Comp(ref comp) => Term::Comp(rebuild_comp(comp, &root_path, plan)),
    }
}

/// The per-term-path action plan for one item: the actions of `actions`
/// re-keyed by their **term** path (the path with the leading item index
/// stripped).
fn plan_of<'act>(actions: &[&'act Action]) -> BTreeMap<OriginPath, Vec<&'act Action>>
{
    let mut plan: BTreeMap<OriginPath, Vec<&Action>> = BTreeMap::new();
    for &action in actions {
        if let Some(term_path) = action.path().and_then(|path| path.get(1 ..)) {
            plan.entry(term_path.to_vec().into())
                .or_default()
                .push(action);
        }
    }
    plan
}

/// The value a terminal action at this path installs (replace / fill / erase /
/// literal / variable), if any.
fn terminal_value(here: &[&Action]) -> Option<Value>
{
    here.iter().find_map(|&action| match *action {
        | Action::Replace {
            to: Subtree::Value(ref value),
            ..
        }
        | Action::FillHole {
            to: Subtree::Value(ref value),
            ..
        }
        | Action::EraseToHole {
            to: Subtree::Value(ref value),
            ..
        } => Some(value.clone()),
        | Action::SetInt { to, .. } => Some(Value::Int(to)),
        | Action::SetVar { ref to, .. } => Some(Value::Var(to.clone())),
        | _ => None,
    })
}

/// The computation a terminal action at this path installs, if any.
fn terminal_comp(here: &[&Action]) -> Option<Comp>
{
    here.iter().find_map(|&action| match *action {
        | Action::Replace {
            to: Subtree::Comp(ref comp),
            ..
        }
        | Action::FillHole {
            to: Subtree::Comp(ref comp),
            ..
        }
        | Action::EraseToHole {
            to: Subtree::Comp(ref comp),
            ..
        } => Some(comp.clone()),
        | _ => None,
    })
}

/// The new binder name a [`Action::Rebind`] of `slot` at this path installs, if
/// any.
fn attr_rebind(
    here: &[&Action],
    slot: BinderSlot,
) -> Option<String>
{
    here.iter().find_map(|&action| match *action {
        | Action::Rebind {
            slot: action_slot,
            ref to,
            ..
        } if action_slot == slot => Some(to.clone()),
        | _ => None,
    })
}

/// The side a [`Action::SetSide`] at this path installs, if any.
fn attr_side(here: &[&Action]) -> Option<Side>
{
    here.iter().find_map(|&action| match *action {
        | Action::SetSide { to, .. } => Some(to),
        | _ => None,
    })
}

/// The new value-ascription a [`Action::SetAnnotation`] (`AnnSlot::Value`) at
/// this path installs, if any.
fn attr_value_annotation(here: &[&Action]) -> Option<ValueType>
{
    here.iter().find_map(|&action| match *action {
        | Action::SetAnnotation {
            slot: AnnSlot::Value,
            to: Some(ref ty),
            ..
        } => Some(ty.clone()),
        | _ => None,
    })
}

/// The grade a [`Action::SetGrade`] at this path installs, if any.
fn attr_grade(here: &[&Action]) -> Option<Grade>
{
    here.iter().find_map(|&action| match *action {
        | Action::SetGrade { to, .. } => Some(to),
        | _ => None,
    })
}

/// Rebuilds a computation at `path`.
fn rebuild_comp(
    comp: &Comp,
    path: &OriginPath,
    plan: &BTreeMap<OriginPath, Vec<&Action>>,
) -> Comp
{
    match rebuild_node(
        RebuildTask::Comp {
            comp,
            path: path.to_owned(),
        },
        plan,
    ) {
        | Some(Rebuilt::Comp(rebuilt)) => rebuilt,
        | _ => comp.clone(),
    }
}

/// The edit locus for a [`SourceEdit`]: [`localize`] over the edit's **old**
/// byte extent (`start_byte .. old_end_byte`) against the old origin map.
///
/// # Contract
/// - requires: `origin` is the origin map of the program *before* the edit;
///   `edit` is the edit applied to that program.
/// - ensures: returns the [`localize`] locus of the edit's old byte span.
/// - provides: the convenience that maps a source edit to its core term locus.
/// - fails: [`None`] when no entry encloses the edit (e.g. a cross-item edit).
/// - panics: none.
#[inline]
#[must_use]
pub fn edit_locus(
    origin: &OriginMap,
    edit: &SourceEdit,
) -> Option<OriginPath>
{
    localize(origin, edit.start_byte.into(), edit.old_end_byte.into())
}

/// The smallest core term enclosing the byte range `[start, end)` in the old
/// program: the **edit locus**.
///
/// This is a byte->node *stabbing* query — which recorded node encloses the
/// span — answered by an **O(depth) descent** of the [`OriginMap`]'s nested
/// byte ranges: the ranges nest, so the smallest enclosing entry is reached by
/// stepping into the containing child at each level rather than scanning the
/// whole map (`stable-origin work`). It is *not* the `gandr-theory-orders`
/// `Interval` containment, which answers node->node *ancestry* in O(1) for the
/// dirty-frontier engine (A2.3, `CST-resynchronization work`) — a different
/// query. The locus is the **smallest-byte-range** entry containing the edit;
/// among entries sharing that minimal range — synthesized elaboration nodes
/// (operator desugaring, the `def` sugar) legitimately share a span — the
/// **outermost** (shortest-path) is chosen, so the locus is a *common ancestor*
/// of every change a contiguous edit induces, not one of several overlapping
/// siblings.
///
/// # Contract
/// - requires: `origin` is the old program's origin map; `start <= end`.
/// - ensures: returns the path of the smallest-range origin entry whose byte
///   range contains `[start, end)`, ties broken toward the *shorter* path (the
///   outermost term of that span); or [`None`] when no recorded entry encloses
///   the range (e.g. an edit straddling two top-level items).
/// - provides: the localization bound a structure-preserving structural diff
///   stays within — for a single contiguous edit whose induced changes all lie
///   within one enclosing term, every path-addressed action of [`diff`] has
///   this locus as a prefix.
/// - fails: [`None`] when no entry encloses the range.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the descent must (i) reach the tightest enclosing entry
///   past same-span synthesized intermediaries, (ii) prefer the outermost of a
///   shared span, and (iii) skip subtrees that do not contain the edit (the
///   O(depth) property). A leaf-exact span, an item-wide span, and a
///   multi-point span over overlapping synthesized siblings distinguish these.
/// - witness: `crate::edit::tests::descent_agrees_with_the_linear_stab_oracle`
///   (equivalence to the reference over every corpus entry span) and
///   `crate::edit::tests::localize_descends_in_depth_not_map_size` (the visit
///   bound); the integration suite's `tests::localization` module exercises the
///   contract end to end.
#[inline]
#[must_use]
pub fn localize(
    origin: &OriginMap,
    start: SourceOffset,
    end: SourceOffset,
) -> Option<OriginPath>
{
    localize_traced(origin, start, end).0
}

/// [`localize`] plus the count of origin entries the descent examined — the
/// O(depth) witness. A full linear stab visits *every* entry; the descent
/// visits only the root item-scan, the root-to-locus path, and the direct
/// children it steps past at each level.
///
/// The result is `debug_assert`-checked against the linear-stab reference
/// ([`localize_reference`]) — an external oracle (adequacy design): the
/// O(depth) descent is only sound if it lands on the same entry the exhaustive
/// minimum would.
#[must_use]
pub(crate) fn localize_traced(
    origin: &OriginMap,
    start: SourceOffset,
    end: SourceOffset,
) -> (Option<OriginPath>, OriginEntryCount)
{
    let (locus, visits) = descend_locus(origin, start, end);
    debug_assert_eq!(
        locus,
        localize_reference(origin, start, end),
        "the O(depth) locus descent must agree with the linear-stab oracle"
    );
    (locus, visits)
}

/// The O(depth) core of [`localize`]: descend the nested origin ranges,
/// tracking the smallest-span containing entry (ties → the shallowest, i.e.
/// shortest path).
///
/// Invariant exploited: an entry's byte range contains every descendant's, so
/// if any descendant of the current node contains `[start, end)` then the
/// child on the path to it does too — stepping into the smallest-span
/// containing child therefore never skips the minimum, and off-path subtrees
/// are never entered. `current` is the single reused path buffer (the descent
/// allocates only as it grows to `best_len`); the locus is `current` truncated
/// to the best prefix.
fn descend_locus(
    origin: &OriginMap,
    start: SourceOffset,
    end: SourceOffset,
) -> (Option<OriginPath>, OriginEntryCount)
{
    let mut visits = 0_usize;

    // Level 0 — the root items (paths `[i]`) are disjoint, so at most one
    // contains a contiguous edit; a straddling edit finds none. `[item]` is a
    // stack array, so this scan allocates nothing.
    let mut item = 0_u32;
    let (root_item, mut best_span) = loop {
        match origin.get_path(&[item]) {
            | Some(entry) => {
                visits = visits.saturating_add(1);
                if contains(entry, start, end).into() {
                    break (item, range_len(entry));
                }
                match item.checked_add(1) {
                    | Some(next) => item = next,
                    | None => return (None, visits.into()),
                }
            },
            | None => return (None, visits.into()),
        }
    };

    // Levels 1.. — step into the smallest-span containing child. `best_len` is
    // the shallowest prefix achieving `best_span`, so equal-span descendants
    // (synthesized nodes sharing the span) never displace their ancestor.
    let mut current = OriginPath::default();
    current.push(root_item);
    let mut best_len = current.len();
    loop {
        let mut chosen: Option<(u32, SourceLength)> = None;
        let mut candidate = 0_u32;
        loop {
            current.push(candidate);
            match origin.get_path(&current) {
                | Some(entry) => {
                    visits = visits.saturating_add(1);
                    if contains(entry, start, end).into() {
                        let span = range_len(entry);
                        if chosen.is_none_or(|(_, best)| span < best) {
                            chosen = Some((candidate, span));
                        }
                    }
                    current.pop();
                    match candidate.checked_add(1) {
                        | Some(next) => candidate = next,
                        | None => break,
                    }
                },
                | None => {
                    current.pop();
                    break;
                },
            }
        }
        match chosen {
            | Some((child, span)) => {
                current.push(child);
                if span < best_span {
                    best_span = span;
                    best_len = current.len();
                }
            },
            | None => break,
        }
    }

    current.truncate(best_len);
    (Some(current), visits.into())
}

/// The reference localizer: an exhaustive linear stab over every origin entry
/// (the pre-`stable-origin work` implementation). Retained as the external
/// oracle [`localize_traced`] `debug_assert`s the O(depth) descent against.
fn localize_reference(
    origin: &OriginMap,
    start: SourceOffset,
    end: SourceOffset,
) -> Option<OriginPath>
{
    let mut best: Option<(&OriginPath, SourceLength)> = None;
    for (path, _id, entry) in origin.iter_paths() {
        if contains(entry, start, end).into() {
            let span = range_len(entry);
            let take = match best {
                | None => true,
                // Smaller range wins; among equal (overlapping synthesized)
                // ranges the outermost (shortest path) wins, so the locus is the
                // tightest common ancestor rather than one overlapping sibling.
                | Some((best_path, best_span)) => {
                    span < best_span || (span == best_span && path.len() < best_path.len())
                },
            };
            if take {
                best = Some((path, span));
            }
        }
    }
    best.map(|(path, _span)| path.clone())
}

/// Whether `entry`'s byte range encloses `[start, end)`.
fn contains(
    entry: &OriginEntry,
    start: SourceOffset,
    end: SourceOffset,
) -> MatchDecision
{
    (entry.byte_range.start <= start.0 && end.0 <= entry.byte_range.end).into()
}

/// A source edit's byte extents — the localizer input that replaced the retired
/// `tree_sitter::InputEdit` (`stable-origin work`).
///
/// The melder is a *batch* parser, so the row/column positions tree-sitter
/// carried for incremental reparse are not consumed here; the localizer needs
/// only the edit's old extent `[start_byte, old_end_byte)`. `new_end_byte` is
/// retained so the descriptor names the replacement's new extent for callers
/// that pair the edit with a re-parse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceEdit
{
    /// The byte offset where the edit begins (shared by old and new source).
    pub start_byte: usize,
    /// The byte offset where the replaced region ends in the *old* source.
    pub old_end_byte: usize,
    /// The byte offset where the replacement ends in the *new* source.
    pub new_end_byte: usize,
}

impl SourceEdit
{
    /// Construct an edit from its old and new byte extents.
    #[inline]
    #[must_use]
    pub const fn new(
        start_byte: SourceOffset,
        old_end_byte: SourceOffset,
        new_end_byte: SourceOffset,
    ) -> Self
    {
        Self {
            start_byte: start_byte.0,
            old_end_byte: old_end_byte.0,
            new_end_byte: new_end_byte.0,
        }
    }
}

/// The byte length of an origin entry's range.
fn range_len(entry: &OriginEntry) -> SourceLength
{
    SourceLength::from(entry.byte_range.end.saturating_sub(entry.byte_range.start))
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::lower::lower_source_total;
    use crate::origin::OriginMap;
    /// The O(depth) descent lands on the same entry the exhaustive linear stab
    /// would, across nested-operator (same-span synthesized chains),
    /// multi-item, sugar, and hole shapes — the localization-soundness
    /// adequacy witness.
    #[test]
    fn descent_agrees_with_the_linear_stab_oracle() -> Result<(), String>
    {
        let sources = [
            "def deep = (1 + (1 + (1 + 1)));\n",
            "def target(x: Integer) -> F Integer {\n  ret (x + 1)\n}\nprint(target)\n",
            "def a = 1;\ndef b = 2;\ndef c = 3;\n",
            "ret 1\n",
            "if true { ret 1 } else { ret 2 }\n",
            "ret ?goal\n",
        ];
        for source in sources {
            let origin = origin_of(source.into())?;
            let spans: Vec<(SourceOffset, SourceOffset)> = origin
                .iter_paths()
                .map(|(_path, _id, entry)| {
                    (entry.byte_range.start.into(), entry.byte_range.end.into())
                })
                .collect();
            // Every recorded span, plus each endpoint as a point probe, must
            // localize identically under the descent and the reference stab.
            for (start, end) in spans {
                for probe in [(start, end), (start, start), (end, end)] {
                    assert_eq!(
                        localize(&origin, probe.0, probe.1),
                        localize_reference(&origin, probe.0, probe.1),
                        "{source:?}: descent and linear stab disagree at {probe:?}"
                    );
                }
            }
        }
        Ok(())
    }

    /// Lower `source` in total mode (every input lowers), surfacing a commit
    /// failure as a message.
    fn origin_of(source: PipelineSource<'_>) -> Result<OriginMap, String>
    {
        lower_source_total(source)
            .map(|lowered| lowered.origin)
            .map_err(|error| format!("{source:?} must lower totally: {error}"))
    }

    /// A program with `shallow` trailing single-line defs after one deeply
    /// nested first item. The deep item's byte layout is the source prefix, so
    /// it is *independent* of `shallow` — the lever the O(depth) test pulls.
    fn deep_then_shallow(
        depth: TreeDepth,
        shallow: ItemCount,
    ) -> String
    {
        let mut nested = String::from("1");
        for _ in 0 .. usize::from(depth) {
            nested = format!("(1 + {nested})");
        }
        let shallow_defs: Vec<String> = (0 .. usize::from(shallow))
            .map(|index| format!("def d{index} = {index};\n"))
            .collect();
        format!("def deep = {nested};\n{}", shallow_defs.concat())
    }
    /// The localizer is O(depth), not O(map size): with the same deeply nested
    /// first item, adding trailing breadth leaves the descent's visit count
    /// unchanged — it never enters an off-path subtree — even as the map grows.
    #[test]
    fn localize_descends_in_depth_not_map_size() -> Result<(), String>
    {
        /// Wide repetition count per depth level for the origin-stability
        /// stress.
        const WIDE_RUN: usize = 40;

        let depth = TreeDepth::from(8);
        let narrow_source = deep_then_shallow(depth, 8.into());
        let wide_source = deep_then_shallow(depth, WIDE_RUN.into());
        let narrow = origin_of(narrow_source.as_str().into())?;
        let wide = origin_of(wide_source.as_str().into())?;

        // The deep item is item 0 in both, byte-identical, so the same innermost
        // span localizes there in both.
        let (start, end) =
            deepest_span(&narrow).ok_or_else(|| "the deep program has entries".to_owned())?;
        let (narrow_locus, narrow_visits) = localize_traced(&narrow, start, end);
        let (wide_locus, wide_visits) = localize_traced(&wide, start, end);

        assert!(narrow_locus.is_some(), "the deep span localizes");
        assert_eq!(
            narrow_locus, wide_locus,
            "the same span localizes to the same locus in both programs"
        );

        // The wide program records strictly more entries; the descent visits the
        // same count regardless — visits are independent of map size.
        assert!(
            wide.len().0 > narrow.len().0,
            "the wide program records more entries: {} vs {}",
            wide.len().0,
            narrow.len().0
        );
        assert_eq!(
            narrow_visits, wide_visits,
            "the descent visits the same entries regardless of trailing breadth \
             ({narrow_visits:?} vs {wide_visits:?})"
        );

        // Visits are bounded by the locus depth (times a small fixed per-level
        // branching), and are far below a full scan of the wider map.
        let locus_depth = TreeDepth::from(narrow_locus.as_ref().map_or(0_usize, |path| path.len()));
        assert!(
            narrow_visits < wide.len(),
            "the descent is not a full scan: {:?} visits < {} entries",
            narrow_visits,
            wide.len().0
        );
        assert!(
            narrow_visits.0 <= locus_depth.0.saturating_mul(6).saturating_add(4),
            "visits {:?} are O(depth={:?})",
            narrow_visits,
            locus_depth
        );
        Ok(())
    }
    /// [`edit_locus`] over a [`SourceEdit`]'s old extent agrees with a direct
    /// [`localize`] of that span — the tree-sitter-free edit descriptor.
    #[test]
    fn edit_locus_maps_a_source_edit_to_its_old_span_locus() -> Result<(), String>
    {
        let origin = origin_of("ret 1\n".into())?;
        let (start, end) = deepest_span(&origin).ok_or_else(|| "entries exist".to_owned())?;
        let edit = SourceEdit::new(start, end, end);
        assert_eq!(
            edit_locus(&origin, &edit),
            localize(&origin, start, end),
            "edit_locus localizes the edit's old extent"
        );
        Ok(())
    }

    /// The deepest (longest-path) origin entry's byte range — the innermost
    /// recorded node, the tightest localization target.
    fn deepest_span(origin: &OriginMap) -> Option<(SourceOffset, SourceOffset)>
    {
        origin
            .iter_paths()
            .max_by_key(|&(path, _id, _entry)| path.len())
            .map(|(_path, _id, entry)| (entry.byte_range.start.into(), entry.byte_range.end.into()))
    }
}
