//! The elaborator-side **kernel bridge**.
//!
//! A total lowering from the checked core CBPV forms ([`crate::syntax`],
//! [`crate::types`]) into the minimal certified kernel's closed **S1
//! vocabulary** ([`gandr_kernel_core`], the kernel-boundary design record
//! §7).
//!
//! # The dependency direction
//!
//! This crate depends on [`gandr_kernel_core`]; the reverse is **forbidden** by
//! the section-2 TCB wall (the kernel depends only on `gandr-kernel-strata`).
//! The bridge is therefore **untrusted, elaborator-side** code: it produces
//! candidate S1 terms and types, and the kernel's own choke point
//! ([`gandr_kernel_core::Environment::add_decl`]) **re-derives** every typing
//! obligation (K2) — the bridge is granted no credence.
//!
//! # What the bridge does
//!
//! * **Rejects out-of-S1 nodes structurally.** S1 has a closed vocabulary with
//!   no constructor for holes/`Unknown`, effects/handlers, the control
//!   fragment, `Native`, declared data (`Data`/`Ctor`/`DataCase`), the
//!   structural stock (`List`/`Record`/`With`/`Prj`), `Sigma`/`Split`, or the
//!   identity fragment (`Path`/`Here`/`Walk`). Each such node is a precise
//!   [`BridgeRejection`] naming the offending form; the bridge **never
//!   panics**.
//! * **Erases the operationally-transparent forms (C4).** A type ascription
//!   [`Value::Annot`] is peeled (it only guides checking); the grade operations
//!   [`Comp::Dup`]/[`Comp::Drop`] lower to their ungraded operational skeletons
//!   (`dup v ⤳ return (v, v)`, `drop v ⤳ return ()`), the S1 image of grade
//!   erasure (S1 thunks are ungraded).
//! * **Resolves names to de Bruijn indices and cross-declaration constants.** A
//!   [`Value::Var`] bound by an enclosing binder becomes a
//!   [`gandr_kernel_core::DeBruijnIndex`]; one naming a prior admitted
//!   declaration (through the [`BridgeContext`] constant map) becomes a
//!   [`Value::Constant`](gandr_kernel_core) admission index; a genuinely free
//!   name is rejected [`BridgeRejection::UnboundName`].
//! * **Applies the value-polarity declaration convention.** A computation
//!   definition enters as a **thunk**: declared type `U C`, body `thunk (…)`,
//!   used through `force` — [`lower_computation_definition`].
//!
//! # Iterative traversal (no input recursion)
//!
//! Every traversal is an explicit **worklist machine** over a goal register, a
//! produced register, a heap frame stack, and (for terms) an explicit binder
//! context — never host-stack recursion on term depth. This meets the
//! `docs/workflow/rust.md` "input recursion: none" discipline (the kernel's
//! `TermFrame`/`TypeFrame` codec worklists and defunctionalized checker are the
//! precedent), so an adversarially deep core term lowers without overflowing.
//!
//! # Levels
//!
//! Core CBPV carries **no universe levels** (its `Universe` is the un-levelled
//! ADR-81 code universe, rejected here — the kernel's levelled universe and
//! explicit lifts are authored kernel-native). A bridged
//! declaration is therefore level-**monomorphic**
//! ([`gandr_kernel_core::LevelSignature::monomorphic`]); the level machinery is
//! exercised only by the kernel-native goldens.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_kernel_core::BaseType;
use gandr_kernel_core::CompTypeId;
use gandr_kernel_core::ComputationId;
use gandr_kernel_core::ConstantIndex;
use gandr_kernel_core::DeBruijnIndex;
use gandr_kernel_core::IntegerLiteral;
use gandr_kernel_core::Literal;
use gandr_kernel_core::Magnitude;
use gandr_kernel_core::Side as KernelSide;
use gandr_kernel_core::Sign;
use gandr_kernel_core::StringLiteral;
use gandr_kernel_core::TermArena;
use gandr_kernel_core::ValueId;
use gandr_kernel_core::ValueTypeId;
use thiserror::Error;

use crate::syntax::Comp;
use crate::syntax::Side as CoreSide;
use crate::syntax::Value;
use crate::types::CompType;
use crate::types::SealId;
use crate::types::ValueType;

/// Why a core form has no image in the closed S1 vocabulary.
///
/// Each variant names the exact offending form (K1: the S1 vocabulary has no
/// constructor for it, so the bridge rejects at construction rather than
/// panicking). [`Self::exclusion_class`] groups the variants into the coarse
/// classes the corpus partition tags. S1's standing-subset growth
/// will retire rejections as formers are admitted;
/// matches are total by policy, so a retired variant is a compile-visible
/// change at every match site.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum BridgeRejection
{
    /// A value variable named no enclosing binder and no admitted declaration.
    #[error(
        "the free name `{0}` is bound by no enclosing binder and names no admitted declaration"
    )]
    UnboundName(String),
    /// A value-position typed hole `?u` (K1: no metavariable exists at S1).
    #[error("a value hole has no S1 image (S1 is a closed, hole-free vocabulary)")]
    ValueHole,
    /// A computation-position typed hole `?u`.
    #[error("a computation hole has no S1 image (S1 is a closed, hole-free vocabulary)")]
    ComputationHole,
    /// A sealed abstract type whose atom the [`BridgeContext`] does not bind.
    ///
    /// The atom is perfectly representable at S1 — the rejection is about the
    /// *binding*, not the former. Only the pass that flattens a sealed module
    /// into kernel declarations knows which admission position an atom landed
    /// at, so lowering one without that binding would mean inventing a
    /// position: an abstraction leak the kernel would then certify.
    #[error(
        "the sealed atom `{}.{}` is bound to no admitted abstract-type declaration",
        .0.declaration(),
        .0.component()
    )]
    UnboundSeal(SealId),
    /// The unknown value type `?` (the Hazelnut hole type).
    #[error("the unknown value type `?` has no S1 image")]
    UnknownValueType,
    /// The unknown computation type `?`.
    #[error("the unknown computation type `?` has no S1 image")]
    UnknownComputationType,
    /// A `perform` of an effect operation (S1's `F` is pure).
    #[error("effect `perform` has no S1 image (S1's returner is pure)")]
    Perform,
    /// An effect `handle`.
    #[error("effect `handle` has no S1 image (S1's returner is pure)")]
    Handle,
    /// A control-fragment `resume`.
    #[error("control `resume` has no S1 image (S1 has no control fragment)")]
    Resume,
    /// A control-fragment `reset`.
    #[error("control `reset` has no S1 image (S1 has no control fragment)")]
    Reset,
    /// A control-fragment `shift`.
    #[error("control `shift` has no S1 image (S1 has no control fragment)")]
    Shift,
    /// A reified stack `stk K` in value position.
    #[error("a reified stack value has no S1 image (S1 has no control fragment)")]
    ReifiedStackValue,
    /// A reified-stack type `Stk(B, C)`.
    #[error("a reified-stack type has no S1 image (S1 has no control fragment)")]
    ReifiedStackType,
    /// A returner `F^ε A` with a non-empty effect row.
    #[error("a returner with a non-empty effect row has no S1 image (S1's returner is pure)")]
    NonEmptyEffectRow,
    /// A `native` primitive call (the recursion substrate).
    #[error("a native primitive call has no S1 image (natives are outside S1)")]
    Native,
    /// A declared-data constructor value `Ctor { … }`.
    #[error("a declared-data constructor has no S1 image (datatypes are S2 description codes)")]
    DataConstructor,
    /// A declared-data eliminator `DataCase`.
    #[error("a declared-data eliminator has no S1 image (datatypes are S2 description codes)")]
    DataEliminator,
    /// A declared-data nominal type `Data { … }`.
    #[error("a declared-data type has no S1 image (datatypes are S2 description codes)")]
    DataType,
    /// A list literal `[v₀, …]`.
    #[error("a list value has no S1 image (List is outside the S1 structural stock)")]
    ListValue,
    /// A list type `List A`.
    #[error("a list type has no S1 image (List is outside the S1 structural stock)")]
    ListType,
    /// A list eliminator `listcase`.
    #[error("a list eliminator has no S1 image (List is outside the S1 structural stock)")]
    ListEliminator,
    /// A record literal `{ℓ = v}`.
    #[error("a record value has no S1 image (Record is outside the S1 structural stock)")]
    RecordValue,
    /// A record type `{ℓ : A}`.
    #[error("a record type has no S1 image (Record is outside the S1 structural stock)")]
    RecordType,
    /// A record projection `v.ℓ`.
    #[error("a record projection has no S1 image (Record is outside the S1 structural stock)")]
    RecordProjection,
    /// A lazy-product introduction `with`.
    #[error("a lazy-product `with` has no S1 image (With is outside the S1 structural stock)")]
    WithComputation,
    /// A lazy-product type `B & B′`.
    #[error("a lazy-product type has no S1 image (With is outside the S1 structural stock)")]
    WithType,
    /// A lazy-product projection `prj`.
    #[error("a lazy-product projection has no S1 image (With is outside the S1 structural stock)")]
    Projection,
    /// A dependent-pair type `Σ(x : A). B`.
    #[error("a dependent-pair type has no S1 image (Sigma is an S2 dependent former)")]
    SigmaType,
    /// A dependent-pair eliminator `split`.
    #[error("a dependent-pair eliminator has no S1 image (Split is an S2 dependent eliminator)")]
    SplitEliminator,
    /// An identity type `Path A x y`.
    #[error("an identity type has no S1 image (Path/identity arrives at B7)")]
    PathType,
    /// A reflexivity proof `here(v)`.
    #[error("a reflexivity proof has no S1 image (Path/identity arrives at B7)")]
    HereProof,
    /// An identity eliminator `walk`.
    #[error("an identity eliminator has no S1 image (Path/identity arrives at B7)")]
    WalkEliminator,
    /// The un-levelled code universe `Type`.
    #[error(
        "the un-levelled code universe has no S1 image (S1's levelled universe is kernel-native)"
    )]
    UniverseType,
    /// A typed machine-numeric literal (`u32`/`u64`/`i32`/`i64`/`f32`/`f64`).
    #[error(
        "a machine-numeric literal has no S1 image (S1's base atoms are Integer/String/Numeric)"
    )]
    MachineNumericLiteral,
    /// A base-type atom outside the S1 stock `{ Integer, String, Numeric }`
    /// (a type variable or a machine-numeric atom).
    #[error("the base atom `{0}` is outside the S1 stock {{ Integer, String, Numeric }}")]
    UnsupportedBaseAtom(String),
}

/// Stable corpus-partition class for one bridge rejection.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BridgeExclusionClass(&'static str);

impl AsRef<str> for BridgeExclusionClass
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

impl BridgeRejection
{
    /// The coarse exclusion class this rejection belongs to — the rationale tag
    /// the corpus partition manifest records per ineligible item.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: a stable, kebab-case class tag; every variant maps to exactly
    ///   one class, and the classes partition the rejection vocabulary.
    /// - provides: the manifest's per-item rationale tag across the crate
    ///   boundary (so the corpus harness need not match on the enum's
    ///   variants).
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn exclusion_class(&self) -> BridgeExclusionClass
    {
        match *self {
            | Self::UnboundName(_) => BridgeExclusionClass("open-free-name"),
            | Self::ValueHole
            | Self::ComputationHole
            | Self::UnknownValueType
            | Self::UnknownComputationType => BridgeExclusionClass("hole-unknown"),
            | Self::Perform
            | Self::Handle
            | Self::Resume
            | Self::Reset
            | Self::Shift
            | Self::ReifiedStackValue
            | Self::ReifiedStackType
            | Self::NonEmptyEffectRow => BridgeExclusionClass("effects-control"),
            | Self::Native => BridgeExclusionClass("native"),
            | Self::DataConstructor | Self::DataEliminator | Self::DataType => {
                BridgeExclusionClass("declared-data")
            },
            | Self::ListValue
            | Self::ListType
            | Self::ListEliminator
            | Self::RecordValue
            | Self::RecordType
            | Self::RecordProjection
            | Self::WithComputation
            | Self::WithType
            | Self::Projection => BridgeExclusionClass("structural-stock"),
            | Self::SigmaType | Self::SplitEliminator => BridgeExclusionClass("sigma-split"),
            | Self::PathType | Self::HereProof | Self::WalkEliminator => {
                BridgeExclusionClass("identity")
            },
            | Self::UniverseType => BridgeExclusionClass("universe"),
            | Self::UnboundSeal(_) => BridgeExclusionClass("unbound-seal"),
            | Self::MachineNumericLiteral | Self::UnsupportedBaseAtom(_) => {
                BridgeExclusionClass("machine-numeric")
            },
        }
    }
}

/// Borrowed source name at the checker-to-kernel bridge.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct BridgeName<'name>(&'name str);

/// Borrowed binder stack used for de Bruijn resolution.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct LocalScope<'scope, 'name>(&'scope [&'name str]);

/// Core integer carried across the checker-to-kernel bridge.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct BridgeInteger(i64);

/// The elaborator's naming environment for the bridge: the map from a free core
/// name to the admission index of the prior kernel declaration it resolves to.
///
/// The corpus items are closed programs, so their `BridgeContext` is empty and
/// every free name is [`BridgeRejection::UnboundName`]; a multi-declaration
/// elaboration populates it so a later declaration references an earlier one
/// through a [`Value::Constant`](gandr_kernel_core) admission index (the
/// append-only environment's cross-declaration reference form).
#[derive(Clone, Debug, Default)]
pub struct BridgeContext
{
    /// Free name → the admission index of the declaration it resolves to.
    constants: BTreeMap<String, ConstantIndex>,
    /// Sealed atom → the admission index of its abstract-type declaration.
    ///
    /// The two halves of a seal's identity are **not** the same object, and the
    /// map is where they meet. Elaborator-side an atom is a
    /// [`SealId`] — a minting serial and its site, chosen while
    /// elaborating; kernel-side it is an *admission position*, chosen by the
    /// order the flattening export admits declarations in. Only the pass that
    /// performs that export knows both, so it supplies the binding here rather
    /// than the bridge guessing one.
    ///
    /// A sealed type whose atom is unbound is [`BridgeRejection::UnboundSeal`],
    /// never a fabricated position: an atom lowered to the wrong declaration
    /// would be an *abstraction leak the kernel would then certify*, which is
    /// the one failure a bridge rejection is cheap enough to prevent outright.
    seals: BTreeMap<SealId, ConstantIndex>,
}

impl BridgeContext
{
    /// An empty naming environment (the closed-program case).
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Bind a free name to a prior declaration's admission index (builder
    /// style).
    #[inline]
    #[must_use]
    pub fn with_constant<N>(
        mut self,
        name: N,
        index: ConstantIndex,
    ) -> Self
    where
        N: Into<String>,
    {
        let _prior = self.constants.insert(name.into(), index);
        self
    }

    /// Bind a sealed atom to the admission index of its abstract-type
    /// declaration (builder style).
    ///
    /// The flattening export calls this once per atom it admits, so the bridge
    /// resolves a sealed type by lookup rather than by construction.
    #[inline]
    #[must_use]
    pub fn with_seal(
        mut self,
        seal: SealId,
        index: ConstantIndex,
    ) -> Self
    {
        let _prior = self.seals.insert(seal, index);
        self
    }

    /// The admission index a sealed atom resolves to, if it is bound.
    #[inline]
    #[must_use]
    pub fn seal(
        &self,
        seal: &SealId,
    ) -> Option<ConstantIndex>
    {
        self.seals.get(seal).copied()
    }

    /// The admission index a free name resolves to, if any.
    #[inline]
    #[must_use]
    fn constant(
        &self,
        name: BridgeName<'_>,
    ) -> Option<ConstantIndex>
    {
        self.constants.get(name.0).copied()
    }
}

// ----- Type lowering (iterative, no binders) -----

/// The goal of the iterative type-lowering machine.
enum TypeGoal<'core>
{
    /// Lower a value type.
    Value(&'core ValueType),
    /// Lower a computation type.
    Comp(&'core CompType),
}

/// The produced register of the type-lowering machine.
#[derive(Clone, Copy)]
enum TypeOut
{
    /// A lowered value-type id.
    Value(ValueTypeId),
    /// A lowered computation-type id.
    Comp(CompTypeId),
}

/// A continuation of the type-lowering machine.
enum TypeFrame<'core>
{
    /// The product's first operand is lowered; lower the second.
    ProductSecond(&'core ValueType),
    /// Both product operands are lowered; build the product.
    ProductBuild(ValueTypeId),
    /// The sum's first operand is lowered; lower the second.
    SumSecond(&'core ValueType),
    /// Both sum operands are lowered; build the sum.
    SumBuild(ValueTypeId),
    /// The thunk's computation type is lowering; build the (ungraded) thunk.
    Thunk,
    /// The returner's result value type is lowering; build the returner.
    Returner,
    /// The arrow's domain is lowered; lower the codomain.
    ArrowCodomain(&'core CompType),
    /// Both arrow operands are lowered; build the arrow.
    ArrowBuild(ValueTypeId),
}

/// Project a value-type id out of the produced register (a mis-polarity is
/// unreachable by construction and falls back to `Unit` rather than panicking).
#[inline]
fn produced_value_type(
    arena: &mut TermArena,
    out: TypeOut,
) -> ValueTypeId
{
    match out {
        | TypeOut::Value(id) => id,
        | TypeOut::Comp(_) => arena.value_type_unit(),
    }
}

/// Project a computation-type id (fallback `F Unit`).
#[inline]
fn produced_comp_type(
    arena: &mut TermArena,
    out: TypeOut,
) -> CompTypeId
{
    match out {
        | TypeOut::Comp(id) => id,
        | TypeOut::Value(_) => {
            let unit = arena.value_type_unit();
            arena.comp_type_returner(unit)
        },
    }
}

/// Lower a core value type into the S1 value-type vocabulary, minting into
/// `arena`.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(id)` — an S1 value-type root minted into `arena` — exactly
///   when every node of the core type is in the S1 value-type stock (base atom
///   over `{ Integer, String }`, unit, product, sum, ungraded thunk); the walk
///   is iterative over an explicit heap frame stack, total on any depth.
/// - provides: the declared-type lowering of [`lower_value_definition`] and the
///   thunk-body's computation type of [`lower_computation_definition`].
/// - fails: [`BridgeRejection`] naming the first out-of-S1 former met.
/// - panics: none.
///
/// # Errors
/// A [`BridgeRejection`] for a `List`/`Record`/`Stk`/`Path`/`Data`/`Universe`/
/// `Sigma`/`Unknown` former or an unsupported base atom.
///
/// # Adequacy
/// - hypothesis: L2/L3 — the accepted stock is pinned by the corpus round-trip
///   and unit goldens; each rejection arm is a pinned L3 residue witnessed by a
///   hand-built core type.
/// - witness: `tests::product_type_lowers`
/// - witness: `tests::record_type_is_rejected`
/// - witness: `tests::list_type_is_rejected`
#[inline]
pub fn lower_value_type(
    context: &BridgeContext,
    arena: &mut TermArena,
    value_type: &ValueType,
) -> Result<ValueTypeId, BridgeRejection>
{
    let out = lower_type(context, arena, TypeGoal::Value(value_type))?;
    Ok(produced_value_type(arena, out))
}

/// Lower a core computation type into the S1 computation-type vocabulary.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(id)` — an S1 computation-type root minted into `arena` —
///   exactly when the core type is a pure returner `F A` (empty effect row) or
///   a function `A → C` over lowerable operands; iterative and total on any
///   depth.
/// - provides: the returner/arrow lowering the definition builders wrap.
/// - fails: [`BridgeRejection::NonEmptyEffectRow`] for an effectful returner,
///   [`BridgeRejection::WithType`] for a lazy product,
///   [`BridgeRejection::UnknownComputationType`] for `?`, or an inner
///   value-type rejection.
/// - panics: none.
///
/// # Errors
/// As `- fails:`.
///
/// # Adequacy
/// - hypothesis: L2/L3 — the arrow/returner acceptance is pinned by the corpus
///   round-trip; the `With`/effect-row/`Unknown` residues are pinned by
///   hand-built core types.
/// - witness: `tests::returner_type_lowers`
/// - witness: `tests::effectful_returner_is_rejected`
#[inline]
pub fn lower_comp_type(
    context: &BridgeContext,
    arena: &mut TermArena,
    comp_type: &CompType,
) -> Result<CompTypeId, BridgeRejection>
{
    let out = lower_type(context, arena, TypeGoal::Comp(comp_type))?;
    Ok(produced_comp_type(arena, out))
}

/// The shared iterative engine for type lowering.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(out)` — the lowered root as a polarity-tagged id — over an
///   explicit goal/produced/frame worklist, so no host-stack recursion on type
///   depth occurs (the "input recursion: none" discipline).
/// - provides: the engine [`lower_value_type`]/[`lower_comp_type`] wrap.
/// - fails: the first [`BridgeRejection`] a former surfaces.
/// - panics: none.
fn lower_type<'core>(
    context: &BridgeContext,
    arena: &mut TermArena,
    root: TypeGoal<'core>,
) -> Result<TypeOut, BridgeRejection>
{
    let mut frames: Vec<TypeFrame<'core>> = Vec::new();
    let mut goal = root;
    'expand: loop {
        let mut produced: TypeOut = match goal {
            | TypeGoal::Value(spec) => match *spec {
                | ValueType::Atom(ref name) => {
                    let atom = base_atom(BridgeName(name.as_str()))?;
                    TypeOut::Value(arena.value_type_base(atom))
                },
                | ValueType::Unit => TypeOut::Value(arena.value_type_unit()),
                | ValueType::Prod(ref first, ref second) => {
                    frames.push(TypeFrame::ProductSecond(second.as_ref()));
                    goal = TypeGoal::Value(first.as_ref());
                    continue 'expand;
                },
                | ValueType::Sum(ref first, ref second) => {
                    frames.push(TypeFrame::SumSecond(second.as_ref()));
                    goal = TypeGoal::Value(first.as_ref());
                    continue 'expand;
                },
                | ValueType::Thunk(_, ref body) => {
                    frames.push(TypeFrame::Thunk);
                    goal = TypeGoal::Comp(body.as_ref());
                    continue 'expand;
                },
                | ValueType::List(_) => return Err(BridgeRejection::ListType),
                | ValueType::Record(_) => return Err(BridgeRejection::RecordType),
                | ValueType::Stk(..) => return Err(BridgeRejection::ReifiedStackType),
                | ValueType::Path { .. } => return Err(BridgeRejection::PathType),
                | ValueType::Data { .. } => return Err(BridgeRejection::DataType),
                | ValueType::Universe => return Err(BridgeRejection::UniverseType),
                | ValueType::Sealed(ref seal) => match context.seal(seal) {
                    | Some(index) => TypeOut::Value(arena.value_type_abstract(index)),
                    | None => {
                        return Err(BridgeRejection::UnboundSeal(seal.clone()));
                    },
                },
                | ValueType::Sigma { .. } => return Err(BridgeRejection::SigmaType),
                | ValueType::Unknown => return Err(BridgeRejection::UnknownValueType),
            },
            | TypeGoal::Comp(spec) => match *spec {
                | CompType::F(ref result, ref row) => {
                    if !bool::from(row.is_empty()) {
                        return Err(BridgeRejection::NonEmptyEffectRow);
                    }
                    frames.push(TypeFrame::Returner);
                    goal = TypeGoal::Value(result.as_ref());
                    continue 'expand;
                },
                | CompType::Arrow(ref domain, ref codomain) => {
                    frames.push(TypeFrame::ArrowCodomain(codomain.as_ref()));
                    goal = TypeGoal::Value(domain.as_ref());
                    continue 'expand;
                },
                | CompType::With(..) => return Err(BridgeRejection::WithType),
                | CompType::Unknown => return Err(BridgeRejection::UnknownComputationType),
            },
        };
        loop {
            let Some(frame) = frames.pop()
            else {
                return Ok(produced);
            };
            match frame {
                | TypeFrame::ProductSecond(second) => {
                    let first = produced_value_type(arena, produced);
                    frames.push(TypeFrame::ProductBuild(first));
                    goal = TypeGoal::Value(second);
                    continue 'expand;
                },
                | TypeFrame::ProductBuild(first) => {
                    let second = produced_value_type(arena, produced);
                    produced = TypeOut::Value(arena.value_type_product(first, second));
                },
                | TypeFrame::SumSecond(second) => {
                    let first = produced_value_type(arena, produced);
                    frames.push(TypeFrame::SumBuild(first));
                    goal = TypeGoal::Value(second);
                    continue 'expand;
                },
                | TypeFrame::SumBuild(first) => {
                    let second = produced_value_type(arena, produced);
                    produced = TypeOut::Value(arena.value_type_sum(first, second));
                },
                | TypeFrame::Thunk => {
                    let body = produced_comp_type(arena, produced);
                    produced = TypeOut::Value(arena.value_type_thunk(body));
                },
                | TypeFrame::Returner => {
                    let result = produced_value_type(arena, produced);
                    produced = TypeOut::Comp(arena.comp_type_returner(result));
                },
                | TypeFrame::ArrowCodomain(codomain) => {
                    let domain = produced_value_type(arena, produced);
                    frames.push(TypeFrame::ArrowBuild(domain));
                    goal = TypeGoal::Comp(codomain);
                    continue 'expand;
                },
                | TypeFrame::ArrowBuild(domain) => {
                    let codomain = produced_comp_type(arena, produced);
                    produced = TypeOut::Comp(arena.comp_type_arrow(domain, codomain));
                },
            }
        }
    }
}

/// The S1 base atom a core type-atom name denotes, or a rejection.
///
/// # Contract
/// - requires: nothing.
/// - ensures: [`BaseType::Integer`] for `"Integer"`, [`BaseType::String`] for
///   `"String"`, [`BaseType::Numeric`] for `"Numeric"`.
/// - provides: the base-atom mapping of [`lower_value_type`]. Core's typed
///   machine numerics (`u32`…`f64`) are outside S1's three atoms and reject.
/// - fails: [`BridgeRejection::UnsupportedBaseAtom`] for any other atom.
/// - panics: none.
#[inline]
fn base_atom(name: BridgeName<'_>) -> Result<BaseType, BridgeRejection>
{
    match name.0 {
        | "Integer" => Ok(BaseType::Integer),
        | "String" => Ok(BaseType::String),
        | "Numeric" => Ok(BaseType::Numeric),
        | _ => Err(BridgeRejection::UnsupportedBaseAtom(String::from(name.0))),
    }
}

// ----- Term lowering (iterative, de Bruijn binder context) -----

/// The goal of the iterative term-lowering machine.
enum TermGoal<'core>
{
    /// Lower a value.
    Value(&'core Value),
    /// Lower a computation.
    Comp(&'core Comp),
}

/// The produced register of the term-lowering machine.
#[derive(Clone, Copy)]
enum TermOut
{
    /// A lowered value id.
    Value(ValueId),
    /// A lowered computation id.
    Comp(ComputationId),
}

/// A continuation of the term-lowering machine (each holds `Copy` ids or
/// borrowed core sub-terms).
enum TermFrame<'core>
{
    /// The pair's first component is lowered; lower the second.
    PairSecond(&'core Value),
    /// Both pair components are lowered; build the pair.
    PairBuild(ValueId),
    /// The injection body is lowered; build the injection on the held side.
    Injection(KernelSide),
    /// The thunk's computation body is lowering; build the value thunk.
    Thunk,
    /// The lambda body is lowered; build the lambda.
    Lambda,
    /// A binder scope closes: pop the innermost context name.
    ScopeExit,
    /// The application head is lowered; lower the argument.
    ApplicationArgument(&'core Value),
    /// Head and argument are lowered; build the application.
    ApplicationBuild(ComputationId),
    /// The returner's value is lowered; build the returner.
    Return,
    /// The forced value is lowered; build the force.
    Force,
    /// The bound computation is lowered; push the binder and lower the body.
    BindBody(&'core str, &'core Comp),
    /// The bind body is lowered; build the bind over the held bound id.
    BindBuild(ComputationId),
    /// The scrutinee is lowered; push the left binder and lower the left arm.
    CaseAfterScrutinee
    {
        /// The left arm `(binder, body)`.
        on_left: (&'core str, &'core Comp),
        /// The right arm `(binder, body)`.
        on_right: (&'core str, &'core Comp),
    },
    /// The left arm is lowered; push the right binder and lower the right arm.
    CaseAfterLeft
    {
        /// The lowered scrutinee id.
        scrutinee: ValueId,
        /// The right arm `(binder, body)`.
        on_right: (&'core str, &'core Comp),
    },
    /// Both arms are lowered; build the case.
    CaseAfterRight
    {
        /// The lowered scrutinee id.
        scrutinee: ValueId,
        /// The lowered left-arm id.
        on_left: ComputationId,
    },
    /// The duplicated value is lowered; build `return (v, v)` (grade erasure).
    DupBuild,
    /// The dropped value is lowered (and discarded); build `return ()`.
    DropBuild,
}

/// Project a value id out of the produced register (a mis-polarity is
/// unreachable by construction and falls back to `Unit` rather than panicking).
#[inline]
fn produced_value(
    arena: &mut TermArena,
    out: TermOut,
) -> ValueId
{
    match out {
        | TermOut::Value(id) => id,
        | TermOut::Comp(_) => arena.value_unit(),
    }
}

/// Project a computation id (fallback `return ()`).
#[inline]
fn produced_computation(
    arena: &mut TermArena,
    out: TermOut,
) -> ComputationId
{
    match out {
        | TermOut::Comp(id) => id,
        | TermOut::Value(_) => {
            let unit = arena.value_unit();
            arena.computation_return(unit)
        },
    }
}

/// Lower a core value into the S1 value vocabulary, minting into `arena`.
///
/// # Contract
/// - requires: `context` names the declarations a free variable may resolve to.
/// - ensures: `Ok(id)` — an S1 value root minted into `arena` — exactly when
///   every node is in the S1 value stock (variable, unit, integer/string
///   literal, pair, injection, thunk) after erasing `Annot`, with every name
///   resolved to a de Bruijn index or a constant admission index; the walk is
///   iterative over an explicit heap frame stack and binder context, total on
///   any depth.
/// - provides: the value-body lowering of [`lower_value_definition`].
/// - fails: [`BridgeRejection`] naming the first out-of-S1 form or free name.
/// - panics: none.
///
/// # Errors
/// A [`BridgeRejection`] for a hole, list, record, stack, `here`, constructor,
/// or machine-numeric literal, or [`BridgeRejection::UnboundName`].
///
/// # Adequacy
/// - hypothesis: L2/L3 — the accepted stock is pinned by the corpus round-trip;
///   each rejection and the de-Bruijn/constant resolution are pinned residues.
/// - witness: `tests::pair_of_literals_lowers`
/// - witness: `tests::annotation_is_erased`
/// - witness: `tests::a_free_name_is_unbound`
/// - witness: `tests::a_bound_variable_resolves_to_a_de_bruijn_index`
#[inline]
pub fn lower_value(
    context: &BridgeContext,
    arena: &mut TermArena,
    value: &Value,
) -> Result<ValueId, BridgeRejection>
{
    let out = lower_term(context, arena, TermGoal::Value(value))?;
    Ok(produced_value(arena, out))
}

/// Lower a core computation into the S1 computation vocabulary.
///
/// # Contract
/// - requires: `context` names the declarations a free variable may resolve to.
/// - ensures: `Ok(id)` — an S1 computation root minted into `arena` — exactly
///   when every node is in the S1 computation stock (lambda, application,
///   return, bind, force, sum case) after erasing the grade operations
///   `dup`/`drop`, with every name resolved; iterative and total on any depth.
/// - provides: the computation-body lowering of
///   [`lower_computation_definition`].
/// - fails: [`BridgeRejection`] naming the first out-of-S1 form or free name.
/// - panics: none.
///
/// # Errors
/// A [`BridgeRejection`] for an effect/control/native/data/list/record/split/
/// walk form, a hole, or [`BridgeRejection::UnboundName`].
///
/// # Adequacy
/// - hypothesis: L2/L3 — the accepted stock is pinned by the corpus round-trip;
///   each rejection and the grade-op erasure are pinned residues.
/// - witness: `tests::returning_a_literal_lowers`
/// - witness: `tests::perform_is_rejected`
/// - witness: `tests::drop_erases_to_return_unit`
#[inline]
pub fn lower_comp(
    context: &BridgeContext,
    arena: &mut TermArena,
    comp: &Comp,
) -> Result<ComputationId, BridgeRejection>
{
    let out = lower_term(context, arena, TermGoal::Comp(comp))?;
    Ok(produced_computation(arena, out))
}

/// The de Bruijn index a binder name resolves to in the current context, or the
/// constant/free-name resolution.
///
/// # Contract
/// - requires: `locals` is the binder stack (innermost last).
/// - ensures: the innermost matching binder yields
///   `Ok(Value::Variable(index))`; a `context` constant yields
///   `Ok(Value::Constant(index))`; neither yields
///   `Err(BridgeRejection::UnboundName)`.
/// - provides: the name-resolution of the term machine's `Var` leaf.
/// - fails: [`BridgeRejection::UnboundName`] for a genuinely free name.
/// - panics: none.
#[inline]
fn resolve_name(
    context: &BridgeContext,
    arena: &mut TermArena,
    locals: LocalScope<'_, '_>,
    name: BridgeName<'_>,
) -> Result<ValueId, BridgeRejection>
{
    if let Some(position) = locals.0.iter().rposition(|&bound| bound == name.0) {
        // Innermost binder is the last slot; its de Bruijn index is 0.
        let steps = locals.0.len().saturating_sub(1).saturating_sub(position);
        let index = u32::try_from(steps).unwrap_or(u32::MAX);
        return Ok(arena.value_variable(DeBruijnIndex::from(index)));
    }
    if let Some(index) = context.constant(name) {
        return Ok(arena.value_constant(index));
    }
    Err(BridgeRejection::UnboundName(String::from(name.0)))
}

/// The shared iterative engine for term lowering.
///
/// # Contract
/// - requires: `context` names the resolvable declarations.
/// - ensures: `Ok(out)` — the lowered root as a polarity-tagged id — over an
///   explicit goal/produced/frame worklist and an explicit binder context (a
///   `ScopeExit` frame per binder), so no host-stack recursion on term depth
///   occurs; `Annot` is peeled and `dup`/`drop` erased in the descent.
/// - provides: the engine [`lower_value`]/[`lower_comp`] wrap.
/// - fails: the first [`BridgeRejection`] a form or a free name surfaces.
/// - panics: none.
fn lower_term<'core>(
    context: &BridgeContext,
    arena: &mut TermArena,
    root: TermGoal<'core>,
) -> Result<TermOut, BridgeRejection>
{
    let mut frames: Vec<TermFrame<'core>> = Vec::new();
    let mut locals: Vec<&'core str> = Vec::new();
    let mut goal = root;
    'expand: loop {
        let mut produced: TermOut = match goal {
            | TermGoal::Value(spec) => match *spec {
                | Value::Var(ref name) => {
                    let id = resolve_name(
                        context,
                        arena,
                        LocalScope(&locals),
                        BridgeName(name.as_str()),
                    )?;
                    TermOut::Value(id)
                },
                | Value::Unit => TermOut::Value(arena.value_unit()),
                | Value::Int(literal) => {
                    TermOut::Value(arena.value_literal(integer_literal(BridgeInteger(literal))))
                },
                | Value::Str(ref text) => TermOut::Value(
                    arena.value_literal(Literal::Text(StringLiteral::new(text.clone()))),
                ),
                | Value::Pair(ref first, ref second) => {
                    frames.push(TermFrame::PairSecond(second.as_ref()));
                    goal = TermGoal::Value(first.as_ref());
                    continue 'expand;
                },
                | Value::Inj(side, ref body) => {
                    frames.push(TermFrame::Injection(kernel_side(side)));
                    goal = TermGoal::Value(body.as_ref());
                    continue 'expand;
                },
                | Value::Thunk(_, ref body) => {
                    frames.push(TermFrame::Thunk);
                    goal = TermGoal::Comp(body.as_ref());
                    continue 'expand;
                },
                | Value::Annot(ref inner, _) => {
                    goal = TermGoal::Value(inner.as_ref());
                    continue 'expand;
                },
                | Value::Num(_) => return Err(BridgeRejection::MachineNumericLiteral),
                | Value::List(_) => return Err(BridgeRejection::ListValue),
                | Value::Record(_) => return Err(BridgeRejection::RecordValue),
                | Value::Hole(_) => return Err(BridgeRejection::ValueHole),
                | Value::Stk(_) => return Err(BridgeRejection::ReifiedStackValue),
                | Value::Here(_) => return Err(BridgeRejection::HereProof),
                | Value::Ctor { .. } => return Err(BridgeRejection::DataConstructor),
            },
            | TermGoal::Comp(spec) => match *spec {
                | Comp::Abs(ref binder, _, ref body) => {
                    locals.push(binder.as_str());
                    frames.push(TermFrame::ScopeExit);
                    frames.push(TermFrame::Lambda);
                    goal = TermGoal::Comp(body.as_ref());
                    continue 'expand;
                },
                | Comp::App(ref head, ref argument) => {
                    frames.push(TermFrame::ApplicationArgument(argument.as_ref()));
                    goal = TermGoal::Comp(head.as_ref());
                    continue 'expand;
                },
                | Comp::Ret(ref value) => {
                    frames.push(TermFrame::Return);
                    goal = TermGoal::Value(value.as_ref());
                    continue 'expand;
                },
                | Comp::Bind(ref bound, ref binder, ref body) => {
                    frames.push(TermFrame::BindBody(binder.as_str(), body.as_ref()));
                    goal = TermGoal::Comp(bound.as_ref());
                    continue 'expand;
                },
                | Comp::Force(ref value) => {
                    frames.push(TermFrame::Force);
                    goal = TermGoal::Value(value.as_ref());
                    continue 'expand;
                },
                | Comp::Case(ref scrutinee, ref on_left, ref on_right) => {
                    frames.push(TermFrame::CaseAfterScrutinee {
                        on_left: (on_left.0.as_str(), on_left.1.as_ref()),
                        on_right: (on_right.0.as_str(), on_right.1.as_ref()),
                    });
                    goal = TermGoal::Value(scrutinee.as_ref());
                    continue 'expand;
                },
                | Comp::Dup(ref value) => {
                    frames.push(TermFrame::DupBuild);
                    goal = TermGoal::Value(value.as_ref());
                    continue 'expand;
                },
                | Comp::Drop(ref value) => {
                    frames.push(TermFrame::DropBuild);
                    goal = TermGoal::Value(value.as_ref());
                    continue 'expand;
                },
                | Comp::ListCase { .. } => return Err(BridgeRejection::ListEliminator),
                | Comp::Split { .. } => return Err(BridgeRejection::SplitEliminator),
                | Comp::DataCase(..) => return Err(BridgeRejection::DataEliminator),
                | Comp::RecordProj { .. } => return Err(BridgeRejection::RecordProjection),
                | Comp::With(..) => return Err(BridgeRejection::WithComputation),
                | Comp::Prj(..) => return Err(BridgeRejection::Projection),
                | Comp::Perform(..) => return Err(BridgeRejection::Perform),
                | Comp::Handle { .. } => return Err(BridgeRejection::Handle),
                | Comp::Resume(..) => return Err(BridgeRejection::Resume),
                | Comp::Reset(_) => return Err(BridgeRejection::Reset),
                | Comp::Shift(..) => return Err(BridgeRejection::Shift),
                | Comp::Hole(_) => return Err(BridgeRejection::ComputationHole),
                | Comp::Native { .. } => return Err(BridgeRejection::Native),
                | Comp::Walk { .. } => return Err(BridgeRejection::WalkEliminator),
            },
        };
        loop {
            let Some(frame) = frames.pop()
            else {
                return Ok(produced);
            };
            match frame {
                | TermFrame::PairSecond(second) => {
                    let first = produced_value(arena, produced);
                    frames.push(TermFrame::PairBuild(first));
                    goal = TermGoal::Value(second);
                    continue 'expand;
                },
                | TermFrame::PairBuild(first) => {
                    let second = produced_value(arena, produced);
                    produced = TermOut::Value(arena.value_pair(first, second));
                },
                | TermFrame::Injection(side) => {
                    let body = produced_value(arena, produced);
                    produced = TermOut::Value(arena.value_injection(side, body));
                },
                | TermFrame::Thunk => {
                    let body = produced_computation(arena, produced);
                    produced = TermOut::Value(arena.value_thunk(body));
                },
                | TermFrame::Lambda => {
                    let body = produced_computation(arena, produced);
                    produced = TermOut::Comp(arena.computation_lambda(body));
                },
                | TermFrame::ScopeExit => {
                    let _popped = locals.pop();
                },
                | TermFrame::ApplicationArgument(argument) => {
                    let head = produced_computation(arena, produced);
                    frames.push(TermFrame::ApplicationBuild(head));
                    goal = TermGoal::Value(argument);
                    continue 'expand;
                },
                | TermFrame::ApplicationBuild(head) => {
                    let argument = produced_value(arena, produced);
                    produced = TermOut::Comp(arena.computation_application(head, argument));
                },
                | TermFrame::Return => {
                    let value = produced_value(arena, produced);
                    produced = TermOut::Comp(arena.computation_return(value));
                },
                | TermFrame::Force => {
                    let value = produced_value(arena, produced);
                    produced = TermOut::Comp(arena.computation_force(value));
                },
                | TermFrame::BindBody(binder, body) => {
                    let bound = produced_computation(arena, produced);
                    locals.push(binder);
                    frames.push(TermFrame::BindBuild(bound));
                    frames.push(TermFrame::ScopeExit);
                    goal = TermGoal::Comp(body);
                    continue 'expand;
                },
                | TermFrame::BindBuild(bound) => {
                    let body = produced_computation(arena, produced);
                    produced = TermOut::Comp(arena.computation_bind(bound, body));
                },
                | TermFrame::CaseAfterScrutinee { on_left, on_right } => {
                    let scrutinee = produced_value(arena, produced);
                    locals.push(on_left.0);
                    frames.push(TermFrame::CaseAfterLeft {
                        scrutinee,
                        on_right,
                    });
                    frames.push(TermFrame::ScopeExit);
                    goal = TermGoal::Comp(on_left.1);
                    continue 'expand;
                },
                | TermFrame::CaseAfterLeft {
                    scrutinee,
                    on_right,
                } => {
                    let on_left = produced_computation(arena, produced);
                    locals.push(on_right.0);
                    frames.push(TermFrame::CaseAfterRight { scrutinee, on_left });
                    frames.push(TermFrame::ScopeExit);
                    goal = TermGoal::Comp(on_right.1);
                    continue 'expand;
                },
                | TermFrame::CaseAfterRight { scrutinee, on_left } => {
                    let on_right = produced_computation(arena, produced);
                    produced = TermOut::Comp(arena.computation_case(scrutinee, on_left, on_right));
                },
                | TermFrame::DupBuild => {
                    let value = produced_value(arena, produced);
                    let pair = arena.value_pair(value, value);
                    produced = TermOut::Comp(arena.computation_return(pair));
                },
                | TermFrame::DropBuild => {
                    // The dropped value is lowered above (so an out-of-S1 node
                    // inside it is still rejected) and then discarded: `drop`
                    // erases to `return ()` (grade erasure, C4).
                    let _discarded = produced_value(arena, produced);
                    let unit = arena.value_unit();
                    produced = TermOut::Comp(arena.computation_return(unit));
                },
            }
        }
    }
}

/// The kernel injection side for a core injection/projection side.
#[inline]
const fn kernel_side(side: CoreSide) -> KernelSide
{
    match side {
        | CoreSide::Fst => KernelSide::Left,
        | CoreSide::Snd => KernelSide::Right,
    }
}

/// The canonical S1 integer literal for a core `i64` literal.
///
/// # Contract
/// - requires: nothing.
/// - ensures: the sign is `Negative` for a strictly-negative `n` (else
///   `NonNegative`, canonical zero pinned by [`IntegerLiteral::new`]) and the
///   magnitude is the canonical decimal of `n.unsigned_abs()`.
/// - provides: the `Value::Int` lowering of the term machine.
/// - fails: never.
/// - panics: none.
#[inline]
fn integer_literal(integer: BridgeInteger) -> Literal
{
    let sign = if integer.0 < 0 {
        Sign::Negative
    }
    else {
        Sign::NonNegative
    };
    let mut remaining = integer.0.unsigned_abs();
    let decimal = if remaining == 0 {
        String::from("0")
    }
    else {
        let mut buffer: Vec<u8> = Vec::new();
        while remaining > 0 {
            let digit = u8::try_from(remaining % 10).unwrap_or(0);
            buffer.push(b'0'.saturating_add(digit));
            remaining /= 10;
        }
        buffer.reverse();
        let mut digits = String::new();
        for byte in buffer {
            digits.push(char::from(byte));
        }
        digits
    };
    let magnitude = Magnitude::from_decimal_text(decimal).unwrap_or_else(Magnitude::zero);
    Literal::Integer(IntegerLiteral::new(sign, magnitude))
}

// ----- Value-polarity declaration lowering (the declaration convention) -----

/// Lower a core **value** definition (declared type + value body) into the S1
/// declared-type and body roots of a `Def`.
///
/// # Contract
/// - requires: `declared` is the value type the elaborator assigned `body`;
///   `context` names the resolvable declarations; both are minted into the same
///   `arena` (the environment's staging arena).
/// - ensures: `Ok((declared_id, body_id))` — the S1 declared value-type root
///   and the S1 value body root — exactly when both lower; the caller finalizes
///   a `Def` over them and re-checks through the choke point.
/// - provides: the value-item lowering of the corpus partition.
/// - fails: the first [`BridgeRejection`] from either lowering.
/// - panics: none.
///
/// # Errors
/// Any [`BridgeRejection`] from lowering the type or the body.
///
/// # Adequacy
/// - hypothesis: L2 — the corpus round-trip pins that a lowered value
///   definition re-admits and round-trips byte-identically.
/// - witness: `tests::a_lowered_value_definition_admits`
#[inline]
pub fn lower_value_definition(
    context: &BridgeContext,
    arena: &mut TermArena,
    value: &Value,
    declared: &ValueType,
) -> Result<(ValueTypeId, ValueId), BridgeRejection>
{
    let declared_id = lower_value_type(context, arena, declared)?;
    let body_id = lower_value(context, arena, value)?;
    Ok((declared_id, body_id))
}

/// Lower a core **computation** definition into the S1 `Def` of a **thunk**
/// (the value-polarity declaration convention).
///
/// A computation definition `f : C` enters the single-polarity kernel as the
/// value declaration `f : U C` with body `thunk (…)`, used through `force`.
///
/// # Contract
/// - requires: `declared` is the computation type the elaborator assigned
///   `comp`; both roots are minted into `arena`.
/// - ensures: `Ok((declared_id, body_id))` where `declared_id` is `U C` (a
///   value thunk type over the lowered `C`) and `body_id` is `thunk c` (a value
///   thunk over the lowered computation) — exactly when both lower; the caller
///   finalizes a `Def` over them.
/// - provides: the computation-item lowering of the corpus partition, realizing
///   the value-polarity declaration convention in the bridge.
/// - fails: the first [`BridgeRejection`] from either lowering.
/// - panics: none.
///
/// # Errors
/// Any [`BridgeRejection`] from lowering the type or the computation.
///
/// # Adequacy
/// - hypothesis: L2 — the corpus round-trip pins that a lowered computation
///   definition enters as a thunk and round-trips byte-identically.
/// - witness: `tests::a_lowered_computation_definition_admits`
#[inline]
pub fn lower_computation_definition(
    context: &BridgeContext,
    arena: &mut TermArena,
    comp: &Comp,
    declared: &CompType,
) -> Result<(ValueTypeId, ValueId), BridgeRejection>
{
    let comp_type = lower_comp_type(context, arena, declared)?;
    let declared_id = arena.value_type_thunk(comp_type);
    let computation = lower_comp(context, arena, comp)?;
    let body_id = arena.value_thunk(computation);
    Ok((declared_id, body_id))
}

#[cfg(test)]
mod tests;
