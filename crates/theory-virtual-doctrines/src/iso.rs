//! **Protype isomorphisms = the certificate surface**
//! (`proposal-vdc-reflection.md` §5.3; ADR-68 D3, ADR-69).
//!
//! A protype isomorphism *is* its pair of mutually-inverse replayable
//! witnesses: there is nothing to lose (the "retained coherence" answer that
//! repairs Cáccamo's witness-erasure defect). Well-formedness — `bwd ∘ fwd ≡
//! id` and `fwd ∘ bwd ≡ id` — is decided by **replay**, exactly the engine's
//! certificate-checking discipline.
//!
//! The groupoid laws (identity, inverse, composition) are constructions on
//! [`IsoWitness`], **composed in the invertible mode**
//! ([`gandr_theory_computads::compose_invertible`]) — unconditional, by design
//! (§4.3; LLV Thm 4.5: over groupoids, dinaturals always compose).
//! [`ProtypeIso`] is the reflected-syntax surface (a pair of [`Proterm`]
//! certificate witnesses); [`ProtypeIso::witness`] resolves it to an
//! [`IsoWitness`] over the engine.

use alloc::boxed::Box;

use gandr_theory_computads::CellStore;
use gandr_theory_computads::compose_invertible;

use crate::boundary::DerivationIndex;
use crate::boundary::IsoValidity;
use crate::boundary::RoundTripIdentity;
use crate::syntax::Proterm;
use crate::vdc::Derivation;
use crate::vdc::Elaborated;

/// A **pair of mutually-inverse replayable witnesses** — the engine-level
/// protype isomorphism the groupoid laws operate on
/// (`proposal-vdc-reflection.md` §5.3).
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "an isomorphism witness is exactly its {forward, backward} derivation pair; validity is replay-decided and the groupoid laws construct it"
)]
pub struct IsoWitness
{
    /// The forward derivation `A ⇒ B`.
    pub fwd: Derivation,
    /// The backward derivation `B ⇒ A`.
    pub bwd: Derivation,
}

impl IsoWitness
{
    /// A witness from a forward/backward derivation pair.
    #[inline]
    #[must_use]
    pub fn new(
        fwd: Derivation,
        bwd: Derivation,
    ) -> Self
    {
        Self { fwd, bwd }
    }

    /// The **identity** isomorphism — the reflexive derivation both ways
    /// (groupoid identity law).
    ///
    /// # Contract
    /// - ensures: `fwd == bwd == reflexive`; valid iff `reflexive` is a
    ///   reflexive certificate (or the trivial transformation) that replays.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn identity(reflexive: Derivation) -> Self
    {
        Self {
            fwd: reflexive.clone(),
            bwd: reflexive,
        }
    }

    /// The **inverse** isomorphism — swap the witnesses (groupoid inverse law).
    ///
    /// # Contract
    /// - ensures: `fwd`/`bwd` swapped; the inverse of a valid iso is valid (the
    ///   round-trip conditions are symmetric).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn inverse(&self) -> Self
    {
        Self {
            fwd: self.bwd.clone(),
            bwd: self.fwd.clone(),
        }
    }

    /// The **composite** `other ∘ self` (`self : A ≅ B`, `other : B ≅ C`) —
    /// composed in the **invertible mode** (groupoid composition law).
    ///
    /// # Contract
    /// - ensures: the forward witness grafts `self.fwd` then `other.fwd` (`A ⇒
    ///   C`) and the backward witness grafts `other.bwd` then `self.bwd` (`C ⇒
    ///   A`); elaboration rides [`gandr_theory_computads::compose_invertible`],
    ///   so the composite of two valid isos is valid unconditionally (§4.3).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn compose(
        &self,
        other: &Self,
    ) -> Self
    {
        Self {
            fwd: Derivation::Graft {
                outer: Box::new(other.fwd.clone()),
                inners: Box::from([self.fwd.clone()]),
            },
            bwd: Derivation::Graft {
                outer: Box::new(self.bwd.clone()),
                inners: Box::from([other.bwd.clone()]),
            },
        }
    }

    /// Whether this witness is a **genuine isomorphism** — both round-trips
    /// replay to the identity (decided by replay, ADR-69).
    ///
    /// # Contract
    /// - ensures: `true` iff `bwd ∘ fwd` and `fwd ∘ bwd` each elaborate (in the
    ///   invertible mode) to a **reflexive** certificate (`peak == joins_at`)
    ///   that replays in `cells`, or to the trivial transformation; a stuck or
    ///   non-reflexive round-trip yields `false`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the validator composes the witness pair and
    ///   replays the round-trips against the store; any mutant admitting a
    ///   non-reflexive or non-replaying round-trip is caught. Boundary: a
    ///   one-direction replay failure (`protype-iso-replay-fail`) is rejected.
    /// - witness: `laws::tests::a_reflexive_witness_pair_is_a_valid_iso`
    /// - witness: `laws::tests::an_iso_whose_one_direction_fails_replay_is_rejected`
    /// - witness: `laws::tests::the_inverse_of_a_valid_iso_is_valid`
    #[inline]
    #[must_use]
    pub fn is_valid(
        &self,
        cells: &CellStore,
    ) -> IsoValidity
    {
        let fwd = self.fwd.elaborate();
        let bwd = self.bwd.elaborate();
        IsoValidity::from(
            bool::from(round_trips_to_identity(&fwd, &bwd, cells))
                && bool::from(round_trips_to_identity(&bwd, &fwd, cells)),
        )
    }
}

/// Whether composing `a` then `b` (invertible mode) yields the identity — a
/// reflexive certificate that replays, or the trivial transformation.
///
/// # Contract
/// - ensures: `true` iff both are the trivial transformation; or both are
///   certificates sharing the seam (`a.joins_at == b.overlap.peak`) whose
///   invertible composite is reflexive (`peak == joins_at`) and replays; or one
///   is trivial and the other a reflexive certificate that replays. A stuck
///   elaboration or an open round-trip yields `false`.
/// - panics: none.
#[inline]
#[must_use]
fn round_trips_to_identity(
    a: &Elaborated,
    b: &Elaborated,
    cells: &CellStore,
) -> RoundTripIdentity
{
    let valid = match (a, b) {
        | (&Elaborated::Trivial, &Elaborated::Trivial) => true,
        | (&Elaborated::Cert(ref ca), &Elaborated::Cert(ref cb)) => {
            if ca.joins_at != cb.overlap.peak {
                return RoundTripIdentity::from(false);
            }
            let composite = compose_invertible(ca, cb);
            composite.overlap.peak == composite.joins_at && bool::from(composite.replay(cells))
        },
        | (&Elaborated::Trivial, &Elaborated::Cert(ref c))
        | (&Elaborated::Cert(ref c), &Elaborated::Trivial) => {
            c.overlap.peak == c.joins_at && bool::from(c.replay(cells))
        },
        | (&Elaborated::Stuck, _) | (_, &Elaborated::Stuck) => false,
    };
    RoundTripIdentity::from(valid)
}

/// A **protype isomorphism** — the reflected-syntax surface: a pair of
/// certificate-witness proterms (`proposal-vdc-reflection.md` §5.3).
///
/// Well-formed iff [`ProtypeIso::is_valid`] — `bwd ∘ fwd ≡ id` and `fwd ∘ bwd ≡
/// id`, decided by replaying the resolved [`IsoWitness`]. Only
/// [`Proterm::Cert`] witnesses are engine-replayable at this stage; a
/// non-certificate witness makes [`ProtypeIso::witness`] `None` (and the iso
/// invalid).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a protype isomorphism is exactly its {forward, backward} witness proterms (spec section 5.3); validity is replay-decided"
)]
pub struct ProtypeIso
{
    /// The forward witness proterm.
    pub fwd: Proterm,
    /// The backward witness proterm.
    pub bwd: Proterm,
}

impl ProtypeIso
{
    /// An isomorphism from a forward/backward witness pair.
    #[inline]
    #[must_use]
    pub fn new(
        fwd: Proterm,
        bwd: Proterm,
    ) -> Self
    {
        Self { fwd, bwd }
    }

    /// Resolve the certificate witnesses to an [`IsoWitness`] over
    /// `derivations`.
    ///
    /// # Contract
    /// - ensures: `Some(witness)` iff both `fwd` and `bwd` are
    ///   [`Proterm::Cert`] proterms referencing derivations present in
    ///   `derivations`; `None` otherwise (a non-certificate witness is not
    ///   engine-replayable at this stage).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn witness(
        &self,
        derivations: &[Derivation],
    ) -> Option<IsoWitness>
    {
        let fwd = resolve_cert(&self.fwd, derivations)?;
        let bwd = resolve_cert(&self.bwd, derivations)?;
        Some(IsoWitness::new(fwd, bwd))
    }

    /// Whether this isomorphism is well-formed — its witnesses resolve and both
    /// round-trips replay to the identity.
    ///
    /// # Contract
    /// - ensures: `true` iff [`ProtypeIso::witness`] resolves and the resolved
    ///   [`IsoWitness::is_valid`] holds against `cells`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn is_valid(
        &self,
        derivations: &[Derivation],
        cells: &CellStore,
    ) -> IsoValidity
    {
        IsoValidity::from(
            self.witness(derivations)
                .is_some_and(|witness| bool::from(witness.is_valid(cells))),
        )
    }
}

/// Resolve a certificate witness proterm to its embedded derivation.
///
/// # Contract
/// - ensures: `Some(derivation)` iff `term` is a [`Proterm::Cert`] whose id is
///   present in `derivations`; `None` for any other proterm form or an absent
///   id.
/// - panics: none.
#[inline]
fn resolve_cert(
    term: &Proterm,
    derivations: &[Derivation],
) -> Option<Derivation>
{
    match *term {
        | Proterm::Cert(id) => derivations
            .get(usize::from(DerivationIndex::from(id)))
            .cloned(),
        | _ => None,
    }
}
