//! The finitary orbit-level representation shared by all three models
//! (design doc: the nominal-automata study §6.4, departed with the research
//! corpus).
//!
//! Control points, registers, arities, and the partial injective register
//! stores make an orbit-infinite nominal automaton a finite handle. The papers'
//! finitary representation of an orbit-finite **strong nominal set** is `X =
//! Σ_{i∈I} 𝔸^{#Xᵢ}` with `Xᵢ` finite: a concrete state is a **control point**
//! `i` together with an injective assignment of names to `Xᵢ`'s registers —
//! "states = control state `i` + a store assigning names to registers in a
//! duplicate-free manner" (NDA §5, before Lemma 5.1; RNTA §5, Lemma 5.1). The
//! **name-dropping** modifications (RNTA Def 5.2; NDA Constr 6.3) relax the
//! store to a *partial* injective map `𝔸^{$X}` — registers may be empty — which
//! is why [`Store`] slots are optional and why the orbit count grows by at most
//! `2^degree` (RNTA Thm 5.5). The **degree** of an automaton — the maximum
//! number of concurrently-live names — is the complexity parameter of every
//! decision procedure in the catalogue (design doc §5; NDA Def 5.1; RNTA Def
//! 4.1).

use alloc::vec::Vec;

use crate::Atom;
use crate::Sort;

/// A **control point**: the orbit index `i` in the strong-nominal-set
/// coproduct `⊔ⱼ 𝔸^{#nⱼ}` (design doc §6.4).
///
/// Equivariant subsets of the state space — the final set of an NDA (Def
/// 5.1), the rewrite-source partition of an RNTA (Def 4.1) — are unions of
/// orbits, hence sets of control points.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Control(u32);

impl Control
{
    /// The first control point of an automaton.
    pub const ZERO: Self = Self(0);
}

impl From<u32> for Control
{
    #[inline]
    fn from(index: u32) -> Self
    {
        Self(index)
    }
}

impl From<usize> for Control
{
    #[inline]
    fn from(index: usize) -> Self
    {
        // Saturates only on platforms where a length cannot fit a `u32`.
        Self(u32::try_from(index).unwrap_or(u32::MAX))
    }
}

impl From<Control> for u32
{
    #[inline]
    fn from(control: Control) -> Self
    {
        control.0
    }
}

impl From<Control> for usize
{
    #[inline]
    fn from(control: Control) -> Self
    {
        // Lossless on every target with `usize` at least 32 bits wide; the
        // fallback only exists so the conversion stays total.
        Self::try_from(control.0).unwrap_or(Self::MAX)
    }
}

/// A **register** of a control point's store.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Register(u32);

impl Register
{
    /// The first register of a store.
    pub const ZERO: Self = Self(0);
}

impl From<u32> for Register
{
    #[inline]
    fn from(index: u32) -> Self
    {
        Self(index)
    }
}

impl From<Register> for u32
{
    #[inline]
    fn from(register: Register) -> Self
    {
        register.0
    }
}

impl From<Register> for usize
{
    #[inline]
    fn from(register: Register) -> Self
    {
        // Lossless on every target with `usize` at least 32 bits wide; the
        // fallback only exists so the conversion stays total.
        Self::try_from(register.0).unwrap_or(Self::MAX)
    }
}

/// The register count of one control point.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Arity(u32);

impl Arity
{
    /// The arity of a control point with no registers.
    pub const ZERO: Self = Self(0);
}

impl From<Arity> for u32
{
    #[inline]
    fn from(arity: Arity) -> Self
    {
        arity.0
    }
}

impl From<usize> for Arity
{
    #[inline]
    fn from(count: usize) -> Self
    {
        // Saturates only on platforms where a length cannot fit a `u32`.
        Self(u32::try_from(count).unwrap_or(u32::MAX))
    }
}

/// The number of control points of an automaton's finite handle.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Controls(u32);

impl From<Controls> for u32
{
    #[inline]
    fn from(controls: Controls) -> Self
    {
        controls.0
    }
}

impl From<usize> for Controls
{
    #[inline]
    fn from(count: usize) -> Self
    {
        // Saturates only on platforms where a length cannot fit a `u32`.
        Self(u32::try_from(count).unwrap_or(u32::MAX))
    }
}

/// The **degree** of an automaton: the maximum number of concurrently-live
/// names over its control points.
///
/// It "corresponds morally to the number of registers" (RNTA §4; NDA Def
/// 5.1 `deg(A) = max |supp(x)|`).
///
/// Every decision procedure in the catalogue is parametrized by the degree
/// (RNTA Thm 6.3/6.6 singly-exponential-parametrized; NDA Thm 5.12
/// PSPACE-parametrized), so it is surfaced as a first-class value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Degree(u32);

impl Degree
{
    /// The degree of an automaton whose control points carry no registers.
    pub const ZERO: Self = Self(0);
}

impl From<Degree> for u32
{
    #[inline]
    fn from(degree: Degree) -> Self
    {
        degree.0
    }
}

impl From<Arity> for Degree
{
    #[inline]
    fn from(arity: Arity) -> Self
    {
        Self(arity.0)
    }
}

/// Rejection of a [`Store`] construction.
///
/// # Contract
/// - provides: the atom whose duplicated assignment violated injectivity.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StoreError<S>
{
    /// The same atom was assigned to two registers; stores assign names to
    /// registers "in a duplicate-free manner" (NDA §5).
    DuplicateName
    {
        /// The atom assigned more than once.
        atom: Atom<S>,
    },
}

impl<S> core::fmt::Display for StoreError<S>
where
    S: Sort,
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::DuplicateName { atom } => write!(
                f,
                "register store assigns atom {} to two registers",
                u32::from(atom.id())
            ),
        }
    }
}

impl<S> core::error::Error for StoreError<S> where S: Sort
{
}

/// A **partial injective register store**: the `𝔸^{$X}` assignment of
/// currently-live names to a control point's registers (RNTA Def 5.2; NDA
/// §6).
///
/// Slots are optional because the name-dropping modification lets a run
/// forget names without deallocating them (the lossiness of RNTA Remark
/// 5.6). Injectivity — no atom held by two registers — is the register
/// paradigm's duplicate-freeness (NDA §5) and is enforced by
/// [`Store::try_new`]; the crate's internal transfer application preserves
/// it by construction and uses [`Store::from_validated`].
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Store<S>
{
    /// The slot contents, one per register; `None` marks an empty register.
    slots: Vec<Option<Atom<S>>>,
}

impl<S> Store<S>
where
    S: Sort,
{
    /// Build a store from explicit slot contents, checking injectivity.
    ///
    /// # Contract
    /// - ensures: on success, no atom appears in two slots.
    /// - provides: a partial injective register store.
    /// - fails: [`StoreError::DuplicateName`] when an atom is assigned twice.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`StoreError::DuplicateName`] with the duplicated atom.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — the injectivity scan is separated from the
    ///   acceptance path solely by the presence or absence of one duplicated
    ///   atom, plus one ordinary injective store asserted exactly.
    /// - witness: `handle::tests::duplicate_assignment_is_rejected`
    /// - witness: `handle::tests::injective_partial_store_is_accepted`
    #[inline]
    pub fn try_new(slots: Vec<Option<Atom<S>>>) -> Result<Self, StoreError<S>>
    {
        for (position, slot) in slots.iter().enumerate() {
            let Some(atom) = *slot
            else {
                continue;
            };
            if slots
                .iter()
                .skip(position.saturating_add(1))
                .any(|later| later == slot)
            {
                return Err(StoreError::DuplicateName { atom });
            }
        }
        return Ok(Self { slots });
    }

    /// A store with `arity` registers, all empty.
    #[inline]
    #[must_use]
    pub fn empty(arity: Arity) -> Self
    {
        return Self {
            slots: alloc::vec![None; usize::try_from(u32::from(arity)).unwrap_or(usize::MAX)],
        };
    }

    /// The store's register count.
    #[inline]
    #[must_use]
    pub fn arity(&self) -> Arity
    {
        return Arity::from(self.slots.len());
    }

    /// The name held in `register`, if the register exists and is filled.
    #[inline]
    #[must_use]
    pub fn name(
        &self,
        register: Register,
    ) -> Option<Atom<S>>
    {
        return self.slots.get(usize::from(register)).copied().flatten();
    }

    /// The freshness test `a # store` (NDA §2): is `atom` absent from every
    /// register — the gate on allocation transitions (NDA Lemma 5.7(2)).
    #[inline]
    #[must_use]
    pub fn freshness(
        &self,
        atom: Atom<S>,
    ) -> Freshness
    {
        return Freshness(!self.slots.contains(&Some(atom)));
    }

    /// Build a store from slots whose injectivity is already established.
    ///
    /// Transfer application preserves injectivity: `Keep` entries copy from
    /// an injective source store, and the single `Allocated` entry carries a
    /// name the freshness gate proved absent from that store.
    ///
    /// # Contract
    /// - requires: `slots` contains no atom twice.
    /// - provides: a partial injective register store.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub(crate) fn from_validated(slots: Vec<Option<Atom<S>>>) -> Self
    {
        return Self { slots };
    }
}

/// A concrete automaton state: a control point plus the register store of
/// currently-live names — the `(i, r)` of the strong-nominal-set
/// representation (design doc §6.4; NDA §5; RNTA §4).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Configuration<S>
{
    /// The control point (orbit index).
    control: Control,
    /// The register store of currently-live names.
    store: Store<S>,
}

impl<S> Configuration<S>
where
    S: Sort,
{
    /// Pair a control point with a store.
    ///
    /// The agreement of the store's length with the control point's arity is
    /// an automaton-level invariant, checked by the automaton constructors.
    #[inline]
    #[must_use]
    pub fn new(
        control: Control,
        store: Store<S>,
    ) -> Self
    {
        return Self { control, store };
    }

    /// The configuration's control point.
    #[inline]
    #[must_use]
    pub fn control(&self) -> Control
    {
        return self.control;
    }

    /// The configuration's register store.
    #[inline]
    #[must_use]
    pub fn store(&self) -> &Store<S>
    {
        return &self.store;
    }
}

/// How one target register is populated by a transition firing.
///
/// This is the defunctionalized register reassignment of the support lemmas
/// (NDA Lemma 5.7; RNTA Lemma 4.2), shared by all three models: a target
/// register either carries a source register's name over, receives the name
/// the transition allocates, or starts empty.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Transfer
{
    /// Carry over the name currently held in the given source register; an
    /// empty source register yields an empty target register (stores are
    /// partial once name-dropping is applied).
    Keep(Register),
    /// Populate with the name this transition allocates — permitted only in
    /// an allocating rule, at most once per transfer (NDA Lemma 5.7(2);
    /// RNTA Lemma 4.2(2)).
    Allocated,
    /// Leave the target register empty.
    Empty,
}

/// The verdict of a membership query.
///
/// # Contract
/// - provides: a named acceptance verdict rather than a bare boolean.
/// - panics: none.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Membership(bool);

impl Membership
{
    /// The queried word or tree is accepted.
    pub const ACCEPTED: Self = Self(true);
    /// The queried word or tree is rejected.
    pub const REJECTED: Self = Self(false);
}

impl From<Membership> for bool
{
    #[inline]
    fn from(membership: Membership) -> Self
    {
        membership.0
    }
}

/// Whether an atom is **fresh for** (`#`) a structure: absent from its
/// support (NDA §2; RNTA §2).
///
/// # Contract
/// - provides: a named freshness verdict rather than a bare boolean.
/// - panics: none.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Freshness(bool);

impl Freshness
{
    /// The atom is absent from the structure's support.
    pub const FRESH: Self = Self(true);
    /// The atom occurs in the structure's support.
    pub const NOT_FRESH: Self = Self(false);
}

impl From<Freshness> for bool
{
    #[inline]
    fn from(freshness: Freshness) -> Self
    {
        freshness.0
    }
}

/// Rejection of an automaton construction: the finite handle failed a
/// well-formedness check of its model's definition.
///
/// # Contract
/// - provides: which structural invariant failed, with the offending control
///   point or register.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AutomatonError
{
    /// A rule, the initial configuration, or the final set names a control
    /// point outside the control table.
    InvalidControl
    {
        /// The offending control index.
        control: Control,
        /// The number of control points the automaton actually has.
        controls: u32,
    },
    /// A store or transfer length disagrees with a control point's arity.
    ArityMismatch
    {
        /// The control point whose arity is violated.
        control: Control,
        /// The control point's declared register count.
        expected: Arity,
        /// The length actually supplied.
        actual: Arity,
    },
    /// A rule reads, writes, or drops a register its source control point
    /// does not have.
    UnknownRegister
    {
        /// The control point the rule fires from.
        control: Control,
        /// The out-of-range register.
        register: Register,
    },
    /// A transfer carries the allocated name in a rule that allocates no
    /// name, or carries it more than once.
    MisplacedAllocatedName
    {
        /// The control point the rule fires from.
        control: Control,
    },
    /// A deallocation rule carries the deallocated register's name into its
    /// target store — violating the name-erasure condition of NDA Def 5.1
    /// ("deallocated names are actually forgotten").
    KeptDeallocatedName
    {
        /// The control point the rule fires from.
        control: Control,
        /// The register whose deallocated name is kept.
        register: Register,
    },
}

impl core::fmt::Display for AutomatonError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::InvalidControl { control, controls } => write!(
                f,
                "control {} is outside the control table of {} control points",
                u32::from(control),
                controls
            ),
            | Self::ArityMismatch {
                control,
                expected,
                actual,
            } => write!(
                f,
                "control {} has arity {} but a store or transfer of length {} was supplied",
                u32::from(control),
                u32::from(expected),
                u32::from(actual)
            ),
            | Self::UnknownRegister { control, register } => write!(
                f,
                "a rule from control {} names register {}, which is out of range",
                u32::from(control),
                u32::from(register)
            ),
            | Self::MisplacedAllocatedName { control } => write!(
                f,
                "a rule from control {} places the allocated name without allocating, or places it twice",
                u32::from(control)
            ),
            | Self::KeptDeallocatedName { control, register } => write!(
                f,
                "a deallocation rule from control {} keeps the deallocated register {}",
                u32::from(control),
                u32::from(register)
            ),
        }
    }
}

impl core::error::Error for AutomatonError
{
}

#[cfg(test)]
mod tests
{
    use alloc::vec;

    use super::Arity;
    use super::Atom;
    use super::Control;
    use super::Freshness;
    use super::Register;
    use super::Sort;
    use super::Store;
    use super::StoreError;
    use crate::Gensym;
    use crate::Unifiability;

    /// A single-role sort for the store tests.
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum Role
    {
        /// An atom-role sort.
        Name,
    }

    impl Sort for Role
    {
        #[inline]
        fn is_unifiable(&self) -> Unifiability
        {
            Unifiability::ATOM_ROLE
        }
    }

    /// A store assigning the same atom to two registers is not injective and
    /// is rejected with the duplicated atom named.
    #[test]
    fn duplicate_assignment_is_rejected()
    {
        let mut gensym = Gensym::new(Role::Name);
        let atom = gensym
            .fresh()
            .expect("a new allocator can mint the first atom");
        let result = Store::try_new(vec![Some(atom), None, Some(atom)]);
        assert_eq!(
            result.expect_err("a duplicated atom violates injectivity"),
            StoreError::DuplicateName { atom },
            "the error names the duplicated atom"
        );
    }

    /// An injective store with an empty middle register is accepted, reports
    /// its arity, and answers per-register lookups.
    #[test]
    fn injective_partial_store_is_accepted()
    {
        let mut gensym = Gensym::new(Role::Name);
        let left = gensym
            .fresh()
            .expect("a new allocator can mint the first atom");
        let right = gensym
            .fresh()
            .expect("a new allocator can mint the second atom");
        let store = Store::try_new(vec![Some(left), None, Some(right)])
            .expect("an injective store is valid");
        assert_eq!(store.arity(), Arity::from(3_usize));
        assert_eq!(store.name(Register::ZERO), Some(left));
        assert_eq!(
            None,
            store.name(Register(1)),
            "the middle register is empty"
        );
        assert_eq!(store.name(Register(2)), Some(right));
        assert_eq!(
            Freshness::NOT_FRESH,
            store.freshness(right),
            "a held name is not fresh for the store"
        );
        assert_eq!(
            Freshness::FRESH,
            store.freshness(
                gensym
                    .fresh()
                    .expect("a fresh atom is absent from the store")
            ),
            "a minted-fresh atom is fresh for the store"
        );
    }

    /// `Store::empty` builds a fully-empty store of the requested arity.
    #[test]
    fn empty_store_has_only_empty_registers()
    {
        let store: Store<Role> = Store::empty(Arity::from(2_usize));
        assert_eq!(store.arity(), Arity::from(2_usize));
        assert_eq!(None, store.name(Register::ZERO));
        assert_eq!(None, store.name(Register(1)));
    }

    /// The [`super::Configuration`] pairing exposes exactly what was paired.
    #[test]
    fn configuration_exposes_control_and_store()
    {
        let mut gensym = Gensym::new(Role::Name);
        let atom = gensym
            .fresh()
            .expect("a new allocator can mint the first atom");
        let store = Store::try_new(vec![Some(atom)]).expect("an injective store is valid");
        let configuration = super::Configuration::new(Control::ZERO, store);
        assert_eq!(Control::ZERO, configuration.control());
        assert_eq!(configuration.store().name(Register::ZERO), Some(atom));
    }

    /// Unused-import guard for the test-module `Atom` path.
    #[test]
    fn atom_path_is_exercised()
    {
        let mut gensym = Gensym::new(Role::Name);
        let atom: Atom<Role> = gensym
            .fresh()
            .expect("a new allocator can mint the first atom");
        assert_eq!(Role::Name, atom.sort());
    }
}
