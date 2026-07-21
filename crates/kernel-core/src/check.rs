//! The zero-inference S1 checker (kernel-boundary.md K2): a bidirectional
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
//! The checker descends structurally through the finite checked term. To stay
//! **total on adversarial depth** (the kernel never diverges — a stack overflow
//! is divergence, and export decode can build an arbitrarily deep term from
//! bytes) it runs as an explicit **defunctionalized machine** ([`run`]) over a
//! goal register, a produced register, a heap frame stack, and an explicit
//! typing-context stack — never mutually recursive methods bounded by a depth
//! budget. This is the standard adversarial-depth machine that
//! [`crate::conv`] and [`crate::export`] already use; it meets the
//! docs/workflow/rust.md "input recursion: none" discipline (gandr-98o).
//!
//! Two self-contained clusters call each other directly, never recursively:
//! [`type_level`] (value-/computation-type formation) is its own iterative walk
//! the machine calls directly, exactly as it calls the already-iterative
//! [`crate::conv`]; the machine defunctionalizes only the term-level
//! self-recursion of synthesis and checking. The context is an explicit `Vec`
//! of [`Held`] slots — `Borrowed` for a declaration-derived type, `Owned` for a
//! type synthesized and bound by `Bind`/`Case` — with explicit scope-exit
//! frames popping it; lookup clones the slot (total via the iterative `Clone`
//! of gandr-i3i).
//!
//! ## Correspondence — old recursive arm ↔ machine step
//!
//! Each arm of the former recursive presentation maps to exactly one goal
//! expansion plus (for a multi-child arm) one continuation frame. The reviewer
//! walks this table:
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
//! | `synth_value` Var/Const/Unit/Lit | leaf → lookup / clone                 | —                                     |
//! | `synth_value` Pair               | `SynthValue(first)`                   | `SynthPairFirst`, `SynthPairSecond`   |
//! | `synth_value` Thunk              | `SynthComp(body)`                     | `SynthThunk`                          |
//! | `synth_value` Lift               | `SynthValue(body)`                    | `SynthLift`                           |
//! | `synth_value` Injection          | leaf → `NotInferable`                 | —                                     |
//! | `check_value` Injection          | `CheckValue(body, summand)`           | —                                     |
//! | `check_value` Pair               | `CheckValue(first, ft)`               | `CheckPairSecond`                     |
//! | `check_value` Thunk              | `CheckComp(body, cod)`                | —                                     |
//! | `check_value` synth-fallthrough  | `SynthValue(value)`                   | `ConvertValue`                        |
//! | `synth_comp` Application         | `SynthComp(head)`                     | `SynthApply`, `ProduceComp`           |
//! | `synth_comp` Force               | `SynthValue(value)`                   | `SynthForce`                          |
//! | `synth_comp` Return             | `SynthValue(value)`                   | `SynthReturn`                         |
//! | `synth_comp` Bind               | `SynthComp(bound)`                    | `SynthBind`, `ScopeExit`              |
//! | `synth_comp` Case               | `SynthValue(scrutinee)`               | `SynthCaseScrutinee/AfterLeft/AfterRight`, `ScopeExit` |
//! | `synth_comp` Lambda             | leaf → `NotInferable`                 | —                                     |
//! | `check_comp` Lambda             | `CheckComp(body, cod)`                | `ScopeExit`                           |
//! | `check_comp` Return             | `CheckValue(value, result)`           | —                                     |
//! | `check_comp` Bind               | `SynthComp(bound)`                    | `CheckBind`, `ScopeExit`              |
//! | `check_comp` Case               | `SynthValue(scrutinee)`               | `CheckCaseScrutinee/AfterLeft`, `ScopeExit` |
//! | `check_comp` synth-fallthrough  | `SynthComp(computation)`              | `ConvertComp`                         |

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::mem;

use gandr_kernel_strata::Level;

use crate::conv::Convertibility;
use crate::conv::convert_comp_type;
use crate::conv::convert_value_type;
use crate::conv::convertible_comp_types;
use crate::env::Environment;
use crate::error::ComputationTypeMismatch;
use crate::error::ExpectedComputationShape;
use crate::error::ExpectedValueShape;
use crate::error::KernelError;
use crate::error::NonInferableForm;
use crate::error::RegisterFault;
use crate::levels::LevelContext;
use crate::term::Computation;
use crate::term::DeBruijnIndex;
use crate::term::Side;
use crate::term::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// A type held either as a borrow into the checked declaration (`Borrowed`,
/// costing nothing) or as an owned type synthesized during checking (`Owned`).
///
/// The context slots and the check-mode expected types are `Held`: a
/// declaration-derived type is a cheap borrow, and a type produced by synthesis
/// (a `Bind`/`Case` binder, or an application's domain) is owned in place.
enum Held<'held, T>
{
    /// A borrow into the declaration being checked.
    Borrowed(&'held T),
    /// A type owned by the machine (synthesized during checking).
    Owned(T),
}

impl<T> Held<'_, T>
{
    /// Borrow the held type, whichever way it is held.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: a shared reference to the held type.
    /// - provides: the uniform read the conversion faces need.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[expect(
        clippy::match_same_arms,
        reason = "the two arms bind a reference of different provenance (a copied 'held borrow vs a fresh borrow of an owned value) that an or-pattern cannot unify"
    )]
    fn get(&self) -> &T
    {
        match *self {
            | Held::Borrowed(inner) => inner,
            | Held::Owned(ref inner) => inner,
        }
    }
}

impl Held<'_, ValueType>
{
    /// Take ownership of the held value type, cloning a borrow.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: the owned value type (a `Borrowed` is cloned iteratively).
    /// - provides: the owned type a shape-mismatch error payload boxes.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    fn into_owned(self) -> ValueType
    {
        match self {
            | Held::Borrowed(inner) => inner.clone(),
            | Held::Owned(inner) => inner,
        }
    }
}

impl Held<'_, CompType>
{
    /// Take ownership of the held computation type, cloning a borrow.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: the owned computation type (a `Borrowed` is cloned).
    /// - provides: the owned type a shape-mismatch error payload boxes.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    fn into_owned(self) -> CompType
    {
        match self {
            | Held::Borrowed(inner) => inner.clone(),
            | Held::Owned(inner) => inner,
        }
    }

    /// A second held reference to the same computation type: a `Borrowed`
    /// reference is copied, an `Owned` type is cloned. Used to check both
    /// `case` branches against one expected type.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: a `Held` denoting the same type; no clone for a `Borrowed`
    ///   expected (the common declaration-check path), one clone for an
    ///   `Owned`.
    /// - provides: the shared expected the case-check propagates to both
    ///   branches without an unbounded clone blow-up.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    fn dup(&self) -> Self
    {
        match *self {
            | Held::Borrowed(inner) => Held::Borrowed(inner),
            | Held::Owned(ref inner) => Held::Owned(inner.clone()),
        }
    }
}

/// Split a held product into its two component value types.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok((first, second))` when the held type is a product — each a
///   `Held` of the same provenance (a borrow of a `Borrowed`, an owned
///   placeholder-swap of an `Owned`); `Err(held)` unchanged otherwise.
/// - provides: the by-value product elimination the pair-check needs.
/// - fails: `Err(held)` when the type is not a product.
/// - panics: none.
fn held_take_product(
    held: Held<'_, ValueType>
) -> Result<(Held<'_, ValueType>, Held<'_, ValueType>), Held<'_, ValueType>>
{
    match held {
        | Held::Borrowed(value_type) => match *value_type {
            | ValueType::Product(ref first, ref second) => {
                Ok((Held::Borrowed(first), Held::Borrowed(second)))
            },
            | _ => Err(Held::Borrowed(value_type)),
        },
        | Held::Owned(value_type) => match take_product(value_type) {
            | Ok((first, second)) => Ok((Held::Owned(*first), Held::Owned(*second))),
            | Err(value_type) => Err(Held::Owned(value_type)),
        },
    }
}

/// Split a held sum into its two summand value types.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok((left, right))` when the held type is a sum; `Err(held)`
///   unchanged otherwise.
/// - provides: the by-value sum elimination the injection-check needs.
/// - fails: `Err(held)` when the type is not a sum.
/// - panics: none.
fn held_take_sum(
    held: Held<'_, ValueType>
) -> Result<(Held<'_, ValueType>, Held<'_, ValueType>), Held<'_, ValueType>>
{
    match held {
        | Held::Borrowed(value_type) => match *value_type {
            | ValueType::Sum(ref left, ref right) => {
                Ok((Held::Borrowed(left), Held::Borrowed(right)))
            },
            | _ => Err(Held::Borrowed(value_type)),
        },
        | Held::Owned(value_type) => match take_sum(value_type) {
            | Ok((left, right)) => Ok((Held::Owned(*left), Held::Owned(*right))),
            | Err(value_type) => Err(Held::Owned(value_type)),
        },
    }
}

/// Take a held thunk type's computation type.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(codomain)` when the held type is a thunk; `Err(held)`
///   unchanged otherwise.
/// - provides: the by-value thunk elimination the thunk-check needs.
/// - fails: `Err(held)` when the type is not a thunk.
/// - panics: none.
fn held_take_thunk(held: Held<'_, ValueType>) -> Result<Held<'_, CompType>, Held<'_, ValueType>>
{
    match held {
        | Held::Borrowed(value_type) => match *value_type {
            | ValueType::Thunk(ref body) => Ok(Held::Borrowed(body)),
            | _ => Err(Held::Borrowed(value_type)),
        },
        | Held::Owned(value_type) => match take_thunk(value_type) {
            | Ok(body) => Ok(Held::Owned(*body)),
            | Err(value_type) => Err(Held::Owned(value_type)),
        },
    }
}

/// Split a held arrow into its domain value type and codomain computation type.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok((domain, codomain))` when the held type is an arrow;
///   `Err(held)` unchanged otherwise.
/// - provides: the by-value arrow elimination the lambda-check needs.
/// - fails: `Err(held)` when the type is not an arrow.
/// - panics: none.
fn held_take_arrow(
    held: Held<'_, CompType>
) -> Result<(Held<'_, ValueType>, Held<'_, CompType>), Held<'_, CompType>>
{
    match held {
        | Held::Borrowed(comp_type) => match *comp_type {
            | CompType::Arrow {
                ref domain,
                ref codomain,
            } => Ok((Held::Borrowed(domain), Held::Borrowed(codomain))),
            | _ => Err(Held::Borrowed(comp_type)),
        },
        | Held::Owned(comp_type) => match take_arrow(comp_type) {
            | Ok((domain, codomain)) => Ok((Held::Owned(*domain), Held::Owned(*codomain))),
            | Err(comp_type) => Err(Held::Owned(comp_type)),
        },
    }
}

/// Take a held returner type's result value type.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(result)` when the held type is a returner; `Err(held)`
///   unchanged otherwise.
/// - provides: the by-value returner elimination the return-check needs.
/// - fails: `Err(held)` when the type is not a returner.
/// - panics: none.
fn held_take_returner(held: Held<'_, CompType>) -> Result<Held<'_, ValueType>, Held<'_, CompType>>
{
    match held {
        | Held::Borrowed(comp_type) => match *comp_type {
            | CompType::Returner(ref result) => Ok(Held::Borrowed(result)),
            | _ => Err(Held::Borrowed(comp_type)),
        },
        | Held::Owned(comp_type) => match take_returner(comp_type) {
            | Ok(result) => Ok(Held::Owned(*result)),
            | Err(comp_type) => Err(Held::Owned(comp_type)),
        },
    }
}

/// Extract an owned product's two components by placeholder-swap.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok((first, second))` when `value_type` is a product, its child
///   boxes moved out and replaced by placeholder leaves; `Err(value_type)`
///   unchanged otherwise.
/// - provides: the owned-product elimination [`held_take_product`] needs
///   without an E0509 move out of a `Drop` type.
/// - fails: `Err(value_type)` when the type is not a product.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching a mutable borrow of the owned type to swap its child boxes in place; the binding is a mutable reference by intent"
)]
fn take_product(mut value_type: ValueType) -> Result<(Box<ValueType>, Box<ValueType>), ValueType>
{
    if let ValueType::Product(first, second) = &mut value_type {
        let first = mem::replace(first, Box::new(ValueType::Unit));
        let second = mem::replace(second, Box::new(ValueType::Unit));
        Ok((first, second))
    }
    else {
        Err(value_type)
    }
}

/// Extract an owned arrow's domain and codomain by placeholder-swap.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok((domain, codomain))` when `comp_type` is an arrow, with its
///   child boxes moved out and replaced in place by placeholder leaves so the
///   drained husk drops in `O(1)`; `Err(comp_type)` returns the value unchanged
///   otherwise.
/// - provides: the by-value arrow elimination the synthesizing checker needs
///   without moving a field out of a `Drop` type (the E0509 interlock the
///   worklist `Drop` forces, gandr-i3i).
/// - fails: `Err(comp_type)` when the type is not an arrow.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching a mutable borrow of the owned type to swap its child boxes in place; the binding is a mutable reference by intent"
)]
fn take_arrow(mut comp_type: CompType) -> Result<(Box<ValueType>, Box<CompType>), CompType>
{
    if let CompType::Arrow { domain, codomain } = &mut comp_type {
        let domain = mem::replace(domain, Box::new(ValueType::Unit));
        let codomain = mem::replace(
            codomain,
            Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        );
        Ok((domain, codomain))
    }
    else {
        Err(comp_type)
    }
}

/// Extract an owned returner's result type by placeholder-swap.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(result)` when `comp_type` is a returner, its child box moved
///   out and replaced by a placeholder leaf; `Err(comp_type)` unchanged
///   otherwise.
/// - provides: the by-value returner elimination the checker needs without an
///   E0509 move out of a `Drop` type.
/// - fails: `Err(comp_type)` when the type is not a returner.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching a mutable borrow of the owned type to swap its child box in place; the binding is a mutable reference by intent"
)]
fn take_returner(mut comp_type: CompType) -> Result<Box<ValueType>, CompType>
{
    if let CompType::Returner(result) = &mut comp_type {
        let result = mem::replace(result, Box::new(ValueType::Unit));
        Ok(result)
    }
    else {
        Err(comp_type)
    }
}

/// Extract an owned thunk type's computation type by placeholder-swap.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(codomain)` when `value_type` is a thunk, its child box moved
///   out and replaced by a placeholder leaf; `Err(value_type)` unchanged
///   otherwise.
/// - provides: the by-value thunk elimination the checker needs without an
///   E0509 move out of a `Drop` type.
/// - fails: `Err(value_type)` when the type is not a thunk.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching a mutable borrow of the owned type to swap its child box in place; the binding is a mutable reference by intent"
)]
fn take_thunk(mut value_type: ValueType) -> Result<Box<CompType>, ValueType>
{
    if let ValueType::Thunk(codomain) = &mut value_type {
        let codomain = mem::replace(
            codomain,
            Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        );
        Ok(codomain)
    }
    else {
        Err(value_type)
    }
}

/// Extract an owned sum type's two summands by placeholder-swap.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok((left, right))` when `value_type` is a sum, its child boxes
///   moved out and replaced by placeholder leaves; `Err(value_type)` unchanged
///   otherwise.
/// - provides: the by-value sum elimination the checker needs without an E0509
///   move out of a `Drop` type.
/// - fails: `Err(value_type)` when the type is not a sum.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching a mutable borrow of the owned type to swap its child boxes in place; the binding is a mutable reference by intent"
)]
fn take_sum(mut value_type: ValueType) -> Result<(Box<ValueType>, Box<ValueType>), ValueType>
{
    if let ValueType::Sum(left, right) = &mut value_type {
        let left = mem::replace(left, Box::new(ValueType::Unit));
        let right = mem::replace(right, Box::new(ValueType::Unit));
        Ok((left, right))
    }
    else {
        Err(value_type)
    }
}

/// A pending type-formation obligation over a borrowed type node — the goal of
/// the iterative [`type_level`] walk.
enum TypeLevelGoal<'ty>
{
    /// The level of a value type.
    Value(&'ty ValueType),
    /// The level of a computation type.
    Comp(&'ty CompType),
}

/// A continuation of the iterative type-formation walk.
enum TypeLevelFrame<'ty>
{
    /// The first (value-type) operand's level is in the register; compute the
    /// second value-type operand's level next (a product or sum).
    MaxSecondValue(&'ty ValueType),
    /// The first (value-type) operand's level is in the register; compute the
    /// second (computation-type) operand's level next (an arrow).
    MaxSecondComp(&'ty CompType),
    /// The second operand's level is in the register; join with the first.
    MaxWith(Level),
    /// The inner level is in the register; check the lift's strictness and
    /// replace it with the target level.
    LiftCheck(&'ty Level),
}

/// The universe level of a well-formed value or computation type — the K1
/// well-formedness gate, computed iteratively so it is total on any depth.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(level)` — the minimal universe level of the type — exactly
///   when every embedded level is in scope, every lift strictly raises, and no
///   successor overflows; a bare `Universe(l)` forms at `l + 1`. The walk is
///   iterative over an explicit heap frame stack, so it is total on any depth.
/// - provides: value-/computation-type formation for
///   [`Checker::check_value_type`] and the machine's `Lift` synthesis.
/// - fails: [`KernelError::LevelVariableOutOfScope`],
///   [`KernelError::LevelArithmetic`], [`KernelError::UniverseViolation`],
///   [`KernelError::LevelOracleFault`].
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
    levels: &LevelContext,
    root: TypeLevelGoal<'_>,
) -> Result<Level, KernelError>
{
    let mut frames: Vec<TypeLevelFrame<'_>> = Vec::new();
    let mut goal = root;
    'expand: loop {
        let mut produced: Level = match goal {
            | TypeLevelGoal::Value(value_type) => match *value_type {
                | ValueType::Base(_) | ValueType::Unit => Level::zero(),
                | ValueType::Universe(ref level) => {
                    levels.check_level_scope(level)?;
                    level.succ()?
                },
                | ValueType::Product(ref first, ref second)
                | ValueType::Sum(ref first, ref second) => {
                    frames.push(TypeLevelFrame::MaxSecondValue(second));
                    goal = TypeLevelGoal::Value(first);
                    continue 'expand;
                },
                | ValueType::Thunk(ref body) => {
                    goal = TypeLevelGoal::Comp(body);
                    continue 'expand;
                },
                | ValueType::Lift {
                    ref inner,
                    ref target,
                } => {
                    frames.push(TypeLevelFrame::LiftCheck(target));
                    goal = TypeLevelGoal::Value(inner);
                    continue 'expand;
                },
            },
            | TypeLevelGoal::Comp(comp_type) => match *comp_type {
                | CompType::Returner(ref result) => {
                    goal = TypeLevelGoal::Value(result);
                    continue 'expand;
                },
                | CompType::Arrow {
                    ref domain,
                    ref codomain,
                } => {
                    frames.push(TypeLevelFrame::MaxSecondComp(codomain));
                    goal = TypeLevelGoal::Value(domain);
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
                    levels.check_level_scope(target)?;
                    levels.check_universe_below(&produced, target)?;
                    produced = target.clone();
                },
            }
        }
    }
}

/// A result produced by the checker machine into its register.
enum Produced
{
    /// A synthesized value type.
    ValueType(ValueType),
    /// A synthesized computation type.
    CompType(CompType),
    /// A check succeeded (no type is carried).
    Checked,
}

/// Project a synthesized value type out of the produced register.
///
/// # Contract
/// - requires: the register was set by a `SynthValue` goal — its variant
///   matches by construction (a value goal produces a value type; the module
///   correspondence table is the wiring audit).
/// - ensures: `Ok(value_type)` — the produced value type.
/// - provides: the fail-closed register projection the value-consuming frames
///   need: a polarity mismatch is unreachable when the machine is wired
///   correctly, and is surfaced rather than trusted (the
///   [`KernelError::LevelOracleFault`] posture) so a wiring defect rejects the
///   declaration instead of fabricating a type that could wrongly convert.
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
fn produced_value_type(produced: Produced) -> Result<ValueType, KernelError>
{
    match produced {
        | Produced::ValueType(value_type) => Ok(value_type),
        | Produced::CompType(_) | Produced::Checked => Err(KernelError::CheckerRegisterFault(
            RegisterFault::ExpectedValueType,
        )),
    }
}

/// Project a synthesized computation type out of the produced register.
///
/// # Contract
/// - requires: the register was set by a `SynthComp` goal (its variant matches
///   by construction, as in [`produced_value_type`]).
/// - ensures: `Ok(comp_type)` — the produced computation type.
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
fn produced_comp_type(produced: Produced) -> Result<CompType, KernelError>
{
    match produced {
        | Produced::CompType(comp_type) => Ok(comp_type),
        | Produced::ValueType(_) | Produced::Checked => Err(KernelError::CheckerRegisterFault(
            RegisterFault::ExpectedCompType,
        )),
    }
}

/// A checking obligation over a borrowed term node — the goal of the machine.
enum Goal<'decl>
{
    /// Synthesize a value's type.
    SynthValue(&'decl Value),
    /// Check a value against an expected value type.
    CheckValue(&'decl Value, Held<'decl, ValueType>),
    /// Synthesize a computation's type.
    SynthComp(&'decl Computation),
    /// Check a computation against an expected computation type.
    CheckComp(&'decl Computation, Held<'decl, CompType>),
}

/// A continuation of the checker machine.
enum Frame<'decl>
{
    /// A pair synth: the first component is synthesizing; the second source is
    /// held.
    SynthPairFirst(&'decl Value),
    /// A pair synth: the first component is synthesized; the second is
    /// synthesizing.
    SynthPairSecond(ValueType),
    /// A thunk synth: the body computation is synthesizing.
    SynthThunk,
    /// A lift synth: the body is synthesizing; the target level is held.
    SynthLift(&'decl Level),
    /// A pair check: the first component checked; check the second against the
    /// held component type.
    CheckPairSecond(&'decl Value, Held<'decl, ValueType>),
    /// A mode-switch value conversion: the value synthesized; convert against
    /// the held expected type.
    ConvertValue(Held<'decl, ValueType>),
    /// An application: the head is synthesized; the argument source is held.
    SynthApply(&'decl Value),
    /// An application: the argument checked; produce the held codomain.
    ProduceComp(CompType),
    /// A force: the value is synthesized.
    SynthForce,
    /// A returner synth: the value is synthesized.
    SynthReturn,
    /// A bind synth: the bound computation is synthesized; the body source is
    /// held. The body's synthesized type becomes the bind's type.
    SynthBind(&'decl Computation),
    /// A case synth: the scrutinee is synthesized; both branch sources are
    /// held.
    SynthCaseScrutinee(&'decl Computation, &'decl Computation),
    /// A case synth: the left branch synthesized; its type and the right
    /// summand are held while the right branch synthesizes.
    SynthCaseAfterLeft
    {
        /// The right branch source.
        on_right: &'decl Computation,
        /// The right summand type (a context slot for the right branch).
        right: ValueType,
    },
    /// A case synth: both branches synthesized; the left type is held for the
    /// convergence check.
    SynthCaseAfterRight
    {
        /// The left branch's synthesized type.
        left_type: CompType,
    },
    /// A mode-switch computation conversion: the computation synthesized;
    /// convert against the held expected type.
    ConvertComp(Held<'decl, CompType>),
    /// A bind check: the bound computation synthesized; the body source and the
    /// expected type are held.
    CheckBind(&'decl Computation, Held<'decl, CompType>),
    /// A case check: the scrutinee synthesized; the branch sources and expected
    /// type are held.
    CheckCaseScrutinee
    {
        /// The left branch source.
        on_left: &'decl Computation,
        /// The right branch source.
        on_right: &'decl Computation,
        /// The expected type both branches check against.
        expected: Held<'decl, CompType>,
    },
    /// A case check: the left branch checked; the right source, right summand,
    /// and expected type are held.
    CheckCaseAfterLeft
    {
        /// The right branch source.
        on_right: &'decl Computation,
        /// The right summand type (a context slot for the right branch).
        right: ValueType,
        /// The expected type the right branch checks against.
        expected: Held<'decl, CompType>,
    },
    /// A binder scope closes: pop the innermost context slot.
    ScopeExit,
}

/// The value type bound at de Bruijn `index` in the explicit context, if in
/// scope. The context head (last slot) is index `0`.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Some(ty)` — an owned clone of the slot — when `index` names a
///   binding, walking `index` slots inward from the top; `None` when it escapes
///   the context. Iterative and closed over the finite context.
/// - provides: the variable rule's lookup.
/// - fails: `None` on an out-of-scope index.
/// - panics: none.
#[inline]
fn lookup(
    context: &[Held<'_, ValueType>],
    index: DeBruijnIndex,
) -> Option<ValueType>
{
    let steps = usize::try_from(u32::from(index)).ok()?;
    let position = context.len().checked_sub(1)?.checked_sub(steps)?;
    context.get(position).map(|slot| slot.get().clone())
}

/// Run the defunctionalized checker machine from an initial goal and context to
/// a verdict.
///
/// # Contract
/// - requires: `context` is the initial typing context (empty for a declaration
///   body); `initial` is the root goal.
/// - ensures: `Ok(produced)` — a synthesized type for a synth goal, or
///   [`Produced::Checked`] for a check goal — exactly when the term checks or
///   synthesizes under the S1 rules; the walk is iterative over a heap frame
///   stack and an explicit context stack, so it is total on any term depth (no
///   input-scaled recursion; gandr-98o).
/// - provides: the shared engine of every S1 checking and synthesis judgment.
/// - fails: any [`KernelError`] a rule surfaces (shape mismatch, non-inferable
///   form, unbound reference, conversion failure, case-branch divergence, or a
///   level fault).
/// - panics: none.
///
/// # Errors
/// Any [`KernelError`].
///
/// # Adequacy
/// - hypothesis: L2/L3 — the golden corpus checks well-typed declarations and
///   rejects ill-typed mutations, and the totality property confirms a verdict
///   on any generated body; the L3 residues are each rule's failure arm (shape
///   mismatch, conversion failure, unbound reference, non-inferable form, case
///   divergence), pinned by negative unit goldens; the defunctionalized walk's
///   totality on adversarial depth is pinned by a small-stack deep-chain
///   witness.
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
#[expect(
    clippy::too_many_lines,
    reason = "the flat defunctionalized checker enumerates every synthesis and checking arm in one goal/frame loop; splitting it would obscure the single machine the correspondence table audits"
)]
fn run<'decl>(
    environment: &'decl Environment,
    levels: &'decl LevelContext,
    mut context: Vec<Held<'decl, ValueType>>,
    initial: Goal<'decl>,
) -> Result<Produced, KernelError>
{
    let mut frames: Vec<Frame<'decl>> = Vec::new();
    let mut goal = initial;
    'expand: loop {
        let mut produced: Produced = match goal {
            | Goal::SynthValue(value) => match *value {
                | Value::Variable(index) => {
                    let synthesized =
                        lookup(&context, index).ok_or(KernelError::UnboundVariable { index })?;
                    Produced::ValueType(synthesized)
                },
                | Value::Constant(index) => {
                    let synthesized = environment
                        .declared_value_type(index)
                        .ok_or(KernelError::UnboundConstant { index })?
                        .clone();
                    Produced::ValueType(synthesized)
                },
                | Value::Unit => Produced::ValueType(ValueType::Unit),
                | Value::Literal(ref literal) => {
                    Produced::ValueType(ValueType::Base(literal.base_type()))
                },
                | Value::Pair(ref first, ref second) => {
                    frames.push(Frame::SynthPairFirst(second));
                    goal = Goal::SynthValue(first);
                    continue 'expand;
                },
                | Value::Thunk(ref body) => {
                    frames.push(Frame::SynthThunk);
                    goal = Goal::SynthComp(body);
                    continue 'expand;
                },
                | Value::Lift {
                    ref target,
                    ref body,
                } => {
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
            | Goal::CheckValue(value, expected) => match *value {
                | Value::Injection(side, ref body) => match held_take_sum(expected) {
                    | Ok((left, right)) => {
                        let summand = match side {
                            | Side::Left => left,
                            | Side::Right => right,
                        };
                        goal = Goal::CheckValue(body, summand);
                        continue 'expand;
                    },
                    | Err(expected) => {
                        return Err(KernelError::ValueShapeMismatch {
                            expected: ExpectedValueShape::Sum,
                            actual: Box::new(expected.into_owned()),
                        });
                    },
                },
                | Value::Pair(ref first, ref second) => match held_take_product(expected) {
                    | Ok((first_type, second_type)) => {
                        frames.push(Frame::CheckPairSecond(second, second_type));
                        goal = Goal::CheckValue(first, first_type);
                        continue 'expand;
                    },
                    | Err(expected) => {
                        return Err(KernelError::ValueShapeMismatch {
                            expected: ExpectedValueShape::Product,
                            actual: Box::new(expected.into_owned()),
                        });
                    },
                },
                | Value::Thunk(ref body) => match held_take_thunk(expected) {
                    | Ok(codomain) => {
                        goal = Goal::CheckComp(body, codomain);
                        continue 'expand;
                    },
                    | Err(expected) => {
                        return Err(KernelError::ValueShapeMismatch {
                            expected: ExpectedValueShape::Thunk,
                            actual: Box::new(expected.into_owned()),
                        });
                    },
                },
                | _ => {
                    frames.push(Frame::ConvertValue(expected));
                    goal = Goal::SynthValue(value);
                    continue 'expand;
                },
            },
            | Goal::SynthComp(computation) => match *computation {
                | Computation::Application(ref head, ref argument) => {
                    frames.push(Frame::SynthApply(argument));
                    goal = Goal::SynthComp(head);
                    continue 'expand;
                },
                | Computation::Force(ref value) => {
                    frames.push(Frame::SynthForce);
                    goal = Goal::SynthValue(value);
                    continue 'expand;
                },
                | Computation::Return(ref value) => {
                    frames.push(Frame::SynthReturn);
                    goal = Goal::SynthValue(value);
                    continue 'expand;
                },
                | Computation::Bind(ref bound, ref body) => {
                    frames.push(Frame::SynthBind(body));
                    goal = Goal::SynthComp(bound);
                    continue 'expand;
                },
                | Computation::Case {
                    ref scrutinee,
                    ref on_left,
                    ref on_right,
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
            | Goal::CheckComp(computation, expected) => match *computation {
                | Computation::Lambda(ref body) => match held_take_arrow(expected) {
                    | Ok((domain, codomain)) => {
                        context.push(domain);
                        frames.push(Frame::ScopeExit);
                        goal = Goal::CheckComp(body, codomain);
                        continue 'expand;
                    },
                    | Err(expected) => {
                        return Err(KernelError::ComputationShapeMismatch {
                            expected: ExpectedComputationShape::Arrow,
                            actual: Box::new(expected.into_owned()),
                        });
                    },
                },
                | Computation::Return(ref value) => match held_take_returner(expected) {
                    | Ok(result) => {
                        goal = Goal::CheckValue(value, result);
                        continue 'expand;
                    },
                    | Err(expected) => {
                        return Err(KernelError::ComputationShapeMismatch {
                            expected: ExpectedComputationShape::Returner,
                            actual: Box::new(expected.into_owned()),
                        });
                    },
                },
                | Computation::Bind(ref bound, ref body) => {
                    frames.push(Frame::CheckBind(body, expected));
                    goal = Goal::SynthComp(bound);
                    continue 'expand;
                },
                | Computation::Case {
                    ref scrutinee,
                    ref on_left,
                    ref on_right,
                } => {
                    frames.push(Frame::CheckCaseScrutinee {
                        on_left,
                        on_right,
                        expected,
                    });
                    goal = Goal::SynthValue(scrutinee);
                    continue 'expand;
                },
                | _ => {
                    frames.push(Frame::ConvertComp(expected));
                    goal = Goal::SynthComp(computation);
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
                    produced = Produced::ValueType(ValueType::Product(
                        Box::new(first_type),
                        Box::new(second_type),
                    ));
                },
                | Frame::SynthThunk => {
                    let body_type = produced_comp_type(produced)?;
                    produced = Produced::ValueType(ValueType::Thunk(Box::new(body_type)));
                },
                | Frame::SynthLift(target) => {
                    let body_type = produced_value_type(produced)?;
                    let body_level = type_level(levels, TypeLevelGoal::Value(&body_type))?;
                    levels.check_level_scope(target)?;
                    levels.check_universe_below(&body_level, target)?;
                    produced = Produced::ValueType(ValueType::Lift {
                        inner: Box::new(body_type),
                        target: target.clone(),
                    });
                },
                | Frame::CheckPairSecond(second, second_type) => {
                    goal = Goal::CheckValue(second, second_type);
                    continue 'expand;
                },
                | Frame::ConvertValue(expected) => {
                    let synthesized = produced_value_type(produced)?;
                    convert_value_type(expected.get(), &synthesized)?;
                    produced = Produced::Checked;
                },
                | Frame::SynthApply(argument) => {
                    let head_type = produced_comp_type(produced)?;
                    match take_arrow(head_type) {
                        | Ok((domain, codomain)) => {
                            frames.push(Frame::ProduceComp(*codomain));
                            goal = Goal::CheckValue(argument, Held::Owned(*domain));
                            continue 'expand;
                        },
                        | Err(other) => {
                            return Err(KernelError::ComputationShapeMismatch {
                                expected: ExpectedComputationShape::Arrow,
                                actual: Box::new(other),
                            });
                        },
                    }
                },
                | Frame::ProduceComp(codomain) => {
                    produced = Produced::CompType(codomain);
                },
                | Frame::SynthForce => {
                    let value_type = produced_value_type(produced)?;
                    match take_thunk(value_type) {
                        | Ok(codomain) => produced = Produced::CompType(*codomain),
                        | Err(other) => {
                            return Err(KernelError::ValueShapeMismatch {
                                expected: ExpectedValueShape::Thunk,
                                actual: Box::new(other),
                            });
                        },
                    }
                },
                | Frame::SynthReturn => {
                    let result_type = produced_value_type(produced)?;
                    produced = Produced::CompType(CompType::Returner(Box::new(result_type)));
                },
                | Frame::SynthBind(body) => {
                    let bound_type = produced_comp_type(produced)?;
                    match take_returner(bound_type) {
                        | Ok(result) => {
                            context.push(Held::Owned(*result));
                            frames.push(Frame::ScopeExit);
                            goal = Goal::SynthComp(body);
                            continue 'expand;
                        },
                        | Err(other) => {
                            return Err(KernelError::ComputationShapeMismatch {
                                expected: ExpectedComputationShape::Returner,
                                actual: Box::new(other),
                            });
                        },
                    }
                },
                | Frame::SynthCaseScrutinee(on_left, on_right) => {
                    let scrutinee_type = produced_value_type(produced)?;
                    match take_sum(scrutinee_type) {
                        | Ok((left, right)) => {
                            context.push(Held::Owned(*left));
                            frames.push(Frame::SynthCaseAfterLeft {
                                on_right,
                                right: *right,
                            });
                            frames.push(Frame::ScopeExit);
                            goal = Goal::SynthComp(on_left);
                            continue 'expand;
                        },
                        | Err(other) => {
                            return Err(KernelError::ValueShapeMismatch {
                                expected: ExpectedValueShape::Sum,
                                actual: Box::new(other),
                            });
                        },
                    }
                },
                | Frame::SynthCaseAfterLeft { on_right, right } => {
                    let left_type = produced_comp_type(produced)?;
                    context.push(Held::Owned(right));
                    frames.push(Frame::SynthCaseAfterRight { left_type });
                    frames.push(Frame::ScopeExit);
                    goal = Goal::SynthComp(on_right);
                    continue 'expand;
                },
                | Frame::SynthCaseAfterRight { left_type } => {
                    let right_type = produced_comp_type(produced)?;
                    match convertible_comp_types(&left_type, &right_type) {
                        | Convertibility::Convertible => {
                            produced = Produced::CompType(left_type);
                        },
                        | Convertibility::Distinct => {
                            return Err(KernelError::CaseBranchMismatch(Box::new(
                                ComputationTypeMismatch::new(left_type, right_type),
                            )));
                        },
                    }
                },
                | Frame::ConvertComp(expected) => {
                    let synthesized = produced_comp_type(produced)?;
                    convert_comp_type(expected.get(), &synthesized)?;
                    produced = Produced::Checked;
                },
                | Frame::CheckBind(body, expected) => {
                    let bound_type = produced_comp_type(produced)?;
                    match take_returner(bound_type) {
                        | Ok(result) => {
                            context.push(Held::Owned(*result));
                            frames.push(Frame::ScopeExit);
                            goal = Goal::CheckComp(body, expected);
                            continue 'expand;
                        },
                        | Err(other) => {
                            return Err(KernelError::ComputationShapeMismatch {
                                expected: ExpectedComputationShape::Returner,
                                actual: Box::new(other),
                            });
                        },
                    }
                },
                | Frame::CheckCaseScrutinee {
                    on_left,
                    on_right,
                    expected,
                } => {
                    let scrutinee_type = produced_value_type(produced)?;
                    match take_sum(scrutinee_type) {
                        | Ok((left, right)) => {
                            let expected_left = expected.dup();
                            context.push(Held::Owned(*left));
                            frames.push(Frame::CheckCaseAfterLeft {
                                on_right,
                                right: *right,
                                expected,
                            });
                            frames.push(Frame::ScopeExit);
                            goal = Goal::CheckComp(on_left, expected_left);
                            continue 'expand;
                        },
                        | Err(other) => {
                            return Err(KernelError::ValueShapeMismatch {
                                expected: ExpectedValueShape::Sum,
                                actual: Box::new(other),
                            });
                        },
                    }
                },
                | Frame::CheckCaseAfterLeft {
                    on_right,
                    right,
                    expected,
                } => {
                    context.push(Held::Owned(right));
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

/// A fresh checker over the environment and level context of one declaration
/// (kernel-boundary.md §3: per-declaration checking spins up a fresh checker;
/// no checker state survives across declarations except the environment).
pub struct Checker<'kernel>
{
    /// The append-only environment, for resolving constant references.
    environment: &'kernel Environment,
    /// The declaration's admitted level context.
    levels: &'kernel LevelContext,
}

impl<'kernel> Checker<'kernel>
{
    /// A fresh checker for one declaration.
    #[inline]
    pub fn new(
        environment: &'kernel Environment,
        levels: &'kernel LevelContext,
    ) -> Self
    {
        Self {
            environment,
            levels,
        }
    }

    /// Check that a value type is well-formed (embedded levels in scope, lift
    /// strictness, no level overflow), discarding its level.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Ok(())` exactly when [`type_level`] succeeds on the type.
    /// - provides: the K1 well-formedness gate for a declared type.
    /// - fails: any [`KernelError`] `type_level` surfaces.
    /// - panics: none.
    ///
    /// # Errors
    /// [`KernelError::LevelVariableOutOfScope`],
    /// [`KernelError::LevelArithmetic`],
    /// [`KernelError::UniverseViolation`], [`KernelError::LevelOracleFault`].
    #[inline]
    pub fn check_value_type(
        &self,
        ty: &ValueType,
    ) -> Result<(), KernelError>
    {
        let _level = type_level(self.levels, TypeLevelGoal::Value(ty))?;
        Ok(())
    }

    /// Check a definition body against its declared value type.
    ///
    /// # Contract
    /// - requires: `declared` is already well-formed
    ///   ([`Self::check_value_type`]).
    /// - ensures: `Ok(())` exactly when `body` checks against `declared` in the
    ///   empty context, via the defunctionalized machine (total on any depth).
    /// - provides: the K2 body check of a `Def`.
    /// - fails: any [`KernelError`] the checker surfaces.
    /// - panics: none.
    ///
    /// # Errors
    /// Any [`KernelError`].
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the golden corpus checks well-typed declarations and
    ///   rejects ill-typed mutations; the L3 residues are each rule's failure
    ///   arm, pinned by the negative unit goldens (shape mismatch, conversion
    ///   failure, unbound reference).
    /// - witness: `check::tests::identity_thunk_checks`
    /// - witness: `check::tests::application_result_is_the_codomain`
    /// - witness: `check::tests::case_requires_convergent_branches`
    /// - witness: `checker::tests` (the integration corpus)
    #[inline]
    pub fn check_definition(
        &self,
        declared: &ValueType,
        body: &Value,
    ) -> Result<(), KernelError>
    {
        let context: Vec<Held<'_, ValueType>> = Vec::new();
        let _checked = run(
            self.environment,
            self.levels,
            context,
            Goal::CheckValue(body, Held::Borrowed(declared)),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;

    use super::Checker;
    use super::Goal;
    use super::Held;
    use super::Produced;
    use super::TypeLevelGoal;
    use super::run;
    use super::type_level;
    use crate::env::Environment;
    use crate::error::KernelError;
    use crate::levels::LevelContext;
    use crate::levels::LevelParamCount;
    use crate::term::Computation;
    use crate::term::DeBruijnIndex;
    use crate::term::Side;
    use crate::term::Value;
    use crate::types::CompType;
    use crate::types::ValueType;

    /// An empty environment.
    fn environment() -> Environment
    {
        Environment::new()
    }

    /// An empty level context over `params` prenex parameters.
    fn level_context(params: u32) -> LevelContext
    {
        LevelContext::admit(LevelParamCount::from(params), Vec::new()).unwrap()
    }

    /// The universe former at constant level `value`.
    fn universe(value: u64) -> ValueType
    {
        ValueType::Universe(Level::constant(LevelConstant::from(value)))
    }

    /// Build the initial context (the head slot is de Bruijn index `0`).
    fn context<'ctx>(slots: &[&'ctx ValueType]) -> Vec<Held<'ctx, ValueType>>
    {
        slots.iter().map(|slot| Held::Borrowed(*slot)).collect()
    }

    /// Synthesize a value's type in a given context.
    fn synth_value(
        environment: &Environment,
        levels: &LevelContext,
        slots: &[&ValueType],
        value: &Value,
    ) -> Result<ValueType, KernelError>
    {
        match run(environment, levels, context(slots), Goal::SynthValue(value))? {
            | Produced::ValueType(value_type) => Ok(value_type),
            | _ => panic!("a value goal must produce a value type"),
        }
    }

    /// Synthesize a computation's type in a given context.
    fn synth_comp(
        environment: &Environment,
        levels: &LevelContext,
        slots: &[&ValueType],
        computation: &Computation,
    ) -> Result<CompType, KernelError>
    {
        match run(
            environment,
            levels,
            context(slots),
            Goal::SynthComp(computation),
        )? {
            | Produced::CompType(comp_type) => Ok(comp_type),
            | _ => panic!("a computation goal must produce a computation type"),
        }
    }

    /// Check a value against an expected type in a given context.
    fn check_value(
        environment: &Environment,
        levels: &LevelContext,
        slots: &[&ValueType],
        value: &Value,
        expected: &ValueType,
    ) -> Result<(), KernelError>
    {
        run(
            environment,
            levels,
            context(slots),
            Goal::CheckValue(value, Held::Borrowed(expected)),
        )
        .map(|_produced| ())
    }

    /// Check a computation against an expected type in a given context.
    fn check_comp(
        environment: &Environment,
        levels: &LevelContext,
        slots: &[&ValueType],
        computation: &Computation,
        expected: &CompType,
    ) -> Result<(), KernelError>
    {
        run(
            environment,
            levels,
            context(slots),
            Goal::CheckComp(computation, Held::Borrowed(expected)),
        )
        .map(|_produced| ())
    }

    #[test]
    fn variable_synthesizes_its_context_type()
    {
        let environment = environment();
        let levels = level_context(0);
        let synthesized = synth_value(
            &environment,
            &levels,
            &[&ValueType::Unit],
            &Value::Variable(DeBruijnIndex::from(0_u32)),
        )
        .unwrap();
        assert_eq!(synthesized, ValueType::Unit, "variable 0 has the head type");
    }

    #[test]
    fn injection_is_not_inferable()
    {
        let environment = environment();
        let levels = level_context(0);
        let injection = Value::Injection(Side::Left, Box::new(Value::Unit));
        assert!(
            matches!(
                synth_value(&environment, &levels, &[], &injection),
                Err(KernelError::NotInferable { .. })
            ),
            "an injection has no synthesizable type"
        );
    }

    #[test]
    fn injection_checks_against_its_sum()
    {
        let environment = environment();
        let levels = level_context(0);
        let sum = ValueType::Sum(
            Box::new(ValueType::Unit),
            Box::new(ValueType::Base(crate::base::BaseType::Integer)),
        );
        let left = Value::Injection(Side::Left, Box::new(Value::Unit));
        assert_eq!(
            Ok(()),
            check_value(&environment, &levels, &[], &left, &sum),
            "inl unit checks against Unit + Integer"
        );
    }

    #[test]
    fn pair_propagates_into_a_checking_component()
    {
        // A pair whose first component is an injection (check-only) checks
        // against a product, exercising the propagation.
        let environment = environment();
        let levels = level_context(0);
        let sum = ValueType::Sum(Box::new(ValueType::Unit), Box::new(ValueType::Unit));
        let product = ValueType::Product(Box::new(sum), Box::new(ValueType::Unit));
        let pair = Value::Pair(
            Box::new(Value::Injection(Side::Right, Box::new(Value::Unit))),
            Box::new(Value::Unit),
        );
        assert_eq!(
            Ok(()),
            check_value(&environment, &levels, &[], &pair, &product),
            "a pair propagates the expected product into a checking component"
        );
    }

    #[test]
    fn identity_thunk_checks()
    {
        // thunk (λ. return v0) : U (Unit → F Unit)
        let environment = environment();
        let levels = level_context(0);
        let checker = Checker::new(&environment, &levels);
        let arrow = CompType::Arrow {
            domain: Box::new(ValueType::Unit),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        };
        let declared = ValueType::Thunk(Box::new(arrow));
        let body = Value::Thunk(Box::new(Computation::Lambda(Box::new(
            Computation::Return(Box::new(Value::Variable(DeBruijnIndex::from(0_u32)))),
        ))));
        assert_eq!(
            Ok(()),
            checker.check_definition(&declared, &body),
            "the identity thunk checks against its declared type"
        );
    }

    #[test]
    fn application_result_is_the_codomain()
    {
        // (force v0) applied to unit, where v0 : U (Unit → F Unit).
        let environment = environment();
        let levels = level_context(0);
        let arrow = CompType::Arrow {
            domain: Box::new(ValueType::Unit),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        };
        let context_type = ValueType::Thunk(Box::new(arrow));
        let application = Computation::Application(
            Box::new(Computation::Force(Box::new(Value::Variable(
                DeBruijnIndex::from(0_u32),
            )))),
            Box::new(Value::Unit),
        );
        let synthesized =
            synth_comp(&environment, &levels, &[&context_type], &application).unwrap();
        assert_eq!(
            synthesized,
            CompType::Returner(Box::new(ValueType::Unit)),
            "applying the forced thunk yields the codomain F Unit"
        );
    }

    #[test]
    fn force_unwraps_a_thunk()
    {
        let environment = environment();
        let levels = level_context(0);
        let returner = CompType::Returner(Box::new(ValueType::Unit));
        let context_type = ValueType::Thunk(Box::new(returner.clone()));
        let force = Computation::Force(Box::new(Value::Variable(DeBruijnIndex::from(0_u32))));
        assert_eq!(
            synth_comp(&environment, &levels, &[&context_type], &force).unwrap(),
            returner,
            "forcing a thunk unwraps its computation type"
        );
    }

    #[test]
    fn case_requires_convergent_branches()
    {
        // case v0 { inl ⇒ return unit | inr ⇒ return (λ...) } — branches at
        // distinct types must be rejected in synth mode.
        let environment = environment();
        let levels = level_context(0);
        let context_type = ValueType::Sum(Box::new(ValueType::Unit), Box::new(ValueType::Unit));
        let case = Computation::Case {
            scrutinee: Box::new(Value::Variable(DeBruijnIndex::from(0_u32))),
            on_left: Box::new(Computation::Return(Box::new(Value::Unit))),
            on_right: Box::new(Computation::Return(Box::new(Value::Thunk(Box::new(
                Computation::Return(Box::new(Value::Unit)),
            ))))),
        };
        assert!(
            matches!(
                synth_comp(&environment, &levels, &[&context_type], &case),
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
        let environment = environment();
        let levels = level_context(0);
        let context_type = ValueType::Sum(Box::new(ValueType::Unit), Box::new(ValueType::Unit));
        let expected = CompType::Returner(Box::new(ValueType::Sum(
            Box::new(ValueType::Unit),
            Box::new(ValueType::Unit),
        )));
        let case = Computation::Case {
            scrutinee: Box::new(Value::Variable(DeBruijnIndex::from(0_u32))),
            on_left: Box::new(Computation::Return(Box::new(Value::Injection(
                Side::Left,
                Box::new(Value::Unit),
            )))),
            on_right: Box::new(Computation::Return(Box::new(Value::Injection(
                Side::Right,
                Box::new(Value::Unit),
            )))),
        };
        assert_eq!(
            Ok(()),
            check_comp(&environment, &levels, &[&context_type], &case, &expected),
            "check-mode case propagates the expected type into both branches"
        );
    }

    #[test]
    fn universe_forms_one_level_up()
    {
        let levels = level_context(0);
        let level = type_level(&levels, TypeLevelGoal::Value(&universe(0))).unwrap();
        assert_eq!(
            level,
            Level::constant(LevelConstant::from(1_u64)),
            "U_0 forms at level 1"
        );
    }

    #[test]
    fn lift_requires_a_strictly_higher_target()
    {
        let levels = level_context(0);
        // Lift Unit to level 1: Unit is at level 0, 0 < 1, so this is well-formed.
        let lifted = ValueType::Lift {
            inner: Box::new(ValueType::Unit),
            target: Level::constant(LevelConstant::from(1_u64)),
        };
        assert_eq!(
            type_level(&levels, TypeLevelGoal::Value(&lifted)).unwrap(),
            Level::constant(LevelConstant::from(1_u64)),
            "lifting Unit to level 1 forms at level 1"
        );
        // Lift Unit to level 0: 0 < 0 fails.
        let degenerate = ValueType::Lift {
            inner: Box::new(ValueType::Unit),
            target: Level::constant(LevelConstant::from(0_u64)),
        };
        assert!(
            matches!(
                type_level(&levels, TypeLevelGoal::Value(&degenerate)),
                Err(KernelError::UniverseViolation(_))
            ),
            "a lift that does not strictly raise the level is rejected"
        );
    }

    #[test]
    fn arrow_level_is_the_join()
    {
        let levels = level_context(0);
        // (U_2 → F Unit): domain level 3, codomain level 0, join 3.
        let arrow = CompType::Arrow {
            domain: Box::new(universe(2)),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        };
        assert_eq!(
            type_level(&levels, TypeLevelGoal::Comp(&arrow)).unwrap(),
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
        assert!(
            matches!(
                super::produced_value_type(super::Produced::Checked),
                Err(KernelError::CheckerRegisterFault(
                    crate::error::RegisterFault::ExpectedValueType
                ))
            ),
            "a non-value register is a surfaced fault, not a fabricated type"
        );
        assert!(
            matches!(
                super::produced_comp_type(super::Produced::ValueType(ValueType::Unit)),
                Err(KernelError::CheckerRegisterFault(
                    crate::error::RegisterFault::ExpectedCompType
                ))
            ),
            "a non-computation register is a surfaced fault, not a fabricated type"
        );
    }
}
