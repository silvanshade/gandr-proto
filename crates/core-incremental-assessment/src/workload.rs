//! The generated programs, and the edits applied to them.
//!
//! # Why the shape is blocks of chains
//!
//! A workload for this measurement has to make the **dirty set knowable in
//! advance**, because the counters are asserted rather than observed. A program
//! of independent definitions would make every dirty set trivially one item and
//! measure nothing about propagation; a program of one long chain would make
//! every dirty set the whole program and measure nothing about locality.
//!
//! So a workload is `blocks` independent chains of `block_length` items each.
//! Inside a chain, item *j* reads item *j−1* under an integer ascription;
//! between chains, nothing is shared. Editing the head of one chain therefore
//! has a dirty set that depends only on the *kind* of edit, and the two kinds
//! bracket the interesting range:
//!
//! - a **value-only** edit replaces an integer literal with another integer
//!   literal. The head re-types; its bound type does not move, so nothing
//!   downstream needs re-typing. The dirty set is one item, and the whole
//!   question of demand-driven invalidation is whether a path can find that out
//!   without walking the program.
//! - a **type-changing** edit replaces the integer literal with a string
//!   literal. The head's bound type moves, the reader below it fails its
//!   ascription, and the failure carries to the end of the chain. The dirty set
//!   is the whole chain, and both paths must do that work.
//!
//! Neither edit changes the item list, so both are pure content edits: the
//! alignment and order-splicing machinery is not what is being measured here.

use alloc::rc::Rc;

use gandr_core_incremental::region::Item;
use gandr_core_incremental::region::Program;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::ValueType;

use crate::boundary::BlockCount;
use crate::boundary::BlockLength;
use crate::boundary::DefinitionKey;
use crate::boundary::ItemCount;
use crate::boundary::LiteralValue;
use crate::boundary::SlotIndex;

/// The kind of single-item edit a measured recheck applies.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditKind
{
    /// Replaces an integer literal with a different integer literal: the item's
    /// content changes and its type does not.
    ValueOnly,
    /// Replaces an integer literal with a string literal: the item's bound type
    /// moves, and every reader below it in its chain fails.
    TypeChanging,
}

/// A generated program: independent chains of ascription-linked definitions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Workload
{
    /// The number of independent chains.
    blocks: BlockCount,
    /// The number of items in each chain.
    block_length: BlockLength,
}

impl Workload
{
    /// Describes a workload of `blocks` chains of `block_length` items.
    ///
    /// # Contract
    /// - requires: both dimensions are non-zero for the generated program to
    ///   have any items; a zero dimension yields an empty program rather than a
    ///   failure.
    /// - ensures: returns the description; no program is built until
    ///   [`Self::program`] is called.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        blocks: BlockCount,
        block_length: BlockLength,
    ) -> Self
    {
        Self {
            blocks,
            block_length,
        }
    }

    /// The number of items the generated program has.
    ///
    /// # Contract
    /// - ensures: returns `blocks × block_length`, saturating rather than
    ///   overflowing.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn item_count(&self) -> ItemCount
    {
        let blocks: usize = self.blocks.into();
        let block_length: usize = self.block_length.into();
        ItemCount::from(blocks.saturating_mul(block_length))
    }

    /// The chain length, which is the dirty-set size of a type-changing edit at
    /// a chain head.
    #[inline]
    #[must_use]
    pub fn block_length(&self) -> BlockLength
    {
        self.block_length
    }

    /// The number of items a recheck must re-type after `edit` is applied at a
    /// chain head.
    ///
    /// This is the quantity the runner asserts against, so it is derived from
    /// the workload's construction rather than read off a run.
    ///
    /// # Contract
    /// - ensures: returns one item for a value-only edit — the edited item
    ///   itself, whose readers are type-stable — and the whole chain for a
    ///   type-changing edit.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn dirty_items(
        &self,
        edit: EditKind,
    ) -> ItemCount
    {
        match edit {
            | EditKind::ValueOnly => ItemCount::from(1_usize),
            | EditKind::TypeChanging => ItemCount::from(usize::from(self.block_length)),
        }
    }

    /// The slot holding the head of the middle chain — the single-item edit
    /// target, chosen away from both ends so no fast path is reached by
    /// accident.
    ///
    /// The append-only resume fast path fires only for an exact append, and an
    /// edit in the middle of a program is neither an append nor a prefix
    /// change; picking the middle keeps the measurement on the general path
    /// deliberately rather than by luck.
    ///
    /// # Contract
    /// - ensures: returns the slot of a chain head inside the program, or
    ///   [`None`] when the workload generates no items.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn middle_block_head(&self) -> Option<SlotIndex>
    {
        let blocks: usize = self.blocks.into();
        let block_length: usize = self.block_length.into();
        if blocks == 0 || block_length == 0 {
            return None;
        }
        let middle = blocks.checked_div(2)?;
        let slot = middle.checked_mul(block_length)?;
        Some(SlotIndex::from(slot))
    }

    /// Builds the program this workload describes.
    ///
    /// # Contract
    /// - ensures: returns `blocks × block_length` items in source order, each
    ///   with a distinct definition name; item *j* of a chain reads item *j−1*
    ///   of the same chain, and chain heads read nothing.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: the generator must produce distinct names, chain-local
    ///   reads, and exactly one head per chain; a workload violating any of
    ///   these would make the asserted dirty sets wrong without any measurement
    ///   looking different.
    /// - witness: `support::chain_heads_are_literals_and_readers_are_linked`
    /// - witness: `support::generated_names_are_distinct`
    #[inline]
    #[must_use]
    pub fn program(&self) -> Program
    {
        let blocks: usize = self.blocks.into();
        let block_length: usize = self.block_length.into();
        let mut items: Vec<Item> = Vec::with_capacity(blocks.saturating_mul(block_length));
        for block in 0 .. blocks {
            let Some(base) = block.checked_mul(block_length)
            else {
                continue;
            };
            for offset in 0 .. block_length {
                let Some(index) = base.checked_add(offset)
                else {
                    continue;
                };
                let Ok(literal) = i64::try_from(index)
                else {
                    continue;
                };
                let name = definition_name(SlotIndex::from(index));
                let term = if offset == 0 {
                    head_term(LiteralValue::from(literal))
                }
                else {
                    let Some(previous) = index.checked_sub(1)
                    else {
                        continue;
                    };
                    reader_term(&DefinitionKey::from(definition_name(SlotIndex::from(
                        previous,
                    ))))
                };
                items.push(Item::new(Some(name), None, term));
            }
        }
        Program::new(items)
    }
}

/// Applies `edit` to the item at `slot`, returning the edited program.
///
/// The item list is untouched: only the term at `slot` is replaced, so the
/// edited program has the same length, the same names, and the same order.
///
/// # Contract
/// - ensures: returns the program with exactly one item's term replaced,
///   preserving that item's name and ascription.
/// - fails: returns [`None`] when `slot` is out of range for `program`, or when
///   the slot's index has no integer literal representation.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: a value-only edit must change the term and leave the inferred
///   type alone, while a type-changing edit must move it; an edit that did
///   neither would make both paths trivially agree.
/// - witness: `support::value_only_edit_preserves_the_bound_type`
/// - witness: `support::type_changing_edit_moves_the_bound_type`
#[inline]
#[must_use]
pub fn apply_edit(
    program: &Program,
    slot: SlotIndex,
    edit: EditKind,
) -> Option<Program>
{
    let index: usize = slot.into();
    let original = program.items.get(index)?;
    let term = match edit {
        | EditKind::ValueOnly => {
            let literal = i64::try_from(index).ok()?;
            head_term(LiteralValue::from(literal.saturating_add(1_i64)))
        },
        | EditKind::TypeChanging => Term::Value(Value::string("retyped")),
    };
    let mut items = program.items.clone();
    let target = items.get_mut(index)?;
    *target = Item::new(original.name.clone(), original.ascription.clone(), term);
    Some(Program::new(items))
}

/// The definition name of the item in `slot`.
fn definition_name(slot: SlotIndex) -> String
{
    let index: usize = slot.into();
    format!("d{index}")
}

/// A chain head's term: a bare integer literal, which infers the rigid integer
/// atom.
fn head_term(literal: LiteralValue) -> Term
{
    let literal: i64 = literal.into();
    Term::Value(Value::int(literal))
}

/// A chain reader's term: the previous definition under an integer ascription,
/// so the item types iff its referent is still an integer.
fn reader_term(previous: &DefinitionKey) -> Term
{
    let previous: &str = previous.as_ref();
    Term::Value(Value::Annot(
        Rc::new(Value::var(previous)),
        Rc::new(ValueType::integer()),
    ))
}
