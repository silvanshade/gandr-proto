//! The tagged **description table** — `DataDesc` *is* the decl table (ADR-54
//! §5; proposal-levitation.md §3).
//!
//! A [`DataDesc`] bundles the minted [`NominalId`], the declared
//! [`SortDesc`] sort set (the description universe's index), the
//! graded/attributed [`ParamDesc`]s and [`CtorDesc`]s (the σ tag over
//! constructors, each with a first-order [`crate::Code`] and a result sort),
//! the reserved [`OpDesc`] operations and [`crate::CellFace`] 2-cells, and
//! the [`DeclPolarity`] (V6). Every one of ADR-54 §5's five flagged extension
//! points is a field here, so the anti-retrofit checklist is satisfied by
//! construction (proposal §3).

use gandr_core_checker::grade::Grade;

use crate::arity::BridgeArity;
use crate::boundary::NominalSerial;
use crate::boundary::RecursiveStatus;
use crate::boundary::SurfaceByteOffset;
use crate::cell::CellFace;
use crate::circuit::CircuitRule;
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

/// The **polarity** of a declared sort or degenerate block (V6): which
/// fixpoint the declaration names.
///
/// Two independent statements, deliberately not fused — the mixed
/// induction-coinduction sweep confirmed the earlier fused clause unsourced
/// (gandr-ng9.20, mixed-sweep-verdict-02): nothing in the swept literature
/// licenses deriving either statement from the other.
///
/// * **Which fixpoint decodes it.** [`DeclPolarity::Data`] is μ-decoded
///   (constructors are producers, eliminated by patterns);
///   [`DeclPolarity::Codata`] is ν-decoded (members read as observations,
///   introduced by copatterns). Both decoders read **one** code grammar; the ν
///   decoder is a later lane, so only the tag ships now.
/// * **Where the decoded type lands.** In gandr's CBPV core the μ decoder
///   targets the positive value universe and the ν decoder the negative
///   computation universe. That placement is gandr's own design choice,
///   *correlated with* the fixpoint here rather than entailed by it: swept
///   calculi carry both fixpoints in one universe, and the anchor calculus
///   makes every coinductive term a value — so universe placement must not be
///   read off the flag, nor the flag off placement.
///
/// The flag only **names** the fixpoint. The polarity-specific obligations —
/// positivity for μ-sorts, productivity for ν-sorts — are discharged in the
/// sorting discipline over the declared sort set ([`crate::check_desc`] and
/// the sign block's sorting rules), never by the tag itself.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeclPolarity
{
    /// `data` — the μ fixpoint; μ-decoded into the positive value universe.
    Data,
    /// `codata` — the ν fixpoint; ν-decoded into the negative computation
    /// universe.
    Codata,
}

/// A declared **sort** of a signature (a 0-cell), with the fixpoint its
/// declaration names.
///
/// The sort set is the **index of the description universe** (the signature
/// unification's sort-index move): a multi-sorted signature is one
/// description over a sort set, with each constructor targeting a sort
/// through [`CtorDesc::result`] and each recursive occurrence naming its sort
/// at [`Code::Var`]. A degenerate `data` / `codata` block declares exactly
/// one sort, named by the block itself. The index is genuine — a sort is
/// never itself a description (no self-hosting), and only finitely many
/// declarations are importable, which is what keeps code equality decidable.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SortDesc
{
    /// The sort's name.
    pub name: Name,
    /// The fixpoint the sort's declaration names (μ for `data`, ν for
    /// `codata`).
    pub polarity: DeclPolarity,
}

impl SortDesc
{
    /// A sort of the given name and polarity.
    #[inline]
    #[must_use]
    pub fn new<N>(
        name: N,
        polarity: DeclPolarity,
    ) -> Self
    where
        N: Into<Name>,
    {
        Self {
            name: name.into(),
            polarity,
        }
    }
}

/// A datatype **parameter** with its grade and attribute decorations
/// (proposal §3).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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

/// A **constructor** (a 1-cell) — its name, payload [`Code`], result sort,
/// and attribute Σ (proposal §3).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CtorDesc
{
    /// The constructor's name.
    pub name: Name,
    /// The constructor's payload code (`1` when nullary; a right-nested product
    /// of field codes otherwise).
    pub code: Code,
    /// The constructor's **result sort** — the declared sort this constructor
    /// targets (the real output-sort slot of the sort-indexed description
    /// universe). In the degenerate single-sort block this is the block's own
    /// sort; a sign-block `data` member writes it as the output port's sort;
    /// a generalized (GADT-style) constructor result populates it with the
    /// annotation's head. Membership in the declared sort set is checked by
    /// [`crate::check_desc`].
    pub result: Name,
    /// The constructor's attribute Σ.
    pub attrs: Attrs,
}

impl CtorDesc
{
    /// A constructor with the given name, payload code, result sort, and
    /// attribute Σ.
    #[inline]
    #[must_use]
    pub fn new<N, R>(
        name: N,
        code: Code,
        result: R,
        attrs: Attrs,
    ) -> Self
    where
        N: Into<Name>,
        R: Into<Name>,
    {
        Self {
            name: name.into(),
            code,
            result: result.into(),
            attrs,
        }
    }

    /// The constructor read as a **single-output operation** over the sort
    /// set — the container view under which the constructor layer and
    /// [`BridgeArity`] carry one shape (the signature unification made the
    /// agreement checkable rather than implicit).
    ///
    /// The payload code denotes a polynomial over sorts and symbolic types:
    /// products multiply factors into a monomial, inline sums contribute one
    /// monomial per summand, an atom-abstraction contributes its body's
    /// factors, and every leaf becomes one input port — a [`Code::Var`] at
    /// its sort, a [`Code::Field`] at its symbolic type head, a
    /// [`Code::Unit`] contributing no port. Every monomial feeds the one
    /// output port, which reads at the result sort.
    ///
    /// # Contract
    /// - ensures: the returned arity composes (every `source` index lies in
    ///   `inputs`, every `dest` index is `0`, one factor count per monomial);
    ///   input ports are positionally named `x0, x1, …` in leaf order.
    /// - panics: never.
    #[inline]
    #[must_use]
    pub fn arity(&self) -> BridgeArity
    {
        fn monomials(code: &Code) -> Vec<Vec<Name>>
        {
            match *code {
                | Code::Unit => vec![Vec::new()],
                | Code::Var(ref sort) => vec![vec![sort.clone()]],
                | Code::Field(ref vref, ..) => {
                    let sort: Name = match *vref {
                        | crate::code::ValueTypeRef::Param(ref name) => name.clone(),
                        | crate::code::ValueTypeRef::Prim(prim) => prim.label().into(),
                        | crate::code::ValueTypeRef::Ctor { ref head, .. } => head.clone(),
                    };
                    vec![vec![sort]]
                },
                | Code::Prod(ref left, ref right) => {
                    let rights = monomials(right);
                    monomials(left)
                        .into_iter()
                        .flat_map(|left_monomial| {
                            rights.iter().map(move |right_monomial| {
                                let mut combined = left_monomial.clone();
                                combined.extend(right_monomial.iter().cloned());
                                combined
                            })
                        })
                        .collect()
                },
                | Code::Sum(ref left, ref right) => {
                    let mut summands = monomials(left);
                    summands.extend(monomials(right));
                    summands
                },
                | Code::Bind(_, ref body) => monomials(body),
            }
        }

        let monomials = monomials(&self.code);
        let mut inputs = Vec::new();
        let mut factors = Vec::new();
        let mut source = Vec::new();
        for monomial in &monomials {
            factors.push(u32::try_from(monomial.len()).unwrap_or(u32::MAX));
            for sort in monomial {
                let index = u32::try_from(inputs.len()).unwrap_or(u32::MAX);
                inputs.push(crate::arity::SortRef::new(
                    format!("x{}", inputs.len()),
                    sort.clone(),
                ));
                source.push(index);
            }
        }
        let dest = vec![0u32; monomials.len()];
        BridgeArity::new(inputs, factors, source, dest, [crate::arity::SortRef::new(
            "result",
            self.result.clone(),
        )])
    }
}

/// A reserved **operation** member (`op f(…) -> R`) — its name, multi-out
/// [`BridgeArity`], and attribute Σ (proposal §3–§4.2).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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

/// The **tagged description** — the decl table for one signature (proposal
/// §3, V2).
///
/// `id` keys interning; `sorts` is the declared sort set (the index of the
/// description universe — one entry per declared sort, the degenerate block
/// contributing exactly one); `params` are the graded/attributed parameters;
/// `ctors` are the constructors (the σ tag), each with a first-order [`Code`]
/// and a result sort; `ops` and `cells` are the reserved operations and
/// 2-cell faces; `circuits` are the 2-cell members whose boundaries are
/// derived from a wiring; `polarity` selects the μ or ν decoder (V6); `attrs`
/// is the datatype's own attribute Σ. There is **no parallel structure**:
/// `DataDesc` *is* the decl table (proposal §3).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DataDesc
{
    /// The minted 0-cell identity (ADR-54 §3.4).
    pub id: NominalId,
    /// The declared **sort set** — the description universe's index. The
    /// degenerate `data` / `codata` block declares exactly one sort, named by
    /// the block; a sign block declares its `sort` members, defaulting to the
    /// block's own name when it writes none.
    pub sorts: Box<[SortDesc]>,
    /// The datatype parameters (each graded and attributed).
    pub params: Box<[ParamDesc]>,
    /// The constructors — the σ tag over the code grammar (the 1-cells).
    pub ctors: Box<[CtorDesc]>,
    /// The reserved operations (ADR-54 §3.1).
    pub ops: Box<[OpDesc]>,
    /// The reserved 2-cell faces (V3).
    pub cells: Box<[CellFace]>,
    /// The reserved **circuit rule** members: a declared sphere with the wiring
    /// that must derive it ([`CircuitRule`]). The declaration table checks the
    /// derived pair against the declared sphere ([`crate::check_desc`]).
    pub circuits: Box<[CircuitRule]>,
    /// The declaration's polarity — `Data` (μ) or `Codata` (ν) (V6).
    ///
    /// This is the whole-declaration tag of the polarity-homogeneous reading:
    /// every entry of `sorts` must agree with it, and [`crate::check_desc`]
    /// enforces the agreement. Sorts of differing polarity in one signature
    /// are the ruled-in growth this field generalizes into (the per-sort tags
    /// in `sorts` are the general carrier); internal polarity alternation
    /// within one sort is a separate universe-change decision, deliberately
    /// not taken (gandr-ng9.20, mixed-sweep-input-02).
    pub polarity: DeclPolarity,
    /// The datatype's own attribute Σ.
    pub attrs: Attrs,
}

impl DataDesc
{
    /// A description from its minted id, parts, and polarity.
    ///
    /// The sort set defaults to the **degenerate single sort** — one entry
    /// named by the declaration itself, at the declaration's polarity (the
    /// `data` / `codata` sugar's reading). A multi-sorted sign block replaces
    /// it through [`DataDesc::with_sorts`].
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
        let sorts = Box::from([SortDesc::new(id.name.clone(), polarity)]);
        Self {
            id,
            sorts,
            params: params.into(),
            ctors: ctors.into(),
            ops: ops.into(),
            cells: cells.into(),
            circuits: Box::default(),
            polarity,
            attrs,
        }
    }

    /// The same description carrying `sorts` as its declared sort set.
    ///
    /// The sign block's `sort` members land here; every existing caller keeps
    /// the degenerate default [`DataDesc::new`] supplies.
    #[inline]
    #[must_use]
    pub fn with_sorts<So>(
        self,
        sorts: So,
    ) -> Self
    where
        So: Into<Box<[SortDesc]>>,
    {
        Self {
            sorts: sorts.into(),
            ..self
        }
    }

    /// The same description carrying `circuits` as its circuit rule members.
    ///
    /// Circuit rules are added through this builder rather than through
    /// [`DataDesc::new`] so that every existing caller keeps the empty default:
    /// a description with no derived-boundary member is exactly what it was.
    #[inline]
    #[must_use]
    pub fn with_circuits<Ci>(
        self,
        circuits: Ci,
    ) -> Self
    where
        Ci: Into<Box<[CircuitRule]>>,
    {
        Self {
            circuits: circuits.into(),
            ..self
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
