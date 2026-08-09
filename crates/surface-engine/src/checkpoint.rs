//! Dependency-validated checkpoints and the validated-resume incremental typer
//! (A2.3; `incremental-pipeline.md` §"Checkpoints and the reuse rule" through
//! §"Derivation merging and identity stability").
//!
//! Landed under `incremental-checkpoint work`.
//!
//! # The rung this builds
//!
//! `incremental-pipeline.md`'s incremental loop is: edit → cold reparse → lower
//! (obligations become holes) → diff → *resume from the nearest valid
//! checkpoint*, re-typing only the affected region and **re-validating** —
//! never blindly reusing — the rest (§"The edit loop"). This module builds that
//! loop at the granularity the current (melder, tree-sitter-free) architecture
//! exposes: the **top-level item**. Items lower independently and are typed
//! against an accumulating [`Ctx`] a [`crate::session::Session`] threads item
//! to item, so the checkpoint of an item is its typing result plus its
//! [`Footprint`] (the context names it read, `crate::footprint`), and the
//! validity condition of §"The soundness condition" specializes to:
//!
//! > an item's cached typing is valid for reuse iff (1) its lowered term is
//! > unchanged and (2) no name in its footprint had its binding change since
//! > the checkpoint.
//!
//! Condition (2) is the trail-aware core (§"The soundness condition": "σ is
//! global, so re-typing a changed subtree can resolve variables downstream
//! cached results mention", read one stratum up: the shared edit-mutable state
//! threaded across items is the name → type context). An edit that changes a
//! definition's *body* but not its *type* leaves its dependents' footprints
//! untouched — they are adopted, not re-typed — which is exactly the
//! "re-validate, not blindly reuse" refinement: reuse is keyed on whether the
//! binding actually changed, not on whether some upstream item was edited.
//!
//! [`Ctx`]: gandr_core_checker::ctx::Ctx
//!
//! # The order structure (§"pipeline-decision-02", the Porter disposition)
//!
//! [`resume`] maintains the item order on a
//! [`gandr_theory_orders::OrderMaintenance`]: the base checkpoints seed one
//! order element per base item (payload = the item's *stable* identity), and an
//! edit is applied by *splicing* — removing deleted elements, inserting new
//! ones after their predecessor — so a **matched item keeps its original order
//! element across the edit** even as inserts/deletes shift every index around
//! it. This is the "keep positions out of the checkpoint key" commitment the
//! spec draws from rust-analyzer / Pterodactyl (§"pipeline-decision-07"): the
//! checkpoint key is the stable identity, and the item's *position* is
//! recovered from the spliced order. The dirty-frontier pass is then driven in
//! that order (dependencies flow forward, so one ordered pass propagates every
//! binding change), the item-granular shadow of Porter's order-maintenance
//! dirty queue.
//!
//! # The differential gate
//!
//! Soundness is the theorem [`resume`] must satisfy and `tests/incremental`
//! checks: for every edit, `resume(base, edited)` yields **exactly** the
//! typings [`checkpoint_source`] computes for the edited program from scratch.
//! Adoption skips re-typing; the gate proves the skips never change the answer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::ctx::Ctx;
use gandr_core_checker::error::TypeError;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::types::CompType;
use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;
use gandr_theory_orders::OrderMaintenance;
use gandr_theory_orders::Pos;

use crate::boundary::DefinitionName;
use crate::boundary::HolePresence;
use crate::boundary::MatchDecision;
use crate::footprint::Footprint;
use crate::footprint::footprint_of;
use crate::lower::Lowered;
use crate::lower::LoweredItem;
use crate::prelude_ctx;
use crate::session::item_type;

/// Number of base checkpoints adopted by an incremental resume.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct AdoptedCheckpointCount(pub usize);

/// Number of source items in the base (adopted) checkpoint set.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BaseItemCount(usize);

/// Index of one source item within the base checkpoint set.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BaseItemIndex(usize);

/// Number of source items in the edited (current) program.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EditedItemCount(usize);

/// Index of one source item within the edited program.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EditedItemIndex(usize);

/// Occurrence count of one definition name across a program's items.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DefinitionMultiplicity(usize);

/// One aligned base/edited item pair in the alignment table.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ItemMatch
{
    /// Item index within the base checkpoint set.
    base: BaseItemIndex,
    /// Item index within the edited program.
    edited: EditedItemIndex,
}

/// The type-level typing outcome of one item — the checkpoint's cached result.
///
/// This is the type-level projection of [`crate::session::ItemOutcome`]:
/// evaluation (the L-machine run) is the driver's concern, so a checkpoint
/// records only what re-typing must reproduce. Two typings compare equal iff
/// they classify the item identically and carry the same type — the equality
/// the differential gate asserts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItemTyping
{
    /// A `def name = …` item that typed successfully.
    Definition
    {
        /// The defined name.
        name: String,
        /// The reported type — the bound *value* type `Ty::Value(A)` for a
        /// value-typed definition (what `name` is as a variable), or the raw
        /// (bare-computation) type otherwise, mirroring
        /// [`crate::session`]'s discipline.
        ty: Ty,
        /// Whether the definition entered scope (a value type binds; a bare
        /// computation type does not — thunk it to name it).
        bound: bool,
    },
    /// An expression item that typed successfully, with its result type.
    Expression
    {
        /// The expression's terminal type.
        ty: Ty,
    },
    /// An item whose typing failed, with the first checker error.
    TypeError
    {
        /// The first typing error.
        error: TypeError,
    },
    /// An item carrying a hole: typing is declined (the parse-completeness
    /// discipline, `incremental-pipeline.md` §"Holes").
    Holey,
    /// An item of unknown sort — the forward-compatible shape for a future
    /// `Term` sort. Never produced today: the sort dispatch is total over
    /// `Term`'s two sorts, and an added sort is a compile-visible change at
    /// every match.
    Unknown,
}

/// One item's checkpoint: its identity (name / ascription / lowered term), its
/// dependency [`Footprint`], and its cached [`ItemTyping`].
///
/// The term is retained as the content key for §"The soundness condition"
/// condition (1): reuse requires the lowered term be unchanged, tested by
/// structural equality against the edited item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemCheckpoint
{
    /// The defined name (`def` items); [`None`] for expression items.
    pub name: Option<String>,
    /// The recorded ascription.
    pub ascription: Option<Ty>,
    /// The lowered term — the content key for the unchanged-region test.
    pub term: Term,
    /// The dependency footprint (context names read).
    pub footprint: Footprint,
    /// The cached typing outcome.
    pub typing: ItemTyping,
}

/// A base checkpoint set: one [`ItemCheckpoint`] per item, in source order —
/// the snapshot [`resume`] validates an edit against.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Checkpoints
{
    /// The per-item checkpoints, in source order.
    pub items: Vec<ItemCheckpoint>,
}

/// The result of a validated resume.
///
/// Carries the per-item typings (in edited source order, comparable to
/// [`checkpoint_source`]) and, per item, whether its base checkpoint was
/// adopted (reused) rather than re-typed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Resume
{
    /// The typing of each edited item, in source order.
    pub typings: Vec<ItemTyping>,
    /// Whether each edited item adopted its base checkpoint (`true`) or was
    /// re-typed (`false`), aligned with [`Self::typings`].
    pub adopted: Vec<bool>,
}

impl Resume
{
    /// The number of items whose base checkpoint was adopted (reused) — the
    /// reuse the incremental path bought over a from-scratch re-type.
    ///
    /// # Contract
    /// - ensures: returns the count of `true` entries in [`Self::adopted`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn adopted_count(&self) -> AdoptedCheckpointCount
    {
        AdoptedCheckpointCount(self.adopted.iter().filter(|&&adopted| adopted).count())
    }
}

/// Types a lowered program from scratch into a [`Checkpoints`] set, against the
/// prelude context.
///
/// This is both the base-checkpoint builder and the differential gate's
/// reference: [`resume`]'s output must equal `checkpoint_source(edited).items`'
/// typings for every edit.
///
/// # Contract
/// - ensures: returns one [`ItemCheckpoint`] per item, each typed against the
///   context as it stood after the preceding items (the session's cross-item
///   threading, type-level).
/// - panics: none.
#[inline]
#[must_use]
pub fn checkpoint_source(lowered: &Lowered) -> Checkpoints
{
    checkpoint_with(lowered, &prelude_ctx())
}

/// Types a lowered program from scratch against an explicit base context (the
/// generalization of [`checkpoint_source`] a REPL session with a non-prelude
/// base would use).
///
/// # Contract
/// - ensures: returns the per-item checkpoints, threading each value-typed
///   definition's binding into the context for the items that follow it.
/// - panics: none.
#[inline]
#[must_use]
pub fn checkpoint_with(
    lowered: &Lowered,
    base_ctx: &Ctx,
) -> Checkpoints
{
    let mut ctx = base_ctx.clone();
    let mut items: Vec<ItemCheckpoint> = Vec::with_capacity(lowered.items.len());
    for item in &lowered.items {
        let footprint = footprint_of(&item.term);
        let typing = type_item(item, &ctx, footprint.has_hole.into());
        thread_binding(&mut ctx, &typing);
        items.push(ItemCheckpoint {
            name: item.name.clone(),
            ascription: item.ascription.clone(),
            term: item.term.clone(),
            footprint,
            typing,
        });
    }
    Checkpoints { items }
}

/// The validated-resume incremental typer: reproduces the from-scratch typing
/// of `edited` while adopting every item whose checkpoint stays valid.
///
/// Against the prelude base context; see [`resume_with`] for an explicit base.
///
/// # Contract
/// - ensures: `resume(base, edited).typings` equals the typings of
///   `checkpoint_source(edited).items` — the differential gate.
/// - provides: the §"The edit loop" validated resume — re-type the dirty
///   frontier, adopt the validated remainder.
/// - panics: none.
#[inline]
#[must_use]
pub fn resume(
    base: &Checkpoints,
    edited: &Lowered,
) -> Resume
{
    resume_with(base, edited, &prelude_ctx())
}

/// [`resume`] against an explicit base context.
///
/// # Contract
/// - requires: `base` was produced by [`checkpoint_with`] against the same
///   `base_ctx` (its cached typings were computed under it).
/// - ensures: the result equals `checkpoint_with(edited, base_ctx)`'s typings.
/// - provides: adoption where the validity condition of §"The soundness
///   condition" holds; a re-type otherwise; and a graceful full-re-type
///   fallback if the order structure cannot admit the edit
///   (`incremental-pipeline.md` §"Graceful degradation").
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: adoption is sound only where structural identity **and**
///   footprint-cleanliness both hold; a body-only edit must adopt its
///   type-stable dependents, while a type-changing edit must re-type them (and
///   a downstream error must surface). The `tests::incremental` corpus — body
///   edits, type-changing edits, inserts, deletes, error-introducing edits, and
///   property-generated single-def edits — distinguishes these.
/// - witness: `tests::incremental` asserts `resume == checkpoint_source` over
///   that corpus and that reuse actually occurs (the engine is not trivially
///   re-typing everything).
#[inline]
#[must_use]
pub fn resume_with(
    base: &Checkpoints,
    edited: &Lowered,
    base_ctx: &Ctx,
) -> Resume
{
    let edited_footprints: Vec<Footprint> = edited
        .items
        .iter()
        .map(|item| footprint_of(&item.term))
        .collect();
    let matches = align_by_name(&base.items, &edited.items);
    let edited_to_base = invert_matches(&matches, EditedItemCount(edited.items.len()));
    let matched_base: BTreeSet<BaseItemIndex> =
        matches.iter().map(|item_match| item_match.base).collect();

    // The names whose binding an edit may have changed, seeded conservatively;
    // grown as the ordered pass re-types definitions whose type moved.
    let mut changed = seed_changed_bindings(base, edited, &matched_base, &edited_to_base);

    // Splice the edit onto the base order; the processing order is the spliced
    // order (dependencies flow forward, so one pass suffices). On any order
    // failure, degrade to a full re-type — still exactly the from-scratch
    // result, only without reuse.
    let Some(order) = splice_order(BaseItemCount(base.items.len()), &edited_to_base)
    else {
        return resume_all_dirty(edited, &edited_footprints, base_ctx);
    };

    let mut ctx = base_ctx.clone();
    let item_count = edited.items.len();
    let mut typings: Vec<Option<ItemTyping>> = vec![None; item_count];
    let mut adopted: Vec<bool> = vec![false; item_count];
    for edited_index in order {
        let (Some(item), Some(footprint)) = (
            edited.items.get(edited_index.0),
            edited_footprints.get(edited_index.0),
        )
        else {
            continue;
        };
        let base_index = edited_to_base.get(edited_index.0).copied().flatten();
        let base_checkpoint = base_index.and_then(|index| base.items.get(index.0));
        let is_adopted = adoptable(item, footprint, base_checkpoint, &changed);
        let typing = if let (true, Some(checkpoint)) = (is_adopted.0, base_checkpoint) {
            checkpoint.typing.clone()
        }
        else {
            let fresh = type_item(item, &ctx, footprint.has_hole.into());
            note_binding_change(&mut changed, item, &fresh, base_checkpoint);
            fresh
        };
        thread_binding(&mut ctx, &typing);
        if let Some(slot) = typings.get_mut(edited_index.0) {
            *slot = Some(typing);
        }
        if let Some(slot) = adopted.get_mut(edited_index.0) {
            *slot = is_adopted.0;
        }
    }

    // Every edited item is produced exactly once by a well-formed spliced order
    // (the splice is a permutation); a gap can only arise from an order-structure
    // anomaly, which degrades to a full re-type rather than a partial answer.
    if typings.iter().any(Option::is_none) {
        return resume_all_dirty(edited, &edited_footprints, base_ctx);
    }
    Resume {
        typings: typings.into_iter().flatten().collect(),
        adopted,
    }
}

/// Whether an edited item may adopt its base checkpoint: it must have a base
/// match, be structurally identical to it (§"The soundness condition" condition
/// 1), and read no changed binding (§"The soundness condition" condition 2, via
/// [`Footprint::intersects`], which also blocks an opaque footprint).
fn adoptable(
    item: &LoweredItem,
    footprint: &Footprint,
    base_checkpoint: Option<&ItemCheckpoint>,
    changed: &BTreeSet<String>,
) -> MatchDecision
{
    MatchDecision(match base_checkpoint {
        | Some(checkpoint) => {
            checkpoint_matches_item(checkpoint, item).0 && !footprint.intersects(changed).0
        },
        | None => false,
    })
}

/// Whether a base checkpoint's identity (name, ascription, lowered term) equals
/// an edited item's — the unchanged-region test.
fn checkpoint_matches_item(
    checkpoint: &ItemCheckpoint,
    item: &LoweredItem,
) -> MatchDecision
{
    MatchDecision(
        checkpoint.name == item.name
            && checkpoint.ascription == item.ascription
            && checkpoint.term == item.term,
    )
}

/// Types one item (type-level, no evaluation), mirroring the session's
/// `process_item`/`define` classification: a holey item is declined; otherwise
/// it types against `ctx` and a value-typed definition reports its bound value
/// type.
fn type_item(
    item: &LoweredItem,
    ctx: &Ctx,
    has_hole: HolePresence,
) -> ItemTyping
{
    if has_hole.0 {
        return ItemTyping::Holey;
    }
    match item_type(item, ctx) {
        | None => ItemTyping::Unknown,
        | Some(Err(error)) => ItemTyping::TypeError { error },
        | Some(Ok(ty)) => match item.name {
            | Some(ref name) => match bound_value_type(&ty) {
                | Some(value_type) => ItemTyping::Definition {
                    name: name.clone(),
                    ty: Ty::Value(value_type),
                    bound: true,
                },
                | None => ItemTyping::Definition {
                    name: name.clone(),
                    ty,
                    bound: false,
                },
            },
            | None => ItemTyping::Expression { ty },
        },
    }
}

/// Splices the edit onto the base order and returns the processing order (the
/// edited item indices in spliced-order sequence), or [`None`] if the order
/// structure rejects the edit.
///
/// The base order carries one element per base item, payload = the base index
/// (the stable identity). Deleted base items are removed; each inserted edited
/// item is placed after its predecessor. A matched item keeps its original
/// order element, so its identity survives the index shifts — the "positions
/// out of the key" property. The returned order is checked to be a permutation
/// of the edited items (the external oracle: the spliced order must present
/// every item exactly once, in dependency order).
fn splice_order(
    base_len: BaseItemCount,
    edited_to_base: &[Option<BaseItemIndex>],
) -> Option<Vec<EditedItemIndex>>
{
    let mut order: OrderMaintenance<u64> = OrderMaintenance::new().ok()?;
    let mut base_pos: Vec<Pos> = Vec::with_capacity(base_len.0);
    for index in 0 .. base_len.0 {
        let id = u64::try_from(index).ok()?;
        base_pos.push({
            let present = order.push_back(id).ok()?;
            core::convert::identity(present)
        });
    }

    // Remove deleted base items (those no edited item matched).
    let matched_base: BTreeSet<BaseItemIndex> = edited_to_base.iter().copied().flatten().collect();
    for index in 0 .. base_len.0 {
        if !matched_base.contains(&BaseItemIndex(index))
            && let Some(pos) = base_pos.get(index)
        {
            let _removed = order.remove(*pos);
        }
    }

    // Place edited items in source order: a matched item reuses its base
    // element; an inserted item is spliced after its predecessor. `id_to_edited`
    // recovers the edited position from the stable payload identity.
    let mut id_to_edited: BTreeMap<u64, EditedItemIndex> = BTreeMap::new();
    let mut next_fresh_id = u64::try_from(base_len.0).ok()?;
    let mut previous: Option<Pos> = None;
    for (edited_index, base_index) in edited_to_base.iter().enumerate() {
        let (id, pos) = match *base_index {
            | Some(index) => {
                let pos = *base_pos.get(index.0)?;
                let id = u64::try_from(index.0).ok()?;
                (id, pos)
            },
            | None => {
                let id = next_fresh_id;
                next_fresh_id = next_fresh_id.checked_add(1)?;
                let pos = match previous {
                    | Some(anchor) => order.insert_after(anchor, id).ok()?,
                    | None => order.push_front(id).ok()?,
                };
                (id, pos)
            },
        };
        let _previous = id_to_edited.insert(id, EditedItemIndex(edited_index));
        previous = Some(pos);
    }

    // Read the spliced order back out and map each element's stable identity to
    // its edited position — the processing order.
    let mut processing: Vec<EditedItemIndex> = Vec::with_capacity(edited_to_base.len());
    for (_pos, &id) in order.iter() {
        processing.push(*{
            let found = id_to_edited.get(&id)?;
            core::convert::identity(found)
        });
    }

    // Oracle: the spliced order must be exactly the edited source order (a
    // forward permutation), or the splice is unsound and we decline it.
    let is_source_order = processing.len() == edited_to_base.len()
        && processing
            .iter()
            .enumerate()
            .all(|(position, index)| position == index.0);
    debug_assert!(
        is_source_order,
        "the spliced order must present the edited items in source order"
    );
    is_source_order.then_some(processing)
}

/// Aligns base checkpoints and edited items by definition name via a longest
/// common subsequence over the name keys, returning matched
/// `(base_index, edited_index)` pairs in increasing order.
///
/// Anonymous items (expressions, `None` name) match each other positionally
/// within the subsequence. The alignment is sound for any inputs: a mis-pairing
/// only forces more re-typing (the paired items fail the structural-identity
/// test, or their names are unstable), never an unsound reuse.
fn align_by_name(
    base: &[ItemCheckpoint],
    edited: &[LoweredItem],
) -> Vec<ItemMatch>
{
    let base_keys: Vec<Option<DefinitionName<'_>>> = base
        .iter()
        .map(|checkpoint| checkpoint.name.as_deref().map(DefinitionName))
        .collect();
    let edited_keys: Vec<Option<DefinitionName<'_>>> = edited
        .iter()
        .map(|item| item.name.as_deref().map(DefinitionName))
        .collect();
    longest_common_subsequence(&base_keys, &edited_keys)
}

/// Threads a value-typed definition's binding into `ctx`, so the items after it
/// type against it (the session's cross-item bridge, type-level).
fn thread_binding(
    ctx: &mut Ctx,
    typing: &ItemTyping,
)
{
    if let ItemTyping::Definition {
        ref name,
        ty: Ty::Value(ref value_type),
        bound: true,
    } = *typing
    {
        ctx.bind(name.clone(), value_type.clone());
    }
}

/// The value type a definition of type `ty` binds into scope, or [`None`] for a
/// bare computation type (which cannot be a variable in CBPV). Mirrors
/// `crate::session`'s private discipline.
fn bound_value_type(ty: &Ty) -> Option<ValueType>
{
    match *ty {
        | Ty::Value(ref value_type) => Some(value_type.clone()),
        | Ty::Comp(CompType::F(ref payload, _)) => Some((**payload).clone()),
        | _ => None,
    }
}

/// Seeds the changed-binding set with the names an edit changes structurally,
/// before any re-typing: deleted definitions, inserted definitions, and any
/// name that is not defined exactly once in *both* programs (a redefinition or
/// shadow, handled conservatively — its items never adopt).
fn seed_changed_bindings(
    base: &Checkpoints,
    edited: &Lowered,
    matched_base: &BTreeSet<BaseItemIndex>,
    edited_to_base: &[Option<BaseItemIndex>],
) -> BTreeSet<String>
{
    let mut changed: BTreeSet<String> = BTreeSet::new();
    // Deleted base definitions: a name that leaves scope.
    for (index, checkpoint) in base.items.iter().enumerate() {
        if !matched_base.contains(&BaseItemIndex(index))
            && let Some(name) = checkpoint.name.as_ref()
        {
            let _inserted = changed.insert(name.clone());
        }
    }
    // Inserted edited definitions: a name that enters (or shadows into) scope.
    for (index, item) in edited.items.iter().enumerate() {
        if edited_to_base.get(index).copied().flatten().is_none()
            && let Some(name) = item.name.as_ref()
        {
            let _inserted = changed.insert(name.clone());
        }
    }
    // Non-unique names: conservatively changed (alignment cannot pin a stable
    // 1:1 pairing, so reuse for them would be unsound).
    for name in unstable_names(base, edited) {
        let _inserted = changed.insert(name);
    }
    changed
}

/// Inverts a base↔edited match list into per-edited-item base indices.
fn invert_matches(
    matches: &[ItemMatch],
    edited_len: EditedItemCount,
) -> Vec<Option<BaseItemIndex>>
{
    let mut edited_to_base: Vec<Option<BaseItemIndex>> = vec![None; edited_len.0];
    for item_match in matches {
        if let Some(slot) = edited_to_base.get_mut(item_match.edited.0) {
            *slot = Some(item_match.base);
        }
    }
    edited_to_base
}

/// A full-re-type resume (every item dirty): the graceful-degradation and
/// oracle path — it recomputes exactly the from-scratch typings.
fn resume_all_dirty(
    edited: &Lowered,
    edited_footprints: &[Footprint],
    base_ctx: &Ctx,
) -> Resume
{
    let mut ctx = base_ctx.clone();
    let mut typings: Vec<ItemTyping> = Vec::with_capacity(edited.items.len());
    for (index, item) in edited.items.iter().enumerate() {
        let has_hole = edited_footprints
            .get(index)
            .is_some_and(|footprint| footprint.has_hole);
        let typing = type_item(item, &ctx, has_hole.into());
        thread_binding(&mut ctx, &typing);
        typings.push(typing);
    }
    let dirty = vec![false; typings.len()];
    Resume {
        typings,
        adopted: dirty,
    }
}

/// Records that a re-typed definition's binding changed, when its fresh
/// contribution differs from the base checkpoint's — the trail-aware
/// propagation (a body edit that keeps the type adds nothing, so type-stable
/// dependents stay adoptable).
fn note_binding_change(
    changed: &mut BTreeSet<String>,
    item: &LoweredItem,
    fresh: &ItemTyping,
    base_checkpoint: Option<&ItemCheckpoint>,
)
{
    let Some(name) = item.name.as_ref()
    else {
        return;
    };
    let base_contribution = base_checkpoint.and_then(|checkpoint| {
        binding_contribution(&checkpoint.typing).map(|(_, value_type)| value_type.clone())
    });
    let fresh_contribution = binding_contribution(fresh).map(|(_, value_type)| value_type.clone());
    if base_contribution != fresh_contribution {
        let _inserted = changed.insert(name.clone());
    }
}

/// The binding a typing contributes to the context: `(name, value type)` for a
/// bound definition, else [`None`].
fn binding_contribution(typing: &ItemTyping) -> Option<(&String, &ValueType)>
{
    match *typing {
        | ItemTyping::Definition {
            ref name,
            ty: Ty::Value(ref value_type),
            bound: true,
        } => Some((name, value_type)),
        | _ => None,
    }
}

/// The definition names that do not occur exactly once in both programs — the
/// redefinition/shadow names reuse must avoid.
fn unstable_names(
    base: &Checkpoints,
    edited: &Lowered,
) -> BTreeSet<String>
{
    let base_counts = name_counts(
        base.items
            .iter()
            .map(|checkpoint| checkpoint.name.as_deref().map(DefinitionName)),
    );
    let edited_counts = name_counts(
        edited
            .items
            .iter()
            .map(|item| item.name.as_deref().map(DefinitionName)),
    );
    let mut unstable: BTreeSet<String> = BTreeSet::new();
    for (name, base_count) in &base_counts {
        let edited_count = edited_counts
            .get(name)
            .copied()
            .unwrap_or(DefinitionMultiplicity(0));
        if base_count.0 != 1 || edited_count.0 != 1 {
            let _inserted = unstable.insert(name.clone());
        }
    }
    for (name, edited_count) in &edited_counts {
        let base_count = base_counts
            .get(name)
            .copied()
            .unwrap_or(DefinitionMultiplicity(0));
        if edited_count.0 != 1 || base_count.0 != 1 {
            let _inserted = unstable.insert(name.clone());
        }
    }
    unstable
}

/// Counts occurrences of each definition name (anonymous items are ignored).
fn name_counts<'names, I>(names: I) -> BTreeMap<String, DefinitionMultiplicity>
where
    I: IntoIterator<Item = Option<DefinitionName<'names>>>,
{
    let mut counts: BTreeMap<String, DefinitionMultiplicity> = BTreeMap::new();
    for name in names.into_iter().flatten() {
        let slot = counts
            .entry(name.0.to_owned())
            .or_insert(DefinitionMultiplicity(0));
        slot.0 = slot.0.saturating_add(1);
    }
    counts
}

/// The longest-common-subsequence match pairs of two key sequences (the same
/// dynamic program `crate::edit` uses for item alignment, kept local so this
/// module stays new-module-shaped).
fn longest_common_subsequence(
    left: &[Option<DefinitionName<'_>>],
    right: &[Option<DefinitionName<'_>>],
) -> Vec<ItemMatch>
{
    let rows = left.len();
    let cols = right.len();
    let stride = cols.saturating_add(1);
    let cells = rows.saturating_add(1).saturating_mul(stride);
    let mut table: Vec<usize> = vec![0_usize; cells];
    let index =
        |row: usize, col: usize| -> usize { row.saturating_mul(stride).saturating_add(col) };

    for row in (0 .. rows).rev() {
        for col in (0 .. cols).rev() {
            let value = if left.get(row) == right.get(col) {
                table
                    .get(index(row.saturating_add(1), col.saturating_add(1)))
                    .copied()
                    .unwrap_or(0_usize)
                    .saturating_add(1)
            }
            else {
                let down = table
                    .get(index(row.saturating_add(1), col))
                    .copied()
                    .unwrap_or(0_usize);
                let right_cell = table
                    .get(index(row, col.saturating_add(1)))
                    .copied()
                    .unwrap_or(0_usize);
                down.max(right_cell)
            };
            if let Some(slot) = table.get_mut(index(row, col)) {
                *slot = value;
            }
        }
    }

    let mut matches: Vec<ItemMatch> = Vec::new();
    let mut row = 0_usize;
    let mut col = 0_usize;
    while row < rows && col < cols {
        if left.get(row) == right.get(col) {
            matches.push(ItemMatch {
                base: BaseItemIndex(row),
                edited: EditedItemIndex(col),
            });
            row = row.saturating_add(1);
            col = col.saturating_add(1);
        }
        else {
            let down = table
                .get(index(row.saturating_add(1), col))
                .copied()
                .unwrap_or(0_usize);
            let right_cell = table
                .get(index(row, col.saturating_add(1)))
                .copied()
                .unwrap_or(0_usize);
            if down >= right_cell {
                row = row.saturating_add(1);
            }
            else {
                col = col.saturating_add(1);
            }
        }
    }
    matches
}
