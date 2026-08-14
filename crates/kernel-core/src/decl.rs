//! The closed declaration vocabulary ([`Declaration`]): `Def` (a typed
//! definition), `Axiom` (a tracked typed hole), and `AbstractType` (a sealed
//! nominal atom), each carrying its own prenex [`LevelSignature`].
//!
//! Growth is deliberate and closed: at S1 the vocabulary is exactly these
//! three. `AbstractType` is the sealing rung's one addition, and it is a
//! *declaration* kind rather than a type former, so the frozen value-type
//! grammar gains no quantifier. Datatype declarations arrive later as levitated
//! description codes (S2), never as a raw-inductive kind (K4).
//!
//! **Declarations are value-polarity at the boundary** (a design decision for
//! the S1 slice): a `Def` pairs a declared *value* type with a *value* body,
//! an `Axiom` a declared value type, and an `AbstractType` a universe kind. A
//! computation definition `f : A → C` enters as a thunk — declared type `U (A →
//! C)`, body `thunk (λ. …)` — and is used through `force`. This keeps the
//! declaration vocabulary single-polarity (one `add_decl` shape, no
//! polarity-mismatch error) and matches CBPV's treatment of top-level bindings
//! as thunkable values.
//!
//! # Arena content and the builder (D1(C))
//!
//! A declaration's term/type content lives in the environment's
//! [`TermArena`], so [`DeclarationContent`] holds **root ids**
//! ([`ValueTypeId`] for the declared type, [`ValueId`] for a `Def` body). The
//! content is built through a [`DeclarationBuilder`] that borrows the arena and
//! records the **content-start** watermark; the finished [`Declaration`]
//! carries that watermark for the admission truncation discipline
//! ([`crate::arena`], [`Environment::add_decl`](crate::Environment::add_decl)).

use alloc::vec::Vec;

use gandr_kernel_strata::LandmarkConstraint;

use crate::arena::ArenaWatermark;
use crate::arena::TermArena;
use crate::arena::ValueId;
use crate::arena::ValueTypeId;
use crate::levels::LevelParamCount;
use crate::term::ConstantIndex;

/// A declaration's prenex level interface: its parameter count and its
/// declared landmark constraints (the declaration is generalized over these
/// and checked against nothing else).
#[derive(Clone, Debug)]
pub struct LevelSignature
{
    /// The prenex level-parameter count.
    params: LevelParamCount,
    /// The declared landmark constraints over those parameters.
    constraints: Vec<LandmarkConstraint>,
}

impl LevelSignature
{
    /// A signature with no level parameters and no constraints — the common
    /// monomorphic case.
    #[inline]
    #[must_use]
    pub fn monomorphic() -> Self
    {
        Self {
            params: LevelParamCount::from(0_u32),
            constraints: Vec::new(),
        }
    }

    /// A signature over `params` prenex level parameters with `constraints`.
    #[inline]
    #[must_use]
    pub fn new(
        params: LevelParamCount,
        constraints: Vec<LandmarkConstraint>,
    ) -> Self
    {
        Self {
            params,
            constraints,
        }
    }

    /// The prenex level-parameter count.
    #[inline]
    #[must_use]
    pub const fn params(&self) -> LevelParamCount
    {
        self.params
    }

    /// The declared landmark constraints, in declaration order.
    #[inline]
    #[must_use]
    pub fn constraints(&self) -> &[LandmarkConstraint]
    {
        &self.constraints
    }
}

/// The content of a declaration: a definition or an axiom, addressing its
/// term/type content by root id into the environment arena.
///
/// The derived equality is **child-id (same-arena) equality, not structural**
/// (the [`crate::term`] caveat); consuming code that needs structural agreement
/// across arenas re-encodes or walks explicitly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationContent
{
    /// A typed definition: a declared value-type root and a fully-elaborated
    /// value-body root, which the checker verifies against the type.
    Def
    {
        /// The declared value type's root id.
        declared: ValueTypeId,
        /// The fully-elaborated value body's root id.
        body: ValueId,
    },
    /// A tracked typed hole: a declared value type with no body. Everything
    /// resting on it is flagged by the choke-point audit.
    Axiom
    {
        /// The declared value type's root id.
        declared: ValueTypeId,
    },
    /// A **sealed abstract type**: a minted nominal atom, declared at a
    /// universe kind and with no unfolding rule.
    ///
    /// It is neither a definition nor a hole, and the distinction is the point.
    /// A [`Def`](Self::Def) carries a body the kernel must verify; an
    /// [`Axiom`](Self::Axiom) *claims an inhabitant* and is therefore tracked
    /// by the audit. An abstract type claims no inhabitant — it introduces an
    /// uninterpreted type constant, a conservative extension — so it is
    /// **not** audited as an axiom, and admitting one leaves the kernel with
    /// no obligation it did not already discharge at admission.
    ///
    /// `kind` must be a [`ValueType::Universe`](crate::ValueType::Universe)
    /// node; the choke point rejects anything else
    /// ([`KernelError::AbstractTypeKindNotUniverse`]), so every
    /// [`ValueType::Abstract`](crate::ValueType::Abstract) naming this
    /// declaration reads its universe level off a node whose shape was already
    /// checked rather than inferring one.
    ///
    /// [`KernelError::AbstractTypeKindNotUniverse`]: crate::KernelError::AbstractTypeKindNotUniverse
    AbstractType
    {
        /// The atom's kind: a universe value-type root id.
        kind: ValueTypeId,
    },
}

impl DeclarationContent
{
    /// The declared value-type root id — the declared type of a definition or
    /// an axiom, and the *kind* of an abstract type.
    ///
    /// The three share one accessor because they share one well-formedness
    /// obligation: whatever the root is, it must form. What differs is what
    /// admission additionally demands of it, and that stays in
    /// [`crate::check`].
    #[inline]
    #[must_use]
    pub const fn declared_id(&self) -> ValueTypeId
    {
        match *self {
            | Self::Def { declared, .. }
            | Self::Axiom { declared }
            | Self::AbstractType { kind: declared } => declared,
        }
    }
}

/// A declaration entering the kernel through the choke point: a level
/// signature, its content roots, the arena watermark its content begins at,
/// and its sealing provenance.
#[derive(Clone, Debug)]
pub struct Declaration
{
    /// The prenex level interface.
    levels: LevelSignature,
    /// The definition, axiom, or abstract-type content (root ids into the
    /// environment arena).
    content: DeclarationContent,
    /// The arena watermark at which this declaration's content began — the
    /// pre-admission mark the choke point truncates to on rejection.
    content_start: ArenaWatermark,
    /// The R3 sealing-provenance slot: the atoms this declaration's sealing
    /// projection rebound, ascending and duplicate-free.
    ///
    /// **It records the projection, never the event.** "This module was sealed"
    /// is a claim about the elaborator's history and the kernel would have to
    /// take it on faith; "this declaration's type was projected onto atoms
    /// `ā`" is a claim about *this declaration's type*, and the choke point
    /// re-derives it by walking that type ([`crate::check`]). Empty for a
    /// declaration no projection touched, which is every declaration the
    /// module layer did not seal.
    provenance: Vec<ConstantIndex>,
}

impl Declaration
{
    /// The prenex level interface.
    #[inline]
    #[must_use]
    pub const fn levels(&self) -> &LevelSignature
    {
        &self.levels
    }

    /// The definition or axiom content.
    #[inline]
    #[must_use]
    pub const fn content(&self) -> &DeclarationContent
    {
        &self.content
    }

    /// The declared value-type root id.
    #[inline]
    #[must_use]
    pub const fn declared_id(&self) -> ValueTypeId
    {
        self.content.declared_id()
    }

    /// The content-start watermark (the pre-admission arena mark).
    #[inline]
    #[must_use]
    pub(crate) const fn content_start(&self) -> ArenaWatermark
    {
        self.content_start
    }

    /// The R3 sealing provenance: the atoms this declaration's projection
    /// rebound, ascending.
    #[inline]
    #[must_use]
    pub fn provenance(&self) -> &[ConstantIndex]
    {
        &self.provenance
    }
}

/// A borrowing builder for one declaration's content.
///
/// It lends the environment arena for minting the declared type and body, then
/// finalizes a [`Declaration`] carrying the recorded content-start watermark.
///
/// # Contract
/// - requires: content for exactly one declaration is minted through
///   [`Self::arena`] between construction and a `def`/`axiom` finisher, with no
///   other allocation into the arena interleaved (the content-start watermark
///   records the arena length at construction).
/// - ensures: the finisher yields a [`Declaration`] whose content roots and
///   watermark describe a contiguous suffix of the arena.
/// - provides: the minimal construction surface tests and the checker-to-kernel
///   bridge use.
/// - fails: never — minting is total.
/// - panics: none.
pub struct DeclarationBuilder<'arena>
{
    /// The borrowed environment arena.
    arena: &'arena mut TermArena,
    /// The arena watermark at construction (where this content begins).
    content_start: ArenaWatermark,
}

impl<'arena> DeclarationBuilder<'arena>
{
    /// Begin building a declaration's content into `arena`, recording the
    /// content-start watermark.
    #[inline]
    pub(crate) fn new(arena: &'arena mut TermArena) -> Self
    {
        let content_start = arena.watermark();
        Self {
            arena,
            content_start,
        }
    }

    /// The borrowed arena, for minting this declaration's content nodes.
    #[inline]
    pub fn arena(&mut self) -> &mut TermArena
    {
        self.arena
    }

    /// Finalize a definition over an already-minted declared type and body.
    #[inline]
    #[must_use]
    pub fn def(
        self,
        levels: LevelSignature,
        declared: ValueTypeId,
        body: ValueId,
    ) -> Declaration
    {
        Declaration {
            levels,
            content: DeclarationContent::Def { declared, body },
            content_start: self.content_start,
            provenance: Vec::new(),
        }
    }

    /// Finalize a definition carrying sealing provenance: the atoms the
    /// projection that produced its declared type rebound.
    ///
    /// # Contract
    /// - requires: nothing — a malformed `provenance` is a *rejection* at the
    ///   choke point, never a construction error, because the kernel grants the
    ///   producer no credence about what it sealed.
    /// - ensures: a [`Declaration`] whose provenance slot carries `provenance`
    ///   verbatim, so admission checks exactly what the artifact would carry.
    /// - provides: the sealed-member construction surface the module layer's
    ///   flattening export uses.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn sealed_def(
        self,
        levels: LevelSignature,
        declared: ValueTypeId,
        body: ValueId,
        provenance: Vec<ConstantIndex>,
    ) -> Declaration
    {
        Declaration {
            levels,
            content: DeclarationContent::Def { declared, body },
            content_start: self.content_start,
            provenance,
        }
    }

    /// Finalize an axiom over an already-minted declared type.
    #[inline]
    #[must_use]
    pub fn axiom(
        self,
        levels: LevelSignature,
        declared: ValueTypeId,
    ) -> Declaration
    {
        Declaration {
            levels,
            content: DeclarationContent::Axiom { declared },
            content_start: self.content_start,
            provenance: Vec::new(),
        }
    }

    /// Finalize a sealed abstract type at an already-minted universe kind.
    ///
    /// # Contract
    /// - requires: nothing — a non-universe `kind` is a rejection at the choke
    ///   point ([`KernelError::AbstractTypeKindNotUniverse`]), not a
    ///   construction error.
    /// - ensures: a [`Declaration`] whose content is
    ///   [`DeclarationContent::AbstractType`] and whose provenance is empty (an
    ///   atom is what a projection *produces*, so it carries no projection of
    ///   its own).
    /// - provides: the atom-minting construction surface.
    /// - fails: never.
    /// - panics: none.
    ///
    /// [`KernelError::AbstractTypeKindNotUniverse`]: crate::KernelError::AbstractTypeKindNotUniverse
    #[inline]
    #[must_use]
    pub fn abstract_type(
        self,
        levels: LevelSignature,
        kind: ValueTypeId,
    ) -> Declaration
    {
        Declaration {
            levels,
            content: DeclarationContent::AbstractType { kind },
            content_start: self.content_start,
            provenance: Vec::new(),
        }
    }
}
