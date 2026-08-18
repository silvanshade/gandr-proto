//! gandr's nominal **sort vocabulary** (ADR-41).
//!
//! [`GandrSort`] tags every machine-minted name-space gandr allocates onto the
//! one shared atom substrate ([`gandr_theory_nominal_automata`]). The substrate
//! is generic over the sort; this enum is the gandr-specific instantiation, and
//! its [`gandr_theory_nominal_automata::Sort::is_unifiable`] implementation
//! draws the load-bearing **atom-role vs variable-role** boundary (ADR-41 D3).

use gandr_theory_nominal_automata::Sort;
use gandr_theory_nominal_automata::Unifiability;

/// The sorts of machine-minted atom gandr allocates.
///
/// The split is by **role**, the boundary
/// [`gandr_theory_nominal_automata::Sort::is_unifiable`] reports: **atom-role**
/// sorts are pure names (minted, compared, freshness- tested, eventually
/// permuted); **variable-role** sorts are substitutable unknowns that enter a
/// solver substitution's domain. Keeping the two disjoint is what preserves
/// unitary most-general unifiers (Urban–Pitts–Gabbay, *Nominal Unification*,
/// TCS 323, 2004); see ADR-41.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GandrSort
{
    /// **Atom-role, reserved.** A machine continuation-environment key
    /// (rendered `%k{id}`): the fresh name `shift` / `perform` α-rename their
    /// captured binder to, so distinct captures never collide.
    ///
    /// **Nothing constructs this sort yet.** The keys are still minted by
    /// `gandr-core-sequent`'s focusing pass, from a bare monotone counter
    /// formatted into the reserved `%`-prefixed namespace; routing them through
    /// [`gandr_theory_nominal_automata::Gensym`] is the migration this variant
    /// is reserved for and is not the state of the tree.
    ContKey,
    /// **Atom-role.** A pipeline hoist binder (rendered `%tmp{n}`): the fresh
    /// name a synthesized `Bind` introduces when a value position is lifted to
    /// a computation (the lowerer).
    TmpHoist,
    /// **Atom-role.** A pipeline hole *address*
    /// ([`crate::syntax::HoleId`]): typing-transparent addressing,
    /// **not** a unification variable — two holes with different
    /// identifiers type identically (ADR-41 D3). The Ψ+σ-bearing object is
    /// the CMTT staging node, not an atom.
    HoleAddr,
    /// **Atom-role, reserved.** A sealed abstract type's identity: the name an
    /// opaque ascription binds one abstract type component to.
    ///
    /// **Sealing does not mint through [`gandr_theory_nominal_automata`], and
    /// the divergence is deliberate.** A seal's identity must be a *function of
    /// its sealing site*, so that an admission point can re-elaborate, re-mint
    /// and refuse a recorded sequence a re-run does not reproduce; a monotone
    /// allocator offers never-reused identities, which is a different property.
    /// `gandr_core_checker::judgements::seal::SealTable` therefore assigns
    /// positional serials keyed on
    /// `gandr_core_checker::judgements::seal::SealSite`, and this
    /// variant records that a sealed atom is atom-role rather than that it
    /// comes from the shared allocator.
    ///
    /// It is a name and never an unknown, which is the load-bearing half. A
    /// sealed type is not a type the checker is waiting to learn — it is one
    /// there is nothing further to learn about, so it must never enter a
    /// solver substitution's domain. Putting it on the atom side of
    /// [`Sort::is_unifiable`] is what makes that structural rather than a
    /// convention someone has to remember.
    SealAtom,
    /// **Variable-role (reserved).** A solver type-variable: a unification
    /// unknown that a substitution binds. No consumer in v0 — the solver is not
    /// yet built; the sort is reserved so the boundary is documented from day
    /// one (ADR-41 D3).
    TyVar,
    /// **Variable-role (reserved).** A solver grade-variable: the coeffect-side
    /// counterpart of [`Self::TyVar`], likewise reserved until the solver
    /// lands.
    GradeVar,
}

/// # Adequacy
/// - hypothesis: L3 — every atom-role variant and every variable-role variant
///   must remain on opposite sides of the unifiability boundary.
/// - witness: `tests::sort_roles_are_exhaustive_and_disjoint`
impl Sort for GandrSort
{
    #[inline]
    fn is_unifiable(&self) -> Unifiability
    {
        match *self {
            | Self::ContKey | Self::TmpHoist | Self::HoleAddr | Self::SealAtom => {
                Unifiability::ATOM_ROLE
            },
            | Self::TyVar | Self::GradeVar => Unifiability::VARIABLE_ROLE,
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn sort_roles_are_exhaustive_and_disjoint()
    {
        let atom_roles = [
            GandrSort::ContKey,
            GandrSort::TmpHoist,
            GandrSort::HoleAddr,
            GandrSort::SealAtom,
        ];
        for sort in atom_roles {
            assert_eq!(sort.is_unifiable(), Unifiability::ATOM_ROLE);
        }

        let variable_roles = [GandrSort::TyVar, GandrSort::GradeVar];
        for sort in variable_roles {
            assert_eq!(sort.is_unifiable(), Unifiability::VARIABLE_ROLE);
        }
    }
}
