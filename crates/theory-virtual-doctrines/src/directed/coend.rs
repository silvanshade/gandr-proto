//! **Ł3 — (co)ends as quantifiers** over reflected signatures, with Fubini and
//! (co)Yoneda as **derived** transformations (`proposal-vdc-reflection.md` §7,
//! Ł3; ADR-68/69).
//!
//! LLV reads `∫` binders as quantifiers over a signature's objects. This module
//! reflects that reading over **finite** reflected signatures: a [`Diagram`] is
//! a finite functor (its carrier the objects the binder ranges over), an
//! [`End`] its product-shaped universal quantifier `∫_x F(x, x)` and a
//! [`Coend`] its coproduct-shaped existential `∫^x F(x, x)`. Fubini
//! ([`fubini_swap`]) and (co)Yoneda ([`coyoneda_collapse`]) are **derived** —
//! not primitive — as the LLV development derives them, realized here as
//! checked transformations with property tests over finite carriers.
//!
//! # The finite / discrete boundary (theorem-grade generality → Agda)
//!
//! Two honesty boundaries, stated precisely:
//!
//! - **Finite carriers.** The (co)end is the literal product / coproduct over a
//!   finite carrier; the theorem-grade statement (arbitrary reflected
//!   signatures, the dinaturality **wedge quotient** in full) rides the Agda
//!   face.
//! - **Discrete (refl-generated) hom.** [`coyoneda_collapse`] collapses the
//!   density coend `∫^x hom(a, x) × F(x)` to `F(a)` because at this rung the
//!   reflected signature's hom is refl-generated ([`discrete_hom_inhabited`]:
//!   `hom(a, x)` is inhabited only at `x == a`), so every off-diagonal summand
//!   is empty. The general (non-discrete) co-Yoneda, where non-identity
//!   morphisms carry summands the wedge quotient then identifies, is Agda-face
//!   territory. Cut remains restricted (the §4.3 gate is its operational face).

use alloc::vec::Vec;

use crate::boundary::DiagramCarrierEmptyStatus;
use crate::boundary::DiagramCarrierLength;
use crate::boundary::DiscreteHomInhabitation;
use crate::vdc::TermRef;

/// A **finite diagram** `F` over a reflected signature's carrier
/// (`proposal-vdc-reflection.md` §7, Ł3).
///
/// The functor an `∫` binder ranges over, reflected as an explicit map from
/// carrier objects to their diagonal components `x ↦ F(x, x)`. The carrier is
/// finite and enumerated; theorem-grade generality (arbitrary reflected
/// signatures) rides the Agda face.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Diagram
{
    /// The `(object, F(object))` entries, one per carrier object, in order.
    pub entries: Vec<(TermRef, TermRef)>,
}

impl Diagram
{
    /// A diagram over the given `(object, component)` entries.
    #[inline]
    #[must_use]
    pub fn new(entries: Vec<(TermRef, TermRef)>) -> Self
    {
        Self { entries }
    }

    /// The component `F(object)` at a carrier object, or `None` when the object
    /// is off the carrier.
    ///
    /// # Contract
    /// - ensures: the component of the **first** `(object, _)` entry, or
    ///   `None`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn component(
        &self,
        object: &TermRef,
    ) -> Option<&TermRef>
    {
        for entry in &self.entries {
            if entry.0 == *object {
                return Some(&entry.1);
            }
        }
        None
    }

    /// The number of carrier objects.
    #[inline]
    #[must_use]
    pub fn len(&self) -> DiagramCarrierLength
    {
        DiagramCarrierLength::from(self.entries.len())
    }

    /// Whether the carrier is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> DiagramCarrierEmptyStatus
    {
        DiagramCarrierEmptyStatus::from(self.entries.is_empty())
    }
}

/// The **end** `∫_x F(x, x)` over a finite discrete carrier — the universal
/// quantifier, realized as the product of the diagram's diagonal components
/// (`proposal-vdc-reflection.md` §7, Ł3).
///
/// The wedge condition is vacuous over a discrete (refl-generated) carrier — no
/// non-identity morphisms constrain the components — so the end is the full
/// product.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct End
{
    /// The product components `F(x, x)`, one per carrier object.
    pub components: Vec<TermRef>,
}

impl End
{
    /// The end of `diagram` — its diagonal components as a product.
    ///
    /// # Contract
    /// - ensures: `components` are the diagram's entry components, in carrier
    ///   order (the discrete-carrier product `Π_x F(x, x)`).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn of(diagram: &Diagram) -> Self
    {
        let mut components = Vec::with_capacity(diagram.entries.len());
        for entry in &diagram.entries {
            components.push(entry.1.clone());
        }
        Self { components }
    }
}

/// The **coend** `∫^x F(x, x)` over a finite discrete carrier — the existential
/// quantifier, realized as the coproduct of the diagram's diagonal components
/// (`proposal-vdc-reflection.md` §7, Ł3).
///
/// The cowedge quotient is trivial over a discrete carrier, so the coend is the
/// full coproduct.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Coend
{
    /// The coproduct summands `F(x, x)`, one per carrier object.
    pub summands: Vec<TermRef>,
}

impl Coend
{
    /// The coend of `diagram` — its diagonal components as a coproduct.
    ///
    /// # Contract
    /// - ensures: `summands` are the diagram's entry components, in carrier
    ///   order (the discrete-carrier coproduct `Σ_x F(x, x)`).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn of(diagram: &Diagram) -> Self
    {
        let mut summands = Vec::with_capacity(diagram.entries.len());
        for entry in &diagram.entries {
            summands.push(entry.1.clone());
        }
        Self { summands }
    }
}

/// A **finite two-variable diagram** `F(x, y)` over a product carrier `Xs × Ys`
/// — the shape a double end `∫_x ∫_y F` quantifies
/// (`proposal-vdc-reflection.md` §7, Ł3).
///
/// Reflected as an explicit map from `(x, y)` index pairs to payloads; the
/// carrier is finite and enumerated.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct BiDiagram
{
    /// The `((x, y), F(x, y))` entries, one per index pair.
    pub entries: Vec<((TermRef, TermRef), TermRef)>,
}

impl BiDiagram
{
    /// A two-variable diagram over the given `((x, y), payload)` entries.
    #[inline]
    #[must_use]
    pub fn new(entries: Vec<((TermRef, TermRef), TermRef)>) -> Self
    {
        Self { entries }
    }

    /// The payloads of the iterated end `∫_x ∫_y F(x, y)` over the discrete
    /// finite carrier — the product components, order-independent.
    ///
    /// # Contract
    /// - ensures: one payload per entry, in entry order; the multiset is the
    ///   iterated-end value that Fubini preserves.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn payloads(&self) -> Vec<TermRef>
    {
        let mut payloads = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            payloads.push(entry.1.clone());
        }
        payloads
    }

    /// The index pairs of the carrier, in entry order.
    ///
    /// # Contract
    /// - ensures: the `(x, y)` key of each entry, in order.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn index_pairs(&self) -> Vec<(TermRef, TermRef)>
    {
        let mut indices = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            indices.push(entry.0.clone());
        }
        indices
    }
}

/// **Fubini** — swap the two `∫` binders (`proposal-vdc-reflection.md` §7, Ł3).
///
/// `∫_x ∫_y F(x, y) ≅ ∫_y ∫_x F(x, y)`. Over the finite carrier the isomorphism
/// is the **transpose bijection** on the index set: each `(x, y)` becomes
/// `(y, x)` with the payload carried unchanged. It is derived, not primitive —
/// the reindexing that witnesses the double end is order-independent — and is
/// its own inverse.
///
/// # Contract
/// - ensures: a [`BiDiagram`] whose entries are `((y, x), F(x, y))` for each
///   `((x, y), F(x, y))` of `f`; the payload multiset ([`BiDiagram::payloads`])
///   is preserved and `fubini_swap(fubini_swap(f)) == f` (an involution).
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise + property — over generated finite bidiagrams the
///   swap is asserted an involution, the index set exactly transposed, and the
///   iterated-end payload multiset preserved (the finite content of Fubini).
/// - witness: `directed::tests::fubini_swap_is_an_involution`
/// - witness: `directed::tests::fubini_preserves_the_iterated_end`
#[inline]
#[must_use]
pub fn fubini_swap(f: &BiDiagram) -> BiDiagram
{
    BiDiagram {
        entries: f
            .entries
            .iter()
            .map(|entry| {
                let ((ref x, ref y), ref payload) = *entry;
                ((y.clone(), x.clone()), payload.clone())
            })
            .collect(),
    }
}

/// **co-Yoneda** — collapse the density coend `∫^x hom(a, x) × F(x)` to `F(a)`
/// (`proposal-vdc-reflection.md` §7, Ł3).
///
/// Over the discrete (refl-generated) reflected signature, every off-diagonal
/// summand of the coend is empty ([`discrete_hom_inhabited`]), so the coend
/// collapses to the single `x == a` summand `hom(a, a) × F(a) ≅ F(a)`. This is
/// the derived co-Yoneda (density) transformation at the finite/discrete rung;
/// the general statement rides the Agda face.
///
/// # Contract
/// - ensures: `Some(F(a))` iff `a` is on `diagram`'s carrier (the surviving
///   diagonal summand); `None` when `a` is off the carrier (an empty coend).
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise + property — over generated finite diagrams the
///   collapse reproduces `F(a)` for every carrier object `a`, and is `None`
///   off-carrier; the boundary is that only the diagonal summand survives.
/// - witness: `directed::tests::coyoneda_collapses_to_the_diagonal_component`
/// - witness: `directed::tests::coyoneda_is_none_off_carrier`
#[inline]
#[must_use]
pub fn coyoneda_collapse(
    a: &TermRef,
    diagram: &Diagram,
) -> Option<TermRef>
{
    for entry in &diagram.entries {
        if bool::from(discrete_hom_inhabited(a, &entry.0)) {
            return Some(entry.1.clone());
        }
    }
    None
}

/// Whether the **discrete** reflected hom `hom(a, x)` is inhabited — at this
/// rung the hom is refl-generated, so it is inhabited exactly on the diagonal
/// (`proposal-vdc-reflection.md` §7, Ł3).
///
/// # Contract
/// - ensures: `true` iff `a == x` (only `refl` inhabits the hom at this rung);
///   the general non-discrete hom is Agda-face territory.
/// - panics: none.
#[inline]
#[must_use]
pub fn discrete_hom_inhabited(
    a: &TermRef,
    x: &TermRef,
) -> DiscreteHomInhabitation
{
    DiscreteHomInhabitation::from(a == x)
}
