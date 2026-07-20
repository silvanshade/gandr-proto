//! The tagged **description table** — `DataDesc` *is* the decl table (ADR-54
//! §5; proposal-levitation.md §3).
//!
//! A [`DataDesc`] bundles the minted [`NominalId`], the graded/attributed
//! [`ParamDesc`]s and [`CtorDesc`]s (the σ tag over constructors, each with a
//! first-order [`crate::Code`]), the reserved [`OpDesc`] operations and
//! [`crate::CellFace`] 2-cells, and the [`DeclPolarity`] (V6). Every one of
//! ADR-54 §5's five flagged extension points is a field here, so the
//! anti-retrofit checklist is satisfied by construction (proposal §3).

use gandr_core_checker::grade::Grade;

use crate::arity::BridgeArity;
use crate::boundary::NominalSerial;
use crate::boundary::RecursiveStatus;
use crate::boundary::SurfaceByteOffset;
use crate::cell::CellFace;
use crate::code::Attrs;
use crate::code::Code;
use crate::code::Name;

/// The minted **0-cell identity** of a datatype (ADR-54 §3.4; ADR-50's "third
/// identity discipline" keys interning on it).
///
/// Carries a per-elaboration `serial` (assigned in declaration order) plus the
/// datatype's `name`, so the id is both distinct and self-describing. Decidable
/// equality includes both, so two descriptions are equal only when they name
/// the same minted datatype.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the minted identity is exactly {serial, name}; the elaborator mints ids in declaration order"
)]
pub struct NominalId
{
    /// The monotone serial assigned in declaration order within one
    /// elaboration.
    pub serial: u64,
    /// The datatype's declared name.
    pub name: Name,
}

impl NominalId
{
    /// Mint an identity with the given serial and name.
    #[inline]
    #[must_use]
    pub fn new<N>(
        serial: NominalSerial,
        name: N,
    ) -> Self
    where
        N: Into<Name>,
    {
        Self {
            serial: u64::from(serial),
            name: name.into(),
        }
    }
}

/// A **surface span** `[start, end)` in source bytes — provenance for a
/// description element.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a span is exactly its half-open byte range; the elaborator fills it from the CST node range"
)]
pub struct SurfaceSpan
{
    /// The start byte (inclusive).
    pub start: SurfaceByteOffset,
    /// The end byte (exclusive).
    pub end: SurfaceByteOffset,
}

impl SurfaceSpan
{
    /// A span over the half-open byte range `[start, end)`.
    #[inline]
    #[must_use]
    pub fn new(
        start: SurfaceByteOffset,
        end: SurfaceByteOffset,
    ) -> Self
    {
        Self { start, end }
    }
}

/// The **polarity** of a declared datatype (V6).
///
/// [`DeclPolarity::Data`] μ-decodes into the positive value universe
/// (constructors are producers, eliminated by patterns);
/// [`DeclPolarity::Codata`] ν-decodes into the negative computation universe
/// (fields read as observations, introduced by copatterns). Both decoders read
/// **one** code grammar; the ν decoder is a later lane, so only
/// the tag ships now.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the μ/ν polarity split is the closed two-way vocabulary V6 fixes; a datatype is data or codata"
)]
pub enum DeclPolarity
{
    /// `data` — μ-decoded into the positive value universe.
    Data,
    /// `codata` — ν-decoded into the negative computation universe.
    Codata,
}

/// A datatype **parameter** with its grade and attribute decorations
/// (proposal §3).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a stage-0 parameter is exactly its {name, grade, attributes}; the elaborator builds it from the type-parameter list"
)]
pub struct ParamDesc
{
    /// The parameter's name (`a` in `Maybe(a)`).
    pub name: Name,
    /// The parameter's grade (erased by the value decoder).
    pub grade: Grade,
    /// The parameter's attribute Σ.
    pub attrs: Attrs,
}

impl ParamDesc
{
    /// A parameter with the given name, grade, and attribute Σ.
    #[inline]
    #[must_use]
    pub fn new<N>(
        name: N,
        grade: Grade,
        attrs: Attrs,
    ) -> Self
    where
        N: Into<Name>,
    {
        Self {
            name: name.into(),
            grade,
            attrs,
        }
    }
}

/// A **constructor** (a 1-cell) — its name, payload [`Code`], and attribute Σ
/// (proposal §3).
///
/// The `result` is the reserved GADT result-type annotation (`Lit(x) :
/// Expr(a)`) as its surface spelling; at stage 0 it is retained for inspection
/// only.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a stage-0 constructor is exactly its {name, code, reserved GADT result, attributes}; the elaborator builds it from a data member"
)]
pub struct CtorDesc
{
    /// The constructor's name.
    pub name: Name,
    /// The constructor's payload code (`1` when nullary; a right-nested product
    /// of field codes otherwise).
    pub code: Code,
    /// The reserved GADT result annotation's surface spelling, if present.
    pub result: Option<Name>,
    /// The constructor's attribute Σ.
    pub attrs: Attrs,
}

impl CtorDesc
{
    /// A constructor with the given name, payload code, GADT result, and
    /// attribute Σ.
    #[inline]
    #[must_use]
    pub fn new<N>(
        name: N,
        code: Code,
        result: Option<Name>,
        attrs: Attrs,
    ) -> Self
    where
        N: Into<Name>,
    {
        Self {
            name: name.into(),
            code,
            result,
            attrs,
        }
    }
}

/// A reserved **operation** member (`op f(…) -> R`) — its name, multi-out
/// [`BridgeArity`], and attribute Σ (proposal §3–§4.2).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a stage-0 operation is exactly its {name, bridge arity, attributes}; the elaborator builds it from an op member"
)]
pub struct OpDesc
{
    /// The operation's name.
    pub name: Name,
    /// The operation's multi-out arity.
    pub arity: BridgeArity,
    /// The operation's attribute Σ.
    pub attrs: Attrs,
}

impl OpDesc
{
    /// An operation with the given name, arity, and attribute Σ.
    #[inline]
    #[must_use]
    pub fn new<N>(
        name: N,
        arity: BridgeArity,
        attrs: Attrs,
    ) -> Self
    where
        N: Into<Name>,
    {
        Self {
            name: name.into(),
            arity,
            attrs,
        }
    }
}

/// The **tagged description** — the decl table for one datatype (proposal §3,
/// V2).
///
/// `id` keys interning; `params` are the graded/attributed parameters; `ctors`
/// are the constructors (the σ tag), each with a first-order [`Code`]; `ops`
/// and `cells` are the reserved operations and 2-cell faces; `polarity` selects
/// the μ or ν decoder (V6); `attrs` is the datatype's own attribute Σ. There is
/// **no parallel structure**: `DataDesc` *is* the decl table (proposal §3).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "DataDesc is the decl table: every ADR-54 §5 extension point is a field here by construction, and the elaborator constructs it from a data/codata block"
)]
pub struct DataDesc
{
    /// The minted 0-cell identity (ADR-54 §3.4).
    pub id: NominalId,
    /// The datatype parameters (each graded and attributed).
    pub params: Box<[ParamDesc]>,
    /// The constructors — the σ tag over the code grammar (the 1-cells).
    pub ctors: Box<[CtorDesc]>,
    /// The reserved operations (ADR-54 §3.1).
    pub ops: Box<[OpDesc]>,
    /// The reserved 2-cell faces (V3).
    pub cells: Box<[CellFace]>,
    /// The declared polarity — `Data` (μ) or `Codata` (ν) (V6).
    pub polarity: DeclPolarity,
    /// The datatype's own attribute Σ.
    pub attrs: Attrs,
}

impl DataDesc
{
    /// A description from its minted id, parts, and polarity.
    ///
    /// The constructor performs **no** well-formedness checking (an ill-formed
    /// description is representable so it can be *rejected with a diagnostic*
    /// by [`crate::check_desc`], not merely unconstructible — proposal §8's
    /// pathological goldens). Callers that need the guarantee run
    /// [`crate::check_desc`].
    #[inline]
    #[must_use]
    pub fn new<Pa, Ct, Op, Ce>(
        id: NominalId,
        params: Pa,
        ctors: Ct,
        ops: Op,
        cells: Ce,
        polarity: DeclPolarity,
        attrs: Attrs,
    ) -> Self
    where
        Pa: Into<Box<[ParamDesc]>>,
        Ct: Into<Box<[CtorDesc]>>,
        Op: Into<Box<[OpDesc]>>,
        Ce: Into<Box<[CellFace]>>,
    {
        Self {
            id,
            params: params.into(),
            ctors: ctors.into(),
            ops: ops.into(),
            cells: cells.into(),
            polarity,
            attrs,
        }
    }

    /// Whether any constructor's payload is recursive (contains an
    /// [`Code::Var`]).
    #[inline]
    #[must_use]
    pub fn is_recursive(&self) -> RecursiveStatus
    {
        self.ctors
            .iter()
            .any(|ctor| bool::from(ctor.code.is_recursive()))
            .into()
    }
}
