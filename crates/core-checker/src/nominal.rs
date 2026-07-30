//! gandr's nominal **sort vocabulary** (`docs/adr/` ADR-41).
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
    /// **Atom-role.** A machine continuation-environment key (rendered
    /// `%k{id}`): the fresh name `shift` / `perform` α-rename their captured
    /// binder to, so distinct captures never collide.
    ContKey,
    /// **Atom-role.** A pipeline hoist binder (rendered `%tmp{n}`): the fresh
    /// name a synthesized `Bind` introduces when a value position is lifted to
    /// a computation (the lowerer).
    TmpHoist,
    /// **Atom-role.** A pipeline hole *address* ([`crate::syntax::HoleId`]):
    /// typing-transparent addressing, **not** a unification variable — two
    /// holes with different identifiers type identically (ADR-41 D3). The
    /// Ψ+σ-bearing object is the CMTT staging node, not an atom.
    HoleAddr,
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

impl Sort for GandrSort
{
    #[inline]
    fn is_unifiable(&self) -> Unifiability
    {
        match *self {
            | Self::ContKey | Self::TmpHoist | Self::HoleAddr => Unifiability::ATOM_ROLE,
            | Self::TyVar | Self::GradeVar => Unifiability::VARIABLE_ROLE,
        }
    }
}
