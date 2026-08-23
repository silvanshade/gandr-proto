#![cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    expect(
        single_field_struct_needs_transparent_repr,
        reason = "every salsa struct and tracked query expands to an id newtype the engine's macros generate; the transparent representation cannot be declared at a definition site this crate writes, and this module is the only one that hosts those macros"
    )
)]

//! The database, its inputs, and the tracked queries — the model under
//! assessment.
//!
//! # The query graph
//!
//! Item typing is expressed as four memoized queries over a database keyed by
//! item identity:
//!
//! ```text
//! item_footprint(slot)              the item's dependency footprint
//! name_table(revision)              which slot binds each name, in source order
//! item_binding(revision, slot)      the type that slot contributes to the context
//! item_unfolding(revision, slot)    the definitional unfolding that slot contributes
//! item_typing(revision, slot)       the slot's typing, against a footprint-restricted context
//! ```
//!
//! `item_typing` reads its item's footprint, resolves each name in it to the
//! slot that binds the name, asks that slot for its binding, rebuilds a context
//! containing **exactly those bindings**, and types the item against it. The
//! footprint is thereby projected into the query's declared dependencies: what
//! the item read is what the engine will invalidate it on.
//!
//! # `item_binding` is the firewall, and the whole experiment rests on it
//!
//! When an item's body changes but its type does not, `item_binding` for that
//! item re-executes — the item really did change — and produces a value equal
//! to the one it produced before. The engine compares, finds equality, and
//! **backdates**: every reader's `item_typing` is left alone, without anything
//! having scanned the program to discover which readers were safe.
//!
//! That is the demand-driven form of the hand-rolled path's adoption rule, and
//! it is also the single point of failure of this assessment. An engine
//! configured to skip the equality comparison would still produce correct
//! answers, still produce a plausible-looking cost table, and have the
//! mechanism being measured switched off. [`crate::measure`] asserts the
//! invalidation wave stops, rather than trusting that it did.
//!
//! # What the encoding is doing here
//!
//! A typing and a binding both carry core types, which are reference-counted
//! and so cannot be retained by the engine ([`crate::arena`]). Both therefore
//! cross into the query graph as bytes, produced by the checkpoint codec
//! `gandr-core-incremental` ships for persistence, and are decoded on the way
//! out. The binding's encoding is deliberately **normalized** — the contributed
//! name and value type, with a placeholder term — so that two bindings compare
//! equal exactly when they bind the same type. Encoding the whole checkpoint
//! instead would make every body edit change the bytes and destroy the
//! firewall.
//!
//! # The contribution is split, because the adoption rule is
//!
//! A context binding threads two things: a name's **type**, and its
//! **definitional unfolding**. Those are separate dependencies with separate
//! invalidation conditions, and collapsing them either loses reuse or loses
//! soundness. The checkpoint engine's own rule distinguishes them — an item's
//! *footprint* is what it read, its *type support* is what it read in a type
//! position, and only the latter is sensitive to a definition's value changing
//! at a constant type — so the query graph distinguishes them too:
//!
//! - `item_typing` depends on `item_binding` for every name in its footprint;
//! - and on `item_unfolding` for every name in its **type support**, which is
//!   the subset whose *values* its typing can consult.
//!
//! That split is what lets a value-only edit stop at the firewall for an
//! ordinary reader while still reaching a reader whose ascription mentions the
//! edited name. Carrying the unfolding for every footprint name instead would
//! be sound and would give up the type-stable reuse the whole exercise is
//! about; carrying it for none would be unsound in exactly the value-mediated
//! way the checkpoint engine's own regression cases pin.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::sync::Arc;

use gandr_core_incremental::checkpoint::Checkpoints;
use gandr_core_incremental::checkpoint::ItemCheckpoint;
use gandr_core_incremental::checkpoint::ItemTyping;
use gandr_core_incremental::checkpoint::checkpoint_with;
use gandr_core_incremental::footprint::Footprint;
use gandr_core_incremental::persistence::decode_checkpoints;
use gandr_core_incremental::persistence::encode_checkpoints;
use gandr_core_incremental::region::Program;
use gandr_core_term::boundary::NameRef;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;
use salsa::Setter as _;

use crate::arena::ItemStore;
use crate::boundary::BoundaryByteCount;
use crate::boundary::DefinitionKey;
use crate::boundary::ItemDigest;
use crate::boundary::RetainedByteCount;
use crate::boundary::SlotIndex;
use crate::ledger::AssessmentError;
use crate::ledger::Ledger;

/// One item's typing, as it crosses into the query graph.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedTyping(Vec<u8>);

impl EncodedTyping
{
    /// The encoded size, which is what the boundary counter accumulates.
    #[inline]
    #[must_use]
    pub fn size(&self) -> BoundaryByteCount
    {
        BoundaryByteCount::from(self.0.len())
    }
}

/// The binding one item contributes to a reader's context, as it crosses into
/// the query graph.
///
/// Normalized to the contributed name and value type, so byte equality is
/// binding equality — the property the firewall depends on.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedBinding(Vec<u8>);

impl EncodedBinding
{
    /// The encoded size, which is what the boundary counter accumulates.
    #[inline]
    #[must_use]
    pub fn size(&self) -> BoundaryByteCount
    {
        BoundaryByteCount::from(self.0.len())
    }
}

/// The definitional unfolding one item contributes, as it crosses into the
/// query graph.
///
/// Separate from [`EncodedBinding`] because it has a different invalidation
/// condition: a body edit at a constant type leaves the binding equal and moves
/// the unfolding, and a reader depends on one, the other, or both.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedUnfolding(Vec<u8>);

impl EncodedUnfolding
{
    /// The encoded size, which is what the boundary counter accumulates.
    #[inline]
    #[must_use]
    pub fn size(&self) -> BoundaryByteCount
    {
        BoundaryByteCount::from(self.0.len())
    }
}

/// One item's identity and content, as the database sees it.
///
/// The split is the point: `index` and `name` are the item's **identity** and
/// do not move under a body edit, so queries keyed on them stay valid; `digest`
/// is its **content**, and moving it is what marks the item dirty. Folding the
/// name into the digest would make every body edit invalidate name resolution
/// for the whole program, which would defeat the measurement without changing
/// an answer.
#[salsa::input(debug)]
pub struct ItemSlotInput
{
    /// The item's position, stable across a body edit.
    #[returns(copy)]
    pub index: SlotIndex,
    /// The item's definition name, or [`None`] for an expression item.
    #[returns(clone)]
    pub name: Option<DefinitionKey>,
    /// The item's content address.
    #[returns(copy)]
    pub digest: ItemDigest,
}

/// The ordered slots of one program revision.
#[salsa::input(debug)]
pub struct ProgramRevision
{
    /// The slots, in source order.
    #[returns(ref)]
    pub slots: Vec<ItemSlotInput>,
}

/// Databases that answer this crate's item-typing queries.
#[salsa::db]
pub trait AssessmentDb: salsa::Database
{
    /// The work counters this database records into.
    fn ledger(&self) -> &Ledger;
}

/// The assessment's database.
#[salsa::db]
#[derive(Clone)]
pub struct EngineDatabase
{
    /// The engine's own storage.
    storage: salsa::Storage<Self>,
    /// The work counters, shared with the event callback.
    ledger: Arc<Ledger>,
}

impl EngineDatabase
{
    /// Creates a database whose event callback records memo reuse and cycle
    /// iteration into a fresh ledger.
    ///
    /// # Contract
    /// - ensures: returns an empty database whose ledger reads zero.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        let ledger = Arc::new(Ledger::new());
        let observed = Arc::clone(&ledger);
        let callback = move |event: salsa::Event| match event.kind {
            | salsa::EventKind::DidValidateMemoizedValue { .. } => {
                observed.record_memo_validation();
            },
            | salsa::EventKind::WillIterateCycle { .. } => {
                observed.record_cycle_iteration();
            },
            | _ => {},
        };
        Self {
            storage: salsa::Storage::new(Some(Box::new(callback))),
            ledger,
        }
    }

    /// The bytes the engine reports retained across its own tables.
    ///
    /// # Contract
    /// - ensures: returns the engine's own accounting of its memo and struct
    ///   storage, which excludes anything held outside the database.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn retained_bytes(&self) -> RetainedByteCount
    {
        let database: &dyn salsa::Database = self;
        let usage = database.memory_usage();
        let queries: usize = usage
            .queries
            .values()
            .map(|info| {
                info.size_of_metadata()
                    .saturating_add(info.size_of_fields())
            })
            .fold(0_usize, usize::saturating_add);
        let structs: usize = usage
            .structs
            .iter()
            .map(|info| {
                info.size_of_metadata()
                    .saturating_add(info.size_of_fields())
            })
            .fold(0_usize, usize::saturating_add);
        RetainedByteCount::from(queries.saturating_add(structs))
    }
}

impl Default for EngineDatabase
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

#[salsa::db]
impl salsa::Database for EngineDatabase
{
}

#[salsa::db]
impl AssessmentDb for EngineDatabase
{
    #[inline]
    fn ledger(&self) -> &Ledger
    {
        &self.ledger
    }
}

/// One item's dependency footprint.
///
/// Depends on the item's content and nothing else, so it re-executes exactly
/// once per edited item — where the hand-rolled path rescans every item in the
/// program on every recheck.
///
/// # Contract
/// - ensures: returns the footprint of the item installed at `slot`'s index.
/// - fails: returns the store or codec failure that prevented reading the item.
/// - panics: none.
#[salsa::tracked]
pub fn item_footprint(
    db: &dyn AssessmentDb,
    slot: ItemSlotInput,
) -> Result<Footprint, AssessmentError>
{
    db.ledger().record_footprint_execution();
    let item = fetch_verified(db, slot)?;
    Ok(gandr_core_incremental::footprint::footprint_of(&item))
}

/// The program's name-resolution table: for each defined name, the slots that
/// define it, in source order.
///
/// **One query, not one dependency edge per candidate.** Resolving a name by
/// scanning the slots ahead of a reader is the obvious shape and it is
/// quadratic: it gives every reader a dependency edge to every slot before it,
/// so the engine's per-revision verification walks a number of edges that grows
/// with the square of the program. Measured, that reproduced inside the query
/// graph exactly the growth the assessment was meant to remove.
///
/// Folding resolution into a single table makes each reader depend on one
/// value. The cost is a coarser firewall — renaming anything re-executes every
/// reader's typing, where the per-slot shape would have invalidated only that
/// name's readers — and it is the trade the flagship consumers of this engine
/// design make too, for the same reason.
///
/// # Contract
/// - ensures: returns every definition name in the revision mapped to its
///   defining slots in increasing source order.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: the table must carry every definition and preserve source
///   order, since shadowing is resolved by taking the last definer ahead of a
///   reader; a table losing order would resolve a shadowed name to the wrong
///   binding without changing any count.
/// - witness: `support::the_name_table_preserves_source_order`
#[salsa::tracked]
pub fn name_table(
    db: &dyn AssessmentDb,
    revision: ProgramRevision,
) -> NameTable
{
    db.ledger().record_name_table_execution();
    let mut table: BTreeMap<DefinitionKey, Vec<SlotIndex>> = BTreeMap::new();
    for slot in revision.slots(db) {
        let Some(name) = slot.name(db)
        else {
            continue;
        };
        table.entry(name).or_default().push(slot.index(db));
    }
    NameTable(table)
}

/// A revision's definition names, each with the slots that define it.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NameTable(BTreeMap<DefinitionKey, Vec<SlotIndex>>);

impl NameTable
{
    /// The slot binding `name` for a reader at `before`: the last definition of
    /// that name strictly ahead of the reader, so shadowing resolves the way
    /// the threaded context does.
    ///
    /// # Contract
    /// - ensures: returns the greatest defining slot strictly less than
    ///   `before`, or [`None`] when the name is unbound there.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn definer(
        &self,
        name: &DefinitionKey,
        before: SlotIndex,
    ) -> Option<SlotIndex>
    {
        let slots = self.0.get(name)?;
        slots.iter().rev().find(|slot| **slot < before).copied()
    }
}

/// The binding one slot contributes to a reader's context.
///
/// **The firewall.** Its value is a function of the contributed name and value
/// type alone, so a body edit that leaves the type in place recomputes an equal
/// value and stops the invalidation wave here.
///
/// # Contract
/// - ensures: returns the normalized encoding of the name and value type the
///   slot binds, or [`None`] when it binds nothing.
/// - fails: returns the store or codec failure that prevented producing it.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: the encoding must be a function of the binding and not of the
///   item, since equality on it is what stops re-execution; an encoding
///   carrying the term would silently re-invalidate every reader on every body
///   edit while still answering correctly.
/// - witness: `firewall::value_only_edit_leaves_the_binding_bytes_equal`
/// - witness: `firewall::type_changing_edit_moves_the_binding_bytes`
#[salsa::tracked]
pub fn item_binding(
    db: &dyn AssessmentDb,
    revision: ProgramRevision,
    slot: ItemSlotInput,
) -> Result<Option<EncodedBinding>, AssessmentError>
{
    db.ledger().record_binding_execution();
    let typing = match *item_typing(db, revision, slot) {
        | Ok(ref encoded) => decode_typing(db, encoded)?,
        | Err(error) => return Err(error),
    };
    let Some((name, value_type)) = contributed_binding(&typing)
    else {
        return Ok(None);
    };
    let encoded = encode_binding(db, &DefinitionKey::from(name), &value_type)?;
    Ok(Some(encoded))
}

/// The definitional unfolding one slot contributes to a reader's context.
///
/// Mirrors the checkpoint engine's threading exactly: a bound value definition
/// contributes its value, and a body carrying an unsolved hole contributes
/// nothing, because a hole is consistent with everything and unfolding into one
/// would let a law be proved through it.
///
/// # Contract
/// - ensures: returns the encoded value the slot unfolds to, or [`None`] when
///   it unfolds to nothing.
/// - fails: returns the store or codec failure that prevented producing it.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: the unfolding must move under a value-only edit where the
///   binding does not, since that difference is the whole reason the two are
///   separate dependencies; an unfolding that tracked the binding would restore
///   the over-adoption the split exists to prevent.
/// - witness: `firewall::a_value_only_edit_moves_the_unfolding_not_the_binding`
#[salsa::tracked]
pub fn item_unfolding(
    db: &dyn AssessmentDb,
    revision: ProgramRevision,
    slot: ItemSlotInput,
) -> Result<Option<EncodedUnfolding>, AssessmentError>
{
    db.ledger().record_unfolding_execution();
    let item = fetch_verified(db, slot)?;
    let typing = match *item_typing(db, revision, slot) {
        | Ok(ref encoded) => decode_typing(db, encoded)?,
        | Err(error) => return Err(error),
    };
    if contributed_binding(&typing).is_none() {
        return Ok(None);
    }
    let Term::Value(ref value) = item.term
    else {
        return Ok(None);
    };
    let (_, residual) = gandr_core_term::subst::subst_holes_value(
        value,
        &gandr_core_term::subst::HoleSubstitution::default(),
    );
    if bool::from(residual) {
        return Ok(None);
    }
    let encoded = encode_unfolding_value(value)?;
    db.ledger().record_boundary_bytes(encoded.size());
    Ok(Some(encoded))
}

/// One item's typing, against a context restricted to its footprint.
///
/// # Contract
/// - ensures: returns the encoded typing of the item at `slot`, computed
///   against exactly the bindings its footprint names, in program order.
/// - fails: returns the store or codec failure that prevented producing it.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: restricting the context to the footprint must not change any
///   answer, which holds exactly when the footprint over-approximates the
///   item's reads; the differential against from-scratch typing is what
///   distinguishes a complete footprint from an incomplete one.
/// - witness: `differential::engine_agrees_with_from_scratch_on_the_workload`
#[salsa::tracked]
pub fn item_typing(
    db: &dyn AssessmentDb,
    revision: ProgramRevision,
    slot: ItemSlotInput,
) -> Result<EncodedTyping, AssessmentError>
{
    db.ledger().record_typing_execution();
    let item = fetch_verified(db, slot)?;
    let index = slot.index(db);
    let footprint = match *item_footprint(db, slot) {
        | Ok(ref footprint) => footprint.clone(),
        | Err(error) => return Err(error),
    };

    let slots = revision.slots(db);
    let type_support = gandr_core_incremental::footprint::type_support_of(&item);
    let mut gathered: Vec<(SlotIndex, String, ValueType)> = Vec::new();
    let mut unfoldings: Vec<(SlotIndex, String, Value)> = Vec::new();

    if footprint.opaque || type_support.opaque {
        // An opaque footprint reads everything, so the query depends on every
        // contribution ahead of it — the conservative shape the hand-rolled
        // rule takes when it refuses to adopt an opaque item.
        for candidate in slots {
            if candidate.index(db) >= index {
                break;
            }
            collect_binding(db, revision, *candidate, &mut gathered)?;
            collect_unfolding(db, revision, *candidate, &mut unfoldings)?;
        }
    }
    else {
        let table = name_table(db, revision);
        for name in &footprint.names {
            let key = DefinitionKey::from(name.clone());
            let Some(defining) = resolve(db, table, slots, &key, index)
            else {
                continue;
            };
            collect_binding(db, revision, defining, &mut gathered)?;
        }
        // Only the names read in a *type* position can have their value
        // consulted, so only those carry an unfolding dependency. Taking every
        // footprint name here would be sound and would forfeit the type-stable
        // reuse this whole comparison is about.
        for name in &type_support.names {
            let key = DefinitionKey::from(name.clone());
            let Some(defining) = resolve(db, table, slots, &key, index)
            else {
                continue;
            };
            collect_unfolding(db, revision, defining, &mut unfoldings)?;
        }
    }

    gathered.sort_by_key(|entry| entry.0);
    unfoldings.sort_by_key(|entry| entry.0);
    let mut ctx = Ctx::new();
    for (_, name, value_type) in gathered {
        ctx.bind(name, value_type);
    }
    for (_, name, value) in unfoldings {
        ctx.define(NameRef::from(name.as_str()), Rc::new(value));
    }

    let typed = checkpoint_with(&Program::new(vec![item]), &ctx);
    let encoded = encode_checkpoints(&typed)?;
    let encoded = EncodedTyping(encoded);
    db.ledger().record_boundary_bytes(encoded.size());
    Ok(encoded)
}

/// Resolves one name to the slot input that defines it for a reader at
/// `before`.
fn resolve(
    db: &dyn AssessmentDb,
    table: &NameTable,
    slots: &[ItemSlotInput],
    name: &DefinitionKey,
    before: SlotIndex,
) -> Option<ItemSlotInput>
{
    let _ = db;
    let defining = table.definer(name, before)?;
    let defining: usize = defining.into();
    slots.get(defining).copied()
}

/// Adds `slot`'s definitional unfolding, if it has one, to the unfoldings being
/// gathered for a reader's context.
fn collect_unfolding(
    db: &dyn AssessmentDb,
    revision: ProgramRevision,
    slot: ItemSlotInput,
    gathered: &mut Vec<(SlotIndex, String, Value)>,
) -> Result<(), AssessmentError>
{
    let unfolding = match *item_unfolding(db, revision, slot) {
        | Ok(Some(ref encoded)) => encoded.clone(),
        | Ok(None) => return Ok(()),
        | Err(error) => return Err(error),
    };
    let Some(name) = slot.name(db)
    else {
        return Ok(());
    };
    let value = decode_unfolding(db, &unfolding)?;
    gathered.push((slot.index(db), name.as_ref().to_owned(), value));
    Ok(())
}

/// Encodes a definitional unfolding through the checkpoint codec, by carrying
/// the value as a checkpoint's term.
fn encode_unfolding_value(value: &Value) -> Result<EncodedUnfolding, AssessmentError>
{
    let checkpoint = ItemCheckpoint {
        name: None,
        ascription: None,
        term: Term::Value(value.clone()),
        footprint: Footprint::default(),
        typing: ItemTyping::Holey,
    };
    let checkpoints = Checkpoints {
        items: vec![checkpoint],
    };
    let bytes = encode_checkpoints(&checkpoints)?;
    Ok(EncodedUnfolding(bytes))
}

/// Decodes a definitional unfolding back into the value it carries.
fn decode_unfolding(
    db: &dyn AssessmentDb,
    unfolding: &EncodedUnfolding,
) -> Result<Value, AssessmentError>
{
    db.ledger().record_boundary_bytes(unfolding.size());
    let checkpoints = decode_checkpoints(&unfolding.0)?;
    let checkpoint = checkpoints
        .items
        .first()
        .ok_or(AssessmentError::MalformedBinding)?;
    match checkpoint.term {
        | Term::Value(ref value) => Ok(value.clone()),
        | Term::Comp(_) => Err(AssessmentError::MalformedBinding),
    }
}

/// Adds `slot`'s binding, if it has one, to the bindings being gathered for a
/// reader's context.
fn collect_binding(
    db: &dyn AssessmentDb,
    revision: ProgramRevision,
    slot: ItemSlotInput,
    gathered: &mut Vec<(SlotIndex, String, ValueType)>,
) -> Result<(), AssessmentError>
{
    let binding = match *item_binding(db, revision, slot) {
        | Ok(Some(ref encoded)) => encoded.clone(),
        | Ok(None) => return Ok(()),
        | Err(error) => return Err(error),
    };
    let (name, value_type) = decode_binding(db, &binding)?;
    gathered.push((slot.index(db), name, value_type));
    Ok(())
}

/// The item at `slot`, with its content address checked against the digest the
/// database holds for it.
fn fetch_verified(
    db: &dyn AssessmentDb,
    slot: ItemSlotInput,
) -> Result<gandr_core_incremental::region::Item, AssessmentError>
{
    let index = slot.index(db);
    let digest = slot.digest(db);
    let item = ItemStore::fetch(index)?;
    let actual = ItemStore::digest(&item)?;
    if actual == digest {
        Ok(item)
    }
    else {
        Err(AssessmentError::DigestMismatch(index))
    }
}

/// The name and value type a typing contributes to the context, mirroring the
/// checkpoint engine's own binding projection: a value-typed definition binds
/// its type, and a returner binds its payload.
fn contributed_binding(typing: &ItemTyping) -> Option<(String, ValueType)>
{
    match *typing {
        | ItemTyping::Definition {
            ref name,
            ref ty,
            bound: true,
        } => {
            let value_type = match *ty {
                | Ty::Value(ref value_type) => value_type.clone(),
                | Ty::Comp(CompType::F(ref payload, _)) => (**payload).clone(),
                | _ => return None,
            };
            Some((name.clone(), value_type))
        },
        | _ => None,
    }
}

/// Encodes a binding into the normalized form byte equality compares.
fn encode_binding(
    db: &dyn AssessmentDb,
    name: &DefinitionKey,
    value_type: &ValueType,
) -> Result<EncodedBinding, AssessmentError>
{
    let name: &str = name.as_ref();
    let checkpoint = ItemCheckpoint {
        name: Some(name.to_owned()),
        ascription: None,
        term: Term::Value(Value::Unit),
        footprint: Footprint::default(),
        typing: ItemTyping::Definition {
            name: name.to_owned(),
            ty: Ty::Value(value_type.clone()),
            bound: true,
        },
    };
    let checkpoints = Checkpoints {
        items: vec![checkpoint],
    };
    let bytes = encode_checkpoints(&checkpoints)?;
    let encoded = EncodedBinding(bytes);
    db.ledger().record_boundary_bytes(encoded.size());
    Ok(encoded)
}

/// Decodes a normalized binding back into the name and value type it carries.
fn decode_binding(
    db: &dyn AssessmentDb,
    binding: &EncodedBinding,
) -> Result<(String, ValueType), AssessmentError>
{
    db.ledger().record_boundary_bytes(binding.size());
    let checkpoints = decode_checkpoints(&binding.0)?;
    let checkpoint = checkpoints
        .items
        .first()
        .ok_or(AssessmentError::MalformedBinding)?;
    contributed_binding(&checkpoint.typing).ok_or(AssessmentError::MalformedBinding)
}

/// Decodes an encoded typing back into the typing it carries.
fn decode_typing(
    db: &dyn AssessmentDb,
    encoded: &EncodedTyping,
) -> Result<ItemTyping, AssessmentError>
{
    db.ledger().record_boundary_bytes(encoded.size());
    let checkpoints = decode_checkpoints(&encoded.0)?;
    let checkpoint = checkpoints
        .items
        .first()
        .ok_or(AssessmentError::MalformedBinding)?;
    Ok(checkpoint.typing.clone())
}

/// Decodes an encoded typing outside the query graph, for the harness to
/// compare against a from-scratch answer.
///
/// # Contract
/// - ensures: returns the typing the bytes carry.
/// - fails: returns the codec failure, or [`AssessmentError::MalformedBinding`]
///   when the payload carries no item.
/// - panics: none.
///
/// # Errors
///
/// Returns the codec failure, or [`AssessmentError::MalformedBinding`] when
/// the payload carries no item.
#[inline]
pub fn typing_of(encoded: &EncodedTyping) -> Result<ItemTyping, AssessmentError>
{
    let checkpoints = decode_checkpoints(&encoded.0)?;
    let checkpoint = checkpoints
        .items
        .first()
        .ok_or(AssessmentError::MalformedBinding)?;
    Ok(checkpoint.typing.clone())
}

/// A database plus the revision installed in it — the engine path's driver.
///
/// Holds the slot inputs so an edit can move exactly the digests that changed,
/// which is what keeps a body edit from marking the whole program dirty.
#[derive(Clone)]
pub struct EngineSession
{
    /// The database the queries run against.
    database: EngineDatabase,
    /// The installed revision's slot list.
    revision: ProgramRevision,
    /// The slot inputs, in source order.
    slots: Vec<ItemSlotInput>,
}

impl EngineSession
{
    /// Installs `program` as the first revision.
    ///
    /// # Contract
    /// - ensures: returns a session whose slots correspond one-to-one with
    ///   `program`'s items, with the items installed in the store the queries
    ///   read through.
    /// - fails: returns the codec failure when an item has no content address.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns the codec failure when an item has no content address.
    #[inline]
    pub fn install(program: &Program) -> Result<Self, AssessmentError>
    {
        ItemStore::install(program)?;
        let database = EngineDatabase::new();
        let mut slots: Vec<ItemSlotInput> = Vec::with_capacity(program.items.len());
        for (index, item) in program.items.iter().enumerate() {
            let digest = ItemStore::digest(item)?;
            let name = item.name.clone().map(DefinitionKey::from);
            slots.push(ItemSlotInput::new(
                &database,
                SlotIndex::from(index),
                name,
                digest,
            ));
        }
        let revision = ProgramRevision::new(&database, slots.clone());
        Ok(Self {
            database,
            revision,
            slots,
        })
    }

    /// Advances the session to `edited`, moving only the inputs whose content
    /// actually changed.
    ///
    /// # Contract
    /// - requires: `edited` has the same item list length as the installed
    ///   revision; a length change is out of this measurement's scope and
    ///   leaves the extra slots untouched.
    /// - ensures: every slot whose item changed carries its new digest, and no
    ///   other input moves.
    /// - fails: returns the store or codec failure that prevented installing.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns the store or codec failure that prevented installing.
    #[inline]
    pub fn apply(
        &mut self,
        edited: &Program,
    ) -> Result<(), AssessmentError>
    {
        ItemStore::install(edited)?;
        for (index, item) in edited.items.iter().enumerate() {
            let digest = ItemStore::digest(item)?;
            let name = item.name.clone().map(DefinitionKey::from);
            let Some(slot) = self.slots.get(index).copied()
            else {
                continue;
            };
            let current_digest = slot.digest(&self.database);
            let current_name = slot.name(&self.database);
            if current_name != name {
                let setter = slot.set_name(&mut self.database);
                let _previous = setter.to(name);
            }
            if current_digest != digest {
                let setter = slot.set_digest(&mut self.database);
                let _previous = setter.to(digest);
            }
        }
        Ok(())
    }

    /// The work counters this session records into.
    #[inline]
    #[must_use]
    pub fn ledger(&self) -> &Ledger
    {
        self.database.ledger()
    }

    /// The bytes the engine reports retained across its own tables.
    #[inline]
    #[must_use]
    pub fn retained_bytes(&self) -> RetainedByteCount
    {
        self.database.retained_bytes()
    }

    /// A per-ingredient breakdown of what the engine reports retained.
    #[inline]
    #[must_use]
    pub fn memory_report(&self) -> String
    {
        self.database.memory_report()
    }

    /// The installed revision's name-resolution table.
    ///
    /// # Contract
    /// - ensures: returns the table every reader's typing resolves its
    ///   footprint's names through.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn name_table(&self) -> NameTable
    {
        let database: &dyn AssessmentDb = &self.database;
        name_table(database, self.revision).clone()
    }

    /// Every item's typing, in source order — the demand shape the hand-rolled
    /// path's signature always produces.
    ///
    /// # Contract
    /// - ensures: returns one typing per slot, in source order.
    /// - fails: returns the first store or codec failure encountered.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns the first store or codec failure encountered.
    #[inline]
    pub fn typings(&self) -> Result<Vec<ItemTyping>, AssessmentError>
    {
        let database: &dyn AssessmentDb = &self.database;
        let mut typings: Vec<ItemTyping> = Vec::with_capacity(self.slots.len());
        for slot in &self.slots {
            match *item_typing(database, self.revision, *slot) {
                | Ok(ref encoded) => {
                    database.ledger().record_boundary_bytes(encoded.size());
                    typings.push(typing_of(encoded)?);
                },
                | Err(error) => return Err(error),
            }
        }
        Ok(typings)
    }

    /// One item's typing — the demand shape the hand-rolled path cannot
    /// express, since its signature produces the whole checkpoint set or
    /// nothing.
    ///
    /// # Contract
    /// - ensures: returns the typing of the item at `slot`, computing only what
    ///   that answer depends on.
    /// - fails: returns [`AssessmentError::MissingSlot`] for an unknown slot,
    ///   or the store or codec failure encountered.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`AssessmentError::MissingSlot`] for an unknown slot, or the
    /// store or codec failure encountered.
    #[inline]
    pub fn typing_at(
        &self,
        slot: SlotIndex,
    ) -> Result<ItemTyping, AssessmentError>
    {
        let database: &dyn AssessmentDb = &self.database;
        let index: usize = slot.into();
        let input = self
            .slots
            .get(index)
            .copied()
            .ok_or(AssessmentError::MissingSlot(slot))?;
        match *item_typing(database, self.revision, input) {
            | Ok(ref encoded) => {
                database.ledger().record_boundary_bytes(encoded.size());
                typing_of(encoded)
            },
            | Err(error) => Err(error),
        }
    }

    /// The total encoded size of every item's memoized typing — the same
    /// instrument the baseline's retained state is measured with, so the two
    /// numbers are comparable.
    ///
    /// # Contract
    /// - ensures: returns the summed encoded size of every slot's typing.
    /// - fails: returns the first store or codec failure encountered.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns the first store or codec failure encountered.
    #[inline]
    pub fn encoded_state_size(&self) -> Result<BoundaryByteCount, AssessmentError>
    {
        let database: &dyn AssessmentDb = &self.database;
        let mut total: usize = 0;
        for slot in &self.slots {
            match *item_typing(database, self.revision, *slot) {
                | Ok(ref encoded) => {
                    let size: usize = encoded.size().into();
                    total = total.saturating_add(size);
                },
                | Err(error) => return Err(error),
            }
        }
        Ok(BoundaryByteCount::from(total))
    }
}

impl EngineSession
{
    /// The normalized binding bytes one slot contributes — the value whose
    /// equality across revisions is what stops the invalidation wave.
    ///
    /// Exposed so the mechanism can be pinned directly rather than inferred
    /// from a count: an assessment that only ever observed the count could not
    /// distinguish a firewall that held from a workload that happened not to
    /// need one.
    ///
    /// # Contract
    /// - ensures: returns the slot's normalized binding encoding, or [`None`]
    ///   when it binds nothing.
    /// - fails: returns [`AssessmentError::MissingSlot`] for an unknown slot,
    ///   or the store or codec failure encountered.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`AssessmentError::MissingSlot`] for an unknown slot, or the
    /// store or codec failure encountered.
    #[inline]
    pub fn binding_bytes(
        &self,
        slot: SlotIndex,
    ) -> Result<Option<EncodedBinding>, AssessmentError>
    {
        let database: &dyn AssessmentDb = &self.database;
        let index: usize = slot.into();
        let input = self
            .slots
            .get(index)
            .copied()
            .ok_or(AssessmentError::MissingSlot(slot))?;
        match *item_binding(database, self.revision, input) {
            | Ok(ref binding) => Ok(binding.clone()),
            | Err(error) => Err(error),
        }
    }

    /// The encoded definitional unfolding one slot contributes.
    ///
    /// The companion to [`Self::binding_bytes`], and exposed for the same
    /// reason: the two contributions have different invalidation conditions,
    /// and a comparison that could not see them separately could not tell a
    /// model that tracked both from one that tracked whichever happened to
    /// answer the workload.
    ///
    /// # Contract
    /// - ensures: returns the slot's encoded unfolding, or [`None`] when it
    ///   unfolds to nothing.
    /// - fails: returns [`AssessmentError::MissingSlot`] for an unknown slot,
    ///   or the store or codec failure encountered.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`AssessmentError::MissingSlot`] for an unknown slot, or the
    /// store or codec failure encountered.
    #[inline]
    pub fn unfolding_bytes(
        &self,
        slot: SlotIndex,
    ) -> Result<Option<EncodedUnfolding>, AssessmentError>
    {
        let database: &dyn AssessmentDb = &self.database;
        let index: usize = slot.into();
        let input = self
            .slots
            .get(index)
            .copied()
            .ok_or(AssessmentError::MissingSlot(slot))?;
        match *item_unfolding(database, self.revision, input) {
            | Ok(ref unfolding) => Ok(unfolding.clone()),
            | Err(error) => Err(error),
        }
    }
}

impl EngineDatabase
{
    /// A per-ingredient breakdown of what the engine reports retained.
    ///
    /// Reported instead of a single total because the total is easy to misread:
    /// the engine's figures are the **stack** sizes of memo metadata and
    /// fields, so a query returning a heap-allocated value contributes the
    /// size of its pointer rather than the size of its bytes. A breakdown
    /// says which ingredient the retention actually sits in.
    ///
    /// # Contract
    /// - ensures: returns one line per ingredient, naming it and its slot
    ///   count, metadata bytes, and field bytes.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn memory_report(&self) -> String
    {
        let database: &dyn salsa::Database = self;
        let usage = database.memory_usage();
        let mut lines: Vec<String> = Vec::new();
        for info in &usage.structs {
            lines.push(format!(
                "  struct {:<24} count {:>6}  metadata {:>10} B  fields {:>10} B",
                info.debug_name(),
                info.count(),
                info.size_of_metadata(),
                info.size_of_fields()
            ));
        }
        for (name, info) in &usage.queries {
            lines.push(format!(
                "  query  {name:<24} count {:>6}  metadata {:>10} B  fields {:>10} B",
                info.count(),
                info.size_of_metadata(),
                info.size_of_fields()
            ));
        }
        lines.sort();
        lines.join("\n")
    }
}
