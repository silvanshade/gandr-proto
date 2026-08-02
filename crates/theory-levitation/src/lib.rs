//! Levitation **stage 0**: datatypes as descriptions, Rust-side (ADR-67;
//! the levitation proposal, with the VDC deltas of
//! `proposal-levitation-addendum-vdc.md`).
//!
//! A datatype's *description* is a first-class value, so generic operations
//! over datatypes are ordinary programs over descriptions — derive-style
//! codegen, wire serialization, structural equality, content-addressed
//! interning, and the uniform handling the polygraph layer needs. This crate is
//! the stage-0 rung of the four-stage ladder (proposal §2): descriptions are
//! **Rust values** and the generic functions are **Rust functions**; the
//! meta-theory functions (`decode`, `μ`, `induction`) stay host-side until
//! stage 1+ moves them, one at a time, into the checker. Nothing here needs a
//! dependent type.
//!
//! # The code universe (`code`)
//!
//! The canonical decl-table shape is the **tagged description** (`tagDesc`,
//! V2): an enumeration of constructors, each carrying a first-order [`Code`].
//! The code grammar's MVP is the finitary first-order fragment `{1, var, ×, σ}`
//! ([`Code::Unit`], [`Code::Var`], [`Code::Prod`], [`Code::Sum`]) plus the V5
//! leaf decorations — a graded, attributed [`Code::Field`] over a core value
//! type, and a [`Code::Bind`] atom-abstraction. The fragment is first-order
//! **so that code equality stays decidable**: higher-order codes
//! (function-typed fields) are *excluded from the fragment*, not merely
//! deferred (proposal §3).
//!
//! Decidable equality is load-bearing, not cosmetic: it is what
//! content-addressing interns on ([`CodeInterner`]) and what the
//! matching-modulo engine compares (proposal §3, ADR-54 §4.3). [`Code`] derives
//! total structural [`Eq`]/[`Hash`]; the [`code`] module's tests witness
//! reflexivity, per-variant distinctness, and use as a hash-map key.
//!
//! # 2-cell faces (`cell`)
//!
//! A rewrite `lhs ~> rhs` over a signature is a **pair of open terms in the
//! free structure over that signature** — elements of the free monad `D⋆(V)`
//! (V3). Stage 0 stores them untyped-but-host-checked as [`FreeTerm`] pairs in
//! a [`CellFace`]; the typed encoding (a dependent Σ over the signature) is the
//! stage-1 refinement and changes only the *checking*, never the encoding
//! (proposal §4.1). The VDC delta: each [`CellFace`] carries derived
//! [`CellVarMeta`] per pattern variable, whose [`Variance`] is the constant
//! [`Variance::Producer`] at stage 0 (no consumer-argument operations exist
//! yet) — the field ships now so the sequent-era refinement is an *update*, not
//! a migration (addendum §A).
//!
//! # Circuit rules (`circuit`)
//!
//! A [`CircuitRule`] is a 2-cell member whose boundary pair is **derived from a
//! wiring** rather than written: its body is a list of port-named frame and
//! redex applications, and the source boundary is that diagram with every redex
//! replaced by its source, the target boundary with every redex replaced by its
//! target ([`derive_boundaries`]). The declared sphere stays the declaration's:
//! [`check_desc`] checks the derived pair against it, so a mis-glued boundary
//! fails at the decl table
//! (`docs/gandr/spec/surface-language/circuit-cells.md`, section "Frame and
//! redex").
//!
//! # Elaborating a circuit block (`elaborate`)
//!
//! A rewrite-sorted port (`rule p : Nat ==> Nat`) is sorted by the 2-cell face
//! and binds the interface pair `(a, b, ρ : a ==> b)` — the shape a hole
//! carries — with the pinned form `rule p : x ==> x′` available when the
//! interface should name its endpoints ([`RewritePort`], [`InterfacePair`]).
//! Instantiating one at a redex line unifies the source against the line's
//! input wiring and binds the target, producing the [`CircuitRedex`] the
//! derivation consumes. A block then elaborates to the boundary language's
//! whiskered composite of its redex inside its frames ([`elaborate_body`],
//! [`WhiskeredCell`]), against the boundaries the sphere check already fixed
//! (`docs/gandr/spec/surface-language/circuit-cells.md`, section "Sorting a
//! rewrite port"; `docs/gandr/spec/surface-language/higher-cells.md`, section
//! "The boundary language").
//!
//! # Multi-out arities (`arity`)
//!
//! A multi-output operation is presented by the **bridge diagram**
//! `A ←s— J —π→ I —t→ B` ([`BridgeArity`], V4): four finite sets and three
//! maps, separating the Π-layer (one operation's named result tuple) from the
//! Σ-layer (destination aggregation, which independently requires a commutative
//! monoid — ADR-49's "Σ-zone" firewall). The encoding is finite sets + maps:
//! content-addressable and first-order (proposal §4.2).
//!
//! # The description table and polarity (`desc`)
//!
//! [`DataDesc`] *is* the decl table (ADR-54 §5): the minted [`NominalId`], the
//! graded/attributed parameters and constructors, the reserved operations and
//! 2-cell faces, and the [`DeclPolarity`] (`Data` μ-decoded | `Codata`
//! ν-decoded, V6). The ν decoder itself is a later lane; only the
//! polarity *tag* ships now.
//!
//! # The generic consumers (`generic`, `intern`, `builtin`)
//!
//! Stage 0 lands **with real consumers**, refuting the named dead-end — "a
//! beautiful decl table nobody decodes" (proposal §11). The description drives:
//!
//! * [`generic_eq`] — structural equality of two [`DescValue`]s guided by a
//!   [`Code`];
//! * [`serialize_value`] — a canonical, deterministic wire encoding of a
//!   [`DescValue`] guided by a [`Code`];
//! * [`serialize_desc`] — the inspectable-IR rendering of a whole [`DataDesc`];
//! * [`CodeInterner`] — content-addressed interning keyed on decidable code
//!   equality.
//!
//! [`builtin`] retrofits primitive formers (`Bool`, pairs, sums, `Option`,
//! `List`) as [`DataDesc`]s so the same generic programs cover builtins and
//! declared data uniformly (proposal §3).
//!
//! # Stage 1: the decoder and typed cells (`decode`, `typed_cell`)
//!
//! The stage-1 dependent-core bill (ADR-81; proposal §6) moves the first meta
//! function from Rust-as-unwritten-meta-theory into a written host function and
//! adds the typed cell face:
//!
//! * [`decode`](fn@decode) / [`decode_desc`] — **large elimination** (bill
//!   feature 3): the decoder `Desc → ValueType`, interpreting the decidable
//!   first-order fragment `{1, var, ×, σ}` (+ the [`Code::Field`] leaf) into
//!   the core value-type universe. It targets the frozen-core code universe
//!   ([`gandr_core_checker::types::ValueType::Universe`], feature 1); the
//!   dependent pair [`gandr_core_checker::types::ValueType::Sigma`] (feature 2)
//!   is the stage-1 capability the decoder will target once a dependent σ
//!   *code* lands (the current fragment is non-dependent — see the
//!   [`decode`](mod@decode) module docs).
//! * [`typed_cell`] — the **typed 2-cell face**: a stage-0 [`CellFace`] refined
//!   with a decoded [`SignatureContext`] (the "dependent Σ over the signature",
//!   addendum §4.1). Additive — [`CellFace`] is reused whole, not rewritten.

extern crate alloc;

pub mod arity;
pub mod boundary;
pub mod builtin;
pub mod cell;
pub mod circuit;
pub mod code;
pub mod decode;
pub mod desc;
pub mod elaborate;
pub mod generic;
pub mod intern;
pub mod typed_cell;
pub mod wellformed;

pub use gandr_core_checker::boundary::ConstructorTag;
pub use gandr_core_checker::boundary::NameRef;
pub use gandr_core_checker::boundary::StringText;

pub use crate::arity::BridgeArity;
pub use crate::arity::SortRef;
pub use crate::boundary::AttributeEmptiness;
pub use crate::boundary::AttributePresence;
pub use crate::boundary::CellEquivalence;
pub use crate::boundary::CellVariableLinearity;
pub use crate::boundary::CircuitNodeBudget;
pub use crate::boundary::ContextTotality;
pub use crate::boundary::DescriptorFactorCount;
pub use crate::boundary::DescriptorFactorIndex;
pub use crate::boundary::DiagnosticMessage;
pub use crate::boundary::FirstOrderStatus;
pub use crate::boundary::GeneratorIndex;
pub use crate::boundary::GenericEquality;
pub use crate::boundary::InternedCodeCount;
pub use crate::boundary::InternerEmptiness;
pub use crate::boundary::LooseInstanceEquality;
pub use crate::boundary::MonomialCount;
pub use crate::boundary::MonomorphicStatus;
pub use crate::boundary::NominalSerial;
pub use crate::boundary::NumeralCount;
pub use crate::boundary::PatternMatch;
pub use crate::boundary::PortArgumentCount;
pub use crate::boundary::RecursiveStatus;
pub use crate::boundary::ReplayClassCount;
pub use crate::boundary::ReplayEquivalence;
pub use crate::boundary::RewriteDepth;
pub use crate::boundary::RoundTripSampleCount;
pub use crate::boundary::RoundTripStatus;
pub use crate::boundary::SerializedDescText;
pub use crate::boundary::SerializedValueBytes;
pub use crate::boundary::SurfaceByteOffset;
pub use crate::boundary::SymbolNameMatchCount;
pub use crate::boundary::SymbolPresence;
pub use crate::boundary::TermPositionIndex;
pub use crate::cell::CellFace;
pub use crate::cell::CellVarMeta;
pub use crate::cell::FreeTerm;
pub use crate::cell::Variance;
pub use crate::circuit::BoundaryReading;
pub use crate::circuit::CircuitBody;
pub use crate::circuit::CircuitDerivationError;
pub use crate::circuit::CircuitFrame;
pub use crate::circuit::CircuitNode;
pub use crate::circuit::CircuitRedex;
pub use crate::circuit::CircuitRule;
pub use crate::circuit::DerivedBoundaries;
pub use crate::circuit::FrameHead;
pub use crate::circuit::derive_boundaries;
pub use crate::circuit::derive_boundaries_within;
pub use crate::circuit::derive_boundary;
pub use crate::circuit::derive_boundary_within;
pub use crate::code::AtomSort;
pub use crate::code::Attr;
pub use crate::code::Attrs;
pub use crate::code::Code;
pub use crate::code::Name;
pub use crate::code::PrimTy;
pub use crate::code::ValueTypeRef;
pub use crate::decode::DecodeError;
pub use crate::decode::decode;
pub use crate::decode::decode_desc;
pub use crate::desc::CtorDesc;
pub use crate::desc::DataDesc;
pub use crate::desc::DeclPolarity;
pub use crate::desc::NominalId;
pub use crate::desc::OpDesc;
pub use crate::desc::ParamDesc;
pub use crate::desc::SurfaceSpan;
pub use crate::elaborate::CircuitElaborationError;
pub use crate::elaborate::InterfacePair;
pub use crate::elaborate::PortFace;
pub use crate::elaborate::PortInstantiationError;
pub use crate::elaborate::RedexOccurrence;
pub use crate::elaborate::RewritePort;
pub use crate::elaborate::WhiskeredCell;
pub use crate::elaborate::elaborate_body;
pub use crate::generic::DescValue;
pub use crate::generic::Payload;
pub use crate::generic::Side;
pub use crate::generic::generic_eq;
pub use crate::generic::serialize_desc;
pub use crate::generic::serialize_value;
pub use crate::intern::CodeId;
pub use crate::intern::CodeInterner;
pub use crate::typed_cell::SignatureContext;
pub use crate::typed_cell::TypedCellFace;
pub use crate::wellformed::WfDiagnostic;
pub use crate::wellformed::WfKind;
pub use crate::wellformed::check_desc;
