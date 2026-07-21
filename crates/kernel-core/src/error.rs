//! The closed kernel error vocabulary ([`KernelError`]) and its evidence
//! payloads.
//!
//! The vocabulary is **honest and closed** (kernel-boundary.md K1): every way
//! a declaration can fail to check surfaces as exactly one of these variants —
//! the checker and the choke point are total and never diverge
//! (docs/workflow/rust.md, "the checker and the machine are total"). Level and
//! universe failures carry the `gandr_kernel_strata` refutation evidence they
//! rest on, so a rejection is itself re-checkable.
//!
//! # Type-carrying payloads are self-contained arena snapshots (D1(C))
//!
//! A synthesized type lives in the environment arena, which the choke point
//! **truncates** after the verdict (dropping this declaration's content and the
//! checker's intermediates on rejection). A diagnostic payload therefore cannot
//! hold a bare arena id — it would dangle. Each type-carrying variant instead
//! owns a small [`TermArena`] **snapshot** of the reachable closure of its
//! root(s), reified before truncation ([`crate::arena`]). The snapshot's node
//! children are `Copy` ids, so the payload's `Drop`/`Clone` are flat and total
//! on any depth — the totality the owned-tree representation needed
//! hand-written worklists to guarantee.
//!
//! There is deliberately **no** non-canonical-level variant: an in-memory
//! `gandr_kernel_strata::Level` is canonical by construction. The
//! reject-non-canonical-input obligation is discharged by construction here and
//! re-armed at the decode boundary in the export reader (B2.2).

use alloc::boxed::Box;
use core::error::Error;
use core::fmt;

use gandr_kernel_strata::EntailmentCountermodel;
use gandr_kernel_strata::LeqRefutation;
use gandr_kernel_strata::Level;
use gandr_kernel_strata::LevelError;
use gandr_kernel_strata::LevelVar;
use gandr_kernel_strata::LoopWitness;
use gandr_kernel_strata::PosetError;

use crate::arena::CompTypeId;
use crate::arena::TermArena;
use crate::arena::ValueTypeId;

/// A checking-only term form met where the checker had to synthesize a type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the check-only S1 term forms are closed by design (kernel-boundary.md K1)"
)]
pub enum NonInferableForm
{
    /// A sum injection: its type is not determined by its payload alone, so it
    /// only checks against an expected sum type.
    Injection,
    /// A lambda: its domain is not annotated, so it only checks against an
    /// expected function type.
    Lambda,
}

/// The register polarity a checker-machine frame required — the payload of
/// [`KernelError::CheckerRegisterFault`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the checker machine has exactly two typed register polarities by design"
)]
pub enum RegisterFault
{
    /// A value-consuming frame found a non-value produced register.
    ExpectedValueType,
    /// A computation-consuming frame found a non-computation produced
    /// register.
    ExpectedCompType,
}

/// The value-type shape an eliminator or checking rule required.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the required value-type shapes are closed by design (kernel-boundary.md K1)"
)]
pub enum ExpectedValueShape
{
    /// A product type `A × B` (a pair was checked against it).
    Product,
    /// A sum type `A + B` (an injection was checked, or a case scrutinized).
    Sum,
    /// A thunk type `U C` (a thunk was checked, or a value was forced).
    Thunk,
}

/// The computation-type shape an eliminator or checking rule required.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the required computation-type shapes are closed by design (kernel-boundary.md K1)"
)]
pub enum ExpectedComputationShape
{
    /// A function type `A → C` (a lambda was checked, or something applied).
    Arrow,
    /// A returner type `F A` (a return was checked, or a bind sequenced).
    Returner,
}

/// A self-contained snapshot of one value type.
///
/// The reified reachable closure plus its root id, so the payload survives the
/// working arena's truncation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueTypeSnapshot
{
    /// The reified arena holding the value type's closure.
    arena: TermArena,
    /// The root id into [`Self::arena`].
    root: ValueTypeId,
}

impl ValueTypeSnapshot
{
    /// Record a value-type snapshot (its reified arena and root id).
    #[inline]
    #[must_use]
    pub(crate) fn new(
        arena: TermArena,
        root: ValueTypeId,
    ) -> Self
    {
        Self { arena, root }
    }

    /// The snapshot's arena.
    #[inline]
    #[must_use]
    pub const fn arena(&self) -> &TermArena
    {
        &self.arena
    }

    /// The value type's root id into [`Self::arena`].
    #[inline]
    #[must_use]
    pub const fn root(&self) -> ValueTypeId
    {
        self.root
    }
}

/// A self-contained snapshot of one computation type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompTypeSnapshot
{
    /// The reified arena holding the computation type's closure.
    arena: TermArena,
    /// The root id into [`Self::arena`].
    root: CompTypeId,
}

impl CompTypeSnapshot
{
    /// Record a computation-type snapshot.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        arena: TermArena,
        root: CompTypeId,
    ) -> Self
    {
        Self { arena, root }
    }

    /// The snapshot's arena.
    #[inline]
    #[must_use]
    pub const fn arena(&self) -> &TermArena
    {
        &self.arena
    }

    /// The computation type's root id into [`Self::arena`].
    #[inline]
    #[must_use]
    pub const fn root(&self) -> CompTypeId
    {
        self.root
    }
}

/// A definitional-inequality between two value types.
///
/// The checker synthesized `actual` where `expected` was required, and the two
/// do not convert. Both roots are reified into one self-contained snapshot
/// arena.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueTypeMismatch
{
    /// The snapshot arena holding both roots' closures.
    arena: TermArena,
    /// The value type the context required (root into [`Self::arena`]).
    expected: ValueTypeId,
    /// The value type the checker synthesized (root into [`Self::arena`]).
    actual: ValueTypeId,
}

impl ValueTypeMismatch
{
    /// Record a value-type mismatch over a shared snapshot arena.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        arena: TermArena,
        expected: ValueTypeId,
        actual: ValueTypeId,
    ) -> Self
    {
        Self {
            arena,
            expected,
            actual,
        }
    }

    /// The snapshot arena holding both roots.
    #[inline]
    #[must_use]
    pub const fn arena(&self) -> &TermArena
    {
        &self.arena
    }

    /// The value type the context required (root into [`Self::arena`]).
    #[inline]
    #[must_use]
    pub const fn expected(&self) -> ValueTypeId
    {
        self.expected
    }

    /// The value type the checker synthesized (root into [`Self::arena`]).
    #[inline]
    #[must_use]
    pub const fn actual(&self) -> ValueTypeId
    {
        self.actual
    }
}

/// A definitional-inequality between two computation types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputationTypeMismatch
{
    /// The snapshot arena holding both roots' closures.
    arena: TermArena,
    /// The computation type the context required (root into [`Self::arena`]).
    expected: CompTypeId,
    /// The computation type the checker synthesized (root into
    /// [`Self::arena`]).
    actual: CompTypeId,
}

impl ComputationTypeMismatch
{
    /// Record a computation-type mismatch over a shared snapshot arena.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        arena: TermArena,
        expected: CompTypeId,
        actual: CompTypeId,
    ) -> Self
    {
        Self {
            arena,
            expected,
            actual,
        }
    }

    /// The snapshot arena holding both roots.
    #[inline]
    #[must_use]
    pub const fn arena(&self) -> &TermArena
    {
        &self.arena
    }

    /// The computation type the context required (root into [`Self::arena`]).
    #[inline]
    #[must_use]
    pub const fn expected(&self) -> CompTypeId
    {
        self.expected
    }

    /// The computation type the checker synthesized (root into
    /// [`Self::arena`]).
    #[inline]
    #[must_use]
    pub const fn actual(&self) -> CompTypeId
    {
        self.actual
    }
}

/// The `gandr_kernel_strata` refutation behind a rejected level-order claim —
/// re-checkable against the two levels by the crate that produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the two level-order decision paths (free oracle, landmark entailment) are closed by design"
)]
pub enum LevelOrderRefutation
{
    /// The free-fragment oracle refuted the claim (no landmark constraints in
    /// scope): a counter-valuation, checkable by
    /// `gandr_kernel_strata::validate_refutation`.
    Free(LeqRefutation),
    /// Landmark entailment refuted the claim under the declaration's poset: a
    /// countermodel, checkable by
    /// `gandr_kernel_strata::validate_entailment_countermodel`.
    Landmark(Box<EntailmentCountermodel>),
}

/// A rejected universe judgment `U_lower : U_upper` (equivalently
/// `lower < upper`): the two levels and the refutation evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UniverseViolation
{
    /// The lower level of the failed strict comparison.
    lower: Level,
    /// The upper level of the failed strict comparison.
    upper: Level,
    /// The refutation evidence behind the rejection.
    evidence: LevelOrderRefutation,
}

impl UniverseViolation
{
    /// Record a universe violation.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        lower: Level,
        upper: Level,
        evidence: LevelOrderRefutation,
    ) -> Self
    {
        Self {
            lower,
            upper,
            evidence,
        }
    }

    /// The lower level of the failed strict comparison.
    #[inline]
    #[must_use]
    pub const fn lower(&self) -> &Level
    {
        &self.lower
    }

    /// The upper level of the failed strict comparison.
    #[inline]
    #[must_use]
    pub const fn upper(&self) -> &Level
    {
        &self.upper
    }

    /// The refutation evidence behind the rejection.
    #[inline]
    #[must_use]
    pub const fn evidence(&self) -> &LevelOrderRefutation
    {
        &self.evidence
    }
}

/// Why a declaration failed to enter the kernel. The vocabulary is closed and
/// total: `Environment::add_decl` either returns a `CheckedId` or exactly one
/// of these — the checker never panics on a term.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the kernel's S1 failure vocabulary is closed by design (kernel-boundary.md K1)"
)]
pub enum KernelError
{
    /// A value variable's de Bruijn index escaped the typing context.
    UnboundVariable
    {
        /// The out-of-range index.
        index: crate::term::DeBruijnIndex,
    },
    /// A constant reference named no prior admitted declaration (out of range
    /// or a forward reference into the append-only environment).
    UnboundConstant
    {
        /// The unresolved constant index.
        index: crate::term::ConstantIndex,
    },
    /// A checking-only term form appeared where a type had to be synthesized.
    NotInferable
    {
        /// The offending form.
        form: NonInferableForm,
    },
    /// A value of the wrong type shape met an eliminator or checking rule.
    ValueShapeMismatch
    {
        /// The value-type shape the rule required.
        expected: ExpectedValueShape,
        /// A snapshot of the value type actually present.
        actual: Box<ValueTypeSnapshot>,
    },
    /// A computation of the wrong type shape met an eliminator or checking
    /// rule.
    ComputationShapeMismatch
    {
        /// The computation-type shape the rule required.
        expected: ExpectedComputationShape,
        /// A snapshot of the computation type actually present.
        actual: Box<CompTypeSnapshot>,
    },
    /// A synthesized value type failed to convert against the expected type.
    ValueTypeMismatch(Box<ValueTypeMismatch>),
    /// A synthesized computation type failed to convert against expected.
    ComputationTypeMismatch(Box<ComputationTypeMismatch>),
    /// The two branches of a synthesized `case` produced inconvertible types.
    CaseBranchMismatch(Box<ComputationTypeMismatch>),
    /// A level variable's index exceeded the declaration's prenex parameter
    /// count.
    LevelVariableOutOfScope
    {
        /// The out-of-scope variable.
        variable: LevelVar,
    },
    /// Level arithmetic stepped past the representable range (a successor at
    /// the numeric ceiling); saturating would silently identify distinct
    /// levels, so the overflow surfaces.
    LevelArithmetic,
    /// A universe or lift judgment `lower < upper` failed, with re-checkable
    /// refutation evidence.
    UniverseViolation(Box<UniverseViolation>),
    /// The declaration's landmark constraints have no model — an inconsistent
    /// level context that would make `U_l : U_l` derivable. The replayable
    /// pumping witness is carried.
    InconsistentLevelConstraints(Box<LoopWitness>),
    /// A landmark-entailment query hit a fault the theory excludes (an admitted
    /// poset does not loop) — surfaced rather than trusted, per the strata
    /// contract.
    LevelOracleFault(PosetError),
    /// The checker machine's produced register held the wrong polarity for its
    /// consuming frame. Unreachable when the goal↔frame correspondence (the
    /// `crate::check` module table) is wired correctly — surfaced rather than
    /// trusted, so a machine wiring defect rejects the declaration
    /// (fail-closed) instead of fabricating a type, which could wrongly accept
    /// an ill-typed declaration (fail-open).
    CheckerRegisterFault(RegisterFault),
    /// An arena id resolved to no node — the K1 weakening surfaced. Unreachable
    /// when minting and lookup are wired correctly (ids are minted only by
    /// arena constructors over allocated children, [`crate::arena`]); surfaced
    /// rather than trusted, so a resolution defect rejects the declaration
    /// (fail-closed) instead of proceeding on a fabricated node.
    ArenaFault,
}

impl From<LevelError> for KernelError
{
    #[inline]
    fn from(error: LevelError) -> Self
    {
        match error {
            | LevelError::Overflow => Self::LevelArithmetic,
        }
    }
}

impl fmt::Display for KernelError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::UnboundVariable { index } => {
                write!(
                    f,
                    "value variable {} is unbound in the typing context",
                    u32::from(index)
                )
            },
            | Self::UnboundConstant { index } => {
                write!(
                    f,
                    "constant reference {} names no prior declaration",
                    usize::from(index)
                )
            },
            | Self::NotInferable { form } => match form {
                | NonInferableForm::Injection => f.write_str(
                    "a sum injection has no synthesizable type; it needs an expected sum type",
                ),
                | NonInferableForm::Lambda => f.write_str(
                    "a lambda has no synthesizable type; it needs an expected function type",
                ),
            },
            | Self::ValueShapeMismatch { expected, .. } => match expected {
                | ExpectedValueShape::Product => f.write_str("expected a product type"),
                | ExpectedValueShape::Sum => f.write_str("expected a sum type"),
                | ExpectedValueShape::Thunk => f.write_str("expected a thunk type"),
            },
            | Self::ComputationShapeMismatch { expected, .. } => match expected {
                | ExpectedComputationShape::Arrow => f.write_str("expected a function type"),
                | ExpectedComputationShape::Returner => f.write_str("expected a returner type"),
            },
            | Self::ValueTypeMismatch(_) => {
                f.write_str("a synthesized value type does not match the expected value type")
            },
            | Self::ComputationTypeMismatch(_) => f.write_str(
                "a synthesized computation type does not match the expected computation type",
            ),
            | Self::CaseBranchMismatch(_) => {
                f.write_str("the branches of a case have inconvertible computation types")
            },
            | Self::LevelVariableOutOfScope { variable } => {
                write!(
                    f,
                    "level variable {variable} exceeds the declaration's prenex parameter count"
                )
            },
            | Self::LevelArithmetic => {
                f.write_str("level arithmetic stepped past the representable range")
            },
            | Self::UniverseViolation(_) => f.write_str(
                "a universe judgment failed: the lower level is not strictly below the upper",
            ),
            | Self::InconsistentLevelConstraints(_) => {
                f.write_str("the declaration's landmark level constraints have no model")
            },
            | Self::LevelOracleFault(_) => {
                f.write_str("the landmark-entailment oracle reported a fault the theory excludes")
            },
            | Self::CheckerRegisterFault(fault) => match fault {
                | RegisterFault::ExpectedValueType => f.write_str(
                    "checker-machine register fault: a value-consuming frame found a non-value register",
                ),
                | RegisterFault::ExpectedCompType => f.write_str(
                    "checker-machine register fault: a computation-consuming frame found a non-computation register",
                ),
            },
            | Self::ArenaFault => {
                f.write_str("an arena id resolved to no node: a resolution fault, surfaced not trusted")
            },
        }
    }
}

impl Error for KernelError
{
}
