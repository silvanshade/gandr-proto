//! The zero-inference S1 checker (K2): a bidirectional
//! type checker that **re-derives everything** and grants the elaborator no
//! credence.
//!
//! # Bidirectional, annotation-free
//!
//! S1 terms carry no type annotations (there is no `Annot` — it is erased at
//! the bridge), so checking is bidirectional: introductions
//! (lambda, injection, pair, thunk, return) **check** against an expected type
//! flowing down from the declaration's declared type; eliminators and atoms
//! (variable, constant, application, force, bind, case, literal, lift)
//! **synthesize** a type flowing up. A synthesizing term used where a type is
//! expected triggers a **conversion** ([`crate::conv`]) at the mode switch.
//! This needs no metavariables and no inference — exactly K2.
//!
//! # Arena-native, ids not owned types (D1(C))
//!
//! Every type the machine touches — the declared type, a context slot, a
//! synthesized type, an expected type flowing into a check — is a **`Copy`
//! [`ValueTypeId`]/[`CompTypeId`]** into the environment [`TermArena`]. The
//! borrowed/owned `Held` split and its clone-on-lookup dissolve: an expected
//! type is one id (no clone to propagate into both `case` branches), a context
//! slot is one id, and eliminating a type former is a **read of its child ids**
//! (`arena.comp_type(id)` matched to `Arrow { domain, codomain }`) — no
//! placeholder-swap, no E0509 interlock. Synthesized types are **minted into
//! the arena** as checker intermediates; the choke point truncates them after
//! the verdict (the admission watermark discipline, [`crate::arena`]).
//!
//! # No term-into-type substitution
//!
//! At S1 no type former embeds a value term (`Product` is non-dependent, the
//! arrow's codomain cannot mention its argument), so application, bind, and
//! case never substitute a term into a type, and a type stored in the context
//! is closed — no de Bruijn shifting of types is ever required. The universe
//! hierarchy is the only "dependent" content, and it lives entirely at the
//! level algebra ([`crate::levels`]).
//!
//! # Totality via defunctionalization (no depth budget)
//!
//! The checker runs as an explicit **defunctionalized machine** ([`run`]) over
//! a goal register, a produced register, a heap frame stack, and an explicit
//! typing-context stack of [`ValueTypeId`] slots — never mutually recursive
//! methods bounded by a depth budget — so it is **total on adversarial depth**
//! (export decode can build an arbitrarily deep term from bytes). This is the
//! standard adversarial-depth machine [`crate::conv`] and [`crate::export`]
//! already use; it meets the docs/workflow/rust.md "input recursion: none"
//! discipline. A node is read out of the arena by a shallow clone
//! (its children are `Copy` ids, so the clone is `O(1)`), releasing the arena
//! borrow before synthesis mints into it. [`type_level`] (value-/computation-
//! type formation) is its own iterative walk the machine calls directly, as it
//! calls the already-iterative [`crate::conv`].
//!
//! ## Correspondence — old recursive arm ↔ machine step
//!
//! Each arm of the former recursive presentation maps to exactly one goal
//! expansion plus (for a multi-child arm) one continuation frame. The reviewer
//! walks this table (a TCB audit artifact):
//!
//! | recursive arm                    | goal push(es)                         | frame(s)                              |
//! | -------------------------------- | ------------------------------------- | ------------------------------------- |
//! | `value_type_level` Base/Unit     | leaf → `Level::zero()`                | —                                     |
//! | `value_type_level` Universe      | leaf → scope, `succ`                  | —                                     |
//! | `value_type_level` Product/Sum   | `Value(first)`                        | `MaxSecondValue`, `MaxWith`           |
//! | `value_type_level` Thunk         | `Comp(body)` (level passes through)   | —                                     |
//! | `value_type_level` Lift          | `Value(inner)`                        | `LiftCheck`                           |
//! | `comp_type_level` Returner       | `Value(result)` (passes through)      | —                                     |
//! | `comp_type_level` Arrow          | `Value(domain)`                       | `MaxSecondComp`, `MaxWith`            |
//! | `synth_value` Var / Const / Unit / Lit | leaf → lookup / resolve               | —                                     |
//! | `synth_value` Pair               | `SynthValue(first)`                   | `SynthPairFirst`, `SynthPairSecond`   |
//! | `synth_value` Thunk              | `SynthComp(body)`                     | `SynthThunk`                          |
//! | `synth_value` Lift              | `SynthValue(body)`                    | `SynthLift`                           |
//! | `synth_value` Injection          | leaf → `NotInferable`                 | —                                     |
//! | `check_value` Injection          | `CheckValue(body, summand)`           | —                                     |
//! | `check_value` Pair               | `CheckValue(first, ft)`               | `CheckPairSecond`                     |
//! | `check_value` Thunk              | `CheckComp(body, cod)`                | —                                     |
//! | `check_value` synth-fallthrough  | `SynthValue(value)`                   | `ConvertValue`                        |
//! | `synth_comp` Application         | `SynthComp(head)`                     | `SynthApply`, `ProduceComp`           |
//! | `synth_comp` Force               | `SynthValue(value)`                   | `SynthForce`                          |
//! | `synth_comp` Return             | `SynthValue(value)`                   | `SynthReturn`                         |
//! | `synth_comp` Bind               | `SynthComp(bound)`                    | `SynthBind`, `ScopeExit`              |
//! | `synth_comp` Case               | `SynthValue(scrutinee)`               | `SynthCaseScrutinee / AfterLeft / AfterRight`, `ScopeExit` |
//! | `synth_comp` Lambda             | leaf → `NotInferable`                 | —                                     |
//! | `check_comp` Lambda             | `CheckComp(body, cod)`                | `ScopeExit`                           |
//! | `check_comp` Return             | `CheckValue(value, result)`           | —                                     |
//! | `check_comp` Bind               | `SynthComp(bound)`                    | `CheckBind`, `ScopeExit`              |
//! | `check_comp` Case               | `SynthValue(scrutinee)`               | `CheckCaseScrutinee/AfterLeft`, `ScopeExit` |
//! | `check_comp` synth-fallthrough  | `SynthComp(computation)`              | `ConvertComp`                         |

use alloc::vec::Vec;

use gandr_kernel_strata::Level;

use crate::arena::CompTypeId;
use crate::arena::ComputationId;
use crate::arena::TermArena;
use crate::arena::ValueId;
use crate::arena::ValueTypeId;
use crate::conv::Convertibility;
use crate::conv::convert_comp_type;
use crate::conv::convert_value_type;
use crate::conv::convertible_comp_types;
use crate::decl::Declaration;
use crate::decl::DeclarationContent;
use crate::env::AdmittedDeclaration;
use crate::error::CompTypeSnapshot;
use crate::error::ComputationTypeMismatch;
use crate::error::ExpectedComputationShape;
use crate::error::ExpectedValueShape;
use crate::error::KernelError;
use crate::error::NonInferableForm;
use crate::error::RegisterFault;
use crate::error::ValueTypeSnapshot;
use crate::levels::LevelContext;
use crate::term::Computation;
use crate::term::DeBruijnIndex;
use crate::term::Side;
use crate::term::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// Resolve a value id to a shallow-cloned node, or the fail-closed arena fault.
///
/// # Contract
/// - requires: `id` was minted into `arena`.
/// - ensures: `Ok(value)` — a shallow (`O(1)`) clone whose children are `Copy`
///   ids, releasing the arena borrow so synthesis may mint into it.
/// - provides: the machine's total node read.
/// - fails: [`KernelError::ArenaFault`] on a dangling id (unreachable under the
///   minting invariant; surfaced not trusted).
/// - panics: none.
#[inline]
fn read_value(
    arena: &TermArena,
    id: ValueId,
) -> Result<Value, KernelError>
{
    arena.value(id).cloned().ok_or(KernelError::ArenaFault)
}

/// Resolve a computation id to a shallow-cloned node, or the arena fault.
#[inline]
fn read_computation(
    arena: &TermArena,
    id: ComputationId,
) -> Result<Computation, KernelError>
{
    arena
        .computation(id)
        .cloned()
        .ok_or(KernelError::ArenaFault)
}

/// A value-type shape mismatch: snapshot the offending type and name the shape.
#[inline]
fn value_shape_mismatch(
    arena: &TermArena,
    expected: ExpectedValueShape,
    actual: ValueTypeId,
) -> KernelError
{
    let (snapshot, root) = arena.snapshot_value_type(actual);
    KernelError::ValueShapeMismatch {
        expected,
        actual: alloc::boxed::Box::new(ValueTypeSnapshot::new(snapshot, root)),
    }
}

/// A computation-type shape mismatch: snapshot the offending type.
#[inline]
fn comp_shape_mismatch(
    arena: &TermArena,
    expected: ExpectedComputationShape,
    actual: CompTypeId,
) -> KernelError
{
    let (snapshot, root) = arena.snapshot_comp_type(actual);
    KernelError::ComputationShapeMismatch {
        expected,
        actual: alloc::boxed::Box::new(CompTypeSnapshot::new(snapshot, root)),
    }
}

/// A pending type-formation obligation over a type-node id — the goal of the
/// iterative [`type_level`] walk.
#[derive(Clone, Copy)]
enum TypeLevelGoal
{
    /// The level of a value type.
    Value(ValueTypeId),
    /// The level of a computation type.
    Comp(CompTypeId),
}

/// A continuation of the iterative type-formation walk.
enum TypeLevelFrame
{
    /// The first (value-type) operand's level is in the register; compute the
    /// second value-type operand's level next (a product or sum).
    MaxSecondValue(ValueTypeId),
    /// The first (value-type) operand's level is in the register; compute the
    /// second (computation-type) operand's level next (an arrow).
    MaxSecondComp(CompTypeId),
    /// The second operand's level is in the register; join with the first.
    MaxWith(Level),
    /// The inner level is in the register; check the lift's strictness and
    /// replace it with the target level.
    LiftCheck(Level),
}

/// The universe level of a well-formed value or computation type — the K1
/// well-formedness gate, computed iteratively so it is total on any depth.
///
/// # Contract
/// - requires: the root id resolves in `arena`.
/// - ensures: `Ok(level)` — the minimal universe level of the type — exactly
///   when every embedded level is in scope, every lift strictly raises, and no
///   successor overflows; a bare `Universe(l)` forms at `l + 1`. The walk is
///   iterative over an explicit heap frame stack, so it is total on any depth.
/// - provides: value-/computation-type formation for [`check_value_type`] and
///   the machine's `Lift` synthesis.
/// - fails: [`KernelError::LevelVariableOutOfScope`],
///   [`KernelError::LevelArithmetic`], [`KernelError::UniverseViolation`],
///   [`KernelError::LevelOracleFault`], [`KernelError::ArenaFault`].
/// - panics: none.
///
/// # Errors
/// As `- fails:`.
///
/// # Adequacy
/// - hypothesis: L3 — the composite level is pinned by `max`/`succ` goldens;
///   the L3 residues are the universe successor (`U_l` at `l+1`), the arrow
///   join, and the lift-strictness boundary, pinned by unit goldens and the
///   strata `Level::lt` refutation.
/// - witness: `check::tests::universe_forms_one_level_up`
/// - witness: `check::tests::lift_requires_a_strictly_higher_target`
/// - witness: `check::tests::arrow_level_is_the_join`
fn type_level(
    arena: &TermArena,
    levels: &LevelContext,
    root: TypeLevelGoal,
) -> Result<Level, KernelError>
{
    let mut frames: Vec<TypeLevelFrame> = Vec::new();
    let mut goal = root;
    'expand: loop {
        let mut produced: Level = match goal {
            | TypeLevelGoal::Value(id) => {
                let value_type = arena.value_type(id).ok_or(KernelError::ArenaFault)?;
                match *value_type {
                    | ValueType::Base(_) | ValueType::Unit => Level::zero(),
                    | ValueType::Universe(ref level) => {
                        levels.check_level_scope(level)?;
                        level.succ()?
                    },
                    | ValueType::Product(first, second) | ValueType::Sum(first, second) => {
                        frames.push(TypeLevelFrame::MaxSecondValue(second));
                        goal = TypeLevelGoal::Value(first);
                        continue 'expand;
                    },
                    | ValueType::Thunk(body) => {
                        goal = TypeLevelGoal::Comp(body);
                        continue 'expand;
                    },
                    | ValueType::Lift { inner, ref target } => {
                        frames.push(TypeLevelFrame::LiftCheck(target.clone()));
                        goal = TypeLevelGoal::Value(inner);
                        continue 'expand;
                    },
                }
            },
            | TypeLevelGoal::Comp(id) => {
                let comp_type = arena.comp_type(id).ok_or(KernelError::ArenaFault)?;
                match *comp_type {
                    | CompType::Returner(result) => {
                        goal = TypeLevelGoal::Value(result);
                        continue 'expand;
                    },
                    | CompType::Arrow { domain, codomain } => {
                        frames.push(TypeLevelFrame::MaxSecondComp(codomain));
                        goal = TypeLevelGoal::Value(domain);
                        continue 'expand;
                    },
                }
            },
        };
        loop {
            let Some(frame) = frames.pop()
            else {
                return Ok(produced);
            };
            match frame {
                | TypeLevelFrame::MaxSecondValue(second) => {
                    frames.push(TypeLevelFrame::MaxWith(produced));
                    goal = TypeLevelGoal::Value(second);
                    continue 'expand;
                },
                | TypeLevelFrame::MaxSecondComp(second) => {
                    frames.push(TypeLevelFrame::MaxWith(produced));
                    goal = TypeLevelGoal::Comp(second);
                    continue 'expand;
                },
                | TypeLevelFrame::MaxWith(first) => {
                    produced = first.max(&produced);
                },
                | TypeLevelFrame::LiftCheck(target) => {
                    levels.check_level_scope(&target)?;
                    levels.check_universe_below(&produced, &target)?;
                    produced = target;
                },
            }
        }
    }
}

/// A result produced by the checker machine into its register.
#[derive(Clone, Copy)]
enum Produced
{
    /// A synthesized value type (root id into the arena).
    ValueType(ValueTypeId),
    /// A synthesized computation type (root id into the arena).
    CompType(CompTypeId),
    /// A check succeeded (no type is carried).
    Checked,
}

/// Project a synthesized value-type id out of the produced register.
///
/// # Contract
/// - requires: the register was set by a `SynthValue` goal — its variant
///   matches by construction (the module correspondence table is the wiring
///   audit).
/// - ensures: `Ok(id)` — the produced value-type id.
/// - provides: the fail-closed register projection the value-consuming frames
///   need: a polarity mismatch is unreachable when the machine is wired
///   correctly, and is surfaced rather than trusted so a wiring defect rejects
///   the declaration instead of fabricating a type that could wrongly convert.
/// - fails: [`KernelError::CheckerRegisterFault`] on a non-value register.
/// - panics: none.
///
/// # Errors
/// [`KernelError::CheckerRegisterFault`].
///
/// # Adequacy
/// - hypothesis: L3 — the fault arm is unreachable through the public surface
///   by construction; it is pinned directly at the projection.
/// - witness: `check::tests::register_polarity_faults_fail_closed`
#[inline]
fn produced_value_type(produced: Produced) -> Result<ValueTypeId, KernelError>
{
    match produced {
        | Produced::ValueType(id) => Ok(id),
        | Produced::CompType(_) | Produced::Checked => Err(KernelError::CheckerRegisterFault(
            RegisterFault::ExpectedValueType,
        )),
    }
}

/// Project a synthesized computation-type id out of the produced register.
///
/// # Contract
/// - requires: the register was set by a `SynthComp` goal (its variant matches
///   by construction, as in [`produced_value_type`]).
/// - ensures: `Ok(id)` — the produced computation-type id.
/// - provides: the fail-closed register projection the computation-consuming
///   frames need (see [`produced_value_type`] for the posture).
/// - fails: [`KernelError::CheckerRegisterFault`] on a non-computation
///   register.
/// - panics: none.
///
/// # Errors
/// [`KernelError::CheckerRegisterFault`].
///
/// # Adequacy
/// - hypothesis: L3 — as [`produced_value_type`].
/// - witness: `check::tests::register_polarity_faults_fail_closed`
#[inline]
fn produced_comp_type(produced: Produced) -> Result<CompTypeId, KernelError>
{
    match produced {
        | Produced::CompType(id) => Ok(id),
        | Produced::ValueType(_) | Produced::Checked => Err(KernelError::CheckerRegisterFault(
            RegisterFault::ExpectedCompType,
        )),
    }
}

/// A checking obligation over a term-node id — the goal of the machine.
#[derive(Clone, Copy)]
enum Goal
{
    /// Synthesize a value's type.
    SynthValue(ValueId),
    /// Check a value against an expected value type.
    CheckValue(ValueId, ValueTypeId),
    /// Synthesize a computation's type.
    SynthComp(ComputationId),
    /// Check a computation against an expected computation type.
    CheckComp(ComputationId, CompTypeId),
}

/// A continuation of the checker machine (every held type is a `Copy` id).
enum Frame
{
    /// A pair synth: the first component is synthesizing; the second source is
    /// held.
    SynthPairFirst(ValueId),
    /// A pair synth: the first component is synthesized; the second is
    /// synthesizing.
    SynthPairSecond(ValueTypeId),
    /// A thunk synth: the body computation is synthesizing.
    SynthThunk,
    /// A lift synth: the body is synthesizing; the target level is held.
    SynthLift(Level),
    /// A pair check: the first component checked; check the second against the
    /// held component type.
    CheckPairSecond(ValueId, ValueTypeId),
    /// A mode-switch value conversion: the value synthesized; convert against
    /// the held expected type.
    ConvertValue(ValueTypeId),
    /// An application: the head is synthesized; the argument source is held.
    SynthApply(ValueId),
    /// An application: the argument checked; produce the held codomain.
    ProduceComp(CompTypeId),
    /// A force: the value is synthesized.
    SynthForce,
    /// A returner synth: the value is synthesized.
    SynthReturn,
    /// A bind synth: the bound computation is synthesized; the body source is
    /// held. The body's synthesized type becomes the bind's type.
    SynthBind(ComputationId),
    /// A case synth: the scrutinee is synthesized; both branch sources are
    /// held.
    SynthCaseScrutinee(ComputationId, ComputationId),
    /// A case synth: the left branch synthesized; its type and the right
    /// summand are held while the right branch synthesizes.
    SynthCaseAfterLeft
    {
        /// The right branch source.
        on_right: ComputationId,
        /// The right summand type (a context slot for the right branch).
        right: ValueTypeId,
    },
    /// A case synth: both branches synthesized; the left type is held for the
    /// convergence check.
    SynthCaseAfterRight
    {
        /// The left branch's synthesized type.
        left_type: CompTypeId,
    },
    /// A mode-switch computation conversion: the computation synthesized;
    /// convert against the held expected type.
    ConvertComp(CompTypeId),
    /// A bind check: the bound computation synthesized; the body source and the
    /// expected type are held.
    CheckBind(ComputationId, CompTypeId),
    /// A case check: the scrutinee synthesized; the branch sources and expected
    /// type are held.
    CheckCaseScrutinee
    {
        /// The left branch source.
        on_left: ComputationId,
        /// The right branch source.
        on_right: ComputationId,
        /// The expected type both branches check against.
        expected: CompTypeId,
    },
    /// A case check: the left branch checked; the right source, right summand,
    /// and expected type are held.
    CheckCaseAfterLeft
    {
        /// The right branch source.
        on_right: ComputationId,
        /// The right summand type (a context slot for the right branch).
        right: ValueTypeId,
        /// The expected type the right branch checks against.
        expected: CompTypeId,
    },
    /// A binder scope closes: pop the innermost context slot.
    ScopeExit,
}

/// The value-type id bound at de Bruijn `index` in the explicit context, if in
/// scope. The context head (last slot) is index `0`.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Some(id)` when `index` names a binding, walking `index` slots
///   inward from the top; `None` when it escapes the context. Iterative and
///   closed over the finite context.
/// - provides: the variable rule's lookup.
/// - fails: `None` on an out-of-scope index.
/// - panics: none.
#[inline]
fn lookup(
    context: &[ValueTypeId],
    index: DeBruijnIndex,
) -> Option<ValueTypeId>
{
    let steps = usize::try_from(u32::from(index)).ok()?;
    let last = context.len().checked_sub(1)?;
    let position = last.checked_sub(steps)?;
    context.get(position).copied()
}

/// Resolve a constant reference to the declared value-type id of a prior
/// admitted declaration.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Some(id)` when `index` names an admitted declaration.
/// - provides: the constant rule's resolver (a not-yet-admitted index is
///   `None`, which the machine turns into `UnboundConstant`).
/// - fails: `None` for an out-of-range index.
/// - panics: none.
#[inline]
fn resolve_constant(
    entries: &[AdmittedDeclaration],
    index: crate::term::ConstantIndex,
) -> Option<ValueTypeId>
{
    entries
        .get(usize::from(index))
        .map(AdmittedDeclaration::declared_id)
}

/// Run the defunctionalized checker machine from an initial goal and context to
/// a verdict, minting synthesized types into `arena`.
///
/// # Contract
/// - requires: `context` is the initial typing context (empty for a declaration
///   body); `initial`'s ids resolve in `arena`; `entries` is the environment's
///   admission log (for constant resolution).
/// - ensures: `Ok(produced)` — a synthesized type id for a synth goal, or
///   [`Produced::Checked`] for a check goal — exactly when the term checks or
///   synthesizes under the S1 rules; the walk is iterative over a heap frame
///   stack and an explicit context stack, so it is total on any term depth (no
///   input-scaled recursion). Synthesized types are minted into `arena` as
///   intermediates the caller truncates after the verdict.
/// - provides: the shared engine of every S1 checking and synthesis judgment.
/// - fails: any [`KernelError`] a rule surfaces (shape mismatch, non-inferable
///   form, unbound reference, conversion failure, case-branch divergence, a
///   level fault, or the fail-closed arena/register faults).
/// - panics: none.
///
/// # Errors
/// Any [`KernelError`].
///
/// # Adequacy
/// - hypothesis: L2/L3 — the golden corpus checks well-typed declarations and
///   rejects ill-typed mutations, and the totality property confirms a verdict
///   on any generated body; the L3 residues are each rule's failure arm, pinned
///   by negative unit goldens; the defunctionalized walk's totality on
///   adversarial depth is pinned by a small-stack deep-chain witness.
/// - witness: `check::tests::variable_synthesizes_its_context_type`
/// - witness: `check::tests::injection_is_not_inferable`
/// - witness: `check::tests::injection_checks_against_its_sum`
/// - witness: `check::tests::pair_propagates_into_a_checking_component`
/// - witness: `check::tests::application_result_is_the_codomain`
/// - witness: `check::tests::force_unwraps_a_thunk`
/// - witness: `check::tests::case_requires_convergent_branches`
/// - witness: `check::tests::case_propagates_the_expected_type`
/// - witness: `hardening::tests::a_deep_pair_definition_checks_totally`
/// - witness: `hardening::tests::a_deep_bind_definition_checks_totally`
/// - witness: `checker::tests` (the integration corpus)
fn run(
    arena: &mut TermArena,
    entries: &[AdmittedDeclaration],
    levels: &LevelContext,
    mut context: Vec<ValueTypeId>,
    initial: Goal,
) -> Result<Produced, KernelError>
{
    let mut frames: Vec<Frame> = Vec::new();
    let mut goal = initial;
    'expand: loop {
        let mut produced: Produced = match goal {
            | Goal::SynthValue(id) => match read_value(arena, id)? {
                | Value::Variable(index) => {
                    let synthesized =
                        lookup(&context, index).ok_or(KernelError::UnboundVariable { index })?;
                    Produced::ValueType(synthesized)
                },
                | Value::Constant(index) => {
                    let synthesized = resolve_constant(entries, index)
                        .ok_or(KernelError::UnboundConstant { index })?;
                    Produced::ValueType(synthesized)
                },
                | Value::Unit => Produced::ValueType(arena.value_type_unit()),
                | Value::Literal(ref literal) => {
                    Produced::ValueType(arena.value_type_base(literal.base_type()))
                },
                | Value::Pair(first, second) => {
                    frames.push(Frame::SynthPairFirst(second));
                    goal = Goal::SynthValue(first);
                    continue 'expand;
                },
                | Value::Thunk(body) => {
                    frames.push(Frame::SynthThunk);
                    goal = Goal::SynthComp(body);
                    continue 'expand;
                },
                | Value::Lift { target, body } => {
                    frames.push(Frame::SynthLift(target));
                    goal = Goal::SynthValue(body);
                    continue 'expand;
                },
                | Value::Injection(..) => {
                    return Err(KernelError::NotInferable {
                        form: NonInferableForm::Injection,
                    });
                },
            },
            | Goal::CheckValue(id, expected) => match read_value(arena, id)? {
                | Value::Injection(side, body) => match arena.value_type(expected) {
                    | Some(&ValueType::Sum(left, right)) => {
                        let summand = match side {
                            | Side::Left => left,
                            | Side::Right => right,
                        };
                        goal = Goal::CheckValue(body, summand);
                        continue 'expand;
                    },
                    | _ => {
                        return Err(value_shape_mismatch(
                            arena,
                            ExpectedValueShape::Sum,
                            expected,
                        ));
                    },
                },
                | Value::Pair(first, second) => match arena.value_type(expected) {
                    | Some(&ValueType::Product(first_type, second_type)) => {
                        frames.push(Frame::CheckPairSecond(second, second_type));
                        goal = Goal::CheckValue(first, first_type);
                        continue 'expand;
                    },
                    | _ => {
                        return Err(value_shape_mismatch(
                            arena,
                            ExpectedValueShape::Product,
                            expected,
                        ));
                    },
                },
                | Value::Thunk(body) => match arena.value_type(expected) {
                    | Some(&ValueType::Thunk(codomain)) => {
                        goal = Goal::CheckComp(body, codomain);
                        continue 'expand;
                    },
                    | _ => {
                        return Err(value_shape_mismatch(
                            arena,
                            ExpectedValueShape::Thunk,
                            expected,
                        ));
                    },
                },
                | Value::Variable(_)
                | Value::Constant(_)
                | Value::Unit
                | Value::Literal(_)
                | Value::Lift { .. } => {
                    frames.push(Frame::ConvertValue(expected));
                    goal = Goal::SynthValue(id);
                    continue 'expand;
                },
            },
            | Goal::SynthComp(id) => match read_computation(arena, id)? {
                | Computation::Application(head, argument) => {
                    frames.push(Frame::SynthApply(argument));
                    goal = Goal::SynthComp(head);
                    continue 'expand;
                },
                | Computation::Force(value) => {
                    frames.push(Frame::SynthForce);
                    goal = Goal::SynthValue(value);
                    continue 'expand;
                },
                | Computation::Return(value) => {
                    frames.push(Frame::SynthReturn);
                    goal = Goal::SynthValue(value);
                    continue 'expand;
                },
                | Computation::Bind(bound, body) => {
                    frames.push(Frame::SynthBind(body));
                    goal = Goal::SynthComp(bound);
                    continue 'expand;
                },
                | Computation::Case {
                    scrutinee,
                    on_left,
                    on_right,
                } => {
                    frames.push(Frame::SynthCaseScrutinee(on_left, on_right));
                    goal = Goal::SynthValue(scrutinee);
                    continue 'expand;
                },
                | Computation::Lambda(_) => {
                    return Err(KernelError::NotInferable {
                        form: NonInferableForm::Lambda,
                    });
                },
            },
            | Goal::CheckComp(id, expected) => match read_computation(arena, id)? {
                | Computation::Lambda(body) => match arena.comp_type(expected) {
                    | Some(&CompType::Arrow { domain, codomain }) => {
                        context.push(domain);
                        frames.push(Frame::ScopeExit);
                        goal = Goal::CheckComp(body, codomain);
                        continue 'expand;
                    },
                    | _ => {
                        return Err(comp_shape_mismatch(
                            arena,
                            ExpectedComputationShape::Arrow,
                            expected,
                        ));
                    },
                },
                | Computation::Return(value) => match arena.comp_type(expected) {
                    | Some(&CompType::Returner(result)) => {
                        goal = Goal::CheckValue(value, result);
                        continue 'expand;
                    },
                    | _ => {
                        return Err(comp_shape_mismatch(
                            arena,
                            ExpectedComputationShape::Returner,
                            expected,
                        ));
                    },
                },
                | Computation::Bind(bound, body) => {
                    frames.push(Frame::CheckBind(body, expected));
                    goal = Goal::SynthComp(bound);
                    continue 'expand;
                },
                | Computation::Case {
                    scrutinee,
                    on_left,
                    on_right,
                } => {
                    frames.push(Frame::CheckCaseScrutinee {
                        on_left,
                        on_right,
                        expected,
                    });
                    goal = Goal::SynthValue(scrutinee);
                    continue 'expand;
                },
                | Computation::Application(..) | Computation::Force(_) => {
                    frames.push(Frame::ConvertComp(expected));
                    goal = Goal::SynthComp(id);
                    continue 'expand;
                },
            },
        };
        loop {
            let Some(frame) = frames.pop()
            else {
                return Ok(produced);
            };
            match frame {
                | Frame::SynthPairFirst(second) => {
                    let first_type = produced_value_type(produced)?;
                    frames.push(Frame::SynthPairSecond(first_type));
                    goal = Goal::SynthValue(second);
                    continue 'expand;
                },
                | Frame::SynthPairSecond(first_type) => {
                    let second_type = produced_value_type(produced)?;
                    produced =
                        Produced::ValueType(arena.value_type_product(first_type, second_type));
                },
                | Frame::SynthThunk => {
                    let body_type = produced_comp_type(produced)?;
                    produced = Produced::ValueType(arena.value_type_thunk(body_type));
                },
                | Frame::SynthLift(target) => {
                    let body_type = produced_value_type(produced)?;
                    let body_level = type_level(arena, levels, TypeLevelGoal::Value(body_type))?;
                    levels.check_level_scope(&target)?;
                    levels.check_universe_below(&body_level, &target)?;
                    produced = Produced::ValueType(arena.value_type_lift(body_type, target));
                },
                | Frame::CheckPairSecond(second, second_type) => {
                    goal = Goal::CheckValue(second, second_type);
                    continue 'expand;
                },
                | Frame::ConvertValue(expected) => {
                    let synthesized = produced_value_type(produced)?;
                    convert_value_type(arena, expected, synthesized)?;
                    produced = Produced::Checked;
                },
                | Frame::SynthApply(argument) => {
                    let head_type = produced_comp_type(produced)?;
                    match arena.comp_type(head_type) {
                        | Some(&CompType::Arrow { domain, codomain }) => {
                            frames.push(Frame::ProduceComp(codomain));
                            goal = Goal::CheckValue(argument, domain);
                            continue 'expand;
                        },
                        | _ => {
                            return Err(comp_shape_mismatch(
                                arena,
                                ExpectedComputationShape::Arrow,
                                head_type,
                            ));
                        },
                    }
                },
                | Frame::ProduceComp(codomain) => {
                    produced = Produced::CompType(codomain);
                },
                | Frame::SynthForce => {
                    let value_type = produced_value_type(produced)?;
                    match arena.value_type(value_type) {
                        | Some(&ValueType::Thunk(codomain)) => {
                            produced = Produced::CompType(codomain);
                        },
                        | _ => {
                            return Err(value_shape_mismatch(
                                arena,
                                ExpectedValueShape::Thunk,
                                value_type,
                            ));
                        },
                    }
                },
                | Frame::SynthReturn => {
                    let result_type = produced_value_type(produced)?;
                    produced = Produced::CompType(arena.comp_type_returner(result_type));
                },
                | Frame::SynthBind(body) => {
                    let bound_type = produced_comp_type(produced)?;
                    match arena.comp_type(bound_type) {
                        | Some(&CompType::Returner(result)) => {
                            context.push(result);
                            frames.push(Frame::ScopeExit);
                            goal = Goal::SynthComp(body);
                            continue 'expand;
                        },
                        | _ => {
                            return Err(comp_shape_mismatch(
                                arena,
                                ExpectedComputationShape::Returner,
                                bound_type,
                            ));
                        },
                    }
                },
                | Frame::SynthCaseScrutinee(on_left, on_right) => {
                    let scrutinee_type = produced_value_type(produced)?;
                    match arena.value_type(scrutinee_type) {
                        | Some(&ValueType::Sum(left, right)) => {
                            context.push(left);
                            frames.push(Frame::SynthCaseAfterLeft { on_right, right });
                            frames.push(Frame::ScopeExit);
                            goal = Goal::SynthComp(on_left);
                            continue 'expand;
                        },
                        | _ => {
                            return Err(value_shape_mismatch(
                                arena,
                                ExpectedValueShape::Sum,
                                scrutinee_type,
                            ));
                        },
                    }
                },
                | Frame::SynthCaseAfterLeft { on_right, right } => {
                    let left_type = produced_comp_type(produced)?;
                    context.push(right);
                    frames.push(Frame::SynthCaseAfterRight { left_type });
                    frames.push(Frame::ScopeExit);
                    goal = Goal::SynthComp(on_right);
                    continue 'expand;
                },
                | Frame::SynthCaseAfterRight { left_type } => {
                    let right_type = produced_comp_type(produced)?;
                    match convertible_comp_types(arena, left_type, right_type) {
                        | Convertibility::Convertible => {
                            produced = Produced::CompType(left_type);
                        },
                        | Convertibility::Distinct => {
                            let (snapshot, left, right) =
                                arena.snapshot_comp_types(left_type, right_type);
                            return Err(KernelError::CaseBranchMismatch(alloc::boxed::Box::new(
                                ComputationTypeMismatch::new(snapshot, left, right),
                            )));
                        },
                    }
                },
                | Frame::ConvertComp(expected) => {
                    let synthesized = produced_comp_type(produced)?;
                    convert_comp_type(arena, expected, synthesized)?;
                    produced = Produced::Checked;
                },
                | Frame::CheckBind(body, expected) => {
                    let bound_type = produced_comp_type(produced)?;
                    match arena.comp_type(bound_type) {
                        | Some(&CompType::Returner(result)) => {
                            context.push(result);
                            frames.push(Frame::ScopeExit);
                            goal = Goal::CheckComp(body, expected);
                            continue 'expand;
                        },
                        | _ => {
                            return Err(comp_shape_mismatch(
                                arena,
                                ExpectedComputationShape::Returner,
                                bound_type,
                            ));
                        },
                    }
                },
                | Frame::CheckCaseScrutinee {
                    on_left,
                    on_right,
                    expected,
                } => {
                    let scrutinee_type = produced_value_type(produced)?;
                    match arena.value_type(scrutinee_type) {
                        | Some(&ValueType::Sum(left, right)) => {
                            context.push(left);
                            frames.push(Frame::CheckCaseAfterLeft {
                                on_right,
                                right,
                                expected,
                            });
                            frames.push(Frame::ScopeExit);
                            goal = Goal::CheckComp(on_left, expected);
                            continue 'expand;
                        },
                        | _ => {
                            return Err(value_shape_mismatch(
                                arena,
                                ExpectedValueShape::Sum,
                                scrutinee_type,
                            ));
                        },
                    }
                },
                | Frame::CheckCaseAfterLeft {
                    on_right,
                    right,
                    expected,
                } => {
                    context.push(right);
                    frames.push(Frame::ScopeExit);
                    goal = Goal::CheckComp(on_right, expected);
                    continue 'expand;
                },
                | Frame::ScopeExit => {
                    let _popped = context.pop();
                },
            }
        }
    }
}

/// Check that a value type is well-formed (embedded levels in scope, lift
/// strictness, no level overflow), discarding its level.
///
/// # Contract
/// - requires: `declared` resolves in `arena`.
/// - ensures: `Ok(())` exactly when [`type_level`] succeeds on the type.
/// - provides: the K1 well-formedness gate for a declared type.
/// - fails: any [`KernelError`] `type_level` surfaces.
/// - panics: none.
///
/// # Errors
/// [`KernelError::LevelVariableOutOfScope`], [`KernelError::LevelArithmetic`],
/// [`KernelError::UniverseViolation`], [`KernelError::LevelOracleFault`],
/// [`KernelError::ArenaFault`].
#[inline]
fn check_value_type(
    arena: &TermArena,
    levels: &LevelContext,
    declared: ValueTypeId,
) -> Result<(), KernelError>
{
    let _level = type_level(arena, levels, TypeLevelGoal::Value(declared))?;
    Ok(())
}

/// Check a declaration's content: its declared type is well-formed and, for a
/// `Def`, its body checks against that type.
///
/// # Contract
/// - requires: `declaration`'s content roots resolve in `arena`; `entries` is
///   the environment's admission log for constant resolution.
/// - ensures: `Ok(())` exactly when the declared type forms and a `Def` body
///   checks against it in the empty context (an `Axiom` checks only its type);
///   synthesized intermediates are minted into `arena` for the caller to
///   truncate.
/// - provides: the K2 body check the choke point runs.
/// - fails: any [`KernelError`] the checker surfaces.
/// - panics: none.
///
/// # Errors
/// Any [`KernelError`].
///
/// # Adequacy
/// - hypothesis: L2 — the golden corpus checks well-typed declarations and
///   rejects ill-typed mutations; the L3 residues are each rule's failure arm,
///   pinned by the negative unit goldens.
/// - witness: `check::tests::identity_thunk_checks`
/// - witness: `checker::tests` (the integration corpus)
#[inline]
pub fn check_declaration(
    arena: &mut TermArena,
    entries: &[AdmittedDeclaration],
    levels: &LevelContext,
    declaration: &Declaration,
) -> Result<(), KernelError>
{
    check_value_type(arena, levels, declaration.declared_id())?;
    match *declaration.content() {
        | DeclarationContent::Def { declared, body } => {
            let context: Vec<ValueTypeId> = Vec::new();
            let _checked = run(
                arena,
                entries,
                levels,
                context,
                Goal::CheckValue(body, declared),
            )?;
            Ok(())
        },
        | DeclarationContent::Axiom { .. } => Ok(()),
    }
}

#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;

    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;

    use super::Goal;
    use super::Produced;
    use super::TypeLevelGoal;
    use super::run;
    use super::type_level;
    use crate::arena::CompTypeId;
    use crate::arena::ComputationId;
    use crate::arena::TermArena;
    use crate::arena::ValueId;
    use crate::arena::ValueTypeId;
    use crate::env::AdmittedDeclaration;
    use crate::error::KernelError;
    use crate::error::RegisterFault;
    use crate::levels::LevelContext;
    use crate::levels::LevelParamCount;
    use crate::term::DeBruijnIndex;
    use crate::term::Side;
    use crate::types::CompType;
    use crate::types::ValueType;

    /// No prior declarations.
    fn no_entries() -> Vec<AdmittedDeclaration>
    {
        Vec::new()
    }

    /// An empty level context over `params` prenex parameters.
    fn level_context(params: LevelParamCount) -> LevelContext
    {
        LevelContext::admit(params, Vec::new()).unwrap()
    }

    /// Synthesize a value's type from an initial context, returning the id.
    fn synth_value(
        arena: &mut TermArena,
        entries: &[AdmittedDeclaration],
        levels: &LevelContext,
        context: Vec<ValueTypeId>,
        value: ValueId,
    ) -> Result<ValueTypeId, KernelError>
    {
        match run(arena, entries, levels, context, Goal::SynthValue(value))? {
            | Produced::ValueType(id) => Ok(id),
            | _ => panic!("a value goal must produce a value type"),
        }
    }

    /// Synthesize a computation's type, returning the id.
    fn synth_comp(
        arena: &mut TermArena,
        entries: &[AdmittedDeclaration],
        levels: &LevelContext,
        context: Vec<ValueTypeId>,
        computation: ComputationId,
    ) -> Result<CompTypeId, KernelError>
    {
        match run(
            arena,
            entries,
            levels,
            context,
            Goal::SynthComp(computation),
        )? {
            | Produced::CompType(id) => Ok(id),
            | _ => panic!("a computation goal must produce a computation type"),
        }
    }

    #[test]
    fn variable_synthesizes_its_context_type()
    {
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        let entries = no_entries();
        let unit = arena.value_type_unit();
        let variable = arena.value_variable(DeBruijnIndex::from(0_u32));
        let synthesized =
            synth_value(&mut arena, &entries, &levels, alloc::vec![unit], variable).unwrap();
        assert_eq!(
            Some(&ValueType::Unit),
            arena.value_type(synthesized),
            "variable 0 has the head type"
        );
    }

    #[test]
    fn injection_is_not_inferable()
    {
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        let entries = no_entries();
        let unit = arena.value_unit();
        let injection = arena.value_injection(Side::Left, unit);
        assert!(
            matches!(
                synth_value(&mut arena, &entries, &levels, Vec::new(), injection),
                Err(KernelError::NotInferable { .. })
            ),
            "an injection has no synthesizable type"
        );
    }

    #[test]
    fn injection_checks_against_its_sum()
    {
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        let entries = no_entries();
        let unit_type = arena.value_type_unit();
        let integer = arena.value_type_base(crate::base::BaseType::Integer);
        let sum = arena.value_type_sum(unit_type, integer);
        let unit = arena.value_unit();
        let left = arena.value_injection(Side::Left, unit);
        assert!(
            run(
                &mut arena,
                &entries,
                &levels,
                Vec::new(),
                Goal::CheckValue(left, sum)
            )
            .is_ok(),
            "inl unit checks against Unit + Integer"
        );
    }

    #[test]
    fn pair_propagates_into_a_checking_component()
    {
        // A pair whose first component is an injection (check-only) checks
        // against a product, exercising the propagation.
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        let entries = no_entries();
        let unit_a = arena.value_type_unit();
        let unit_b = arena.value_type_unit();
        let sum = arena.value_type_sum(unit_a, unit_b);
        let unit_c = arena.value_type_unit();
        let product = arena.value_type_product(sum, unit_c);
        let inner_unit = arena.value_unit();
        let injection = arena.value_injection(Side::Right, inner_unit);
        let second_unit = arena.value_unit();
        let pair = arena.value_pair(injection, second_unit);
        assert!(
            run(
                &mut arena,
                &entries,
                &levels,
                Vec::new(),
                Goal::CheckValue(pair, product)
            )
            .is_ok(),
            "a pair propagates the expected product into a checking component"
        );
    }

    #[test]
    fn application_result_is_the_codomain()
    {
        // (force v0) applied to unit, where v0 : U (Unit → F Unit).
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        let entries = no_entries();
        let domain = arena.value_type_unit();
        let result = arena.value_type_unit();
        let returner = arena.comp_type_returner(result);
        let arrow = arena.comp_type_arrow(domain, returner);
        let context_type = arena.value_type_thunk(arrow);
        let variable = arena.value_variable(DeBruijnIndex::from(0_u32));
        let force = arena.computation_force(variable);
        let unit = arena.value_unit();
        let application = arena.computation_application(force, unit);
        let synthesized = synth_comp(
            &mut arena,
            &entries,
            &levels,
            alloc::vec![context_type],
            application,
        )
        .unwrap();
        assert!(
            matches!(arena.comp_type(synthesized), Some(&CompType::Returner(_))),
            "applying the forced thunk yields the codomain F Unit"
        );
    }

    #[test]
    fn force_unwraps_a_thunk()
    {
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        let entries = no_entries();
        let result = arena.value_type_unit();
        let returner = arena.comp_type_returner(result);
        let context_type = arena.value_type_thunk(returner);
        let variable = arena.value_variable(DeBruijnIndex::from(0_u32));
        let force = arena.computation_force(variable);
        let synthesized = synth_comp(
            &mut arena,
            &entries,
            &levels,
            alloc::vec![context_type],
            force,
        )
        .unwrap();
        assert_eq!(
            synthesized, returner,
            "forcing a thunk unwraps its computation type (the same thunk-body id)"
        );
    }

    #[test]
    fn case_requires_convergent_branches()
    {
        // case v0 { inl ⇒ return unit | inr ⇒ return (thunk ...) } — branches at
        // distinct types must be rejected in synth mode.
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        let entries = no_entries();
        let unit_a = arena.value_type_unit();
        let unit_b = arena.value_type_unit();
        let context_type = arena.value_type_sum(unit_a, unit_b);
        let scrutinee = arena.value_variable(DeBruijnIndex::from(0_u32));
        let left_unit = arena.value_unit();
        let on_left = arena.computation_return(left_unit);
        let inner_unit = arena.value_unit();
        let inner_return = arena.computation_return(inner_unit);
        let thunk = arena.value_thunk(inner_return);
        let on_right = arena.computation_return(thunk);
        let case = arena.computation_case(scrutinee, on_left, on_right);
        assert!(
            matches!(
                synth_comp(
                    &mut arena,
                    &entries,
                    &levels,
                    alloc::vec![context_type],
                    case
                ),
                Err(KernelError::CaseBranchMismatch(_))
            ),
            "synth-mode case with divergent branch types is rejected"
        );
    }

    #[test]
    fn case_propagates_the_expected_type()
    {
        // In check mode both branches are checked against the expected type,
        // so an injection branch (check-only) is admissible.
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        let entries = no_entries();
        let unit_a = arena.value_type_unit();
        let unit_b = arena.value_type_unit();
        let context_type = arena.value_type_sum(unit_a, unit_b);
        let sum_a = arena.value_type_unit();
        let sum_b = arena.value_type_unit();
        let expected_sum = arena.value_type_sum(sum_a, sum_b);
        let expected = arena.comp_type_returner(expected_sum);
        let scrutinee = arena.value_variable(DeBruijnIndex::from(0_u32));
        let left_unit = arena.value_unit();
        let left_injection = arena.value_injection(Side::Left, left_unit);
        let on_left = arena.computation_return(left_injection);
        let right_unit = arena.value_unit();
        let right_injection = arena.value_injection(Side::Right, right_unit);
        let on_right = arena.computation_return(right_injection);
        let case = arena.computation_case(scrutinee, on_left, on_right);
        assert!(
            run(
                &mut arena,
                &entries,
                &levels,
                alloc::vec![context_type],
                Goal::CheckComp(case, expected),
            )
            .is_ok(),
            "check-mode case propagates the expected type into both branches"
        );
    }

    #[test]
    fn universe_forms_one_level_up()
    {
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        let universe = arena.value_type_universe(Level::constant(LevelConstant::from(0_u64)));
        let level = type_level(&arena, &levels, TypeLevelGoal::Value(universe)).unwrap();
        assert_eq!(
            level,
            Level::constant(LevelConstant::from(1_u64)),
            "U_0 forms at level 1"
        );
    }

    #[test]
    fn lift_requires_a_strictly_higher_target()
    {
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        // Lift Unit to level 1: Unit is at level 0, 0 < 1, so this is well-formed.
        let inner = arena.value_type_unit();
        let lifted = arena.value_type_lift(inner, Level::constant(LevelConstant::from(1_u64)));
        assert_eq!(
            type_level(&arena, &levels, TypeLevelGoal::Value(lifted)).unwrap(),
            Level::constant(LevelConstant::from(1_u64)),
            "lifting Unit to level 1 forms at level 1"
        );
        // Lift Unit to level 0: 0 < 0 fails.
        let inner2 = arena.value_type_unit();
        let degenerate = arena.value_type_lift(inner2, Level::constant(LevelConstant::from(0_u64)));
        assert!(
            matches!(
                type_level(&arena, &levels, TypeLevelGoal::Value(degenerate)),
                Err(KernelError::UniverseViolation(_))
            ),
            "a lift that does not strictly raise the level is rejected"
        );
    }

    #[test]
    fn arrow_level_is_the_join()
    {
        let mut arena = TermArena::new();
        let levels = level_context(LevelParamCount::from(0_u32));
        // (U_2 → F Unit): domain level 3, codomain level 0, join 3.
        let domain = arena.value_type_universe(Level::constant(LevelConstant::from(2_u64)));
        let result = arena.value_type_unit();
        let returner = arena.comp_type_returner(result);
        let arrow = arena.comp_type_arrow(domain, returner);
        assert_eq!(
            type_level(&arena, &levels, TypeLevelGoal::Comp(arrow)).unwrap(),
            Level::constant(LevelConstant::from(3_u64)),
            "the arrow forms at the join of its parts' levels"
        );
    }

    #[test]
    fn register_polarity_faults_fail_closed()
    {
        // The register projections are unreachable with a mismatched polarity
        // when the machine is wired per the correspondence table; pinned
        // directly here so a wiring defect rejects (fail-closed) rather than
        // fabricating a type that could wrongly convert.
        let mut arena = TermArena::new();
        let unit = arena.value_type_unit();
        assert!(
            matches!(
                super::produced_value_type(super::Produced::Checked),
                Err(KernelError::CheckerRegisterFault(
                    RegisterFault::ExpectedValueType
                ))
            ),
            "a non-value register is a surfaced fault, not a fabricated type"
        );
        assert!(
            matches!(
                super::produced_comp_type(super::Produced::ValueType(unit)),
                Err(KernelError::CheckerRegisterFault(
                    RegisterFault::ExpectedCompType
                ))
            ),
            "a non-computation register is a surfaced fault, not a fabricated type"
        );
    }
}
