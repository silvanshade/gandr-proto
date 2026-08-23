//! The minimal certified kernel's **S1 core**: the pure polarized CBPV
//! value/computation term and type language, the `Def`/`Axiom` declaration
//! vocabulary, the append-only environment and its single `add_decl` choke
//! point, the zero-inference S1 checker, and the C5-quarantined
//! definitional-equality conversion (decisions: the minimal certified
//! kernel, universe stratification).
//!
//! This is the second `gandr-kernel-*` subcrate — **trusted by the
//! kernel-boundary naming rule** — and depends only on
//! [`gandr_kernel_strata`] plus the dependency-free
//! [`gandr_kernel_conversion_trace`] seam, and on `core`/`alloc`: the sharpest
//! form of the TCB dependency wall (no other workspace crate, no external
//! runtime dependency). The conversion trace is an in-process session artifact
//! with consumer-owned identities, not a persistence or wire format.
//!
//! # The five boundary disciplines, as this crate holds them
//!
//! * **K1 — closed input.** The term, type, and declaration vocabularies are
//!   closed enums: no hole, no metavariable, no mark, no annotation, no
//!   effect-row constructor, no datatype kind **exists to be represented**
//!   (preference order: illegal states unrepresentable, then rejected at
//!   decode, then rejected at check). Levels are canonical by construction
//!   ([`gandr_kernel_strata::Level`]), so a non-canonical level is
//!   unrepresentable in memory.
//! * **K2 — zero inference.** [`Environment::add_decl`] re-derives every typing
//!   and well-formedness obligation; the elaborator's output gets no credence.
//!   Checking is bidirectional over annotation-free terms, needing no
//!   metavariables.
//! * **K3 — one choke point.** [`Environment::add_decl`] is the only checked
//!   entry; [`Environment::add_decl_unchecked`] is the single warned bypass,
//!   present from day one and tracked; a [`CheckedId`] is unforgeable outside
//!   the crate; [`Environment::audit`] is the `#print axioms` analogue. One
//!   checked/unchecked bit, no trust lattice.
//! * **K4 — derived schemes are never trusted.** No derived scheme exists at S1
//!   (datatypes and their eliminators arrive as description codes at S2); the
//!   discipline is honoured by there being nothing to trust yet.
//! * **K5 — re-checkable export.** The [`write()`] export writer serializes an
//!   [`Environment`] to canonical, self-contained bytes and the [`read`]
//!   validating reader replays them through the choke point (obligations
//!   E1–E6); the [`decode`] structural parser rejects a non-canonical artifact
//!   over a closed error vocabulary.
//!
//! # The S1 surface
//!
//! The pure polarized core (S1): literals over the rigid
//! base atoms ([`BaseType`]), unit, pair, and sum values, thunks, functions,
//! `let`/`force`/`case` computations, explicit universe lifts, and the universe
//! rule `U_l : U_m` iff `l < m` over the [`gandr_kernel_strata`] level algebra.
//! Excluded at S1 and deliberately unrepresentable: effects and handlers, the
//! control fragment, general recursion, datatype declarations, `Sigma`/`Split`,
//! `Path`/identity, `List`/`Record`/`With`, holes, marks, and annotations.
//!
//! # Conversion (C5)
//!
//! Conversion runs on the evidence/value fragment and **never evaluates a
//! computation** ([`crate::conv`]). At S1 the checker's conversion is type-only
//! and coincides with structural equality (canonical levels); the value/
//! computation conversion is present, quarantined, and unused by checking — the
//! precise algorithm and the vacuity of the β laws at S1 are the seed of the
//! term-conversion design pass.

#![no_std]

extern crate alloc;

// The sharing-collapse probe records into a thread-local, which `core` and
// `alloc` do not provide. It exists only under `cfg(test)`, so the shipped
// crate stays `no_std` and gains no dependency from it.
#[cfg(test)]
extern crate std;

mod arena;
mod base;
mod check;
mod conv;
mod decl;
mod env;
mod error;
mod export;
mod levels;
#[cfg(test)]
mod memo;
#[cfg(test)]
mod probe;
mod support;
mod term;
mod types;

pub use arena::CompTypeId;
pub use arena::ComputationId;
pub use arena::TermArena;
pub use arena::ValueId;
pub use arena::ValueTypeId;
pub use base::BaseType;
pub use base::FractionDigits;
pub use base::IntegerLiteral;
pub use base::Literal;
pub use base::Magnitude;
pub use base::NumericLiteral;
pub use base::Sign;
pub use base::StringLiteral;
pub use conv::Convertibility;
pub use conv::ReplayStatus;
pub use conv::TraceId as ConversionTraceId;
pub use conv::convertible_comp_types;
pub use conv::convertible_computations;
pub use conv::convertible_computations_with_sink;
pub use conv::convertible_value_types;
pub use conv::convertible_values;
pub use conv::convertible_values_with_sink;
pub use conv::replay_computations;
pub use conv::replay_values;
pub use decl::Declaration;
pub use decl::DeclarationBuilder;
pub use decl::DeclarationContent;
pub use decl::LevelSignature;
pub use env::AxiomReport;
pub use env::CheckedId;
pub use env::Environment;
pub use error::CompTypeSnapshot;
pub use error::ComputationTypeMismatch;
pub use error::ExpectedComputationShape;
pub use error::ExpectedValueShape;
pub use error::KernelError;
pub use error::LevelOrderRefutation;
pub use error::NonInferableForm;
pub use error::RegisterFault;
pub use error::UniverseViolation;
pub use error::ValueTypeMismatch;
pub use error::ValueTypeSnapshot;
pub use export::AdmissionMark;
pub use export::ArtifactHeaderBytes;
pub use export::ArtifactImage;
pub use export::DecodeError;
pub use export::DecodeMetrics;
pub use export::DecodedArtifact;
pub use export::DecodedDeclaration;
pub use export::EncodedArtifact;
pub use export::ExpandedWork;
pub use export::FORMAT_VERSION_V1;
pub use export::MAX_ARTIFACT_EXPANDED_WORK;
pub use export::MAX_DECODED_LEVEL_OFFSET;
pub use export::MAX_EXPANDED_TERM_WORK;
pub use export::MAX_TABLE_ENTRIES;
pub use export::MalformedSite;
pub use export::NODE_TAG_TABLE;
pub use export::NodeTagDescription;
pub use export::NodeTagVerdict;
pub use export::ReadError;
pub use export::ReservedKind;
pub use export::ReservedSlot;
pub use export::SegmentCount;
pub use export::SegmentSpans;
pub use export::SegmentedArtifact;
pub use export::Segments;
pub use export::TableEntryCount;
pub use export::TagSite;
pub use export::decode;
pub use export::read;
pub use export::write;
pub use export::write_segmented;
pub use gandr_kernel_conversion_trace::ConversionDecision;
pub use gandr_kernel_conversion_trace::NullSink;
pub use gandr_kernel_conversion_trace::TraceSink;
pub use levels::LevelContext;
pub use levels::LevelParamCount;
pub use term::Computation;
pub use term::ConstantIndex;
pub use term::DeBruijnIndex;
pub use term::Side;
pub use term::Value;
pub use types::CompType;
pub use types::ValueType;
