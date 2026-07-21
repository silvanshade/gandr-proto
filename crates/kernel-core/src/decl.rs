//! The closed declaration vocabulary ([`Declaration`]): `Def` (a typed
//! definition) and `Axiom` (a tracked typed hole), each carrying its own
//! prenex [`LevelSignature`] (kernel-boundary.md §3).
//!
//! Growth is deliberate and closed: at S1 the vocabulary is exactly these two.
//! Datatype declarations arrive later as levitated description codes (S2),
//! never as a raw-inductive kind (K4).
//!
//! **Declarations are value-polarity at the boundary** (a design decision for
//! the S1 slice): a `Def` pairs a declared *value* type with a *value* body,
//! and an `Axiom` a declared value type. A computation definition `f : A → C`
//! enters as a thunk — declared type `U (A → C)`, body `thunk (λ. …)` — and is
//! used through `force`. This keeps the declaration vocabulary single-polarity
//! (one `add_decl` shape, no polarity-mismatch error) and matches CBPV's
//! treatment of top-level bindings as thunkable values.
//!
//! # Arena content and the builder (D1(C), gandr-5t3)
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

/// A declaration's prenex level interface: its parameter count and its
/// declared landmark constraints (ADR-78 — the declaration is generalized over
/// these and checked against nothing else).
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
#[expect(
    clippy::exhaustive_enums,
    reason = "the kernel's declaration vocabulary is closed by design (kernel-boundary.md §3)"
)]
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
}

impl DeclarationContent
{
    /// The declared value-type root id, common to both a definition and an
    /// axiom.
    #[inline]
    #[must_use]
    pub const fn declared_id(&self) -> ValueTypeId
    {
        match *self {
            | Self::Def { declared, .. } | Self::Axiom { declared } => declared,
        }
    }
}

/// A declaration entering the kernel through the choke point: a level
/// signature, its content roots, and the arena watermark its content begins at.
#[derive(Clone, Debug)]
pub struct Declaration
{
    /// The prenex level interface.
    levels: LevelSignature,
    /// The definition or axiom content (root ids into the environment arena).
    content: DeclarationContent,
    /// The arena watermark at which this declaration's content began — the
    /// pre-admission mark the choke point truncates to on rejection.
    content_start: ArenaWatermark,
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
/// - provides: the minimal construction surface tests and the B2.3 bridge use.
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
        }
    }
}
