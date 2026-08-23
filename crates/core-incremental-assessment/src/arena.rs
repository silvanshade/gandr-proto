//! The out-of-database item store the query graph reads through.
//!
//! # Why the items cannot live in the database
//!
//! gandr's core terms are reference-counted ([`std::rc::Rc`]) and therefore
//! neither [`Send`] nor [`Sync`]. The engine requires both of every value it
//! retains, and requires its database to be [`Send`]. An [`Item`] consequently
//! cannot be a database field, an input field, or a query result — not as a
//! matter of configuration, but structurally.
//!
//! So the items live beside the database in a thread-local store, and what
//! enters the database is each item's **content address**. This is the engine's
//! own documented on-demand-input pattern, in which the real data sits outside
//! the database behind an accessor and only a change-detectable summary is
//! tracked; the adaptation here is that the accessor must be thread-local
//! rather than a database field, for the reason above.
//!
//! # The invariant, and why a query checks it
//!
//! Reading the store inside a query is sound only because the store's content
//! is a function of the digest the query already read: an item changes exactly
//! when its digest changes. If that ever stopped holding, the engine would
//! reuse a memo whose inputs had moved underneath it, and every measurement
//! taken afterwards would be meaningless while still looking entirely
//! plausible. The typing query therefore re-derives the digest of the item it
//! fetched and compares it against the one its input carries, so the invariant
//! is checked on every execution rather than assumed.

use std::cell::RefCell;

use gandr_core_incremental::persistence::address_of;
use gandr_core_incremental::region::Item;
use gandr_core_incremental::region::Program;

use crate::boundary::ItemDigest;
use crate::boundary::SlotIndex;
use crate::ledger::AssessmentError;

thread_local! {
    /// The items of the installed revision, indexed by slot.
    static INSTALLED: RefCell<Vec<Item>> = const { RefCell::new(Vec::new()) };
}

/// The accessor for the thread-local item store.
///
/// A namespace rather than a value: the store is process state the query graph
/// reaches by slot, and giving it an owner handle would suggest more than one
/// could exist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ItemStore;

impl ItemStore
{
    /// Installs `program`'s items as the revision the query graph reads.
    ///
    /// # Contract
    /// - ensures: slot *i* resolves to `program`'s item *i* until the next
    ///   install.
    /// - fails: returns [`AssessmentError::StoreUnavailable`] when the store is
    ///   already borrowed.
    /// - panics: none — the borrow is fallible rather than asserted.
    ///
    /// # Errors
    /// Returns [`AssessmentError::StoreUnavailable`] when the store is already
    /// borrowed by another reader on this thread.
    #[inline]
    pub fn install(program: &Program) -> Result<(), AssessmentError>
    {
        INSTALLED.with(|installed| {
            let mut installed = installed
                .try_borrow_mut()
                .map_err(|_error| AssessmentError::StoreUnavailable)?;
            installed.clear();
            installed.extend(program.items.iter().cloned());
            Ok(())
        })
    }

    /// Empties the store.
    ///
    /// # Contract
    /// - ensures: every slot resolves to [`AssessmentError::MissingSlot`]
    ///   afterwards.
    /// - fails: returns [`AssessmentError::StoreUnavailable`] when the store is
    ///   already borrowed.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`AssessmentError::StoreUnavailable`] when the store is already
    /// borrowed by another reader on this thread.
    #[inline]
    pub fn clear() -> Result<(), AssessmentError>
    {
        INSTALLED.with(|installed| {
            let mut installed = installed
                .try_borrow_mut()
                .map_err(|_error| AssessmentError::StoreUnavailable)?;
            installed.clear();
            Ok(())
        })
    }

    /// The item installed at `slot`.
    ///
    /// Returns an owned item because the store is behind a dynamic borrow and a
    /// query holds its item across further query calls; the clone is shallow,
    /// since a term's children are reference-counted.
    ///
    /// # Contract
    /// - ensures: returns the installed item at `slot`.
    /// - fails: returns [`AssessmentError::MissingSlot`] when no item is
    ///   installed there, and [`AssessmentError::StoreUnavailable`] when the
    ///   store is already mutably borrowed.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`AssessmentError::MissingSlot`] when no item is installed at
    /// `slot`, and [`AssessmentError::StoreUnavailable`] when the store is
    /// mutably borrowed.
    #[inline]
    pub fn fetch(slot: SlotIndex) -> Result<Item, AssessmentError>
    {
        INSTALLED.with(|installed| {
            let installed = installed
                .try_borrow()
                .map_err(|_error| AssessmentError::StoreUnavailable)?;
            let index: usize = slot.into();
            let item = installed
                .get(index)
                .ok_or(AssessmentError::MissingSlot(slot))?;
            Ok(item.clone())
        })
    }

    /// The content address of one item, computed through the checkpoint tier's
    /// own program addressing.
    ///
    /// # Contract
    /// - ensures: returns the address of the single-item program holding
    ///   `item`, so equal digests mean structurally equal items.
    /// - fails: returns [`AssessmentError::Boundary`] when the item carries a
    ///   form the codec has no process-independent representation for.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: the digest must move under a content edit and hold still
    ///   otherwise, since it is the engine's only view of an item; a constant
    ///   digest would make every recheck report perfect reuse.
    /// - witness: `support::digest_moves_with_the_term`
    ///
    /// # Errors
    /// Returns [`AssessmentError::Boundary`] carrying the codec's account of
    /// the form it cannot represent process-independently.
    #[inline]
    pub fn digest(item: &Item) -> Result<ItemDigest, AssessmentError>
    {
        let program = Program::new(vec![item.clone()]);
        let address = address_of(&program)?;
        Ok(ItemDigest::from(address))
    }
}
