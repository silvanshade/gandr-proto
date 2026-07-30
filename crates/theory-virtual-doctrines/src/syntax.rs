//! The **reflected judgment layer** — `FVDblTT`'s protypes and proterms as data
//! the checker validates against a [`crate::CellStoreVdc`].
//!
//! Design reference: `../../../docs/gandr/spec/proposal-vdc-reflection.md`
//! §5.2; ADR-68 D3.
//!
//! Four judgment families, **first-order** — no dependent types anywhere (the
//! reflection face does not require levitation stage 1). [`Protype`]s are the
//! loose-arrow types (paths, relations, tensor products, extensions,
//! tabulators, and products); [`Proterm`]s are the cells inhabiting them, with
//! a [`Proterm::Cert`] constructor that **embeds an engine derivation** — so
//! everything the completion loop emits is already a term of the reflection
//! face, and every closed proterm elaborates back down to a derivation the
//! engine can replay.

use alloc::boxed::Box;

use gandr_theory_levitation::Name;
use gandr_theory_levitation::NameRef;

use crate::vdc::RelationRef;
use crate::vdc::SignatureRef;
use crate::vdc::TermRef;

/// A **proterm hypothesis variable** — a name bound in the seam-composable
/// hypothesis chain `Φ` (`proposal-vdc-reflection.md` §5.2).
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProVar(Name);

impl ProVar
{
    /// A proterm variable of the given name.
    #[inline]
    #[must_use]
    pub fn new(name: NameRef<'_>) -> Self
    {
        Self(Name::from(name))
    }

    /// The variable's name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> NameRef<'_>
    {
        NameRef::from(self.0.as_ref())
    }
}

/// A reference to an **embedded engine derivation** — an index into the
/// checking environment's derivation registry (`proposal-vdc-reflection.md`
/// §5.2, the `Cert` constructor).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DerivationId(crate::boundary::DerivationIndex);

impl DerivationId
{
    /// A derivation id from an environment index.
    #[inline]
    #[must_use]
    pub fn new(index: crate::boundary::DerivationIndex) -> Self
    {
        Self(index)
    }

    /// The environment index this id names.
    #[inline]
    #[must_use]
    pub fn index(self) -> crate::boundary::DerivationIndex
    {
        self.0
    }
}

impl From<crate::boundary::DerivationIndex> for DerivationId
{
    #[inline]
    fn from(value: crate::boundary::DerivationIndex) -> Self
    {
        Self::new(value)
    }
}

impl From<DerivationId> for crate::boundary::DerivationIndex
{
    #[inline]
    fn from(value: DerivationId) -> Self
    {
        value.0
    }
}

/// A **protype** — `FVDblTT`'s loose-arrow type, first-order
/// (`proposal-vdc-reflection.md` §5.2).
///
/// The constructor menu is the **query surface's specification** (§6.1): the
/// unit path protype, relation protypes, the ⊙ seam composite, the ⊲/⊳
/// extensions, the tabulator object-former hook, products, and the terminal
/// unit.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Protype
{
    /// The **path** protype `s ⤳ t` (the unit): a finite rewrite sequence
    /// between two terms of one signature; `refl` is the empty path.
    Path
    {
        /// The signature both endpoints inhabit.
        sig: SignatureRef,
        /// The left endpoint `s`.
        lhs: TermRef,
        /// The right endpoint `t`.
        rhs: TermRef,
    },
    /// A **relation** protype `R(s # t)`: the loose arrow `R` framed at the
    /// term pair `(s, t)`.
    Rel
    {
        /// The relation interface `R`.
        rel: RelationRef,
        /// The source-side framing term `s`.
        lhs: TermRef,
        /// The target-side framing term `t`.
        rhs: TermRef,
    },
    /// The **⊙ seam composite** `l ⊙ r` at a mid-signature (∃-shaped): exists
    /// only as the overlap-indexed family (the *virtual* weakening).
    Compose
    {
        /// The left protype.
        l: Box<Self>,
        /// The signature the seam is taken over.
        mid: SignatureRef,
        /// The right protype.
        r: Box<Self>,
    },
    /// The **right extension** `dom ⊲ cod` (Kan-shaped): the best consumer
    /// completing the seam.
    ExtendR
    {
        /// The domain protype.
        dom: Box<Self>,
        /// The codomain protype.
        cod: Box<Self>,
    },
    /// The **left extension** `cod ⊳ dom` (Kan-shaped).
    ExtendL
    {
        /// The codomain protype.
        cod: Box<Self>,
        /// The domain protype.
        dom: Box<Self>,
    },
    /// The **tabulator** `{|R|}` — the relation's instantiation table as an
    /// object-former hook.
    Tabulate
    {
        /// The relation whose instances are tabulated.
        rel: RelationRef,
    },
    /// The **product** of two protypes with pointwise projections.
    Product(Box<Self>, Box<Self>),
    /// The terminal **unit** protype.
    Unit,
}

/// A **proterm** — a cell inhabiting a protype, first-order
/// (`proposal-vdc-reflection.md` §5.2).
///
/// The [`Proterm::Cert`] constructor embeds an engine certificate; the rest are
/// the introduction/elimination forms of the constructor menu, checked
/// bidirectionally over two-sided contexts ([`crate::check`]).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Proterm
{
    /// A hypothesis **variable** from the chain `Φ`.
    Var(ProVar),
    /// **refl** — the empty rewrite path at a term (the unit introduction).
    Refl
    {
        /// The signature the term inhabits.
        sig: SignatureRef,
        /// The term the empty path is at.
        term: TermRef,
    },
    /// **Path induction** — eliminate a path by its value on `refl` (the unit's
    /// universal property).
    PathInd
    {
        /// The motive protype the induction targets.
        motive: Box<Protype>,
        /// The base case (the value on `refl`).
        base: Box<Self>,
        /// The path being inducted on.
        scrut: Box<Self>,
    },
    /// **⊙-introduction** — a pair witnessing the seam composite at a chosen
    /// mid-term.
    Pair
    {
        /// The left component.
        l: Box<Self>,
        /// The mid-term the seam is witnessed at.
        via: TermRef,
        /// The right component.
        r: Box<Self>,
    },
    /// **⊙-elimination** — seam induction (case on a seam composite).
    SeamInd
    {
        /// The seam composite being eliminated.
        scrut: Box<Self>,
        /// The arm, parameterized by the two components.
        arm: Box<Self>,
    },
    /// **⊲/⊳-introduction** — abstract over a hypothesis (extension intro).
    Lam
    {
        /// The hypothesis abstracted.
        hyp: ProVar,
        /// The body.
        body: Box<Self>,
    },
    /// **⊲/⊳-elimination** — apply an extension to an argument (checked on the
    /// variance side).
    App
    {
        /// The extension being applied.
        f: Box<Self>,
        /// The argument.
        arg: Box<Self>,
    },
    /// **Product introduction** — a pair inhabiting a product protype.
    ProdIntro
    {
        /// The left component.
        l: Box<Self>,
        /// The right component.
        r: Box<Self>,
    },
    /// **Left projection** of a product proterm.
    ProjL(Box<Self>),
    /// **Right projection** of a product proterm.
    ProjR(Box<Self>),
    /// The **unit** proterm — the sole inhabitant of [`Protype::Unit`].
    UnitTerm,
    /// An **embedded engine certificate** — the reflection of an engine
    /// derivation as a proterm.
    Cert(DerivationId),
}
