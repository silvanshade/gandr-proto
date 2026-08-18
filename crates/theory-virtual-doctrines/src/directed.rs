//! **The LLV directed fragment** over the reflected layer
//! (§7; decisions **ADR-68** /
//! **ADR-69**).
//!
//! `FVDblTT` (the [`crate::syntax`] judgment layer) is deliberately first-order
//! and undirected: what it cannot express is exactly what the polygraph program
//! eventually wants to *say* — statements about **endo**-relations with a
//! variable at both polarities, dinaturality-shaped coherence, quantification
//! over all instantiations of a seam. That is LLV's territory (Laretto,
//! Loregian & Veltri, *Di- is for Directed*, POPL 2026): (co)ends as
//! quantifiers, hom as **directed** equality with a J-eliminator restricted by
//! polarity so symmetry is underivable, entailments proof-relevant.
//!
//! This module is **strictly additive** over the reflection face and the
//! engine: it reflects the directed reading, it does not re-implement the
//! rewrite engine. It stages behind levitation stage 1 (ADR-81 —
//! `ValueType::Universe` + `Sigma` + `decode`, landed), which is the gate §7
//! names, because dipresheaf-shaped protypes over reflected signatures need the
//! typed `Desc` layer.
//!
//! # The ladder (§7)
//!
//! - [`context`] — **Ł1, variance-sorted reflected contexts.** Reflected object
//!   variables carry a [`Variance`] (`x : I` vs `x : Iᵒᵖ`), and the §4.2 engine
//!   variance metadata becomes *checkable* rather than merely derived
//!   ([`DirectedContext::check_cell_variance`]). **Design record:** the `−ᵒᵖ`
//!   involution lives on the reflected [`OpSig`] **only** ([`OpSig::op`]); the
//!   frozen core [`crate::vdc::SignatureRef`] is untouched — LLV's `−ᵒᵖ` is
//!   added on *reflected* signatures, never on the engine's objects.
//! - [`hom`] — **Ł2, hom-as-directed-equality.** The path protype upgraded to
//!   LLV's hom-with-J: [`DirectedHom`] with the polarity-restricted directed
//!   J-eliminator ([`hom::check_directed_j`]). **Symmetry is underivable by
//!   construction** — the same design stance as the without-K unifier one level
//!   down (`proposal-data-patterns.md` §4.4): a motive that would transport
//!   backward puts the moving endpoint in the contravariant source slot, which
//!   the polarity side condition refuses, exactly as a K/UIP unifier would
//!   collapse orientation. Semantically a directed cell with no inverse cannot
//!   be transported backward.
//! - [`coend`] — **Ł3, (co)ends as quantifiers.** `∫` binders over reflected
//!   signatures ([`coend::End`] / [`coend::Coend`]), with **Fubini**
//!   ([`coend::fubini_swap`]) and **(co)Yoneda** ([`coend::coyoneda_collapse`])
//!   as *derived* transformations, realized as checked derivations with
//!   property tests over **finite** reflected signatures. Theorem-grade
//!   generality (arbitrary reflected signatures, the dinaturality quotient in
//!   full) rides the Agda face; cut remains restricted (the §4.3 gate is its
//!   operational face).
//! - [`boundary`] — **Ł4, the boundary theorem.** On invertible-cell signatures
//!   full cut is admissible — the internal groupoid theorem (LLV Thm 4.5),
//!   lifting F1's directed gate on the coherence lane. Realized as
//!   [`boundary::directed_cut`]: cut-elaboration succeeds **unconditionally**
//!   when every participating cell is invertible (routing to
//!   [`gandr_theory_decomposition_spaces::compose_invertible`], no gate), and
//!   otherwise consults the acyclicity gate
//!   ([`gandr_theory_decomposition_spaces::compose_directed`]). The
//!   theorem-grade *statement* (all invertible signatures, not the finite
//!   fixtures the property tests exercise) is Agda-face territory.
//!
//! # Out of scope (direction only)
//!
//! LLV's **directed univalence** (`homSet(A, B) ≅ A ⇒ B`) — the second-order
//! step LLV's own future work names — is **not scoped** here. It is a reflected
//! bead-ready residual: it needs the directed-hom *type* to be an object of the
//! reflected universe (levitation stage 1's `Universe`) and a transport law
//! relating it to the reflected function space, neither of which this fragment
//! builds.
//!
//! # Soundness posture (§8; ADR-68 D4)
//!
//! As with the undirected face, gandr carries **checkers and property tests**;
//! the theorem-grade claims are **not made in Rust**. The directed J's
//! soundness, the general Fubini/(co)Yoneda naturality, and the boundary
//! theorem for *all* invertible signatures ride the Agda face. What this
//! module carries is engineering
//! evidence: the polarity side condition as a total checker, the finite
//! (co)end transformations as replay-anchored constructions, and per-rung
//! property tests.
//!
//! # Draft-ADR candidates (none minted here)
//!
//! ADR-68 and ADR-69 cover this lane's decisions; this module mints **no** ADR.
//! Two observations are recorded as draft-ADR candidates for the orchestrator:
//! (1) whether the directed **variance** enum should be promoted to a shared
//! kind carried by the reflected universe once directed univalence is scoped
//! (versus the module-local reading here); and (2) whether the finite-carrier
//! **(co)end** representation ([`coend::Diagram`]) becomes the canonical
//! reflected end-object once the typed `Desc` layer can express dipresheaves
//! (versus its present role as a property-test vehicle).

pub mod boundary;
pub mod coend;
pub mod context;
pub mod hom;

pub use crate::directed::boundary::CutOutcome;
pub use crate::directed::boundary::all_participating_invertible;
pub use crate::directed::boundary::directed_cut;
pub use crate::directed::coend::BiDiagram;
pub use crate::directed::coend::Coend;
pub use crate::directed::coend::Diagram;
pub use crate::directed::coend::End;
pub use crate::directed::coend::coyoneda_collapse;
pub use crate::directed::coend::discrete_hom_inhabited;
pub use crate::directed::coend::fubini_swap;
pub use crate::directed::context::DirectedContext;
pub use crate::directed::context::OpSig;
pub use crate::directed::context::Variance;
pub use crate::directed::context::VarianceError;
pub use crate::directed::hom::DirectedHom;
pub use crate::directed::hom::DirectedJ;
pub use crate::directed::hom::JError;
pub use crate::directed::hom::MotiveShape;
pub use crate::directed::hom::check_directed_j;
