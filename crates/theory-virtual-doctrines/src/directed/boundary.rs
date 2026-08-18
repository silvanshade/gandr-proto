//! **Ł4 — the boundary theorem** (`proposal-vdc-reflection.md` §7, Ł4;
//! ADR-68/69; §4.3).
//!
//! The directed lane gates certificate composition on variable-flow acyclicity
//! ([`gandr_theory_decomposition_spaces::compose_directed`]) — a mixed-variance
//! seam can cycle the flow and be declined. The boundary theorem says that on
//! **invertible-cell signatures** the gate is unnecessary: full cut is
//! admissible, because over groupoids dinaturals always compose (LLV Thm 4.5,
//! the operational warrant for
//! [`gandr_theory_decomposition_spaces::compose_invertible`]). This is F1's
//! directed gate lifted on the coherence lane.
//!
//! [`directed_cut`] realizes exactly that operational content: when every
//! participating cell of both certificates is invertible
//! ([`all_participating_invertible`]), it routes to the **ungated**
//! `compose_invertible` and returns [`CutOutcome::Coherent`] — unconditionally,
//! never declined. Otherwise it consults the acyclicity gate and reports
//! [`CutOutcome::Directed`] (passed) or [`CutOutcome::Declined`] (a cycle).
//!
//! # The theorem-grade statement rides the Agda face
//!
//! What is realized here is the **rule** — cut needs no gate on the invertible
//! lane — exercised over finite fixtures. The theorem-grade *statement* (cut is
//! admissible for **every** invertible signature, the internal groupoid theorem
//! §6.3 names as the first Agda-face target) rides the Agda face on the IU
//! groupoid tower; the property tests here are engineering evidence, not the
//! theorem.

use gandr_theory_cell_complexes::CellStore;
use gandr_theory_coherent_resolutions::Tracelet;
use gandr_theory_decomposition_spaces::CompositionObstruction;
use gandr_theory_decomposition_spaces::compose_directed;
use gandr_theory_decomposition_spaces::compose_invertible;

use crate::boundary::CertificateInvertibility;
use crate::boundary::CutCoherence;
use crate::boundary::CutDeclination;

/// The outcome of a [`directed_cut`] (`proposal-vdc-reflection.md` §7, Ł4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CutOutcome
{
    /// Admitted **unconditionally** — every participating cell is invertible,
    /// so the cut rode the ungated `compose_invertible` (the boundary
    /// theorem / the coherence lane, LLV Thm 4.5).
    Coherent(Tracelet),
    /// Admitted through the **acyclicity gate** — the directed lane's
    /// variable-flow graph was acyclic and `compose_directed` composed.
    Directed(Tracelet),
    /// **Declined** by the acyclicity gate — a variable-flow cycle obstructs
    /// the directed composition (canonically a shared mixed-variance seam
    /// hole).
    Declined(CompositionObstruction),
}

impl CutOutcome
{
    /// The composite certificate, when the cut was admitted (either lane).
    ///
    /// # Contract
    /// - ensures: `Some` for [`CutOutcome::Coherent`] /
    ///   [`CutOutcome::Directed`], `None` for [`CutOutcome::Declined`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn tracelet(&self) -> Option<&Tracelet>
    {
        match *self {
            | Self::Coherent(ref t) | Self::Directed(ref t) => Some(t),
            | Self::Declined(_) => None,
        }
    }

    /// Whether the cut was admitted unconditionally (the invertible / coherence
    /// lane).
    #[inline]
    #[must_use]
    pub fn is_coherent(&self) -> CutCoherence
    {
        CutCoherence::from(matches!(*self, Self::Coherent(_)))
    }

    /// Whether the cut was declined by the acyclicity gate.
    #[inline]
    #[must_use]
    pub fn is_declined(&self) -> CutDeclination
    {
        CutDeclination::from(matches!(*self, Self::Declined(_)))
    }
}

/// **The directed cut** — compose two directed certificates, routing off the
/// invertible boundary (`proposal-vdc-reflection.md` §7, Ł4).
///
/// When every participating cell of both certificates is invertible
/// ([`all_participating_invertible`]), the cut is admissible unconditionally
/// (the boundary theorem): it rides the ungated
/// [`gandr_theory_decomposition_spaces::compose_invertible`] and yields
/// [`CutOutcome::Coherent`]. Otherwise it consults the acyclicity gate
/// ([`gandr_theory_decomposition_spaces::compose_directed`]):
/// [`CutOutcome::Directed`] when the seam variable-flow graph is acyclic,
/// [`CutOutcome::Declined`] when a cycle obstructs it.
///
/// # Contract
/// - requires: `a.joins_at == b.overlap.peak` (the sequential seam) for the
///   admitted composite to **replay**; admissibility itself (which lane, and
///   whether the gate declines) is decided from the cells' invertibility and
///   variance alone.
/// - ensures: [`CutOutcome::Coherent`] iff both certificates are wholly
///   invertible (never [`CutOutcome::Declined`] on that lane — the boundary
///   theorem); otherwise the gate's verdict — [`CutOutcome::Directed`] on an
///   acyclic seam, [`CutOutcome::Declined`] carrying the
///   [`CompositionObstruction`] cycle otherwise. Never diverges or panics.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence + property — an all-invertible ground chain always
///   yields [`CutOutcome::Coherent`] whose composite replays (the boundary
///   theorem's operational content, over generated chain lengths); a
///   mixed-variance (non-invertible) cycle yields [`CutOutcome::Declined`]
///   carrying the flow cycle. Boundary: the same seam is `Coherent` when its
///   cells are invertible and `Declined` when they are directed and cyclic.
/// - witness: `directed::tests::an_invertible_chain_cuts_unconditionally`
/// - witness: `directed::tests::a_mixed_variance_cycle_is_declined`
/// - witness: `directed::tests::the_invertible_lane_is_never_declined`
#[inline]
#[must_use]
pub fn directed_cut(
    a: &Tracelet,
    b: &Tracelet,
    store: &CellStore,
) -> CutOutcome
{
    if bool::from(all_participating_invertible(a, store))
        && bool::from(all_participating_invertible(b, store))
    {
        return CutOutcome::Coherent(compose_invertible(a, b));
    }
    match compose_directed(a, b, store) {
        | Ok(composite) => CutOutcome::Directed(composite),
        | Err(obstruction) => CutOutcome::Declined(obstruction),
    }
}

/// Whether **every** cell a certificate fires is an invertible joinability
/// certificate (`proposal-vdc-reflection.md` §7, Ł4; §4.2 `invertible` flag).
///
/// Reads the live [`gandr_theory_cell_complexes::CellMeta::invertible`] of each
/// cell in `path_a` then `path_b`. A stale cell id (absent from the store) is
/// **conservatively** not invertible — invertibility cannot be certified for a
/// cell that is not there.
///
/// # Contract
/// - ensures: `true` iff every [`gandr_theory_cell_complexes::CellId`] the
///   certificate fires resolves in `store` and carries `meta.invertible`;
///   vacuously `true` for a certificate with empty paths (a groupoid identity).
/// - panics: none.
#[inline]
#[must_use]
pub fn all_participating_invertible(
    cert: &Tracelet,
    store: &CellStore,
) -> CertificateInvertibility
{
    CertificateInvertibility::from(cert.path_a.iter().chain(&cert.path_b).all(|step| {
        store
            .get(step.cell)
            .is_some_and(|cell| bool::from(cell.meta.invertible))
    }))
}
