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
use crate::normal_form::canonicalize;

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
    pub fn intern(
        &mut self,
        diagram: &Wiring,
    ) -> Interned
    {
        let canonicalization = canonicalize(diagram);
        let form = canonicalization.form().clone();
        let relabelling = canonicalization.relabelling().clone();
        if let Some(&id) = self.ids.get(&form) {
            return Interned {
                id,
                relabelling,
                arrival: Arrival::Held,
            };
        }
        // First arrival: the identifier is the table's current length, which
        // is what makes it an index into `forms` rather than a digest.
        let id = DiagramId(u32::try_from(self.forms.len()).unwrap_or(u32::MAX));
        self.forms.push(form.clone());
        let _absent = self.ids.insert(form, id);
        return Interned {
            id,
            relabelling,
            arrival: Arrival::Fresh,
        };
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
    use alloc::vec::Vec;

    use super::Arrival;
    use super::DiagramInterner;
    use crate::interface::Generator;
    use crate::interface::GeneratorLabel;
    use crate::interface::GeneratorSort;
    use crate::interface::Interface;
    use crate::interface::Wire;
    use crate::interface::WireCount;
    use crate::interface::Wiring;

    /// The name of a fixture generator.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct FixtureName<'name>(&'name str);

    /// A value-sorted generator label.
    fn value(name: FixtureName<'_>) -> GeneratorLabel
    {
        return GeneratorLabel::new(name.0, GeneratorSort::Value);
    }

    /// A two-step diagram: `f` then `g`, boundary in at `f`, out at `g`.
    fn spine() -> Wiring
    {
        return Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value(FixtureName("f")), [Wire(0)], [Wire(1)]),
                Generator::new(value(FixtureName("g")), [Wire(1)], [Wire(2)]),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        )
        .expect("the two-step diagram is well formed");
    }

    /// The SAME diagram, presented with its hyperedges in the other order and
    /// its wires renumbered.
    fn spine_permuted() -> Wiring
    {
        return Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value(FixtureName("g")), [Wire(0)], [Wire(1)]),
                Generator::new(value(FixtureName("f")), [Wire(2)], [Wire(0)]),
            ],
            Interface::new([Wire(2)], [Wire(1)]),
        )
        .expect("a renumbering of the two-step diagram is still one");
    }

    /// A different diagram: the same generators wired the other way round.
    fn reversed() -> Wiring
    {
        return Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value(FixtureName("g")), [Wire(0)], [Wire(1)]),
                Generator::new(value(FixtureName("f")), [Wire(1)], [Wire(2)]),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        )
        .expect("the reversed diagram is well formed");
    }

    /// Two unequal `Wiring` values with one canonical form receive one
    /// identifier, and the second arrival is reported as held.
    ///
    /// **The separating witness of this module.** An interner keyed on
    /// `Wiring`'s own presentation equality compiles, is deterministic, and
    /// passes every obvious test — intern one value twice, get one identifier;
    /// intern two different diagrams, get two. It fails only here, on two
    /// spellings of one diagram, which is the case interning exists to
    /// collapse and the case a careless suite never constructs.
    #[test]
    fn two_presentations_of_one_diagram_intern_alike()
    {
        let (left, right) = (spine(), spine_permuted());
        assert_ne!(
            left, right,
            "the two presentations are unequal as wirings, or this separates nothing"
        );

        let mut table = DiagramInterner::new();
        let first = table.intern(&left);
        let second = table.intern(&right);

        assert_eq!(
            first.id(),
            second.id(),
            "one diagram, one identifier, whatever it was spelled like"
        );
        assert_eq!(
            Arrival::Fresh,
            first.arrival(),
            "the first arrival is fresh"
        );
        assert_eq!(
            Arrival::Held,
            second.arrival(),
            "the second presentation finds the diagram already held"
        );
        assert_eq!(
            1_usize,
            usize::from(table.diagram_count()),
            "a table of diagrams holds one, where a table of spellings holds two"
        );
        assert_ne!(
            first.relabelling(),
            second.relabelling(),
            "the two witnesses differ, so this is two presentations and not one input twice"
        );
    }

    /// Diagrams that the canon separates receive different identifiers.
    #[test]
    fn distinct_diagrams_take_distinct_identifiers()
    {
        let mut table = DiagramInterner::new();
        let family = alloc::vec![spine(), reversed()];
        let ids: Vec<_> = family.iter().map(|w| table.intern(w).id()).collect();
        assert_ne!(
            ids.first(),
            ids.get(1_usize),
            "the table does not collapse what the canon separates"
        );
        assert_eq!(
            family.len(),
            usize::from(table.diagram_count()),
            "one identifier per distinct diagram"
        );
    }

    /// The returned relabelling really carries the caller's presentation onto
    /// the interned form.
    ///
    /// The witness exists so a caller can check the identity it was handed
    /// rather than trust it, so the witness itself has to be checkable.
    #[test]
    fn the_relabelling_carries_the_presentation_to_the_form()
    {
        let diagram = spine_permuted();
        let mut table = DiagramInterner::new();
        let interned = table.intern(&diagram);
        let form = table
            .form(interned.id())
            .expect("the identifier was issued");
        interned
            .relabelling()
            .verify(&diagram, form)
            .expect("the returned witness carries the presentation onto the held form");
    }
}
