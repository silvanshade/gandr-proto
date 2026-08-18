//! Per-run **type interning**: a content-addressed identity for [`Ty`] giving
//! O(1) equality.
//!
//! Equal ids witness structural equality, so an id comparison answers in O(1)
//! what a structural `==` answers in time proportional to the type's size.
//! That identity is what a consumer builds a fast path on: the checker crate's
//! subsumption relation short-circuits its *reflexive* leg with it, since two
//! types that intern to the same id are structurally equal and hence
//! (consistent) subtypes with no descent. Reflexivity is admissible rather
//! than a rule (`type-system.md` §"Algorithmic subtyping and the worklist
//! solver"), so every such fast path is a pure optimization over a structural
//! relation this crate does not define.
//!
//! No shipping crate keys anything on a [`TypeId`] yet. The facility is
//! exercised by this module's own tests and by the checker crate's interned
//! subsumption rows; a consumer that adopts it inherits the resolve guard
//! below rather than a second identity notion.
//!
//! # Intern-hit discipline (Lean 4 `Expr` interning, adapted)
//!
//! Hit detection follows the Lean `Expr.equal` ladder — pointer check →
//! cached-hash check → structural recurse — degenerated for gandr's v0 types
//! (which have **no type substitution**, so the value graph interned here is
//! immutable):
//!
//! - the **cached side-table word** is the FNV content hash ([`type_hash`]),
//!   the bucket key; a differing hash is a definitive "distinct" in O(1). The
//!   Lean `Expr.Data` word also packs structural bits (approx-depth, a
//!   loose-bvar / metavariable range) to prune α / instantiation work — those
//!   are **deliberately omitted here** because v0 types carry no metavariables
//!   and no loose de Bruijn indices, so every such bit would be uniformly
//!   trivial; the hash word alone is the honest v0 realization, and the door to
//!   a richer word stays open (see below);
//! - the **pointer check** degenerates: the interner keys on owned [`Ty`]
//!   values, so an equal id already witnesses shared identity — the id *is* the
//!   canonicalized pointer;
//! - the **structural recurse** is the derived `Eq`, run only to break a
//!   hash-bucket collision (rare).
//!
//! # Content-key discipline
//!
//! Interning covers the immutable value graph, and every content-bearing field
//! is part of the key: a [`crate::grade::Grade`] inside a `Thunk`
//! and an [`crate::effect::EffectRow`] inside an `F` both participate — the
//! derived `Hash` / `Eq` descend into them, and both are canonical (`EffectRow`
//! is a name-ordered `BTreeMap`, so union order is immaterial). Thunks that
//! differ only in grade, or returners that differ only in row, therefore intern
//! to *distinct* ids.
//!
//! # The term-face design's ADR-49 doors (binding on this interim)
//!
//! The key is the type's own canonical form, so per-node-kind **canonical child
//! ordering** is already admitted and exploited (records are a name-ordered
//! `BTreeMap`, rows likewise) — no ordering is baked into an id. And a
//! [`TypeId`] is an opaque dense index: nothing in the key shape or the
//! resolve API assumes an **arity-1** result, so a later multi-output
//! canonicalization (the Σ-zone term face) can land without a key-shape change.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::hash::Hash as _;
use core::hash::Hasher as _;

use crate::boundary::InternedTypeCount;
use crate::boundary::InternerEmptyStatus;
use crate::boundary::TypeHash;
use crate::boundary::TypeTableIndex;
use crate::types::Ty;

/// A canonical, interned identity for a type — O(1) equality for the
/// unchanged-type optimization (Porter's per-node dual-type cache).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TypeId(u32);

impl TypeId
{
    /// The id as a table index.
    #[inline]
    fn index(self) -> TypeTableIndex
    {
        usize::try_from(self.0).unwrap_or(usize::MAX).into()
    }
}

/// A hash-consing interner over [`Ty`]: equal types intern to the same
/// [`TypeId`], giving O(1) equality.
///
/// Equality is canonical over grades and effect rows (`Grade` is `Fin`/`Omega`,
/// `EffectRow` is a name-ordered `BTreeMap`), so an incremental dual-type cache
/// compares node types in O(1) instead of the structural O(tree-size) `==`.
/// Deterministic (a fixed FNV content hash, no randomized hashing), so interned
/// ids are golden-stable within a run.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeInterner
{
    /// Buckets of interned ids keyed by content hash; collisions (rare) are
    /// resolved by structural equality within a bucket.
    buckets: BTreeMap<u64, Vec<TypeId>>,
    /// Interned types indexed by id.
    types: Vec<Ty>,
}

impl TypeInterner
{
    /// Builds an empty interner.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Interns a type, returning its canonical id (minting a fresh one for a
    /// not-yet-seen type).
    ///
    /// # Contract
    /// - ensures: returns a [`TypeId`] equal to a prior call's id iff `ty`
    ///   equals the prior argument (structural `==`, canonical over grades and
    ///   rows); distinct types get distinct ids (up to the `2^32` id capacity,
    ///   beyond which ids saturate and may alias — far past any real type
    ///   table).
    /// - panics: none.
    #[inline]
    pub fn intern(
        &mut self,
        ty: &Ty,
    ) -> TypeId
    {
        let hash = u64::from(type_hash(ty));
        if let Some(bucket) = self.buckets.get(&hash) {
            for &id in bucket {
                if self.types.get(usize::from(id.index())) == Some(ty) {
                    return id;
                }
            }
        }
        // Beyond `u32::MAX` distinct types the id saturates and would alias
        // (documented capacity). Make that loud in debug; release stays total.
        debug_assert!(
            self.types.len() < usize::try_from(u32::MAX).unwrap_or(usize::MAX),
            "TypeInterner id space exhausted (2^32 distinct types)"
        );
        let id = TypeId(u32::try_from(self.types.len()).unwrap_or(u32::MAX));
        self.types.push(ty.clone());
        self.buckets.entry(hash).or_default().push(id);
        id
    }

    /// Resolves an interned id back to its type.
    ///
    /// # Contract
    /// - ensures: returns the type interned at `id`, or `None` for an id this
    ///   interner did not mint.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn resolve(
        &self,
        id: TypeId,
    ) -> Option<&Ty>
    {
        self.types.get(usize::from(id.index()))
    }

    /// The number of distinct interned types.
    #[inline]
    #[must_use]
    pub fn len(&self) -> InternedTypeCount
    {
        self.types.len().into()
    }

    /// Whether nothing has been interned yet.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> InternerEmptyStatus
    {
        self.types.is_empty().into()
    }
}

/// The deterministic content hash of a type — the unchanged-type
/// optimization's O(1) early-cutoff primitive.
///
/// Equal types hash equal; a differing hash is a definitive "changed" verdict
/// in O(1), and an equal hash is confirmed by the interner's structural check.
///
/// # Contract
/// - ensures: returns a stable 64-bit FNV-1a hash of `ty` consistent with its
///   `Eq` (equal types hash equal); deterministic across runs (no randomized
///   seed).
/// - panics: none.
#[inline]
#[must_use]
pub fn type_hash(ty: &Ty) -> TypeHash
{
    let mut hasher = FnvHasher::new();
    ty.hash(&mut hasher);
    hasher.finish().into()
}

/// A deterministic FNV-1a hasher (no randomized seed, `core`-only), so type
/// hashes are golden-stable across runs — unlike the standard library's
/// randomized default hasher.
#[repr(transparent)]
struct FnvHasher
{
    /// The running 64-bit FNV-1a state.
    state: u64,
}

impl FnvHasher
{
    /// The FNV-1a 64-bit offset basis.
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    /// The FNV-1a 64-bit prime.
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    /// A fresh hasher seeded at the FNV offset basis.
    #[inline]
    fn new() -> Self
    {
        Self {
            state: Self::OFFSET_BASIS,
        }
    }
}

impl core::hash::Hasher for FnvHasher
{
    #[inline]
    fn finish(&self) -> u64
    {
        self.state
    }

    #[inline]
    fn write(
        &mut self,
        bytes: &[u8],
    )
    {
        let mut state = self.state;
        for &byte in bytes {
            state ^= u64::from(byte);
            state = state.wrapping_mul(Self::PRIME);
        }
        self.state = state;
    }
}

#[cfg(test)]
mod tests
{
    use super::TypeId;
    use super::TypeInterner;
    use crate::boundary::InternedTypeCount;
    use crate::grade::Grade;
    use crate::types::CompType;
    use crate::types::Ty;
    use crate::types::ValueType;

    /// structurally distinct type mints a fresh id (miss). Each `fresh_nested`
    /// call allocates fresh `Rc`s throughout, so a same-id verdict is genuine
    /// content-addressing, not aliasing.
    #[test]
    fn intern_dedups_by_content_hit_and_miss()
    {
        let mut interner = TypeInterner::new();
        let a = interner.intern(&Ty::Value(fresh_nested()));
        let b = interner.intern(&Ty::Value(fresh_nested()));
        assert_eq!(
            a, b,
            "address-distinct structural equals dedup to one id (hit)"
        );
        assert_eq!(
            InternedTypeCount::from(1),
            interner.len(),
            "no fresh id was minted for the duplicate"
        );

        // A structurally distinct type is a miss: a fresh id.
        let other = interner.intern(&Ty::Value(ValueType::integer()));
        assert_ne!(a, other, "a distinct type gets a distinct id (miss)");
        assert_eq!(
            InternedTypeCount::from(2),
            interner.len(),
            "the distinct type was minted"
        );
    }

    /// A small nested value type, built fresh (all-new `Rc`s) on each call so
    /// two invocations are structurally equal yet share no interior address.
    /// Address-distinctness is what makes the dedup verdict content-addressing
    /// rather than aliasing.
    fn fresh_nested() -> ValueType
    {
        ValueType::prod(
            ValueType::list(ValueType::atom("A")),
            ValueType::thunk(Grade::OMEGA, CompType::returner(ValueType::integer())),
        )
    }

    /// Content-key discipline: a [`Grade`] is part of the key, so two thunks
    /// that differ only in grade intern to distinct ids.
    #[test]
    fn grade_is_part_of_the_content_key()
    {
        let mut interner = TypeInterner::new();
        let omega = interner.intern(&Ty::Value(ValueType::thunk(
            Grade::OMEGA,
            CompType::returner(ValueType::integer()),
        )));
        let one = interner.intern(&Ty::Value(ValueType::thunk(
            Grade::ONE,
            CompType::returner(ValueType::integer()),
        )));
        assert_ne!(omega, one, "grade participates in the intern key");
    }

    /// [`TypeInterner::resolve`] is the guard every id-keyed reader passes
    /// through: an id past this interner's minted range resolves to nothing, so
    /// a reader that honours the `Option` cannot observe a type this interner
    /// never minted. The unminted id is constructed directly here because no
    /// public caller can reach that state.
    #[test]
    fn resolve_of_an_unminted_id_is_none()
    {
        const UNMINTED_TYPE_ID: u32 = 999;
        let mut interner = TypeInterner::new();
        let minted = interner.intern(&Ty::Value(ValueType::integer()));
        assert!(
            interner.resolve(minted).is_some(),
            "a minted id resolves to its type"
        );
        assert!(
            interner.resolve(TypeId(UNMINTED_TYPE_ID)).is_none(),
            "an id past the minted range resolves to nothing"
        );
    }

    /// The two sorts stay apart under one interner: a value type and a
    /// computation type mint distinct ids and resolve to distinct [`Ty`]
    /// variants, which is what lets an id-keyed reader refuse a cross-sort
    /// pair without a structural comparison.
    #[test]
    fn the_two_sorts_resolve_to_distinct_variants()
    {
        let mut interner = TypeInterner::new();
        let value_id = interner.intern(&Ty::Value(ValueType::integer()));
        let comp_id = interner.intern(&Ty::Comp(CompType::returner(ValueType::integer())));
        assert_ne!(value_id, comp_id, "the two sorts mint distinct ids");
        assert!(
            matches!(interner.resolve(value_id), Some(&Ty::Value(_))),
            "the value id resolves to the value sort"
        );
        assert!(
            matches!(interner.resolve(comp_id), Some(&Ty::Comp(_))),
            "the computation id resolves to the computation sort"
        );
    }
}
