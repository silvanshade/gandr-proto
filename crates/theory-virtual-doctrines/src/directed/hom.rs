//! **Ł2 — hom as directed equality** with the polarity-restricted directed J
//! rule (`proposal-vdc-reflection.md` §7, Ł2; ADR-68/69).
//!
//! The undirected face's path protype ([`crate::syntax::Protype::Path`], `s ⤳
//! t`) is symmetric — a rewrite path can be read either way. Its directed
//! refinement is LLV's **hom** ([`DirectedHom`], `hom_I(a ⇝ b)`), where the
//! source `a` is contravariant and the target `b` covariant, and the
//! eliminator is a **directed** J whose motive is restricted by polarity.
//!
//! # Symmetry is underivable *by construction*
//!
//! Directed path induction transports a witness *forward* along the moving
//! covariant endpoint: from `p : hom(a ⇝ b)` and a motive covariant in that
//! endpoint, it yields the family's value at `b`. Symmetry — `hom(a ⇝ b) →
//! hom(b ⇝ a)` — needs the motive `C(x) = hom(x ⇝ a)`, which places the moving
//! endpoint in the **contravariant source** slot
//! ([`MotiveShape::ContravariantSource`]); the polarity side condition
//! ([`check_directed_j`]) refuses exactly that shape. This is the same design
//! stance as the without-K unifier one level down (`proposal-data-patterns.md`
//! §4.4): a K/UIP unifier would collapse orientation just as a contravariant
//! motive would. Semantically, a directed cell with no inverse has no backward
//! transport — an oriented engine cell `a ~> b` that does not replay `b ~> a`
//! witnesses `hom(a ⇝ b)` but not `hom(b ⇝ a)` (property-tested against the
//! engine).
//!
//! The theorem-grade statement (directed J is sound and symmetry is *not*
//! admissible in the reflected directed type theory) rides the Agda face; what
//! this module carries is the total
//! polarity checker plus the property tests.

use crate::boundary::DirectedHomReflexivity;
use crate::boundary::MotiveCovariance;
use crate::vdc::SignatureRef;
use crate::vdc::TermRef;

/// A **directed hom** protype `hom_I(a ⇝ b)` — the directed refinement of
/// [`crate::syntax::Protype::Path`] (`proposal-vdc-reflection.md` §7, Ł2).
///
/// The source `a` stands in the contravariant slot, the target `b` in the
/// covariant slot; unlike the undirected path, the two endpoints are **not**
/// interchangeable.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a directed hom is exactly its {signature, contravariant source, covariant target}; the directed J reads all three and the reflection constructs it"
)]
pub struct DirectedHom
{
    /// The signature both endpoints inhabit.
    pub sig: SignatureRef,
    /// The contravariant **source** endpoint `a`.
    pub src: TermRef,
    /// The covariant **target** endpoint `b`.
    pub tgt: TermRef,
}

impl DirectedHom
{
    /// The directed hom `hom_sig(src ⇝ tgt)`.
    #[inline]
    #[must_use]
    pub fn new(
        sig: SignatureRef,
        src: TermRef,
        tgt: TermRef,
    ) -> Self
    {
        Self { sig, src, tgt }
    }

    /// The **reflexive** directed hom `hom_sig(term ⇝ term)` — the diagonal the
    /// directed J's base case inhabits.
    #[inline]
    #[must_use]
    pub fn refl(
        sig: SignatureRef,
        term: TermRef,
    ) -> Self
    {
        Self {
            sig,
            src: term.clone(),
            tgt: term,
        }
    }

    /// Whether this hom is on the diagonal (`src == tgt`) — reflexive.
    #[inline]
    #[must_use]
    pub fn is_reflexive(&self) -> DirectedHomReflexivity
    {
        DirectedHomReflexivity::from(self.src == self.tgt)
    }
}

/// Which slot of the motive's produced hom the **moving** (covariant) endpoint
/// occupies (`proposal-vdc-reflection.md` §7, Ł2).
///
/// The directed J transports along the scrutinee's covariant target endpoint;
/// the motive `C(x)` abstracts that endpoint. Only a motive that keeps it
/// covariant is admissible — a contravariant use is the symmetry shape the
/// polarity side condition refuses.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MotiveShape
{
    /// `C(x) = hom(fixed ⇝ x)` — the moving endpoint stays in the covariant
    /// **target** slot. **J-admissible.**
    CovariantTarget,
    /// `C(x) = hom(x ⇝ fixed)` — the moving endpoint is in the contravariant
    /// **source** slot. This is the **symmetry** shape; the polarity side
    /// condition refuses it.
    ContravariantSource,
    /// `C(x) = hom(fixed ⇝ fixed)` — the moving endpoint is unused (a constant
    /// family). **J-admissible** (trivially covariant).
    Constant,
}

impl MotiveShape
{
    /// Whether this motive respects the covariance the directed J requires.
    ///
    /// # Contract
    /// - ensures: `true` for [`MotiveShape::CovariantTarget`] and
    ///   [`MotiveShape::Constant`]; `false` for
    ///   [`MotiveShape::ContravariantSource`] (the symmetry shape).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn is_covariant(self) -> MotiveCovariance
    {
        MotiveCovariance::from(!matches!(self, Self::ContravariantSource))
    }
}

/// A **directed-J elimination** — the motive shape plus the fixed endpoint the
/// motive holds constant (`proposal-vdc-reflection.md` §7, Ł2).
///
/// The moving endpoint is supplied by the scrutinee's target; `fixed` is the
/// other endpoint the motive's produced hom pins.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a directed-J elimination is exactly its {motive shape, fixed endpoint}; the checker reads both and the moving endpoint comes from the scrutinee"
)]
pub struct DirectedJ
{
    /// Which slot the motive puts the moving endpoint in.
    pub motive: MotiveShape,
    /// The endpoint the motive holds fixed.
    pub fixed: TermRef,
}

impl DirectedJ
{
    /// The directed-J elimination with motive `motive` fixing `fixed`.
    #[inline]
    #[must_use]
    pub fn new(
        motive: MotiveShape,
        fixed: TermRef,
    ) -> Self
    {
        Self { motive, fixed }
    }

    /// The **symmetry** elimination for a scrutinee whose source is `src` — the
    /// motive `C(x) = hom(x ⇝ src)` that would transport backward.
    ///
    /// # Contract
    /// - ensures: [`MotiveShape::ContravariantSource`] fixing `src`; passing it
    ///   to [`check_directed_j`] always yields [`JError::MotiveNotCovariant`]
    ///   (symmetry is underivable).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn symmetry(src: TermRef) -> Self
    {
        Self {
            motive: MotiveShape::ContravariantSource,
            fixed: src,
        }
    }
}

/// Why a directed-J elimination is ill-formed (`proposal-vdc-reflection.md` §7,
/// Ł2).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum JError
{
    /// The motive places the moving endpoint in the contravariant source slot
    /// — the **symmetry** shape the polarity side condition refuses (the
    /// directedness protection, ADR-69).
    MotiveNotCovariant,
}

/// **Check** a directed-J elimination against a directed-hom scrutinee and
/// return the transported hom (`proposal-vdc-reflection.md` §7, Ł2).
///
/// The scrutinee's covariant target endpoint is the moving endpoint. The motive
/// must be covariant in it ([`MotiveShape::is_covariant`]); a contravariant
/// motive — the only shape that could produce the reversed `hom(b ⇝ a)` — is
/// the symmetry shape and is refused, so **symmetry is underivable by
/// construction**.
///
/// # Contract
/// - ensures: `Ok(hom(j.fixed ⇝ scrut.tgt))` for
///   [`MotiveShape::CovariantTarget`] (the moving endpoint transported into the
///   covariant slot); `Ok(hom(j.fixed ⇝ j.fixed))` for
///   [`MotiveShape::Constant`]; the result inhabits the same signature as
///   `scrut`.
/// - fails: [`JError::MotiveNotCovariant`] for
///   [`MotiveShape::ContravariantSource`] — the polarity side condition, which
///   is exactly the symmetry case.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — a covariant motive transports forward to
///   `hom(fixed ⇝ b)`; the contravariant (symmetry) motive asserts
///   [`JError::MotiveNotCovariant`]; the boundary is that
///   [`DirectedJ::symmetry`] is refused for every generated hom.
/// - witness: `directed::tests::directed_j_transports_along_a_covariant_motive`
/// - witness: `directed::tests::directed_j_refuses_the_symmetry_motive`
/// - witness: `directed::tests::symmetry_is_never_derivable`
#[inline]
pub fn check_directed_j(
    scrut: &DirectedHom,
    j: &DirectedJ,
) -> Result<DirectedHom, JError>
{
    if !bool::from(j.motive.is_covariant()) {
        return Err(JError::MotiveNotCovariant);
    }
    match j.motive {
        | MotiveShape::CovariantTarget => Ok(DirectedHom {
            sig: scrut.sig.clone(),
            src: j.fixed.clone(),
            tgt: scrut.tgt.clone(),
        }),
        | MotiveShape::Constant => Ok(DirectedHom {
            sig: scrut.sig.clone(),
            src: j.fixed.clone(),
            tgt: j.fixed.clone(),
        }),
        | MotiveShape::ContravariantSource => Err(JError::MotiveNotCovariant),
    }
}
