//! Interning on the canon — the consumer the normal form was built for.
//!
//! [`crate::normal_form`] decides when two circuit terms denote the same
//! diagram. This module is what that decision is *for*: a table that gives one
//! identifier to one diagram, so that a consumer holding two presentations of
//! the same thing holds one name for it.
//!
//! # The whole point is which equality the table keys on
//!
//! [`crate::interface::Wiring`] has an `Eq`, and it is **presentation
//! identity**: two wirings that draw the same diagram with different wire
//! numbers are unequal under it. [`CanonicalDiagram`] has an `Eq`, and it is
//! **diagram identity**. An interner keyed on the first is a table of
//! spellings; an interner keyed on the second is a table of diagrams.
//!
//! Both compile, both are deterministic, and both satisfy the obvious test —
//! intern the same value twice, get the same identifier. The difference shows
//! only when two *different presentations of one diagram* arrive, which is the
//! case interning exists to collapse and the case a careless test never
//! constructs. So the separating witness in this module's suite is exactly
//! that case, and it is the one that must not be weakened.
//!
//! # Interning returns its witness
//!
//! [`DiagramInterner::intern`] returns the identifier **and** the relabelling
//! that carried the caller's presentation onto the interned form. That is the
//! same discipline [`crate::normal_form::same_diagram`] follows on its
//! affirmative arm, and for the same reason: a caller that receives a bare
//! "these are the same" holds an equality it cannot check, and an unguarded
//! equality shortcut is what the engines' standing warning forbids. A caller
//! that receives the relabelling can verify the claim it was handed.
//!
//! # What this does not do
//!
//! It does not canonicalize construction terms, and it does not discharge the
//! metatheory's `Rigid.canon-sound` at the circuit rung — that obligation is
//! over the carrier's merger-and-contraction terms and is a different object,
//! as [`crate::normal_form`] already records. This table interns *diagrams*.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::interface::Wiring;
use crate::normal_form::CanonicalDiagram;
use crate::normal_form::Relabelling;

/// The identifier one diagram is interned under.
///
/// Assigned in first-arrival order, which makes it a *table* index and not a
/// content address: two interners fed the same diagrams in different orders
/// assign different identifiers. That is intentional and is the difference
/// between this and a digest — an identifier here is meaningful against the
/// table that issued it, and nothing else.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DiagramId(u32);

impl From<u32> for DiagramId
{
    #[inline]
    fn from(id: u32) -> Self
    {
        return Self(id);
    }
}

impl From<DiagramId> for u32
{
    #[inline]
    fn from(id: DiagramId) -> Self
    {
        return id.0;
    }
}

/// How many distinct diagrams a table holds.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DiagramCount(usize);

impl From<usize> for DiagramCount
{
    #[inline]
    fn from(count: usize) -> Self
    {
        return Self(count);
    }
}

impl From<DiagramCount> for usize
{
    #[inline]
    fn from(count: DiagramCount) -> Self
    {
        return count.0;
    }
}

/// Whether an interning call added the diagram or found it already held.
///
/// A named verdict rather than a boolean because it is the observable that
/// separates a diagram table from a spelling table, and a bare `bool` at that
/// position reads as an implementation detail rather than as the claim.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Arrival
{
    /// This call added the diagram to the table.
    Fresh,
    /// The table already held this diagram, under some other presentation or
    /// this one.
    Held,
}

/// The result of interning one presentation: its identifier and its witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Interned
{
    /// The identifier the diagram is held under.
    id: DiagramId,
    /// The renumbering carrying the caller's presentation onto the form.
    relabelling: Relabelling,
    /// Whether this call is what added the diagram to the table.
    arrival: Arrival,
}

impl Interned
{
    /// Returns the identifier the diagram is held under.
    #[inline]
    #[must_use]
    pub const fn id(&self) -> DiagramId
    {
        return self.id;
    }

    /// Returns the renumbering onto the interned form.
    ///
    /// The witness is returned rather than consumed so a caller can check the
    /// identity it was handed rather than trust it.
    #[inline]
    #[must_use]
    pub const fn relabelling(&self) -> &Relabelling
    {
        return &self.relabelling;
    }

    /// Returns whether this call added the diagram rather than finding it.
    ///
    /// This is the observable that separates a diagram table from a spelling
    /// table: interning a second presentation of a diagram already held must
    /// report [`Arrival::Held`].
    #[inline]
    #[must_use]
    pub const fn arrival(&self) -> Arrival
    {
        return self.arrival;
    }
}

/// A table giving one identifier to one diagram.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagramInterner
{
    /// Canonical forms to their identifiers, ordered so iteration is
    /// deterministic — hash-order iteration is denied workspace-wide.
    ids: BTreeMap<CanonicalDiagram, DiagramId>,
    /// Identifiers to their forms, indexed by identifier.
    forms: Vec<CanonicalDiagram>,
}

impl DiagramInterner
{
    /// Creates an empty table.
    #[inline]
    #[must_use]
    pub const fn new() -> Self
    {
        return Self {
            ids: BTreeMap::new(),
            forms: Vec::new(),
        };
    }

    /// Interns one presentation, returning its identifier and its witness.
    ///
    /// # Contract
    /// - requires: nothing; every wiring canonicalizes.
    /// - ensures: the returned identifier is the one already held for
    ///   `diagram`'s canonical form when the table holds it, and a fresh
    ///   identifier otherwise. **Two different presentations of one diagram
    ///   receive one identifier**, with different relabellings — that is the
    ///   whole obligation, and it is what distinguishes this from a table keyed
    ///   on presentation equality. Interning is idempotent on the identifier
    ///   and total.
    /// - provides: the consumer the diagram normal form was built for.
    /// - fails: never; a table cannot refuse a diagram.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the decision surface is "same diagram, same
    ///   identifier; different diagram, different identifier", and the case
    ///   that separates a diagram table from a spelling table is two unequal
    ///   `Wiring` values with one canonical form.
    /// - witness: `intern::tests::two_presentations_of_one_diagram_intern_alike`
    /// - witness: `intern::tests::distinct_diagrams_take_distinct_identifiers`
    /// - witness: `intern::tests::the_relabelling_carries_the_presentation_to_the_form`
    #[inline]
    #[expect(
        clippy::todo,
        reason = "gandr-ng9.6 scaffold: the interning body is the implementor deliverable"
    )]
    pub fn intern(
        &mut self,
        diagram: &Wiring,
    ) -> Interned
    {
        todo!("canonicalize {diagram:?}, look its form up in self.ids, assign on absence");
    }

    /// Returns the canonical form held under an identifier.
    ///
    /// # Contract
    /// - requires: nothing; an identifier this table never issued is refused by
    ///   returning [`None`] rather than by indexing.
    /// - ensures: `Some` exactly for identifiers this table issued.
    /// - provides: the read side of the table.
    /// - fails: [`None`] on an unknown identifier.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn form(
        &self,
        id: DiagramId,
    ) -> Option<&CanonicalDiagram>
    {
        return self.forms.get(usize::try_from(id.0).ok()?);
    }

    /// Returns how many distinct diagrams the table holds.
    #[inline]
    #[must_use]
    pub fn diagram_count(&self) -> DiagramCount
    {
        return DiagramCount::from(self.forms.len());
    }

    /// Returns the forms in identifier order.
    #[inline]
    #[must_use]
    pub fn forms(&self) -> Box<[CanonicalDiagram]>
    {
        return self.forms.clone().into_boxed_slice();
    }
}

#[cfg(test)]
mod tests
{
    /// Two unequal `Wiring` values with one canonical form receive one
    /// identifier, and the second arrival is reported as held.
    ///
    /// **The separating witness of this module.** An interner keyed on
    /// `Wiring`'s own presentation equality compiles, is deterministic, and
    /// passes every obvious test — intern one value twice, get one identifier;
    /// intern two different diagrams, get two. It fails only here, on two
    /// spellings of one diagram, which is the case interning exists to
    /// collapse and the case a careless suite never constructs. Weakening this
    /// test turns the table back into a table of spellings without changing a
    /// line of it.
    #[test]
    #[ignore = "gandr-ng9.6: awaits the interning body"]
    #[expect(
        clippy::todo,
        reason = "gandr-ng9.6 scaffold: the test body is the implementor deliverable"
    )]
    fn two_presentations_of_one_diagram_intern_alike()
    {
        todo!(
            "build two Wiring values that are unequal under Eq and canonicalize to one form,  intern both, assert one identifier and that the second reports Arrival::Held,  and assert the two relabellings differ so the case is not two equal inputs"
        );
    }

    /// Diagrams that the canon separates receive different identifiers.
    #[test]
    #[ignore = "gandr-ng9.6: awaits the interning body"]
    #[expect(
        clippy::todo,
        reason = "gandr-ng9.6 scaffold: the test body is the implementor deliverable"
    )]
    fn distinct_diagrams_take_distinct_identifiers()
    {
        todo!(
            "intern a family the canon distinguishes and assert the identifier count equals  the family size, so the table is not collapsing what the canon separates"
        );
    }

    /// The returned relabelling really carries the caller's presentation onto
    /// the interned form.
    ///
    /// The witness exists so a caller can check the identity it was handed
    /// rather than trust it, so the witness itself has to be checkable.
    #[test]
    #[ignore = "gandr-ng9.6: awaits the interning body"]
    #[expect(
        clippy::todo,
        reason = "gandr-ng9.6 scaffold: the test body is the implementor deliverable"
    )]
    fn the_relabelling_carries_the_presentation_to_the_form()
    {
        todo!(
            "intern a wiring, then verify the returned relabelling against the form the  table holds under the returned identifier"
        );
    }
}
