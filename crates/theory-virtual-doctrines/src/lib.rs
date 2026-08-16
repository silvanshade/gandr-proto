//! **The VDC reflection face** — the rewrite layer reflected as a virtual
//! double category, with `FVDblTT` as its internal language (the reflection
//! decisions: the formal reading and reflection face, and certificate
//! identity and composition).
//!
//! This crate is **strictly additive** and reflects — it does not re-implement
//! — the engine. It consumes [`gandr_theory_levitation`] (descriptions,
//! [`gandr_theory_levitation::FreeTerm`]) and [`gandr_theory_computads`]
//! (cells, tracelets, replay, two-mode composition), and exposes a first-order
//! judgment layer plus a query API over them.
//!
//! # The layers
//!
//! - [`vdc`] — the [`Vdc`] interface (objects / tight arrows / loose arrows /
//!   multi-ary cells, with restriction and multicategorical composition) and
//!   its [`CellStoreVdc`] instance: the dictionary reading of ADR-68 D1
//!   realized as the cell store realizes it ([`SignatureRef`], [`SigMorphism`],
//!   [`RelationRef`], [`Derivation`]).
//! - [`cartesian`] — checked W-actions and deterministic witnesses for
//!   projection, diagonal, and complete product-structure preservation.
//! - [`syntax`] — the reflected judgment layer: [`Protype`]s and [`Proterm`]s
//!   (`FVDblTT`'s four judgment families), **first-order, no dependent types**,
//!   with [`Proterm::Cert`] embedding an engine derivation.
//! - [`check`] — bidirectional [`Checker`] over two-sided [`Context`]s; the
//!   engine-fact rules are validated by **replay elaboration** (ADR-69).
//! - [`iso`] — [`ProtypeIso`] / [`IsoWitness`]: protype isomorphisms are
//!   **paired replayable witnesses** with groupoid laws, composed in the
//!   invertible mode ([`gandr_theory_computads::compose_invertible`]).
//! - [`query`] — the constructor-menu [`Query`] surface (path induction over
//!   rewrite traces, per-overlap seam composition, extension queries,
//!   instantiation tables).
//! - [`directed`] — the **LLV directed fragment** over the reflected layer, and
//!   the crate's largest face: variance-carrying contexts, [`DirectedHom`] with
//!   a J-eliminator restricted by polarity so symmetry stays underivable, and
//!   (co)ends as quantifiers with the Fubini and coYoneda operations. It is
//!   what [`syntax`] deliberately cannot express, and it stages behind
//!   levitation stage 1.
//!
//! # Soundness posture (§8; ADR-68 D4)
//!
//! gandr implements **checkers and property tests**; the **theorem-grade claims
//! are not made in Rust**. Nasu's syntax–semantics biadjunction (soundness +
//! completeness — "reflection loses nothing") and the groupoid-fragment
//! certificate-composition theorem ride the **Agda face** of the metatheory
//! tree. What this crate carries is engineering evidence:
//! per-rule property tests plus replay elaboration (each judgment-layer rule
//! that claims an engine fact replays the underlying certificate).
//!
//! # Open observations (none minted here)
//!
//! The reflection decisions cover this lane; this crate mints no decision
//! record. Two observations are recorded as open:
//! (1) the loose-arrow **granularity** (a named relation as a *set* of
//! generating cells, adopted here per §10.1's F0 reading, versus the
//! single-cell reading); and (2) the reflected-cell **identity model** (a
//! [`Derivation`] tree quotiented by replay-equivalence, elaborated to the
//! engine only through [`gandr_theory_computads::compose_invertible`]) — pin
//! either by ADR only if a later stage makes the reflected syntax canonical.

extern crate alloc;

pub mod boundary;
pub mod cartesian;
pub mod check;
pub mod directed;
pub mod iso;
pub mod query;
pub mod syntax;
pub mod vdc;

pub use crate::boundary::CartesianDiagonalPreservation;
pub use crate::boundary::CartesianProjectionPreservation;
pub use crate::boundary::CartesianStructurePreservation;
pub use crate::boundary::CertificateInvertibility;
pub use crate::boundary::CheckContextDeclaration;
pub use crate::boundary::CutCoherence;
pub use crate::boundary::CutDeclination;
pub use crate::boundary::DerivationIndex;
pub use crate::boundary::DerivationReplay;
pub use crate::boundary::DescTableEmptyStatus;
pub use crate::boundary::DescTableLength;
pub use crate::boundary::DiagramCarrierEmptyStatus;
pub use crate::boundary::DiagramCarrierLength;
pub use crate::boundary::DirectedContextDeclaration;
pub use crate::boundary::DirectedHomReflexivity;
pub use crate::boundary::DirectedObjectCovariance;
pub use crate::boundary::DiscreteHomInhabitation;
pub use crate::boundary::IsoValidity;
pub use crate::boundary::MotiveCovariance;
pub use crate::boundary::RewriteCompletion;
pub use crate::boundary::RewriteReachability;
pub use crate::boundary::RewriteStepBudget;
pub use crate::boundary::RoundTripIdentity;
pub use crate::boundary::SigMorphismIdentity;
pub use crate::boundary::VdcCellEquality;
pub use crate::cartesian::CartesianLawError;
pub use crate::cartesian::CartesianWitness;
pub use crate::cartesian::WCartesianAction;
pub use crate::check::CheckError;
pub use crate::check::Checker;
pub use crate::check::Context;
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
pub use crate::iso::IsoWitness;
pub use crate::iso::ProtypeIso;
pub use crate::query::ExtensionCandidate;
pub use crate::query::InstanceRow;
pub use crate::query::InstanceTable;
pub use crate::query::Query;
pub use crate::query::RewritePath;
pub use crate::query::SeamComposite;
pub use crate::syntax::DerivationId;
pub use crate::syntax::ProVar;
pub use crate::syntax::Proterm;
pub use crate::syntax::Protype;
pub use crate::vdc::CellStoreVdc;
pub use crate::vdc::Derivation;
pub use crate::vdc::DescTable;
pub use crate::vdc::Elaborated;
pub use crate::vdc::RelationRef;
pub use crate::vdc::SigMorphism;
pub use crate::vdc::SignatureRef;
pub use crate::vdc::TermRef;
pub use crate::vdc::Vdc;
