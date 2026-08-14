//! A generic **nominal atom** substrate for gandr's machine-minted names.
//!
//! gandr mints machine-internal names in several independent places — the
//! pipeline lowerer's hoist binders (`%tmp{n}`) and hole identifiers, the
//! typing machine's captured-continuation keys (`%k{id}`), and a future solver
//! type-variable / grade-variable pool. Each was its own counter plus format,
//! each separately re-establishing the invariant that the names it mints are
//! distinct. This crate is the **shared atom fabric** those name-spaces are
//! being consolidated onto: one allocation discipline ([`Gensym`]) producing
//! sort-tagged [`Atom`]s whose identities are distinct **within a single
//! allocator**. That per-allocator guarantee is exactly the scope its consumers
//! need: gandr runs one allocator per sort per pass (one `Lowerer` hoist/hole
//! allocator), so the names any one pass mints are mutually distinct.
//!
//! **The consolidation is partial, and the tree is the record of how far it
//! got.** The lowerer's hoist binders and hole addresses mint here. The
//! continuation keys do not: they are still `format!("%k{n}")` off a bare
//! counter in `gandr-core-sequent`'s focusing pass. Sealing minted a third
//! discipline deliberately — an opaque ascription's abstract-type identity has
//! to be a *function of the sealing site* so an admission point can re-mint and
//! compare, which a monotone allocator cannot offer, so `gandr-core-checker`'s
//! seal table assigns positional serials instead. Two of the sort vocabulary's
//! six roles are constructed today.
//!
//! The consolidation is the structural answer to a measured hazard: keying a
//! continuation environment by the *source* binder name (dynamic scoping)
//! once made a well-typed term loop to a stuck step-limit; the fix is to
//! α-rename each capture to a fresh machine-unique name. Folding the ad-hoc
//! counters into one audited primitive replaces per-site re-provings of
//! "minted names are distinct" with a single one (the address-distinctness
//! invariant a cut-based / SAX-style addressing discipline wants).
//!
//! # The atom-vs-variable sort boundary
//!
//! [`Sort::is_unifiable`] splits the atoms one allocator mints into two roles:
//!
//! - **atom-role** — pure names, only minted / compared / freshness-tested /
//!   (eventually) permuted: continuation keys, hoist binders, the typing-
//!   transparent hole *address*, and the reserved channel / world / role /
//!   capability classes;
//! - **variable-role** — substitutable unknowns that enter a substitution's
//!   domain (`π·X`): the solver's type- and grade-variables.
//!
//! The boundary is load-bearing, not cosmetic: keeping unification variables in
//! a disjoint sort is what preserves **unitary** most-general unifiers
//! (Urban–Pitts–Gabbay, *Nominal Unification*, Theoretical Computer Science
//! 323, 2004); collapsing them into the atom pool drops into equivariant
//! unification. `is_unifiable` is gandr's project-side realization of that
//! boundary — it is *not* anchored to "Pitts, *Nominal Sets*, Remark 3.10"
//! (a freshness-quantifier proposition, not this boundary).
//!
//! # What this crate is, and is not
//!
//! The crate is the atom space, the monotone allocator, and the sort trait
//! **plus the explicit-dealloc automaton layer** that reclaims atoms.
//!
//! **Only the first is consumed.** [`Gensym`], [`Sort`] and [`Unifiability`]
//! are the three items other crates name; the automaton layer below — three
//! model inventories, the membership procedure, the name-dropping
//! construction, and the decision-procedure catalogue — has no caller in the
//! workspace and is waiting on the bounded-alphabet restriction and the
//! classical back-end that every remaining catalogue procedure bottlenecks on.
//! The ratio is stated here rather than left to a reader to discover: the two
//! layers are kept in one crate deliberately, because the automaton half is
//! where the resource-lifecycle work lands, and splitting it would buy a
//! tidier number at the price of a crate, a name, and a category argument.
//!
//! The adopted design is the nominal-automata study, which left this repository
//! with the research corpus; the landed modules map onto it as follows:
//!
//! - [`handle`] — the finitary orbit-level representation (control points,
//!   registers, partial injective stores, degree): design doc §6.4.
//! - [`letter`] — the extended alphabet `Â = { ⟦a, a⟧, ⟦a⟧ }` plus free uses:
//!   NDA §3.
//! - [`nda`] — the **NDA** model inventory (NDA Def 5.1) and the **membership**
//!   decision procedure (forward reachability; on a DDA the deterministic
//!   online monitor): design doc §4, §6.3, §7.3.
//! - [`rnna`] — the **RNNA** model inventory (`FoSSaCS` 2017): design doc §2.2.
//! - [`rnta`] — the **RNTA** model inventory (RNTA Def 4.1) and nominal terms
//!   (RNTA Def 3.1): design doc §3.
//! - [`dropping`] — the **name-dropping modification** `A_⊥` (NDA Constr 6.3 /
//!   Thm 6.9; RNTA Def 5.3 / Thm 5.5), step ① of the shared name-dropping /
//!   bounded-alphabet / reduce-to-classical template: design doc §5.
//! - [`catalogue`] — the decision-procedure register (membership, emptiness,
//!   inclusion, equivalence, determinization, Kleene) with the papers' theorem
//!   numbers: design doc §6.3, §10.
//!
//! Still reserved behind this API until their consuming features land:
//!
//! - the swapping-list **permutation** (`Perm`) and its action / freshness `#`
//!   discipline (arrives with the α / equivariance metatheory);
//! - the finite-**support** skin (`BTreeSet<Atom>` with native `⊆`) over which
//!   scope-sets and contextual substitution are hosted (arrives with macros; it
//!   widens [`Sort`] with `Ord` — and `Hash` for hash-set support — so
//!   `Atom<S>` can key the support container);
//! - **nominal unification** (arrives with the real solver);
//! - the bounded-alphabet S-restriction and the classical NFA/NFTA back-end
//!   (steps ② and ③ of the design doc §5 template), the RNTA top-down run,
//!   determinization (NDA Thm 8.14), and the Kleene expression compiler (NDA
//!   Thm 7.19/7.20) — recorded residuals of the landed slice.
//!
//! The crate is generic over the sort so it carries no gandr vocabulary; gandr
//! supplies its own `enum GandrSort : Sort`.

extern crate alloc;

pub mod catalogue;
pub mod dropping;
pub mod handle;
pub mod letter;
pub mod nda;
pub mod rnna;
pub mod rnta;

pub use crate::catalogue::CATALOGUE;
pub use crate::catalogue::CatalogueEntry;
pub use crate::catalogue::Model;
pub use crate::catalogue::Problem;
pub use crate::catalogue::Status;
pub use crate::dropping::name_dropping;
pub use crate::handle::Arity;
pub use crate::handle::AutomatonError;
pub use crate::handle::Configuration;
pub use crate::handle::Control;
pub use crate::handle::Controls;
pub use crate::handle::Degree;
pub use crate::handle::Freshness;
pub use crate::handle::Membership;
pub use crate::handle::Register;
pub use crate::handle::Store;
pub use crate::handle::StoreError;
pub use crate::handle::Transfer;
pub use crate::letter::Letter;
pub use crate::nda::Nda;
pub use crate::nda::Rule;
pub use crate::nda::RuleKind;
pub use crate::rnna::Rnna;
pub use crate::rnna::RnnaRule;
pub use crate::rnna::RnnaRuleKind;
pub use crate::rnta::ChildTarget;
pub use crate::rnta::NodeKind;
pub use crate::rnta::Rnta;
pub use crate::rnta::RntaRule;
pub use crate::rnta::Term;

/// The dense identity an [`Atom`] carries within its sort.
///
/// A transparent `u32` carrier matches the pipeline's `HoleId` without changing
/// layout, while the wrapper keeps atom identities from crossing crate-defined
/// signatures as anonymous integers.
///
/// # Contract
/// - provides: a copyable dense identity scoped by an atom's sort.
/// - panics: none.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AtomId(u32);

impl AtomId
{
    /// The first identity minted by a new [`Gensym`].
    pub const ZERO: Self = Self(0);
    /// The terminal counter value at which [`Gensym::fresh`] reports
    /// [`GensymExhausted`].
    pub const MAX: Self = Self(u32::MAX);

    /// Return the identity after one successful mint.
    #[inline]
    #[must_use]
    const fn successor(self) -> Option<Self>
    {
        match self.0.checked_add(1) {
            | Some(next) => Some(Self(next)),
            | None => None,
        }
    }
}

impl From<AtomId> for u32
{
    #[inline]
    fn from(id: AtomId) -> Self
    {
        id.0
    }
}

/// Semantic truth value for the atom-role versus variable-role boundary.
///
/// # Contract
/// - provides: a named boundary around whether a sort denotes substitutable
///   unification variables.
/// - panics: none.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Unifiability(bool);

impl Unifiability
{
    /// Pure atom-role names: minted, compared, freshness-tested, and permuted.
    pub const ATOM_ROLE: Self = Self(false);
    /// Variable-role names: substitutable unknowns in a solver domain.
    pub const VARIABLE_ROLE: Self = Self(true);
}

impl From<Unifiability> for bool
{
    #[inline]
    fn from(unifiability: Unifiability) -> Self
    {
        unifiability.0
    }
}

/// A **sort** classifying the atoms one [`Gensym`] mints.
///
/// gandr instantiates this with a single `GandrSort` enum whose variants tag
/// each machine-minted name-space; the crate stays generic so it carries no
/// gandr-specific vocabulary. The only behaviour a sort must supply is the
/// atom-vs-variable role split ([`Self::is_unifiable`]).
pub trait Sort: Copy + Eq + core::fmt::Debug
{
    /// Whether atoms of this sort are **unification variables** — substitutable
    /// unknowns that enter a substitution's domain (`π·X`) — rather than pure
    /// **atom-role** names (minted, compared, freshness-tested, permuted only).
    ///
    /// Keeping the two roles in disjoint sorts is what preserves *unitary*
    /// most-general unifiers (Urban–Pitts–Gabbay, *Nominal Unification*, TCS
    /// 323, 2004); see the module documentation.
    fn is_unifiable(&self) -> Unifiability;
}

/// Exhaustion of one [`Gensym`]'s atom-id space.
///
/// # Contract
/// - provides: the sort whose allocator is exhausted and the terminal counter
///   value at which minting stopped.
/// - ensures: receiving this error means no atom was minted and the allocator's
///   counter was left unchanged.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GensymExhausted<S>
{
    /// The sort whose allocator cannot mint another atom.
    sort: S,
    /// The terminal counter value that has no successor.
    exhausted: AtomId,
}

impl<S> GensymExhausted<S>
where
    S: Sort,
{
    /// The sort whose allocator is exhausted.
    #[inline]
    #[must_use]
    pub fn sort(&self) -> S
    {
        self.sort
    }

    /// The terminal counter value that has no successor.
    #[inline]
    #[must_use]
    pub fn exhausted_id(&self) -> AtomId
    {
        self.exhausted
    }
}

impl<S> core::fmt::Display for GensymExhausted<S>
where
    S: Sort,
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        write!(
            f,
            "atom id space is exhausted at {}",
            u32::from(self.exhausted)
        )
    }
}

impl<S> core::error::Error for GensymExhausted<S> where S: Sort
{
}

/// A **nominal atom**: a sort-tagged, machine-minted name.
///
/// Two atoms are equal exactly when they share a sort *and* an identity, so an
/// atom-role and a variable-role atom that happen to share an [`AtomId`] are
/// still distinct. Atoms are minted only by [`Gensym::fresh`]; their fields are
/// private, so an atom can never be forged with an arbitrary identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Atom<S>
{
    /// The sort this atom belongs to — its role tag.
    sort: S,
    /// The atom's identity within its sort; assigned distinctly per
    /// [`Gensym::fresh`].
    id: AtomId,
}

impl<S> Atom<S>
where
    S: Sort,
{
    /// This atom's sort.
    #[inline]
    #[must_use]
    pub fn sort(&self) -> S
    {
        self.sort
    }

    /// This atom's identity within its sort.
    #[inline]
    #[must_use]
    pub fn id(&self) -> AtomId
    {
        self.id
    }

    /// Whether this atom is a unification variable ([`Sort::is_unifiable`] of
    /// its sort).
    #[inline]
    #[must_use]
    pub fn is_unifiable(&self) -> Unifiability
    {
        self.sort.is_unifiable()
    }
}

/// A **monotone atom allocator** for one sort: the M1 allocation discipline.
///
/// Each [`Self::fresh`] hands out the next identity from a strictly increasing
/// counter, so every atom one `Gensym` mints is distinct from all the others it
/// has minted. The counter is never rolled back — identities leak harmlessly on
/// backtrack rather than being recycled (ADR-41 D2) — which is exactly why the
/// distinctness it guarantees is sound regardless of the consumer's
/// speculative-execution / rollback behaviour.
///
/// **Cloning forks the mint sequence.** A clone continues from the same `next`,
/// so the two allocators thereafter mint the *same* identities — the
/// backtrack-safety above is about rewinding one allocator, not duplicating it.
/// `Copy` is therefore deliberately **not** derived (so a fork is always an
/// explicit `clone`); `Clone` is kept only because the embedding state needs it
/// (e.g. the typing machine's cloneable, resumable state), and a clone must not
/// mint into the same atom space as its origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Gensym<S>
{
    /// The sort every atom this allocator mints carries.
    sort: S,
    /// The identity the next [`Self::fresh`] will assign; monotone.
    next: AtomId,
}

impl<S> Gensym<S>
where
    S: Sort,
{
    /// A fresh allocator for `sort`, minting identities from `0`.
    #[inline]
    #[must_use]
    pub fn new(sort: S) -> Self
    {
        Self {
            sort,
            next: AtomId::ZERO,
        }
    }

    /// Tries to mint the next atom of this allocator's sort.
    ///
    /// # Errors
    /// Returns [`GensymExhausted`] when no successor identity remains.
    ///
    /// # Contract
    /// - ensures: on success, the returned atom is **distinct** from every atom
    ///   this allocator has previously minted (its identity is the strictly
    ///   increasing counter) — so all atoms from one `Gensym` are pairwise
    ///   distinct: the per-allocator address-distinctness invariant.
    /// - ensures: on success, identities are **monotone and never reused** —
    ///   not recycled on the consumer's rollback (M1; ADR-41 D2).
    /// - errors: returns [`GensymExhausted`] when the counter has reached
    ///   [`AtomId::MAX`]; in that case no atom is minted and the counter is
    ///   left unchanged, so exhaustion can never duplicate an identity.
    /// - panics: none.
    #[inline]
    pub fn fresh(&mut self) -> Result<Atom<S>, GensymExhausted<S>>
    {
        let id = self.next;
        let next = id.successor().ok_or(GensymExhausted {
            sort: self.sort,
            exhausted: id,
        })?;
        self.next = next;
        Ok(Atom {
            sort: self.sort,
            id,
        })
    }

    /// The sort this allocator stamps onto every atom it mints.
    #[inline]
    #[must_use]
    pub fn sort(&self) -> S
    {
        self.sort
    }

    /// The number of atoms minted so far. While below [`AtomId::MAX`], this is
    /// also the identity the next successful [`Self::fresh`] will assign; at
    /// [`AtomId::MAX`], the allocator is exhausted.
    #[inline]
    #[must_use]
    pub fn minted(&self) -> AtomId
    {
        self.next
    }
}

#[cfg(test)]
mod tests
{
    use proptest::prelude::*;

    use super::AtomId;
    use super::Gensym;
    use super::Sort;
    use super::Unifiability;

    /// A two-role sort used only by the tests: one atom-role and one
    /// variable-role variant, enough to exercise the `is_unifiable` gate and
    /// cross-sort distinctness.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Role
    {
        /// An atom-role sort (not a unification variable).
        Addr,
        /// A variable-role sort (a unification variable).
        Var,
    }

    impl Sort for Role
    {
        #[inline]
        fn is_unifiable(&self) -> Unifiability
        {
            match *self {
                | Self::Addr => Unifiability::ATOM_ROLE,
                | Self::Var => Unifiability::VARIABLE_ROLE,
            }
        }
    }

    /// Atoms minted by one allocator carry the contiguous identities `0, 1, …`
    /// and the `minted` count tracks them — so the names are pairwise distinct,
    /// the global invariant that source-name keying once violated.
    #[test]
    fn one_allocator_mints_contiguous_distinct_identities()
    {
        const CEILING: AtomId = AtomId(1_000);
        let mut gensym = Gensym::new(Role::Addr);
        let mut expected = AtomId::ZERO;
        while expected < CEILING {
            assert_eq!(
                gensym
                    .fresh()
                    .expect("test ceiling is far below atom-id exhaustion")
                    .id(),
                expected,
                "ids are the contiguous mint order"
            );
            expected = expected
                .successor()
                .expect("test ceiling is far below atom-id exhaustion");
        }
        assert_eq!(
            CEILING,
            gensym.minted(),
            "minted count tracks the allocator"
        );
    }

    /// Equal identities under *different* sorts are still distinct atoms — atom
    /// identity is `(sort, id)`, not `id` alone.
    #[test]
    fn equal_ids_in_different_sorts_are_distinct()
    {
        let mut addr = Gensym::new(Role::Addr);
        let mut var = Gensym::new(Role::Var);
        let left = addr
            .fresh()
            .expect("a new allocator can mint the first atom");
        let right = var
            .fresh()
            .expect("a new allocator can mint the first atom");
        assert_eq!(
            left.id(),
            right.id(),
            "the two allocators are at the same identity"
        );
        assert_ne!(
            left, right,
            "but the atoms differ because their sorts differ"
        );
    }

    /// The allocator stamps its own sort onto every atom it mints, and both the
    /// allocator's [`Gensym::sort`] and the minted atom's [`super::Atom::sort`]
    /// report that same sort — across both roles.
    #[test]
    fn sort_is_carried_from_allocator_to_minted_atom()
    {
        for role in [Role::Addr, Role::Var] {
            let mut gensym = Gensym::new(role);
            assert_eq!(
                gensym.sort(),
                role,
                "the allocator reports the sort it was constructed with"
            );
            let atom = gensym
                .fresh()
                .expect("a new allocator can mint the first atom");
            assert_eq!(
                atom.sort(),
                role,
                "the minted atom carries the allocator's sort"
            );
        }
    }

    /// The `is_unifiable` gate reflects the sort's role on the minted atom.
    #[test]
    fn is_unifiable_reflects_the_sort_role()
    {
        let mut addr = Gensym::new(Role::Addr);
        let mut var = Gensym::new(Role::Var);
        assert_eq!(
            Unifiability::ATOM_ROLE,
            addr.fresh()
                .expect("a new allocator can mint the first atom")
                .is_unifiable(),
            "atom-role atoms are not unifiable"
        );
        assert_eq!(
            Unifiability::VARIABLE_ROLE,
            var.fresh()
                .expect("a new allocator can mint the first atom")
                .is_unifiable(),
            "variable-role atoms are unifiable"
        );
    }

    /// Once the counter reaches the terminal value, `fresh` returns a typed
    /// error without moving the counter; repeated calls therefore cannot mint a
    /// duplicate terminal id.
    #[test]
    fn max_id_exhaustion_returns_error_without_duplicate_or_mutation()
    {
        let mut gensym = Gensym {
            sort: Role::Addr,
            next: AtomId(u32::MAX - 1),
        };

        let last = gensym
            .fresh()
            .expect("the id before exhaustion still has a successor");
        assert_eq!(
            AtomId(u32::MAX - 1),
            last.id(),
            "the final successful mint uses the last id with a successor"
        );
        assert_eq!(
            AtomId::MAX,
            gensym.minted(),
            "the counter advances to the exhausted terminal value"
        );

        let first_error = gensym
            .fresh()
            .expect_err("the terminal counter value has no successor");
        assert_eq!(Role::Addr, first_error.sort());
        assert_eq!(AtomId::MAX, first_error.exhausted_id());
        assert_eq!(
            AtomId::MAX,
            gensym.minted(),
            "exhaustion leaves the counter unchanged"
        );

        let second_error = gensym
            .fresh()
            .expect_err("exhaustion remains an error instead of reusing an id");
        assert_eq!(
            first_error, second_error,
            "repeated exhaustion reports the same state without minting"
        );
        assert_eq!(
            AtomId::MAX,
            gensym.minted(),
            "repeated exhaustion still leaves the counter unchanged"
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// For any mint count the identities an allocator hands out are strictly
        /// increasing, hence pairwise distinct — distinctness holds at every run
        /// length, not just the fixed unit-test count.
        #[test]
        fn minted_identities_strictly_increase(count in 1u32 .. 4_096)
        {
            let mut gensym = Gensym::new(Role::Var);
            let count = AtomId(count);
            let mut prev = gensym
                .fresh()
                .expect("proptest range is far below atom-id exhaustion")
                .id();
            let mut seen = AtomId::ZERO
                .successor()
                .expect("zero has a successor");
            while seen < count {
                let next = gensym
                    .fresh()
                    .expect("proptest range is far below atom-id exhaustion")
                    .id();
                prop_assert!(next > prev, "ids strictly increase, hence are distinct");
                prev = next;
                seen = seen
                    .successor()
                    .expect("proptest range is far below atom-id exhaustion");
            }
        }
    }
}
