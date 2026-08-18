//! The **virtual-double-category interface** and its **cell-store instance**
//! (`proposal-vdc-reflection.md` §5.1; ADR-68 D1/D3).
//!
//! [`Vdc`] is the shape the §3 dictionary test checks and the reflection face
//! assumes: objects, tight arrows, loose arrows, and multi-ary cells, with
//! restriction and multicategorical cell composition. [`CellStoreVdc`] is the
//! instance over the rewrite layer — the dictionary reading of ADR-68 D1
//! realized as the cell store realizes it:
//!
//! | VDC datum        | realization                                            |
//! | ---------------- | ------------------------------------------------------ |
//! | objects          | [`SignatureRef`] — a described signature or a product  |
//! | tight arrows     | [`SigMorphism`] — a generator renaming (function comp) |
//! | loose arrows     | [`RelationRef`] — a named set of generating cells      |
//! | multi-ary cells  | [`Derivation`] — tracelet-backed derived transforms    |
//! | restriction      | frame precomposition, split by content-addressing      |
//!
//! **This is an API to the engine, not a second engine.** Every cell datum is a
//! reflected object whose *meaning* is delegated to the engine: composition
//! rides [`gandr_theory_decomposition_spaces::compose_invertible`], and
//! identity/composition are quotiented by **replay-equivalence**
//! ([`gandr_theory_coherent_resolutions::replay_equivalent`], ADR-69 D1) rather
//! than structural equality. No rewriting is re-implemented here.
//!
//! # Store schema — which cells are storable, and their identity
//!
//! The store the reflection reads holds exactly the storable generators:
//! former rules, the invertibility overlay, and completion fillers. The
//! monoidal-class coherence cells are graft-lemmas — with canonical trees they
//! are derivable and never stored — which is why a [`RelationRef`] is a *set of
//! generating cells* rather than an arbitrary cell set, and why the "pure"
//! sublanguage keeps the rules (the doctrine-level `VCoh ⊇ rule` marking):
//! completion runs one dimension up over that rule alphabet. This sharpens the
//! replay-equivalence identity above — because rule-composites are the
//! completion's alphabet, decidable normal-form identity on a **convergent**
//! fragment compares rule-words, not raw [`Derivation`] trees; the
//! replay-equivalence quotient becomes rule-word equality wherever the fragment
//! converges.

use alloc::boxed::Box;
use alloc::vec::Vec;

use gandr_theory_cell_complexes::CellId;
use gandr_theory_cell_complexes::CellStore;
use gandr_theory_coherent_resolutions::Tracelet;
use gandr_theory_coherent_resolutions::replay_equivalent;
use gandr_theory_decomposition_spaces::compose_invertible;
use gandr_theory_levitation::FreeTerm;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::NameRef;
use gandr_theory_levitation::NominalId;
use gandr_theory_levitation::SignDesc;

use crate::boundary::DerivationReplay;
use crate::boundary::DescTableEmptyStatus;
use crate::boundary::DescTableLength;
use crate::boundary::SigMorphismIdentity;
use crate::boundary::VdcCellEquality;

/// A **described signature** — the VDC object (`proposal-vdc-reflection.md` §2,
/// "objects = described signatures … or a finite product of them").
///
/// A single datatype's description ([`SignatureRef::Single`]) or a finite
/// product interface ([`SignatureRef::Product`]); the empty product
/// (`Product([])`) is the terminal signature, the unit object.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SignatureRef
{
    /// A single described datatype, named by its minted [`NominalId`].
    Single(NominalId),
    /// A finite product of signatures (an interface of sorts); the empty
    /// product is the unit object.
    Product(Box<[Self]>),
}

impl SignatureRef
{
    /// The single-datatype signature over `id`.
    #[inline]
    #[must_use]
    pub fn single(id: NominalId) -> Self
    {
        Self::Single(id)
    }

    /// The terminal (unit) signature — the empty product.
    #[inline]
    #[must_use]
    pub fn unit() -> Self
    {
        Self::Product(Box::from([]))
    }

    /// The product signature over `parts`.
    #[inline]
    #[must_use]
    pub fn product(parts: Box<[Self]>) -> Self
    {
        Self::Product(parts)
    }
}

/// A **term** in the free structure over a signature.
///
/// The reflection-face spelling of a [`gandr_theory_levitation::FreeTerm`]
/// (`proposal-vdc-reflection.md` §5.2, the `TermRef` of the protype/proterm
/// grammar).
///
/// A protype's faces (`Path { lhs, rhs }`, `Rel { lhs, rhs }`) are `TermRef`s;
/// the newtype gives the reflected layer its own vocabulary over the engine's
/// `D⋆(V)` term type.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TermRef(FreeTerm);

impl TermRef
{
    /// A reflected term over a [`FreeTerm`].
    #[inline]
    #[must_use]
    pub fn new(term: FreeTerm) -> Self
    {
        Self(term)
    }

    /// The underlying free-structure term.
    #[inline]
    #[must_use]
    pub fn term(&self) -> &FreeTerm
    {
        &self.0
    }
}

/// A **signature morphism** — the VDC tight arrow
/// (`proposal-vdc-reflection.md` §2, "tight arrows = signature morphisms").
///
/// A sort-respecting **generator renaming** with an explicit source and target
/// signature: `map` lists the generators that move (a name absent from `map`
/// maps to itself), so composition is function composition and the identity is
/// the empty renaming. The `map` is kept **canonical** — sorted by source name,
/// with identity entries dropped — so equal renamings are structurally equal
/// and the tight-category laws hold on the nose.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SigMorphism
{
    /// The source signature.
    pub src: SignatureRef,
    /// The target signature.
    pub tgt: SignatureRef,
    /// The generators that move, `(from, to)`, canonical: sorted by `from`, no
    /// identity entries.
    pub map: Box<[(Name, Name)]>,
}

impl SigMorphism
{
    /// The identity morphism on `sig` — the empty renaming.
    ///
    /// # Contract
    /// - ensures: `src == tgt == sig` and an empty `map` (every generator maps
    ///   to itself).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn identity(sig: SignatureRef) -> Self
    {
        Self {
            src: sig.clone(),
            tgt: sig,
            map: Box::from([]),
        }
    }

    /// A morphism `src → tgt` renaming the listed generators; the renaming is
    /// canonicalized (identity pairs dropped, sorted by source name).
    ///
    /// # Contract
    /// - ensures: `self.src == src`, `self.tgt == tgt`, and `self.map` is the
    ///   canonical form of `pairs` — no `(x, x)` entries, sorted by the source
    ///   name — so two renamings denoting the same function are structurally
    ///   equal.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn renaming(
        src: SignatureRef,
        tgt: SignatureRef,
        pairs: Vec<(Name, Name)>,
    ) -> Self
    {
        Self {
            src,
            tgt,
            map: canonical_map(pairs),
        }
    }

    /// The image of a generator name under this renaming — the name itself when
    /// it is not moved.
    ///
    /// # Contract
    /// - ensures: the `to` of the `(from, to)` entry whose `from` equals
    ///   `name`, or `name` unchanged when no entry matches.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn apply_name(
        &self,
        name: NameRef<'_>,
    ) -> Name
    {
        for pair in &self.map {
            if pair.0.as_ref() == name.as_ref() {
                return pair.1.clone();
            }
        }
        Name::from(name)
    }

    /// Whether this is the identity renaming (an empty canonical map).
    #[inline]
    #[must_use]
    pub fn is_identity(&self) -> SigMorphismIdentity
    {
        SigMorphismIdentity::from(self.map.is_empty())
    }
}

/// Compose two renamings **`f` then `g`** (apply `f`, then `g`) as functions on
/// generator names.
///
/// # Contract
/// - requires: for a boundary-respecting composite, `f.tgt == g.src` (the
///   composite is `f.src → g.tgt`); the map composition itself is total and
///   ignores the boundary.
/// - ensures: a canonical renaming whose `apply_name(n) == g.apply_name(f
///   .apply_name(n))` for every name, with `src == f.src` and `tgt == g.tgt`;
///   composition is associative and has [`SigMorphism::identity`] as a
///   two-sided unit (both by function composition, preserved through
///   canonicalization).
/// - panics: none.
#[inline]
#[must_use]
fn compose_renaming(
    f: &SigMorphism,
    g: &SigMorphism,
) -> SigMorphism
{
    let mut domain: Vec<Name> = Vec::new();
    for pair in f.map.iter().chain(g.map.iter()) {
        if !domain.contains(&pair.0) {
            domain.push(pair.0.clone());
        }
    }
    let mut pairs: Vec<(Name, Name)> = Vec::with_capacity(domain.len());
    for name in domain {
        let moved = f.apply_name(NameRef::from(name.as_ref()));
        let image = g.apply_name(NameRef::from(moved.as_ref()));
        pairs.push((name, image));
    }
    SigMorphism {
        src: f.src.clone(),
        tgt: g.tgt.clone(),
        map: canonical_map(pairs),
    }
}

/// Canonicalize a renaming: drop identity pairs, dedup on source name (first
/// wins), and sort by source name.
///
/// # Contract
/// - ensures: no `(x, x)` entries; at most one entry per source name (the first
///   occurrence's target); sorted by source name — the canonical form on which
///   [`SigMorphism`] equality is structural.
/// - panics: none.
#[inline]
fn canonical_map(pairs: Vec<(Name, Name)>) -> Box<[(Name, Name)]>
{
    let mut out: Vec<(Name, Name)> = Vec::with_capacity(pairs.len());
    for (from, to) in pairs {
        if from == to {
            continue;
        }
        let mut already_seen = false;
        for existing in &out {
            if existing.0 == from {
                already_seen = true;
                break;
            }
        }
        if already_seen {
            continue;
        }
        out.push((from, to));
    }
    out.sort_unstable();
    out.into_boxed_slice()
}

/// A **relation interface** — the VDC loose arrow `R : I ⇸ J`
/// (`proposal-vdc-reflection.md` §2, "loose arrows = relation interfaces over
/// term syntax: a set of generating cells").
///
/// A named set of generating cells (by [`CellId`] into the shared store)
/// between two signatures, carrying two **restriction frames** — the
/// accumulated tight morphisms a [`Vdc::restrict`] precomposes on each side.
/// Restriction is **split/strict** because signature morphisms compose
/// associatively and the frames are content-addressed data (ADR-68 D1): `R[id #
/// id] = R` and `R[s∘s′ # t∘t′] = R[s # t][s′ # t′]` hold definitionally on the
/// frames.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RelationRef
{
    /// The relation's name.
    pub name: Name,
    /// The source signature `I` (after any restriction).
    pub src: SignatureRef,
    /// The target signature `J` (after any restriction).
    pub tgt: SignatureRef,
    /// The generating cells (by id into the [`CellStoreVdc`] store).
    pub generators: Box<[CellId]>,
    /// The accumulated left (source-side) restriction frame; identity for an
    /// unrestricted relation.
    pub left: SigMorphism,
    /// The accumulated right (target-side) restriction frame; identity for an
    /// unrestricted relation.
    pub right: SigMorphism,
}

impl RelationRef
{
    /// An unrestricted relation `src ⇸ tgt` named `name` over `generators`
    /// (both restriction frames identity).
    ///
    /// # Contract
    /// - ensures: `left` and `right` are the identity morphisms on `src` and
    ///   `tgt` respectively; the generating cells are carried verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        name: NameRef<'_>,
        src: SignatureRef,
        tgt: SignatureRef,
        generators: Box<[CellId]>,
    ) -> Self
    {
        Self {
            name: Name::from(name),
            left: SigMorphism::identity(src.clone()),
            right: SigMorphism::identity(tgt.clone()),
            src,
            tgt,
            generators,
        }
    }
}

/// A **reflected multi-ary cell** — a certificate-backed derived transformation
/// `R₁; …; Rₙ ⇒ S` (`proposal-vdc-reflection.md` §2).
///
/// The cell is a "derivation object whose leaves are generating cells": a small
/// derivation tree whose *meaning* is engine replay (ADR-69 D1). A leaf is
/// either the trivial identity transformation or an embedded engine
/// [`Tracelet`], and [`Derivation::Graft`] is multicategorical composition.
/// Elaboration folds the tree to an engine certificate (or the trivial
/// transformation) and replay decides equality.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Derivation
{
    /// The **identity cell** on a relation — the trivial transformation, no
    /// engine steps (`R ⇒ R`).
    Id(RelationRef),
    /// An **embedded engine certificate** with its reflected loose-arrow
    /// boundary (`dom ⇒ cod`).
    Cert
    {
        /// The engine certificate this cell embeds (a leaf of the derivation).
        cert: Box<Tracelet>,
        /// The domain chain of loose arrows the cell consumes.
        dom: Box<[RelationRef]>,
        /// The codomain loose arrow the cell produces.
        cod: RelationRef,
    },
    /// **Multicategorical composition** (grafting) — the `outer` cell with each
    /// input slot filled by an `inners` cell.
    Graft
    {
        /// The outer cell.
        outer: Box<Self>,
        /// The cells grafted into the outer cell's input slots.
        inners: Box<[Self]>,
    },
}

/// The engine-level content a [`Derivation`] elaborates to.
///
/// `Trivial` is the identity transformation (no engine steps); `Cert` is a
/// single engine tracelet. Replay-equivalence of two elaborations
/// ([`elaborations_replay_equivalent`]) is the identity criterion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Elaborated
{
    /// The identity transformation — no engine steps.
    Trivial,
    /// A single engine certificate.
    Cert(Box<Tracelet>),
    /// The elaboration failed — the derivation is not engine-realizable at this
    /// stage (a grafting shape outside the sequential-invertible case, or a
    /// broken seam).
    Stuck,
}

impl Derivation
{
    /// The domain chain of loose arrows this cell consumes.
    ///
    /// # Contract
    /// - ensures: `[r]` for `Id(r)`; the recorded `dom` for a `Cert`; the
    ///   concatenation of the inners' domains for a `Graft`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn dom(&self) -> Vec<RelationRef>
    {
        let mut out = Vec::new();
        let mut stack = alloc::vec![self];
        while let Some(cell) = stack.pop() {
            match *cell {
                | Self::Id(ref r) => out.push(r.clone()),
                | Self::Cert { ref dom, .. } => out.extend_from_slice(dom),
                | Self::Graft { ref inners, .. } => {
                    for inner in inners.iter().rev() {
                        stack.push(inner);
                    }
                },
            }
        }
        out
    }

    /// The codomain loose arrow this cell produces.
    ///
    /// # Contract
    /// - ensures: `r` for `Id(r)`; the recorded `cod` for a `Cert`; the outer
    ///   cell's codomain for a `Graft`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn cod(&self) -> RelationRef
    {
        let mut current = self;
        loop {
            match *current {
                | Self::Id(ref r) => return r.clone(),
                | Self::Cert { ref cod, .. } => return cod.clone(),
                | Self::Graft { ref outer, .. } => current = outer,
            }
        }
    }

    /// Elaborate this cell to its engine content, folding grafting through the
    /// engine's sequential invertible composition
    /// ([`gandr_theory_decomposition_spaces::compose_invertible`]).
    ///
    /// # Contract
    /// - ensures: [`Elaborated::Trivial`] for an `Id` (and for a `Graft` whose
    ///   inners are all identities, elaborating to the outer);
    ///   [`Elaborated::Cert`] for a `Cert` leaf and for a sequential graft
    ///   `Graft { outer, [inner] }` whose inner then outer share the seam
    ///   `inner.joins_at == outer.overlap.peak` (the composite is
    ///   `compose_invertible(inner, outer)`); [`Elaborated::Stuck`] for any
    ///   grafting shape outside that case or a mismatched seam. Never rewrites
    ///   — it only chains recorded certificates.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn elaborate(&self) -> Elaborated
    {
        enum ElaborateFrame<'derivation>
        {
            Enter(&'derivation Derivation),
            FinishSequential,
        }

        let mut frames = alloc::vec![ElaborateFrame::Enter(self)];
        let mut values = Vec::new();
        while let Some(frame) = frames.pop() {
            match frame {
                | ElaborateFrame::Enter(cell) => match *cell {
                    | Self::Id(_) => values.push(Elaborated::Trivial),
                    | Self::Cert { ref cert, .. } => values.push(Elaborated::Cert(cert.clone())),
                    | Self::Graft {
                        ref outer,
                        ref inners,
                    } => {
                        if inners.iter().all(|inner| matches!(*inner, Self::Id(_))) {
                            frames.push(ElaborateFrame::Enter(outer));
                        }
                        else {
                            match inners.first() {
                                | Some(inner) if inners.len() == 1 => {
                                    frames.push(ElaborateFrame::FinishSequential);
                                    frames.push(ElaborateFrame::Enter(outer));
                                    frames.push(ElaborateFrame::Enter(inner));
                                },
                                | _ => values.push(Elaborated::Stuck),
                            }
                        }
                    },
                },
                | ElaborateFrame::FinishSequential => {
                    let outer = values.pop().unwrap_or(Elaborated::Stuck);
                    let inner = values.pop().unwrap_or(Elaborated::Stuck);
                    values.push(graft_sequential(&inner, &outer));
                },
            }
        }
        values.pop().unwrap_or(Elaborated::Stuck)
    }

    /// Whether this cell **replays** in `store` — its engine content is a valid
    /// certificate (ADR-69: replayed, not trusted).
    ///
    /// # Contract
    /// - ensures: `true` for the trivial transformation (an `Id`, no steps to
    ///   run); the certificate's [`Tracelet::replay`] for an engine content;
    ///   `false` for a [`Elaborated::Stuck`] elaboration.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn replays(
        &self,
        store: &CellStore,
    ) -> DerivationReplay
    {
        let replays = match self.elaborate() {
            | Elaborated::Trivial => true,
            | Elaborated::Cert(cert) => bool::from(cert.replay(store)),
            | Elaborated::Stuck => false,
        };
        DerivationReplay::from(replays)
    }
}

/// Chain `inner` **then** `outer` at their shared seam.
///
/// # Contract
/// - ensures: [`Elaborated::Cert`] of
///   [`gandr_theory_decomposition_spaces::compose_invertible`] when both are
///   certificates sharing the seam (`inner.joins_at == outer.overlap.peak`);
///   the non-trivial side unchanged when the other is trivial;
///   [`Elaborated::Stuck`] on a mismatched seam or a stuck operand.
/// - panics: none.
#[inline]
fn graft_sequential(
    inner: &Elaborated,
    outer: &Elaborated,
) -> Elaborated
{
    match (inner, outer) {
        | (&Elaborated::Trivial, other) | (other, &Elaborated::Trivial) => other.clone(),
        | (&Elaborated::Cert(ref i), &Elaborated::Cert(ref o)) => {
            if i.joins_at == o.overlap.peak {
                Elaborated::Cert(Box::new(compose_invertible(i, o)))
            }
            else {
                Elaborated::Stuck
            }
        },
        | (&Elaborated::Stuck, _) | (_, &Elaborated::Stuck) => Elaborated::Stuck,
    }
}

/// Whether two elaborations denote the **same transformation** up to replay
/// (ADR-69 D1).
///
/// # Contract
/// - ensures: two trivial elaborations agree; two certificates agree iff
///   [`gandr_theory_coherent_resolutions::replay_equivalent`] holds; a trivial
///   and a certificate agree iff the certificate is reflexive (`peak ==
///   joins_at`) and replays; a [`Elaborated::Stuck`] never agrees with anything
///   (an unrealized cell is not a transformation).
/// - panics: none.
#[inline]
#[must_use]
fn elaborations_replay_equivalent(
    a: &Elaborated,
    b: &Elaborated,
    store: &CellStore,
) -> VdcCellEquality
{
    let equal = match (a, b) {
        | (&Elaborated::Trivial, &Elaborated::Trivial) => true,
        | (&Elaborated::Cert(ref ca), &Elaborated::Cert(ref cb)) => {
            bool::from(replay_equivalent(ca, cb, store))
        },
        | (&Elaborated::Trivial, &Elaborated::Cert(ref c))
        | (&Elaborated::Cert(ref c), &Elaborated::Trivial) => {
            c.overlap.peak == c.joins_at && bool::from(c.replay(store))
        },
        | (&Elaborated::Stuck, _) | (_, &Elaborated::Stuck) => false,
    };
    VdcCellEquality::from(equal)
}

/// A **description registry** — the signature namespace the reflection reads
/// (`proposal-vdc-reflection.md` §5.1, the `DescTable` of `CellStoreVdc`).
///
/// A thin owned map from [`NominalId`] to [`SignDesc`]:
/// `gandr-theory-levitation` mints descriptions but keeps no table, so the
/// reflection layer organizes them into the object namespace the [`Vdc`]
/// instance resolves signatures against.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DescTable
{
    /// The descriptions, in insertion order.
    descs: Vec<SignDesc>,
}

impl DescTable
{
    /// An empty registry.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Register a description, returning its signature reference.
    ///
    /// # Contract
    /// - ensures: the description is appended and its [`SignatureRef::Single`]
    ///   returned; a later [`DescTable::get`] on the same [`NominalId`] finds
    ///   it.
    /// - panics: none.
    #[inline]
    pub fn insert(
        &mut self,
        desc: SignDesc,
    ) -> SignatureRef
    {
        let id = desc.id.clone();
        self.descs.push(desc);
        SignatureRef::Single(id)
    }

    /// The description with the given identity.
    ///
    /// # Contract
    /// - ensures: `Some` iff a description with a structurally-equal
    ///   [`NominalId`] was inserted.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        id: &NominalId,
    ) -> Option<&SignDesc>
    {
        self.descs.iter().find(|desc| desc.id == *id)
    }

    /// The number of registered descriptions.
    #[inline]
    #[must_use]
    pub fn len(&self) -> DescTableLength
    {
        DescTableLength::from(self.descs.len())
    }

    /// Whether the registry is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> DescTableEmptyStatus
    {
        DescTableEmptyStatus::from(self.descs.is_empty())
    }
}

/// The **virtual-double-category interface** the dictionary test checks and the
/// reflection face assumes (`proposal-vdc-reflection.md` §5.1).
///
/// Objects, tight arrows, loose arrows, and multi-ary cells, with total
/// restriction (split/fibrational by content-addressing), multicategorical cell
/// composition (grafting), and the replay-equivalence cell-equality of ADR-69
/// D1. "Virtual" is load-bearing: loose composites are **not** part of the
/// interface — they exist only as the overlap-indexed family the query surface
/// enumerates ([`crate::query`]).
pub trait Vdc
{
    /// The objects (described signatures).
    type Obj;
    /// The tight arrows (signature morphisms), strictly associative/unital.
    type Tight;
    /// The loose arrows (relation interfaces).
    type Loose;
    /// The multi-ary cells (replay-checked derived transformations).
    type Cell;

    /// The identity tight arrow on an object.
    ///
    /// # Contract
    /// - ensures: a tight arrow that is a two-sided unit for
    ///   [`Vdc::comp_tight`].
    /// - panics: none.
    #[must_use]
    fn id_tight(
        &self,
        o: &Self::Obj,
    ) -> Self::Tight;

    /// Compose two tight arrows, `f` then `g`.
    ///
    /// # Contract
    /// - requires: `f`'s target is `g`'s source for a boundary-respecting
    ///   composite.
    /// - ensures: an associative composite with [`Vdc::id_tight`] as unit.
    /// - panics: none.
    #[must_use]
    fn comp_tight(
        &self,
        f: &Self::Tight,
        g: &Self::Tight,
    ) -> Self::Tight;

    /// The source object of a loose arrow.
    ///
    /// # Contract
    /// - ensures: the loose arrow's source signature.
    /// - panics: none.
    #[must_use]
    fn src(
        &self,
        r: &Self::Loose,
    ) -> Self::Obj;

    /// The target object of a loose arrow.
    ///
    /// # Contract
    /// - ensures: the loose arrow's target signature.
    /// - panics: none.
    #[must_use]
    fn tgt(
        &self,
        r: &Self::Loose,
    ) -> Self::Obj;

    /// Restriction `R[s # t]` — total, split/fibrational by content-addressing.
    ///
    /// # Contract
    /// - requires: `s`'s target is `src(r)` and `t`'s target is `tgt(r)`.
    /// - ensures: the restricted loose arrow `s.src ⇸ t.src`; `restrict(r, id,
    ///   id) == r` and restriction composes (`R[s∘s′ # t∘t′] = R[s # t][s′ #
    ///   t′]`).
    /// - panics: none.
    #[must_use]
    fn restrict(
        &self,
        r: &Self::Loose,
        s: &Self::Tight,
        t: &Self::Tight,
    ) -> Self::Loose;

    /// The domain chain of a cell (`R₁; …; Rₙ`), globular after restriction.
    ///
    /// # Contract
    /// - ensures: the loose arrows the cell consumes, in order.
    /// - panics: none.
    #[must_use]
    fn cell_dom(
        &self,
        c: &Self::Cell,
    ) -> Vec<Self::Loose>;

    /// The codomain of a cell (`S`).
    ///
    /// # Contract
    /// - ensures: the loose arrow the cell produces.
    /// - panics: none.
    #[must_use]
    fn cell_cod(
        &self,
        c: &Self::Cell,
    ) -> Self::Loose;

    /// The identity cell on a loose arrow (the trivial derivation).
    ///
    /// # Contract
    /// - ensures: a cell `r ⇒ r` that is a unit for [`Vdc::compose_cells`] up
    ///   to [`Vdc::cells_equal`].
    /// - panics: none.
    #[must_use]
    fn id_cell(
        &self,
        r: &Self::Loose,
    ) -> Self::Cell;

    /// Multicategorical composition (grafting); the identity is
    /// replay-equivalence.
    ///
    /// # Contract
    /// - ensures: the cell obtained by grafting each `inners` cell into an
    ///   input slot of `outer`; associative and unital up to
    ///   [`Vdc::cells_equal`].
    /// - panics: none.
    #[must_use]
    fn compose_cells(
        &self,
        outer: &Self::Cell,
        inners: &[Self::Cell],
    ) -> Self::Cell;

    /// Whether two cells are the same transformation — replay both, compare
    /// (ADR-69 D1).
    ///
    /// # Contract
    /// - ensures: `true` iff the cells share a loose-arrow boundary and their
    ///   engine elaborations are replay-equivalent.
    /// - panics: none.
    #[must_use]
    fn cells_equal(
        &self,
        a: &Self::Cell,
        b: &Self::Cell,
    ) -> VdcCellEquality;
}

/// The **cell-store instance** of [`Vdc`] — the dictionary reading of ADR-68 D1
/// over the live rewrite layer (`proposal-vdc-reflection.md` §5.1).
///
/// Objects are [`SignatureRef`]s resolved against `descs`; tight arrows are
/// [`SigMorphism`]s; loose arrows are [`RelationRef`]s whose generating cells
/// live in `cells`; cells are [`Derivation`]s replayed against `cells`.
#[derive(Clone, Copy, Debug)]
pub struct CellStoreVdc<'store>
{
    /// The signature namespace (`gandr-theory-levitation` descriptions).
    pub descs: &'store DescTable,
    /// The content-addressed cell store (`gandr-theory-computads`).
    pub cells: &'store CellStore,
}

impl<'store> CellStoreVdc<'store>
{
    /// The instance over a description registry and a cell store.
    #[inline]
    #[must_use]
    pub fn new(
        descs: &'store DescTable,
        cells: &'store CellStore,
    ) -> Self
    {
        Self { descs, cells }
    }
}

impl Vdc for CellStoreVdc<'_>
{
    type Cell = Derivation;
    type Loose = RelationRef;
    type Obj = SignatureRef;
    type Tight = SigMorphism;

    #[inline]
    fn id_tight(
        &self,
        o: &SignatureRef,
    ) -> SigMorphism
    {
        SigMorphism::identity(o.clone())
    }

    #[inline]
    fn comp_tight(
        &self,
        f: &SigMorphism,
        g: &SigMorphism,
    ) -> SigMorphism
    {
        compose_renaming(f, g)
    }

    #[inline]
    fn src(
        &self,
        r: &RelationRef,
    ) -> SignatureRef
    {
        r.src.clone()
    }

    #[inline]
    fn tgt(
        &self,
        r: &RelationRef,
    ) -> SignatureRef
    {
        r.tgt.clone()
    }

    #[inline]
    fn restrict(
        &self,
        r: &RelationRef,
        s: &SigMorphism,
        t: &SigMorphism,
    ) -> RelationRef
    {
        RelationRef {
            name: r.name.clone(),
            src: s.src.clone(),
            tgt: t.src.clone(),
            generators: r.generators.clone(),
            left: compose_renaming(s, &r.left),
            right: compose_renaming(t, &r.right),
        }
    }

    #[inline]
    fn cell_dom(
        &self,
        c: &Derivation,
    ) -> Vec<RelationRef>
    {
        c.dom()
    }

    #[inline]
    fn cell_cod(
        &self,
        c: &Derivation,
    ) -> RelationRef
    {
        c.cod()
    }

    #[inline]
    fn id_cell(
        &self,
        r: &RelationRef,
    ) -> Derivation
    {
        Derivation::Id(r.clone())
    }

    #[inline]
    fn compose_cells(
        &self,
        outer: &Derivation,
        inners: &[Derivation],
    ) -> Derivation
    {
        Derivation::Graft {
            outer: Box::new(outer.clone()),
            inners: inners.to_vec().into_boxed_slice(),
        }
    }

    #[inline]
    fn cells_equal(
        &self,
        a: &Derivation,
        b: &Derivation,
    ) -> VdcCellEquality
    {
        let equal = a.dom() == b.dom()
            && a.cod() == b.cod()
            && bool::from(elaborations_replay_equivalent(
                &a.elaborate(),
                &b.elaborate(),
                self.cells,
            ));
        VdcCellEquality::from(equal)
    }
}
