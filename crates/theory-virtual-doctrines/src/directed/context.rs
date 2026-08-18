//! **Ł1 — variance-sorted reflected contexts** (`proposal-vdc-reflection.md`
//! §7, Ł1; §4.2; ADR-68/69).
//!
//! LLV's contexts sort object variables by **variance**: a variable is either
//! covariant (`x : I`, a target / producer slot) or contravariant (`x : Iᵒᵖ`, a
//! source / consumer slot). This module reflects that discipline over the
//! rewrite layer, and — the payoff §7 Ł1 names — turns the §4.2 engine variance
//! metadata (derived live by [`gandr_theory_cell_complexes::CellMeta`]) from
//! *derived* into *checkable*: [`DirectedContext::check_cell_variance`] rejects
//! an engine cell whose hole occurs at a polarity the context did not sort it
//! at.
//!
//! # Design record — `−ᵒᵖ` on reflected signatures only
//!
//! The involution `−ᵒᵖ` LLV adds is realized **exclusively** on the reflected
//! [`OpSig`] ([`OpSig::op`]) — the frozen core [`crate::vdc::SignatureRef`]
//! carries no `op` and is not touched. LLV's types include `−ᵒᵖ` and functor
//! categories; the reflection face adds `op` on *reflected* signatures, so the
//! engine's object layer stays exactly the undirected VDC of ADR-68. A directed
//! variable is thus `(name, OpSig)` where the [`OpSig`] pairs a plain
//! [`crate::vdc::SignatureRef`] with the [`Variance`] slot it stands in.

use alloc::vec::Vec;

use gandr_theory_cell_complexes::Cell;
use gandr_theory_cell_complexes::CellVariance;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::NameRef;

use crate::boundary::DirectedContextDeclaration;
use crate::boundary::DirectedObjectCovariance;
use crate::vdc::SignatureRef;

/// The **variance** a reflected object variable is sorted at
/// (`proposal-vdc-reflection.md` §7, Ł1) — the directed polarity `x : I` vs `x
/// : Iᵒᵖ`.
///
/// This is the closed two-way polarity vocabulary of directed type theory. The
/// engine's [`CellVariance`] has a third case, [`CellVariance::Mixed`] — a hole
/// spanning both polarities — which is **not** a directed variance (it is the
/// dinaturality shape Ł2's directedness protects); [`Variance::of_cell`] maps
/// it to `None`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Variance
{
    /// **Covariant** — the variable ranges over `I` (a producer / target slot).
    Covariant,
    /// **Contravariant** — the variable ranges over `Iᵒᵖ` (a consumer / source
    /// slot).
    Contravariant,
}

impl Variance
{
    /// The **opposite** variance — the `−ᵒᵖ` involution on the polarity itself.
    ///
    /// # Contract
    /// - ensures: [`Variance::Contravariant`] for [`Variance::Covariant`] and
    ///   vice versa; `flip(flip(v)) == v` (an involution).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn flip(self) -> Self
    {
        match self {
            | Self::Covariant => Self::Contravariant,
            | Self::Contravariant => Self::Covariant,
        }
    }

    /// The directed variance an engine [`CellVariance`] witnesses, or `None`
    /// when the hole is [`CellVariance::Mixed`] (spans both polarities — not a
    /// directed variance).
    ///
    /// # Contract
    /// - ensures: [`Variance::Covariant`] for [`CellVariance::Producer`],
    ///   [`Variance::Contravariant`] for [`CellVariance::Consumer`], and `None`
    ///   for [`CellVariance::Mixed`] — the mapping that makes the §4.2 metadata
    ///   checkable against a directed context.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn of_cell(variance: CellVariance) -> Option<Self>
    {
        match variance {
            | CellVariance::Producer => Some(Self::Covariant),
            | CellVariance::Consumer => Some(Self::Contravariant),
            | CellVariance::Mixed => None,
        }
    }
}

/// A **reflected signature with a variance slot** — the directed object a
/// variable stands in, `I` (covariant) or `Iᵒᵖ` (contravariant)
/// (`proposal-vdc-reflection.md` §7, Ł1).
///
/// The `op` involution ([`OpSig::op`]) lives here, on the *reflected*
/// signature, never on the frozen [`SignatureRef`] core.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OpSig
{
    /// The underlying reflected signature (a frozen-core [`SignatureRef`]).
    pub sig: SignatureRef,
    /// The variance slot the signature stands in.
    pub variance: Variance,
}

impl OpSig
{
    /// The **covariant** sorting of `sig` (`x : I`).
    #[inline]
    #[must_use]
    pub fn covariant(sig: SignatureRef) -> Self
    {
        Self {
            sig,
            variance: Variance::Covariant,
        }
    }

    /// The **contravariant** sorting of `sig` (`x : Iᵒᵖ`).
    #[inline]
    #[must_use]
    pub fn contravariant(sig: SignatureRef) -> Self
    {
        Self {
            sig,
            variance: Variance::Contravariant,
        }
    }

    /// The **opposite** object `−ᵒᵖ` — the same underlying signature at the
    /// flipped variance.
    ///
    /// # Contract
    /// - ensures: the same `sig` with [`Variance::flip`] applied; `op(op(x)) ==
    ///   x` (an involution), realized on the reflected signature alone — the
    ///   frozen [`SignatureRef`] is unchanged.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — `op` is asserted an involution over
    ///   generated signatures and both variances; the boundary is that the
    ///   underlying `sig` never changes.
    /// - witness: `directed::tests::op_is_an_involution_on_reflected_signatures`
    #[inline]
    #[must_use]
    pub fn op(&self) -> Self
    {
        Self {
            sig: self.sig.clone(),
            variance: self.variance.flip(),
        }
    }

    /// Whether this object stands in the covariant slot.
    #[inline]
    #[must_use]
    pub fn is_covariant(&self) -> DirectedObjectCovariance
    {
        DirectedObjectCovariance::from(matches!(self.variance, Variance::Covariant))
    }
}

/// A **variance-sorted reflected context** — object variables paired with the
/// directed object ([`OpSig`]) they range over (`proposal-vdc-reflection.md`
/// §7, Ł1).
///
/// The undirected face's [`crate::check::Context`] is two-sided but
/// variance-blind; the directed context sorts each variable by variance
/// instead, so the polarity side condition of the directed J
/// ([`crate::directed::hom`]) and the variance check below can consult it.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DirectedContext
{
    /// The variance-sorted object variables, in binding order.
    pub vars: Vec<(Name, OpSig)>,
}

/// Why an engine cell's variance disagrees with a directed context
/// (`proposal-vdc-reflection.md` §7, Ł1; §4.2).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum VarianceError
{
    /// A cell hole naming a declared object variable is sorted at the opposite
    /// polarity from the context.
    Mismatch
    {
        /// The object variable the hole names.
        var: Name,
        /// The variance the context sorted it at.
        declared: Variance,
        /// The variance the engine derived for the hole.
        derived: Variance,
    },
    /// A cell hole naming a declared object variable is [`CellVariance::Mixed`]
    /// — it spans both polarities, so it cannot be sorted at a single directed
    /// variance (the dinaturality shape Ł2's directedness protects, ADR-69 D3).
    MixedHole
    {
        /// The object variable the mixed hole names.
        var: Name,
    },
}

impl DirectedContext
{
    /// An empty variance-sorted context.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Sort `name` at the directed object `obj`.
    #[inline]
    #[must_use]
    pub fn with_var(
        mut self,
        name: NameRef<'_>,
        obj: OpSig,
    ) -> Self
    {
        self.vars.push((Name::from(name), obj));
        self
    }

    /// The variance `name` is sorted at — the **innermost** binding, or `None`
    /// when `name` is undeclared.
    ///
    /// # Contract
    /// - ensures: the [`Variance`] of the last `(name, _)` binding, or `None`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn variance_of(
        &self,
        name: NameRef<'_>,
    ) -> Option<Variance>
    {
        for binding in self.vars.iter().rev() {
            if binding.0.as_ref() == name.as_ref() {
                return Some(binding.1.variance);
            }
        }
        None
    }

    /// The signature `name` ranges over — the **innermost** binding, or `None`
    /// when `name` is undeclared.
    ///
    /// # Contract
    /// - ensures: the [`SignatureRef`] of the last `(name, _)` binding, or
    ///   `None`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn signature_of(
        &self,
        name: NameRef<'_>,
    ) -> Option<&SignatureRef>
    {
        for binding in self.vars.iter().rev() {
            if binding.0.as_ref() == name.as_ref() {
                return Some(&binding.1.sig);
            }
        }
        None
    }

    /// Whether `name` is declared.
    ///
    /// # Contract
    /// - ensures: `true` iff `name` appears in `vars`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn declares(
        &self,
        name: NameRef<'_>,
    ) -> DirectedContextDeclaration
    {
        for binding in &self.vars {
            if binding.0.as_ref() == name.as_ref() {
                return DirectedContextDeclaration::from(true);
            }
        }
        DirectedContextDeclaration::from(false)
    }

    /// **Check** an engine cell's derived variance against this context — the
    /// §4.2 metadata made *checkable* (`proposal-vdc-reflection.md` §7, Ł1).
    ///
    /// Every metavariable of `cell` whose name is a declared object variable
    /// must be derived at the variance the context sorted it at; a hole the
    /// engine classified [`CellVariance::Mixed`] cannot inhabit a single
    /// directed variance at all. Holes the context does not declare are the
    /// cell's own local data and are not constrained.
    ///
    /// # Contract
    /// - ensures: `Ok(())` iff, for each of `cell`'s derived
    ///   [`gandr_theory_cell_complexes::CellVarMeta`] whose name this context
    ///   declares, the [`Variance::of_cell`] of its derived [`CellVariance`]
    ///   equals the declared [`Variance`]; a `Mixed` hole naming a declared
    ///   variable fails.
    /// - fails: [`VarianceError::Mismatch`] for a declared hole at the opposite
    ///   polarity; [`VarianceError::MixedHole`] for a declared hole the engine
    ///   classified `Mixed`.
    /// - panics: none.
    ///
    /// # Errors
    /// See the `- fails:` clause.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — a producer hole sorted covariantly passes;
    ///   the same hole sorted contravariantly asserts
    ///   [`VarianceError::Mismatch`]; a `Mixed` hole (a name at both
    ///   polarities) sorted at either variance asserts
    ///   [`VarianceError::MixedHole`].
    /// - witness: `directed::tests::a_producer_hole_checks_covariantly`
    /// - witness: `directed::tests::a_producer_hole_sorted_contravariantly_is_rejected`
    /// - witness: `directed::tests::a_mixed_hole_cannot_be_sorted_at_a_directed_variance`
    #[inline]
    pub fn check_cell_variance(
        &self,
        cell: &Cell,
    ) -> Result<(), VarianceError>
    {
        for var_meta in &cell.meta.vars {
            let name: &str = &var_meta.var.name;
            let Some(declared) = self.variance_of(NameRef::from(name))
            else {
                continue;
            };
            match Variance::of_cell(var_meta.variance) {
                | None => {
                    return Err(VarianceError::MixedHole {
                        var: Name::from(name),
                    });
                },
                | Some(derived) if derived != declared => {
                    return Err(VarianceError::Mismatch {
                        var: Name::from(name),
                        declared,
                        derived,
                    });
                },
                | Some(_) => {},
            }
        }
        Ok(())
    }
}
