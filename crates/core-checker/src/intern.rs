//! Per-run **type interning**: a content-addressed identity for [`Ty`] giving
//! O(1) equality and — on an intern hit — O(1) reflexive/atom subtyping.
//!
//! Two consumers share this one facility:
//!
//! - the **incremental marking** layer ([`crate::mark`]) keys Porter's per-node
//!   dual-type cache on interned [`TypeId`]s, so an unchanged node compares its
//!   type in O(1) instead of the structural O(tree-size) `==`;
//! - **subtyping** ([`TypeInterner::subtype`]) reuses the same identity to
//!   short-circuit the *reflexive* leg of the subtype relation: two types that
//!   intern to the same id are structurally equal, hence (consistent) subtypes,
//!   with no structural descent. Reflexivity is admissible, never a rule
//!   (`type-system.md` §"Algorithmic subtyping and the worklist solver"), so
//!   this is a pure optimization — the structural
//!   [`crate::subtype::value_subtype`] / [`crate::subtype::comp_subtype`]
//!   decides every non-reflexive pair.
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
//! is part of the key: a [`crate::grade::Grade`] inside a `Thunk` and an
//! [`crate::effect::EffectRow`] inside an `F` both participate — the derived
//! `Hash` / `Eq` descend into them, and both are canonical (`EffectRow` is a
//! name-ordered `BTreeMap`, so union order is immaterial). Thunks that differ
//! only in grade, or returners that differ only in row, therefore intern to
//! *distinct* ids.
//!
//! # ADR-49 doors (`proposal-term-face.md`, binding on this interim)
//!
//! The key is the type's own canonical form, so per-node-kind **canonical child
//! ordering** is already admitted and exploited (records are a name-ordered
//! `BTreeMap`, rows likewise) — no ordering is baked into an id. And a
//! [`TypeId`] is an opaque dense index: nothing in the key shape or the
//! resolve / subtype API assumes an **arity-1** result, so a later multi-output
//! canonicalization (the Σ-zone term face) can land without a key-shape change.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::hash::Hash as _;
use core::hash::Hasher as _;

use crate::boundary::InternedTypeCount;
use crate::boundary::InternerEmptyStatus;
use crate::boundary::SubtypeDecision;
use crate::boundary::TypeHash;
use crate::boundary::TypeTableIndex;
use crate::subtype::comp_subtype;
use crate::subtype::value_subtype;
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

    /// Decides `sub ≲ sup` on two interned ids, using intern identity as an
    /// O(1) reflexive/atom short-circuit before any structural descent — the
    /// interned analogue of the `core::ptr::eq` fast path in
    /// [`crate::subtype::value_subtype`], but catching
    /// *structurally* equal address-distinct types too (they share an id).
    ///
    /// # Contract
    /// - requires: `sub` and `sup` were both minted by *this* interner (an id
    ///   from another interner, or one never minted, resolves to `None` and so
    ///   returns `false`).
    /// - ensures: returns `true` iff the type at `sub` is a consistent subtype
    ///   of the type at `sup` — same-sort structural subtyping via
    ///   [`crate::subtype::value_subtype`] / [`crate::subtype::comp_subtype`].
    ///   Identical ids return `true` in O(1) (reflexivity, admissible per
    ///   `type-system.md` §"Algorithmic subtyping and the worklist solver"); a
    ///   value id against a computation id returns `false` (no value type is a
    ///   subtype of a computation type, and vice versa). The verdict equals the
    ///   structural relation on the resolved types, so the id short-circuit is
    ///   a pure optimization.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — the id-equality short-circuit never disagrees with
    ///   structural subtyping (it only pre-empts the reflexive case, which the
    ///   structural relation also accepts).
    /// - witness: `tests::interned_subtype_agrees_with_structural_value` /
    ///   `_comp` (the fast path equals structural descent on every generated
    ///   pair) plus the deterministic reflexive-hit and structural-fallback
    ///   tests.
    #[inline]
    #[must_use]
    pub fn subtype(
        &self,
        sub: TypeId,
        sup: TypeId,
    ) -> SubtypeDecision
    {
        // Reflexive/atom intern hit: equal ids ⟹ structurally identical types
        // ⟹ (consistent) subtypes, decided in O(1) without any descent.
        if sub == sup {
            return true.into();
        }
        match (self.resolve(sub), self.resolve(sup)) {
            | (Some(&Ty::Value(ref lo)), Some(&Ty::Value(ref hi))) => value_subtype(lo, hi),
            | (Some(&Ty::Comp(ref lo)), Some(&Ty::Comp(ref hi))) => comp_subtype(lo, hi),
            // Cross-sort (a value type and a computation type never relate) or an
            // id this interner never minted.
            | _ => false.into(),
        }
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
    use proptest::prelude::*;

    use super::TypeId;
    use super::TypeInterner;
    use crate::boundary::InternedTypeCount;
    use crate::grade::Grade;
    use crate::strategies::arb_comp_type;
    use crate::strategies::arb_value_type;
    use crate::subtype::comp_subtype;
    use crate::subtype::value_subtype;
    use crate::types::CompType;
    use crate::types::Ty;
    use crate::types::ValueType;
    /// Interning is by content, not address: two independent (address-distinct)
    /// constructions of the same type dedup to the SAME id (hit), while a
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

    /// Reflexive/atom hit: an address-distinct rebuild interns to the same id,
    /// so [`TypeInterner::subtype`] returns `true` in O(1) — and it agrees with
    /// the structural [`value_subtype`] run over the two address-distinct
    /// values (the non-vacuous reflexive descent).
    #[test]
    fn subtype_reflexive_hit_agrees_with_structural_descent()
    {
        let t1 = fresh_nested();
        let t2 = fresh_nested();
        // Structural descent (addresses differ ⇒ the ptr::eq fast path in
        // `value_subtype` cannot pre-empt): the relation holds by reflexivity.
        assert!(
            bool::from(value_subtype(&t1, &t2)),
            "reflexivity holds structurally"
        );

        let mut interner = TypeInterner::new();
        let a = interner.intern(&Ty::Value(t1));
        let b = interner.intern(&Ty::Value(t2));
        assert_eq!(a, b, "the rebuild deduped to the same id");
        assert!(
            bool::from(interner.subtype(a, b)),
            "same id ⇒ O(1) reflexive subtype hit"
        );
    }

    /// A small nested value type, built fresh (all-new `Rc`s) on each call so
    /// two invocations are structurally equal yet share no interior address —
    /// the address-distinctness the reflexive-hit test needs so the structural
    /// `value_subtype` descent is genuinely exercised (not pre-emptied by the
    /// `core::ptr::eq` fast path).
    fn fresh_nested() -> ValueType
    {
        ValueType::prod(
            ValueType::list(ValueType::atom("A")),
            ValueType::thunk(Grade::OMEGA, CompType::returner(ValueType::integer())),
        )
    }

    /// Structural fallback on distinct ids: `U_ω B <: U_1 B` (because `1 ⊑ ω`)
    /// but not conversely. The two thunks intern to distinct ids, so
    /// [`TypeInterner::subtype`] takes the structural path and matches
    /// [`value_subtype`] in both directions.
    #[test]
    fn subtype_structural_fallback_on_distinct_ids()
    {
        let omega = ValueType::thunk(Grade::OMEGA, CompType::returner(ValueType::integer()));
        let one = ValueType::thunk(Grade::ONE, CompType::returner(ValueType::integer()));

        let mut interner = TypeInterner::new();
        let omega_id = interner.intern(&Ty::Value(omega.clone()));
        let one_id = interner.intern(&Ty::Value(one.clone()));
        assert_ne!(omega_id, one_id, "different grades ⇒ different ids");

        assert!(
            bool::from(interner.subtype(omega_id, one_id)),
            "U_ω B <: U_1 B (1 ⊑ ω)"
        );
        assert_eq!(
            interner.subtype(omega_id, one_id),
            value_subtype(&omega, &one),
            "the interned verdict matches structural subtyping"
        );
        assert!(
            !bool::from(interner.subtype(one_id, omega_id)),
            "U_1 B </: U_ω B (directional)"
        );
        assert_eq!(
            interner.subtype(one_id, omega_id),
            value_subtype(&one, &omega),
            "the interned negative verdict matches structural subtyping"
        );
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

    /// Cross-sort ids never relate, and an id this interner never minted
    /// resolves to nothing and so returns `false`.
    #[test]
    fn subtype_cross_sort_and_unminted_are_false()
    {
        const UNMINTED_TYPE_ID: u32 = 999;
        let mut interner = TypeInterner::new();
        let value_id = interner.intern(&Ty::Value(ValueType::integer()));
        let comp_id = interner.intern(&Ty::Comp(CompType::returner(ValueType::integer())));
        assert!(
            !bool::from(interner.subtype(value_id, comp_id)),
            "a value type is not a subtype of a computation type"
        );
        assert!(
            !bool::from(interner.subtype(comp_id, value_id)),
            "nor the converse"
        );
        // An id past the minted range resolves to `None` ⇒ `false`.
        let unminted = TypeId(UNMINTED_TYPE_ID);
        assert!(
            !bool::from(interner.subtype(value_id, unminted)),
            "an unminted id never relates"
        );
    }

    proptest! {
        /// The interned [`TypeInterner::subtype`] verdict equals the structural
        /// [`value_subtype`] on the same pair — the id short-circuit is a pure
        /// optimization, agreeing with structural descent on every pair. `lo`
        /// and `hi` are independent random types, so most pairs take the
        /// structural fallback (distinct ids) and a coincidentally-equal pair
        /// takes the O(1) id hit; either way the verdicts must agree.
        #[test]
        fn interned_subtype_agrees_with_structural_value(
            lo in arb_value_type(3_u32),
            hi in arb_value_type(3_u32),
        ) {
            let mut interner = TypeInterner::new();
            let lo_id = interner.intern(&Ty::Value(lo.clone()));
            let hi_id = interner.intern(&Ty::Value(hi.clone()));
            prop_assert_eq!(interner.subtype(lo_id, hi_id), value_subtype(&lo, &hi));
        }

        /// The computation-sort analogue of
        /// [`interned_subtype_agrees_with_structural_value`].
        #[test]
        fn interned_subtype_agrees_with_structural_comp(
            lo in arb_comp_type(3_u32),
            hi in arb_comp_type(3_u32),
        ) {
            let mut interner = TypeInterner::new();
            let lo_id = interner.intern(&Ty::Comp(lo.clone()));
            let hi_id = interner.intern(&Ty::Comp(hi.clone()));
            prop_assert_eq!(interner.subtype(lo_id, hi_id), comp_subtype(&lo, &hi));
        }
    }
}
