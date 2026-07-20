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
//! # Totality via a depth budget
//!
//! The checker descends structurally through the finite checked term. To stay
//! **total on adversarial depth** (the kernel never diverges — a stack
//! overflow would be divergence), each descent spends one unit of a recursion
//! budget ([`Depth::LIMIT`]); exhausting it returns
//! [`KernelError::DepthLimitExceeded`] rather than overflowing the stack. This
//! recursive presentation is the direct, auditable reference (mirroring the
//! role of the recursive checker in the untrusted `gandr-core-checker`); the
//! budget-free **defunctionalized** adversarial-depth machine is a tracked
//! follow-up (crate STATUS). The context is a borrowed cons-list, so binder
//! entry allocates nothing.

use alloc::boxed::Box;
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
use crate::levels::LevelContext;
use crate::term::Computation;
use crate::term::DeBruijnIndex;
use crate::term::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// A recursion-depth counter bounding the checker's structural descent, so the
/// kernel stays total on adversarial-depth input.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
struct Depth(u32);

impl Depth
{
    /// The maximum structural descent depth. Chosen to keep the checker's own
    /// call stack well within a small (spawned-thread) stack; terms nested
    /// past it are rejected conservatively rather than risking overflow. The
    /// defunctionalized machine removes the bound entirely (tracked follow-up).
    const LIMIT: u32 = 512;

    /// The starting depth (the declaration root).
    #[inline]
    const fn start() -> Self
    {
        Self(0)
    }

    /// Descend one level, or reject at the budget.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Ok(depth + 1)` while within [`Self::LIMIT`].
    /// - provides: the totality bound of every recursive checker step.
    /// - fails: [`KernelError::DepthLimitExceeded`] at the budget.
    /// - panics: none.
    #[inline]
    fn deepen(self) -> Result<Self, KernelError>
    {
        match self.0.checked_add(1_u32) {
            | Some(next) if next <= Self::LIMIT => Ok(Self(next)),
            | _ => Err(KernelError::DepthLimitExceeded),
        }
    }
}

/// The typing context: a borrowed cons-list of value-variable types, indexed
/// by de Bruijn distance (the head binds index `0`). Types are closed, so no
/// shifting is ever needed and binder entry allocates nothing.
enum Context<'ctx>
{
    /// The empty context.
    Empty,
    /// A binding: the bound variable's type and the enclosing context.
    Bound
    {
        /// The bound variable's (closed) value type.
        ty: &'ctx ValueType,
        /// The enclosing context.
        outer: &'ctx Self,
    },
}

impl<'ctx> Context<'ctx>
{
    /// The value type bound at de Bruijn `index`, if in scope.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Some(ty)` when `index` names a binding, walking `index`
    ///   enclosing binders outward; `None` when it escapes the context.
    /// - provides: the variable rule's lookup.
    /// - fails: `None` on an out-of-scope index.
    /// - panics: none.
    #[inline]
    fn lookup(
        &self,
        index: DeBruijnIndex,
    ) -> Option<&'ctx ValueType>
    {
        let mut remaining = u32::from(index);
        let mut cursor = self;
        loop {
            match *cursor {
                | Context::Empty => return None,
                | Context::Bound { ty, outer } => {
                    if remaining == 0_u32 {
                        return Some(ty);
                    }
                    match remaining.checked_sub(1_u32) {
                        | Some(next) => {
                            remaining = next;
                            cursor = outer;
                        },
                        | None => return None,
                    }
                },
            }
        }
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
    /// - ensures: `Ok(())` exactly when [`Self::value_type_level`] succeeds.
    /// - provides: the K1 well-formedness gate for a declared type.
    /// - fails: any [`KernelError`] `value_type_level` surfaces.
    /// - panics: none.
    ///
    /// # Errors
    /// [`KernelError::LevelVariableOutOfScope`],
    /// [`KernelError::LevelArithmetic`],
    /// [`KernelError::UniverseViolation`], [`KernelError::LevelOracleFault`],
    /// [`KernelError::DepthLimitExceeded`].
    #[inline]
    pub fn check_value_type(
        &self,
        ty: &ValueType,
    ) -> Result<(), KernelError>
    {
        let _level = self.value_type_level(Depth::start(), ty)?;
        Ok(())
    }

    /// Check a definition body against its declared value type.
    ///
    /// # Contract
    /// - requires: `declared` is already well-formed
    ///   ([`Self::check_value_type`]).
    /// - ensures: `Ok(())` exactly when `body` checks against `declared` in the
    ///   empty context.
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
        self.check_value(&Context::Empty, Depth::start(), body, declared)
    }

    /// The universe level of a well-formed value type (also its
    /// well-formedness gate).
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Ok(level)` — the minimal universe level of `ty` — exactly
    ///   when every embedded level is in scope, every lift strictly raises, and
    ///   no successor overflows; a bare `Universe(l)` type-forms at `l + 1`
    ///   (the rule `U_l : U_{l+1}`).
    /// - provides: value-type formation and the K1 well-formedness check.
    /// - fails: [`KernelError::LevelVariableOutOfScope`],
    ///   [`KernelError::LevelArithmetic`], [`KernelError::UniverseViolation`]
    ///   (a lift whose inner level is not strictly below its target),
    ///   [`KernelError::LevelOracleFault`],
    ///   [`KernelError::DepthLimitExceeded`].
    /// - panics: none.
    ///
    /// # Errors
    /// As `- fails:`.
    ///
    /// # Adequacy
    /// - hypothesis: L2/L3 — the level of composite types is pinned by
    ///   `max`/`succ` goldens; the L3 residues are the universe successor
    ///   (`U_l` at `l+1`) and the lift-strictness boundary, pinned by unit
    ///   goldens and the strata `Level::lt` refutation.
    /// - witness: `check::tests::universe_forms_one_level_up`
    /// - witness: `check::tests::lift_requires_a_strictly_higher_target`
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic matching of a borrowed type node; every binding is a shared reference by intent"
    )]
    fn value_type_level(
        &self,
        depth: Depth,
        ty: &ValueType,
    ) -> Result<Level, KernelError>
    {
        let depth = depth.deepen()?;
        match ty {
            | ValueType::Base(_) | ValueType::Unit => Ok(Level::zero()),
            | ValueType::Product(first, second) | ValueType::Sum(first, second) => {
                let first_level = self.value_type_level(depth, first)?;
                let second_level = self.value_type_level(depth, second)?;
                Ok(first_level.max(&second_level))
            },
            | ValueType::Thunk(body) => self.comp_type_level(depth, body),
            | ValueType::Universe(level) => {
                self.levels.check_level_scope(level)?;
                let bumped = level.succ()?;
                Ok(bumped)
            },
            | ValueType::Lift { inner, target } => {
                let inner_level = self.value_type_level(depth, inner)?;
                self.levels.check_level_scope(target)?;
                self.levels.check_universe_below(&inner_level, target)?;
                Ok(target.clone())
            },
        }
    }

    /// The universe level of a well-formed computation type.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Ok(level)` — `F A` at `A`'s level, `A → C` at the join of
    ///   the domain and codomain levels — when well-formed.
    /// - provides: computation-type formation.
    /// - fails: as [`Self::value_type_level`].
    /// - panics: none.
    ///
    /// # Errors
    /// As [`Self::value_type_level`].
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the arrow's join is pinned by a domain/codomain pair
    ///   at distinct levels.
    /// - witness: `check::tests::arrow_level_is_the_join`
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic matching of a borrowed type node; every binding is a shared reference by intent"
    )]
    fn comp_type_level(
        &self,
        depth: Depth,
        ty: &CompType,
    ) -> Result<Level, KernelError>
    {
        let depth = depth.deepen()?;
        match ty {
            | CompType::Returner(result) => self.value_type_level(depth, result),
            | CompType::Arrow { domain, codomain } => {
                let domain_level = self.value_type_level(depth, domain)?;
                let codomain_level = self.comp_type_level(depth, codomain)?;
                Ok(domain_level.max(&codomain_level))
            },
        }
    }

    /// Synthesize a value's type.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Ok(ty)` when the value has a synthesizable type in `context`
    ///   (variable, constant, unit, literal, pair, thunk, lift).
    /// - provides: the synthesizing half of value checking.
    /// - fails: [`KernelError::UnboundVariable`],
    ///   [`KernelError::UnboundConstant`], [`KernelError::NotInferable`] (an
    ///   injection), the shape/level errors of a lift, and
    ///   [`KernelError::DepthLimitExceeded`].
    /// - panics: none.
    ///
    /// # Errors
    /// As `- fails:` plus anything a sub-derivation surfaces.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — each atom's synthesized type is pinned by a unit
    ///   golden; the L3 residues are the unbound-variable and unbound-constant
    ///   boundaries and the not-inferable injection.
    /// - witness: `check::tests::variable_synthesizes_its_context_type`
    /// - witness: `check::tests::injection_is_not_inferable`
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic matching of a borrowed term node; every binding is a shared reference by intent"
    )]
    fn synth_value(
        &self,
        context: &Context<'_>,
        depth: Depth,
        value: &Value,
    ) -> Result<ValueType, KernelError>
    {
        let depth = depth.deepen()?;
        match value {
            | Value::Variable(index) => context
                .lookup(*index)
                .cloned()
                .ok_or(KernelError::UnboundVariable { index: *index }),
            | Value::Constant(index) => self
                .environment
                .declared_value_type(*index)
                .cloned()
                .ok_or(KernelError::UnboundConstant { index: *index }),
            | Value::Unit => Ok(ValueType::Unit),
            | Value::Literal(literal) => Ok(ValueType::Base(literal.base_type())),
            | Value::Pair(first, second) => {
                let first_type = self.synth_value(context, depth, first)?;
                let second_type = self.synth_value(context, depth, second)?;
                Ok(ValueType::Product(
                    Box::new(first_type),
                    Box::new(second_type),
                ))
            },
            | Value::Thunk(body) => {
                let body_type = self.synth_comp(context, depth, body)?;
                Ok(ValueType::Thunk(Box::new(body_type)))
            },
            | Value::Lift { target, body } => {
                let body_type = self.synth_value(context, depth, body)?;
                let body_level = self.value_type_level(depth, &body_type)?;
                self.levels.check_level_scope(target)?;
                self.levels.check_universe_below(&body_level, target)?;
                Ok(ValueType::Lift {
                    inner: Box::new(body_type),
                    target: target.clone(),
                })
            },
            | Value::Injection(..) => Err(KernelError::NotInferable {
                form: NonInferableForm::Injection,
            }),
        }
    }

    /// Check a value against an expected type.
    ///
    /// # Contract
    /// - requires: `expected` is a well-formed value type.
    /// - ensures: `Ok(())` exactly when the value has type `expected` — the
    ///   checking introductions (injection, pair, thunk) propagate the expected
    ///   type into their sub-terms; every other value synthesizes and converts.
    /// - provides: the checking half of value checking.
    /// - fails: [`KernelError::ValueShapeMismatch`] (an introduction met a
    ///   non-matching expected shape), [`KernelError::ValueTypeMismatch`] (a
    ///   mode-switch conversion failed), or any sub-derivation error.
    /// - panics: none.
    ///
    /// # Errors
    /// As `- fails:`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the propagation arms are pinned by a nested injection
    ///   inside a pair (which only checks, never synthesizes); the mode-switch
    ///   conversion is pinned by a variable of the wrong type.
    /// - witness: `check::tests::injection_checks_against_its_sum`
    /// - witness: `check::tests::pair_propagates_into_a_checking_component`
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic matching of borrowed term and type nodes; every binding is a shared reference by intent"
    )]
    fn check_value(
        &self,
        context: &Context<'_>,
        depth: Depth,
        value: &Value,
        expected: &ValueType,
    ) -> Result<(), KernelError>
    {
        let depth = depth.deepen()?;
        match value {
            | Value::Injection(side, body) => match expected {
                | ValueType::Sum(left, right) => {
                    let summand = match side {
                        | crate::term::Side::Left => left,
                        | crate::term::Side::Right => right,
                    };
                    self.check_value(context, depth, body, summand)
                },
                | _ => Err(KernelError::ValueShapeMismatch {
                    expected: ExpectedValueShape::Sum,
                    actual: Box::new(expected.clone()),
                }),
            },
            | Value::Pair(first, second) => match expected {
                | ValueType::Product(first_type, second_type) => {
                    self.check_value(context, depth, first, first_type)?;
                    self.check_value(context, depth, second, second_type)
                },
                | _ => Err(KernelError::ValueShapeMismatch {
                    expected: ExpectedValueShape::Product,
                    actual: Box::new(expected.clone()),
                }),
            },
            | Value::Thunk(body) => match expected {
                | ValueType::Thunk(codomain) => self.check_comp(context, depth, body, codomain),
                | _ => Err(KernelError::ValueShapeMismatch {
                    expected: ExpectedValueShape::Thunk,
                    actual: Box::new(expected.clone()),
                }),
            },
            | _ => {
                let synthesized = self.synth_value(context, depth, value)?;
                convert_value_type(expected, &synthesized)
            },
        }
    }

    /// Synthesize a computation's type.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: `Ok(ty)` when the computation has a synthesizable type
    ///   (application, force, return, bind, case).
    /// - provides: the synthesizing half of computation checking.
    /// - fails: [`KernelError::ComputationShapeMismatch`] (application of a
    ///   non-function, bind of a non-returner),
    ///   [`KernelError::ValueShapeMismatch`] (force/case of a
    ///   non-thunk/non-sum), [`KernelError::NotInferable`] (a lambda),
    ///   [`KernelError::CaseBranchMismatch`] (inconvertible branches), or a
    ///   sub-derivation error.
    /// - panics: none.
    ///
    /// # Errors
    /// As `- fails:`.
    ///
    /// # Adequacy
    /// - hypothesis: L2/L3 — application/force/bind/case results are pinned by
    ///   corpus goldens; the L3 residues are the eliminator shape boundaries
    ///   and the synth-mode case-branch convertibility, each pinned by a
    ///   negative unit golden.
    /// - witness: `check::tests::application_result_is_the_codomain`
    /// - witness: `check::tests::force_unwraps_a_thunk`
    /// - witness: `check::tests::case_requires_convergent_branches`
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic matching of a borrowed term node; every binding is a shared reference by intent"
    )]
    fn synth_comp(
        &self,
        context: &Context<'_>,
        depth: Depth,
        computation: &Computation,
    ) -> Result<CompType, KernelError>
    {
        let depth = depth.deepen()?;
        match computation {
            | Computation::Application(head, argument) => {
                let head_type = self.synth_comp(context, depth, head)?;
                match take_arrow(head_type) {
                    | Ok((domain, codomain)) => {
                        self.check_value(context, depth, argument, &domain)?;
                        Ok(*codomain)
                    },
                    | Err(other) => Err(KernelError::ComputationShapeMismatch {
                        expected: ExpectedComputationShape::Arrow,
                        actual: Box::new(other),
                    }),
                }
            },
            | Computation::Force(value) => {
                let value_type = self.synth_value(context, depth, value)?;
                match take_thunk(value_type) {
                    | Ok(codomain) => Ok(*codomain),
                    | Err(other) => Err(KernelError::ValueShapeMismatch {
                        expected: ExpectedValueShape::Thunk,
                        actual: Box::new(other),
                    }),
                }
            },
            | Computation::Return(value) => {
                let result_type = self.synth_value(context, depth, value)?;
                Ok(CompType::Returner(Box::new(result_type)))
            },
            | Computation::Bind(bound, body) => {
                let bound_type = self.synth_comp(context, depth, bound)?;
                match take_returner(bound_type) {
                    | Ok(result) => {
                        let extended = Context::Bound {
                            ty: &result,
                            outer: context,
                        };
                        self.synth_comp(&extended, depth, body)
                    },
                    | Err(other) => Err(KernelError::ComputationShapeMismatch {
                        expected: ExpectedComputationShape::Returner,
                        actual: Box::new(other),
                    }),
                }
            },
            | Computation::Case {
                scrutinee,
                on_left,
                on_right,
            } => {
                let scrutinee_type = self.synth_value(context, depth, scrutinee)?;
                match take_sum(scrutinee_type) {
                    | Ok((left, right)) => {
                        let left_context = Context::Bound {
                            ty: &left,
                            outer: context,
                        };
                        let left_type = self.synth_comp(&left_context, depth, on_left)?;
                        let right_context = Context::Bound {
                            ty: &right,
                            outer: context,
                        };
                        let right_type = self.synth_comp(&right_context, depth, on_right)?;
                        match convertible_comp_types(&left_type, &right_type) {
                            | Convertibility::Convertible => Ok(left_type),
                            | Convertibility::Distinct => Err(KernelError::CaseBranchMismatch(
                                Box::new(ComputationTypeMismatch::new(left_type, right_type)),
                            )),
                        }
                    },
                    | Err(other) => Err(KernelError::ValueShapeMismatch {
                        expected: ExpectedValueShape::Sum,
                        actual: Box::new(other),
                    }),
                }
            },
            | Computation::Lambda(_) => Err(KernelError::NotInferable {
                form: NonInferableForm::Lambda,
            }),
        }
    }

    /// Check a computation against an expected type.
    ///
    /// # Contract
    /// - requires: `expected` is a well-formed computation type.
    /// - ensures: `Ok(())` exactly when the computation has type `expected` —
    ///   lambda, return, bind, and case propagate the expected type; every
    ///   other computation synthesizes and converts.
    /// - provides: the checking half of computation checking.
    /// - fails: [`KernelError::ComputationShapeMismatch`] (a lambda/return met
    ///   a non-matching expected shape),
    ///   [`KernelError::ComputationTypeMismatch`] (a mode-switch conversion
    ///   failed), or a sub-derivation error.
    /// - panics: none.
    ///
    /// # Errors
    /// As `- fails:`.
    ///
    /// # Adequacy
    /// - hypothesis: L2/L3 — lambda checking is pinned by the identity corpus
    ///   golden; the propagation of `case`/`bind` expected types is pinned by a
    ///   branch that only checks; the mode-switch conversion by a wrong-typed
    ///   application.
    /// - witness: `check::tests::identity_thunk_checks`
    /// - witness: `check::tests::case_propagates_the_expected_type`
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "ergonomic matching of borrowed term and type nodes; every binding is a shared reference by intent"
    )]
    fn check_comp(
        &self,
        context: &Context<'_>,
        depth: Depth,
        computation: &Computation,
        expected: &CompType,
    ) -> Result<(), KernelError>
    {
        let depth = depth.deepen()?;
        match computation {
            | Computation::Lambda(body) => match expected {
                | CompType::Arrow { domain, codomain } => {
                    let extended = Context::Bound {
                        ty: domain,
                        outer: context,
                    };
                    self.check_comp(&extended, depth, body, codomain)
                },
                | _ => Err(KernelError::ComputationShapeMismatch {
                    expected: ExpectedComputationShape::Arrow,
                    actual: Box::new(expected.clone()),
                }),
            },
            | Computation::Return(value) => match expected {
                | CompType::Returner(result) => self.check_value(context, depth, value, result),
                | _ => Err(KernelError::ComputationShapeMismatch {
                    expected: ExpectedComputationShape::Returner,
                    actual: Box::new(expected.clone()),
                }),
            },
            | Computation::Bind(bound, body) => {
                let bound_type = self.synth_comp(context, depth, bound)?;
                match take_returner(bound_type) {
                    | Ok(result) => {
                        let extended = Context::Bound {
                            ty: &result,
                            outer: context,
                        };
                        self.check_comp(&extended, depth, body, expected)
                    },
                    | Err(other) => Err(KernelError::ComputationShapeMismatch {
                        expected: ExpectedComputationShape::Returner,
                        actual: Box::new(other),
                    }),
                }
            },
            | Computation::Case {
                scrutinee,
                on_left,
                on_right,
            } => {
                let scrutinee_type = self.synth_value(context, depth, scrutinee)?;
                match take_sum(scrutinee_type) {
                    | Ok((left, right)) => {
                        let left_context = Context::Bound {
                            ty: &left,
                            outer: context,
                        };
                        self.check_comp(&left_context, depth, on_left, expected)?;
                        let right_context = Context::Bound {
                            ty: &right,
                            outer: context,
                        };
                        self.check_comp(&right_context, depth, on_right, expected)
                    },
                    | Err(other) => Err(KernelError::ValueShapeMismatch {
                        expected: ExpectedValueShape::Sum,
                        actual: Box::new(other),
                    }),
                }
            },
            | _ => {
                let synthesized = self.synth_comp(context, depth, computation)?;
                convert_comp_type(expected, &synthesized)
            },
        }
    }
}

#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;

    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;

    use super::Checker;
    use super::Context;
    use super::Depth;
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
        LevelContext::admit(LevelParamCount::from(params), alloc::vec::Vec::new()).unwrap()
    }

    /// The universe former at constant level `value`.
    fn universe(value: u64) -> ValueType
    {
        ValueType::Universe(Level::constant(LevelConstant::from(value)))
    }

    #[test]
    fn variable_synthesizes_its_context_type()
    {
        let environment = environment();
        let levels = level_context(0);
        let checker = Checker::new(&environment, &levels);
        let context = Context::Bound {
            ty: &ValueType::Unit,
            outer: &Context::Empty,
        };
        let synthesized = checker
            .synth_value(
                &context,
                Depth::start(),
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
        let checker = Checker::new(&environment, &levels);
        let injection = Value::Injection(Side::Left, Box::new(Value::Unit));
        assert!(
            matches!(
                checker.synth_value(&Context::Empty, Depth::start(), &injection),
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
        let checker = Checker::new(&environment, &levels);
        let sum = ValueType::Sum(
            Box::new(ValueType::Unit),
            Box::new(ValueType::Base(crate::base::BaseType::Integer)),
        );
        let left = Value::Injection(Side::Left, Box::new(Value::Unit));
        assert_eq!(
            Ok(()),
            checker.check_value(&Context::Empty, Depth::start(), &left, &sum),
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
        let checker = Checker::new(&environment, &levels);
        let sum = ValueType::Sum(Box::new(ValueType::Unit), Box::new(ValueType::Unit));
        let product = ValueType::Product(Box::new(sum), Box::new(ValueType::Unit));
        let pair = Value::Pair(
            Box::new(Value::Injection(Side::Right, Box::new(Value::Unit))),
            Box::new(Value::Unit),
        );
        assert_eq!(
            Ok(()),
            checker.check_value(&Context::Empty, Depth::start(), &pair, &product),
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
        let checker = Checker::new(&environment, &levels);
        let arrow = CompType::Arrow {
            domain: Box::new(ValueType::Unit),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        };
        let context = Context::Bound {
            ty: &ValueType::Thunk(Box::new(arrow)),
            outer: &Context::Empty,
        };
        let application = Computation::Application(
            Box::new(Computation::Force(Box::new(Value::Variable(
                DeBruijnIndex::from(0_u32),
            )))),
            Box::new(Value::Unit),
        );
        let synthesized = checker
            .synth_comp(&context, Depth::start(), &application)
            .unwrap();
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
        let checker = Checker::new(&environment, &levels);
        let returner = CompType::Returner(Box::new(ValueType::Unit));
        let context = Context::Bound {
            ty: &ValueType::Thunk(Box::new(returner.clone())),
            outer: &Context::Empty,
        };
        let force = Computation::Force(Box::new(Value::Variable(DeBruijnIndex::from(0_u32))));
        assert_eq!(
            checker
                .synth_comp(&context, Depth::start(), &force)
                .unwrap(),
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
        let checker = Checker::new(&environment, &levels);
        let context = Context::Bound {
            ty: &ValueType::Sum(Box::new(ValueType::Unit), Box::new(ValueType::Unit)),
            outer: &Context::Empty,
        };
        let case = Computation::Case {
            scrutinee: Box::new(Value::Variable(DeBruijnIndex::from(0_u32))),
            on_left: Box::new(Computation::Return(Box::new(Value::Unit))),
            on_right: Box::new(Computation::Return(Box::new(Value::Thunk(Box::new(
                Computation::Return(Box::new(Value::Unit)),
            ))))),
        };
        assert!(
            matches!(
                checker.synth_comp(&context, Depth::start(), &case),
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
        let checker = Checker::new(&environment, &levels);
        let context = Context::Bound {
            ty: &ValueType::Sum(Box::new(ValueType::Unit), Box::new(ValueType::Unit)),
            outer: &Context::Empty,
        };
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
            checker.check_comp(&context, Depth::start(), &case, &expected),
            "check-mode case propagates the expected type into both branches"
        );
    }

    #[test]
    fn universe_forms_one_level_up()
    {
        let environment = environment();
        let levels = level_context(0);
        let checker = Checker::new(&environment, &levels);
        let level = checker
            .value_type_level(Depth::start(), &universe(0))
            .unwrap();
        assert_eq!(
            level,
            Level::constant(LevelConstant::from(1_u64)),
            "U_0 forms at level 1"
        );
    }

    #[test]
    fn lift_requires_a_strictly_higher_target()
    {
        let environment = environment();
        let levels = level_context(0);
        let checker = Checker::new(&environment, &levels);
        // Lift Unit to level 1: Unit is at level 0, 0 < 1, so this is well-formed.
        let lifted = ValueType::Lift {
            inner: Box::new(ValueType::Unit),
            target: Level::constant(LevelConstant::from(1_u64)),
        };
        assert_eq!(
            checker.value_type_level(Depth::start(), &lifted).unwrap(),
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
                checker.value_type_level(Depth::start(), &degenerate),
                Err(KernelError::UniverseViolation(_))
            ),
            "a lift that does not strictly raise the level is rejected"
        );
    }

    #[test]
    fn arrow_level_is_the_join()
    {
        let environment = environment();
        let levels = level_context(0);
        let checker = Checker::new(&environment, &levels);
        // (U_2 → F Unit): domain level 3, codomain level 0, join 3.
        let arrow = CompType::Arrow {
            domain: Box::new(universe(2)),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        };
        assert_eq!(
            checker.comp_type_level(Depth::start(), &arrow).unwrap(),
            Level::constant(LevelConstant::from(3_u64)),
            "the arrow forms at the join of its parts' levels"
        );
    }

    #[test]
    fn depth_budget_rejects_adversarial_nesting()
    {
        // Build a left-nested pair chain deeper than the budget iteratively
        // (no recursion in construction), then confirm the checker rejects it
        // totally rather than overflowing.
        let environment = environment();
        let levels = level_context(0);
        let checker = Checker::new(&environment, &levels);
        let mut value = Value::Unit;
        for _step in 0_u32 .. 4_000_u32 {
            value = Value::Pair(Box::new(value), Box::new(Value::Unit));
        }
        assert_eq!(
            checker.synth_value(&Context::Empty, Depth::start(), &value),
            Err(KernelError::DepthLimitExceeded),
            "a term nested past the budget is rejected, not a stack overflow"
        );
    }
}
