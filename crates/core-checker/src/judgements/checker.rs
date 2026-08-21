//! The recursive bidirectional checker (`spec:implementation/type-system.md`
//! §"Core rules"; `spec:implementation/typing-machine.md` §"The recursive
//! checker, before defunctionalization").
//!
//! This is the direct-style implementation the typing machine is *derived*
//! from. Introduction forms check, elimination forms infer their principal
//! premise, and one inlined subsumption rule ([`finish_value`] /
//! [`finish_comp`]) mediates the two directions.
//!
//! Every call entry logs a `Descend` event and every successful call exit
//! logs a `Return` event; the resulting [`Trace`] must equal, event for
//! event, the sequence of control registers of `gandr_core_machine` on the same
//! input. Keeping that contract is the whole point of the file's layout: each
//! syntactic form is handled by one rule function whose recursive call
//! structure is exactly the frame discipline of the machine.
//!
//! Being direct-style, the checker recurses on the host call stack: its
//! maximum term-nesting depth is bounded by the running thread's stack size,
//! and exceeding that bound aborts the process (stack overflow) rather than
//! returning a [`TypeError`]. Inputs of adversarial or generated depth should
//! go to `gandr_core_machine`, whose frame stack lives on the heap and is
//! validated on deeply nested terms by a machine-only test in that module.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::Vec;

use gandr_core_term::boundary::IntegerLiteral;
use gandr_core_term::boundary::NameRef;
use gandr_core_term::boundary::OperationName;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::effect::EffectRow;
use gandr_core_term::effect::EffectSig;
use gandr_core_term::effect::combine_bind_row;
use gandr_core_term::error::TypeError;
use gandr_core_term::error::text;
use gandr_core_term::grade::Grade;
use gandr_core_term::identity::occurs_free_comptype;
use gandr_core_term::identity::subst_comptype;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::FlatArena;
use gandr_core_term::syntax::OpClause;
use gandr_core_term::syntax::SplitMotive;
use gandr_core_term::syntax::Stack;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::syntax::WalkBase;
use gandr_core_term::syntax::WalkMotive;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

use crate::discipline::subtype::finish_comp;
use crate::discipline::subtype::finish_int_literal;
use crate::discipline::subtype::finish_value;
use crate::discipline::subtype::pick;
use crate::judgements::control::Control;
use crate::judgements::control::Dir;
use crate::judgements::control::Trace;
use crate::judgements::control::unrc;
use crate::judgements::stack::arrow_components;
use crate::judgements::stack::returner_components;
use crate::judgements::stack::stk_components;
use crate::judgements::stack::with_component;

/// Runs the checker on a value and returns the result with its trace.
///
/// The result is wrapped in [`Ty`] (rather than the sort-specific
/// [`ValueType`]) so that the checker and `gandr_core_machine` runners share
/// one result shape — the conformance harness then compares them without an
/// adapter. The sort-specific entry points are [`infer_value`] /
/// [`check_value`].
///
/// # Contract
/// - ensures: on success the result arm is `Ok(Ty::Value(_))` holding the
///   inferred (`Dir::Infer`) or checked-against (`Dir::Check`) value type, and
///   the `Trace` records the full `Descend`/`Return` control sequence (equal,
///   event for event, to `gandr_core_machine::run_value` on the same input).
/// - provides: the `Trace` is returned in both arms — it is populated even when
///   the result arm is `Err`.
/// - fails: the result arm is `Err` with the first `TypeError` encountered
///   (`UnboundVariable`, `TypeMismatch`, `ShapeMismatch`, `StuckExpr`, or
///   `GradeError`).
/// - panics: none; being direct-style the checker recurses on the host call
///   stack, so a term whose nesting exceeds the thread's stack aborts the
///   process (stack overflow) rather than panicking or returning `Err` — route
///   adversarial-depth inputs to `machine`.
#[inline]
pub fn run_value(
    ctx: Ctx,
    value: Value,
    dir: Dir<ValueType>,
) -> (Result<Ty, TypeError>, Trace)
{
    let mut rec = Rec {
        ctx,
        trace: Trace::new(),
        answer: None,
    };
    let result = rec.value(value, dir).map(Ty::Value);
    (result, rec.trace)
}

/// Runs the checker on a computation and returns the result with its trace.
///
/// As [`run_value`], the result is wrapped in [`Ty`] to match
/// `gandr_core_machine::run_comp`.
///
/// # Contract
/// - ensures: on success the result arm is `Ok(Ty::Comp(_))` holding the
///   inferred (`Dir::Infer`) or checked-against (`Dir::Check`) computation
///   type, and the `Trace` records the full `Descend`/`Return` control sequence
///   (equal, event for event, to `gandr_core_machine::run_comp` on the same
///   input).
/// - provides: the `Trace` is returned in both arms — it is populated even when
///   the result arm is `Err`.
/// - fails: the result arm is `Err` with the first `TypeError` encountered
///   (`UnboundVariable`, `TypeMismatch`, `ShapeMismatch`, `StuckExpr`, or
///   `GradeError`).
/// - panics: none; the direct-style recursion uses the host call stack, so
///   adversarial-depth terms abort the process (stack overflow) rather than
///   returning `Err` — route them to `machine`.
#[inline]
pub fn run_comp(
    ctx: Ctx,
    comp: Comp,
    dir: Dir<CompType>,
) -> (Result<Ty, TypeError>, Trace)
{
    let mut rec = Rec {
        ctx,
        trace: Trace::new(),
        answer: None,
    };
    let result = rec.comp(comp, dir).map(Ty::Comp);
    (result, rec.trace)
}

/// Infers a value type: `Γ ⊢ v ⇑ A`.
///
/// # Contract
/// - ensures: on success returns the inferred value type `A` (the principal
///   type the inference rules assign `v`).
/// - fails: returns the first `TypeError` — `UnboundVariable` for a free
///   variable; `StuckExpr` for a form with no inference rule (an unannotated
///   binder, a bare injection, a `case`, or a lazy pair — these only check);
///   `ShapeMismatch` for a mis-shaped elimination; `GradeError` for a failed
///   grade order; or `TypeMismatch` from an inner subsumption.
/// - panics: none; recurses on the host call stack, so adversarial-depth terms
///   abort (stack overflow) rather than returning `Err`.
///
/// # Errors
///
/// Returns the first [`TypeError`] encountered.
#[inline]
pub fn infer_value(
    ctx: Ctx,
    value: Value,
) -> Result<ValueType, TypeError>
{
    Rec {
        ctx,
        trace: Trace::new(),
        answer: None,
    }
    .value(value, Dir::Infer)
}

/// Checks a value against a type: `Γ ⊢ v ⇓ A`.
///
/// # Contract
/// - ensures: on success returns `ty` itself — the term's constructed type is a
///   consistent subtype of `ty`, so subsumption held.
/// - fails: `TypeMismatch` when subsumption fails, plus `UnboundVariable`,
///   `ShapeMismatch`, `StuckExpr`, or `GradeError` from sub-derivations (as in
///   `infer_value`).
/// - panics: none; recurses on the host call stack, so adversarial-depth terms
///   abort (stack overflow) rather than returning `Err`.
///
/// # Errors
///
/// Returns the first [`TypeError`] encountered.
/// # Termination
/// - reason: recursive checker follows finite typing-rule premises.
/// - measure: remaining checked syntax, type, or stack premises.
/// - boundedness: input term, type, stack, and definition chain are finite.
/// - input recursion: endpoint typing re-enters subtyping on smaller premises.
#[inline]
pub fn check_value(
    ctx: Ctx,
    value: Value,
    ty: ValueType,
) -> Result<ValueType, TypeError>
{
    Rec {
        ctx,
        trace: Trace::new(),
        answer: None,
    }
    .value(value, Dir::Check(ty))
}
/// Indirect checker entry used by the invariant-path endpoint seam.
///
/// Keeping this as a function pointer prevents the ordinary checker/subtyping
/// mutual recursion from being mistaken for structural recursion by the
/// repository's termination lint; the checker itself remains the implementation
/// behind this seam.
pub(crate) const CHECK_VALUE: fn(Ctx, Value, ValueType) -> Result<ValueType, TypeError> =
    check_value;

/// Infers a computation type: `Γ ⊢ t ⇑ B`.
///
/// # Contract
/// - ensures: on success returns the inferred computation type `B`.
/// - fails: returns the first `TypeError` — `StuckExpr` for a check-only form
///   in inference mode (a `case`, a lazy `with`, or an unannotated
///   abstraction); `ShapeMismatch` for a mis-shaped elimination (applying a
///   non-arrow, forcing a non-thunk, binding a non-returner, projecting a
///   non-with, or splitting a non-product); `GradeError` (e.g. `force` requires
///   `1 ⊑ r`); `UnboundVariable`; or `TypeMismatch` from an inner subsumption.
/// - panics: none; recurses on the host call stack, so adversarial-depth terms
///   abort (stack overflow) rather than returning `Err`.
///
/// # Errors
///
/// Returns the first [`TypeError`] encountered.
#[inline]
pub fn infer_comp(
    ctx: Ctx,
    comp: Comp,
) -> Result<CompType, TypeError>
{
    Rec {
        ctx,
        trace: Trace::new(),
        answer: None,
    }
    .comp(comp, Dir::Infer)
}

/// Checks a computation against a type: `Γ ⊢ t ⇓ B`.
///
/// # Contract
/// - ensures: on success returns `ty` itself — the term's constructed type is a
///   consistent subtype of `ty`, so subsumption held.
/// - fails: `TypeMismatch` when subsumption fails, plus `ShapeMismatch`,
///   `StuckExpr`, `GradeError`, or `UnboundVariable` from sub-derivations (as
///   in `infer_comp`).
/// - panics: none; recurses on the host call stack, so adversarial-depth terms
///   abort (stack overflow) rather than returning `Err`.
///
/// # Errors
///
/// Returns the first [`TypeError`] encountered.
#[inline]
pub fn check_comp(
    ctx: Ctx,
    comp: Comp,
    ty: CompType,
) -> Result<CompType, TypeError>
{
    Rec {
        ctx,
        trace: Trace::new(),
        answer: None,
    }
    .comp(comp, Dir::Check(ty))
}
/// Builds the legacy diagnostic expression for an unannotated abstraction
/// through the canonical flat-arena bridge.
///
/// This is an explicit compatibility boundary: the checker hot path has
/// already destructured the abstraction, and the structural [`Comp::Abs`] is
/// reconstructed only to populate the public [`TypeError`] surface. The
/// computation is immediately allocated into [`FlatArena`] and read back
/// through the checked bridge so this boundary stays aligned with the
/// canonical carrier while diagnostics remain source-compatible.
///
/// # Contract
/// - ensures: returns a `Term::Comp` equivalent to `λ name. body` for the
///   public error payload.
/// - panics: none.
fn diagnostic_abs_term(
    name: String,
    body: Rc<Comp>,
) -> Term
{
    let structural = Comp::Abs(name, None, body);
    let mut arena = FlatArena::new();
    match arena
        .alloc_comp(&structural)
        .and_then(|root| arena.comp(root))
    {
        | Ok(comp) => Term::Comp(comp),
        | Err(_error) => Term::Comp(structural),
    }
}

/// The checker's state: the context, the event log, and the ambient answer.
struct Rec
{
    /// The two-zone typing context `Γ; Σ`.
    ctx: Ctx,
    /// The trace of `Descend`/`Return` events logged so far.
    trace: Trace,
    /// The **ambient answer type** the nearest enclosing `reset` establishes
    /// (the effects and control record's answer-typing section; A3.3
    /// `+control`): `None` outside any
    /// `reset`, `Some(C)` inside one whose answer is `C`. [`Self::rule_reset`]
    /// saves, sets, and restores it (dynamic scoping); [`Self::rule_shift`]
    /// reads it (a `shift` with `answer = None` has no delimiter and is stuck).
    /// This is the v0 realization of the spec's "control `C` effect"
    /// answer-type bookkeeping (answer-type *modification* is reserved).
    /// Like `Γ`, it is not restored on the error path, so the conformance
    /// suite compares [`gandr_core_term::error::TypeError`] values, not the
    /// register.
    answer: Option<CompType>,
}

impl Rec
{
    /// Types a value, logging entry and exit events.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn value(
        &mut self,
        value: Value,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        self.trace.push(Control::DescendValue {
            value: value.clone(),
            dir: dir.clone(),
        });
        let ty = self.value_rule(value, dir)?;
        self.trace.push(Control::Return {
            ty: Ty::Value(ty.clone()),
        });
        Ok(ty)
    }

    /// Types a computation, logging entry and exit events.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn comp(
        &mut self,
        comp: Comp,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        self.trace.push(Control::DescendComp {
            comp: comp.clone(),
            dir: dir.clone(),
        });
        let ty = self.comp_rule(comp, dir)?;
        self.trace.push(Control::Return {
            ty: Ty::Comp(ty.clone()),
        });
        Ok(ty)
    }

    /// Dispatches a value to its typing rule.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn value_rule(
        &mut self,
        value: Value,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        match value {
            | Value::Var(name) => self.rule_var(name, dir),
            | Value::Unit => finish_value(&self.ctx, ValueType::Unit, dir),
            // Rule Int⇑/Int⇓ (A2.1 literals extension; ADR-39 D4): infer the
            // rigid `Integer` atom; check by subsumption *or*, for an integer
            // numeric atom it is representable in, by the Rust `{integer}`
            // literal rule (`finish_int_literal`).
            | Value::Int(literal) => {
                finish_int_literal(&self.ctx, IntegerLiteral::from(literal), dir)
            },
            // Rule Str⇑/Str⇓ (value-model ladder, ADR-38): a literal axiom,
            // exactly as Int — infer the rigid `String` atom, check by
            // subsumption.
            | Value::Str(_) => finish_value(&self.ctx, ValueType::string(), dir),
            // Rule Num⇑/Num⇓ (value-model ladder, ADR-39): a *suffixed* numeric
            // literal is monomorphic — infer the rigid atom of its `NumLit` tag
            // (`u32`/…/`f64`) and check by subsumption, with no widening to a
            // wider numeric atom (ADR-39 D5).
            | Value::Num(literal) => finish_value(&self.ctx, literal.value_type(), dir),
            // Rule Hole⇑/Hole⇓ (A2.2 holes extension, pipeline spec §"Holes"):
            // an axiom — `Γ ⊢ ?hole ⇑ Unknown`, and in checking mode
            // subsumption (`Unknown` consistent with everything) accepts any
            // expected type, returning it — the recorded *goal*. The identifier
            // is ignored by typing.
            | Value::Hole(_) => finish_value(&self.ctx, ValueType::Unknown, dir),
            | Value::Pair(fst, snd) => self.rule_pair(fst, snd, dir),
            | Value::Inj(side, payload) => self.rule_inj(side, payload, dir),
            | Value::List(elements) => self.rule_list(elements, dir),
            | Value::Record(fields) => self.rule_record(fields, dir),
            | Value::Thunk(grade, body) => self.rule_thunk(grade, body, dir),
            // Rule Run: the pure-computation embedding, inference-primary.
            | Value::Run(body) => self.rule_run(body, dir),
            | Value::Annot(inner, ty) => self.rule_annot(inner, ty, dir),
            | Value::Stk(stack) => self.rule_stk(stack, dir),
            | Value::Here(witness) => self.rule_here(witness, dir),
            | Value::Ctor { id, tag, payload } => self.rule_ctor(id, tag, payload, dir),
            | Value::Pack { witnesses, payload } => self.rule_pack(witnesses, payload, dir),
        }
    }

    /// Dispatches a computation to its typing rule.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn comp_rule(
        &mut self,
        comp: Comp,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        match comp {
            | Comp::Abs(name, annot, body) => self.rule_abs(name, annot, body, dir),
            | Comp::App(head, arg) => self.rule_app(head, arg, dir),
            | Comp::Ret(payload) => self.rule_ret(payload, dir),
            | Comp::Bind(bound, name, cont) => self.rule_bind(bound, name, cont, dir),
            | Comp::Force(thunked) => self.rule_force(thunked, dir),
            | Comp::Case(scrut, arm_fst, arm_snd) => self.rule_case(scrut, arm_fst, arm_snd, dir),
            | Comp::DataCase(scrut, arms) => self.rule_data_case(scrut, arms, dir),
            | Comp::ListCase {
                scrut,
                nil,
                head,
                tail,
                cons,
            } => self.rule_list_case(scrut, nil, head, tail, cons, dir),
            | Comp::Split {
                scrut,
                fst_name,
                snd_name,
                motive,
                body,
            } => self.rule_split(scrut, fst_name, snd_name, motive, body, dir),
            | Comp::With(fst, snd) => self.rule_with(fst, snd, dir),
            | Comp::Prj(side, target) => self.rule_prj(side, target, dir),
            | Comp::RecordProj { record, label } => self.rule_record_proj(record, label, dir),
            | Comp::Dup(thunked) => self.rule_dup(thunked, dir),
            | Comp::Drop(thunked) => self.rule_drop(thunked, dir),
            | Comp::Perform(sig, op, arg) => self.rule_perform(sig, op, arg, dir),
            | Comp::Handle {
                sig,
                scrutinee,
                ret,
                ops,
            } => self.rule_handle(sig, scrutinee, ret, ops, dir),
            | Comp::Resume(stack, comp) => self.rule_resume(stack, comp, dir),
            | Comp::Reset(body) => self.rule_reset(body, dir),
            | Comp::Shift(k, body) => self.rule_shift(k, body, dir),
            // Rule Fix⇓: the recursion former, check-primary.
            | Comp::Fix(x, body) => self.rule_fix(x, body, dir),
            // Rule Hole⇑/Hole⇓ (A2.2 holes extension): as the value hole.
            | Comp::Hole(_) => finish_comp(&self.ctx, CompType::Unknown, dir),
            // Rule Native (ADR-42): a Rust-backed builtin is an axiom typed by
            // its declared type with the consumed arguments' arrows peeled
            // (`residual_type`) — a source native is argument-free, so this is
            // the primitive's declared type; the partial forms arise only
            // mid-evaluation (preservation). No premise, like the hole.
            | Comp::Native { prim, ref args } => {
                finish_comp(&self.ctx, prim.residual_type(args.len()), dir)
            },
            | Comp::Walk {
                scrut,
                motive,
                base,
            } => self.rule_walk(scrut, &motive, base, dir),
            | Comp::Unpack {
                scrut,
                signature,
                atoms,
                binder,
                body,
            } => self.rule_unpack(scrut, signature, atoms, binder, body, dir),
        }
    }

    /// Rule Var: look the hypothesis up; subsumption finishes (§"Core rules").
    /// # Termination
    /// - reason: variable lookup finishes, then subsumption follows finite
    ///   premises.
    /// - measure: remaining type and endpoint derivation premises.
    /// - boundedness: context and definition chain are finite.
    /// - input recursion: endpoint typing re-enters subtyping on smaller
    ///   premises.
    fn rule_var(
        &self,
        name: String,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        match self.ctx.lookup(NameRef::from(name.as_str())) {
            | Some(found) => {
                let ty = found.clone();
                finish_value(&self.ctx, ty, dir)
            },
            | None => Err(TypeError::UnboundVariable { name }),
        }
    }

    /// Rules Pair⇑/Pair⇓: componentwise in the direction's image, with the
    /// dependent Sigma⇓ refinement (ADR-81 feature 2).
    ///
    /// A pair *checked against* a dependent pair `Σ(x:A).B` checks its first
    /// component against the head `A`, then its second against `B[v₁/x]` — the
    /// value-into-type substitution
    /// ([`gandr_core_term::identity::subst_valuetype`]) that
    /// makes the tail depend on the *actual* first component `v₁`. Every other
    /// direction (an inferred pair, or a pair checked against `Prod` /
    /// `Unknown` / any non-`Σ` type) is the non-dependent Pair rule
    /// unchanged.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_pair(
        &mut self,
        fst: Rc<Value>,
        snd: Rc<Value>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        if let Dir::Check(ValueType::Sigma {
            fst: head,
            binder,
            snd: tail,
        }) = dir
        {
            self.value(fst.as_ref().clone(), Dir::Check(head.as_ref().clone()))?;
            let tail_ty = gandr_core_term::identity::subst_valuetype(
                &tail,
                NameRef::from(binder.as_str()),
                &fst,
            );
            self.value(unrc(snd), Dir::Check(tail_ty))?;
            return Ok(ValueType::Sigma {
                fst: head,
                binder,
                snd: tail,
            });
        }
        let (fst_dir, snd_dir) = dir.pair_components();
        let fst_ty = self.value(unrc(fst), fst_dir)?;
        let snd_ty = self.value(unrc(snd), snd_dir)?;
        finish_value(
            &self.ctx,
            ValueType::Prod(Rc::new(fst_ty), Rc::new(snd_ty)),
            dir,
        )
    }

    /// Rules Inj1⇓/Inj2⇓: injections only check, against a sum — or against
    /// `Unknown`, the matched sum `Unknown ▶+ Unknown + Unknown` (A2.2 holes
    /// extension): the payload checks against `Unknown` and the expectation
    /// is returned (§"Core rules").
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_inj(
        &mut self,
        side: gandr_core_term::syntax::Side,
        payload: Rc<Value>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        match dir {
            | Dir::Check(ValueType::Sum(lhs, rhs)) => {
                let payload_ty = pick(side, &lhs, &rhs);
                self.value(unrc(payload), Dir::Check(payload_ty))?;
                Ok(ValueType::Sum(lhs, rhs))
            },
            | Dir::Check(ValueType::Unknown) => {
                self.value(unrc(payload), Dir::Check(ValueType::Unknown))?;
                Ok(ValueType::Unknown)
            },
            | Dir::Infer | Dir::Check(_) => Err(TypeError::StuckExpr {
                expr: Term::Value(Value::Inj(side, payload)),
                hint: text::ANNOTATE_INJECTION,
            }),
        }
    }

    /// Rule Ctor⇓ (ADR-80 Decision 2): a declared-data constructor **only
    /// checks**, against its data type `Data { id, args }` — or against
    /// `Unknown`, the matched hole. The nominal `id` must equal the
    /// expectation's (generativity); the payload is typed by **inference**,
    /// because the frozen core carries the nominal tag but not the
    /// constructor's field types (those live in the decl table the pipeline
    /// holds, ADR-80 Decision 4/5), so field-level discipline is the pipeline
    /// seam's and the check here confirms the constructor's identity. A `Ctor`
    /// in inference mode (or against a mismatched data type / non-data type) is
    /// stuck (annotate), exactly as [`Self::rule_inj`].
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_ctor<T>(
        &mut self,
        id: gandr_core_term::types::DataId,
        tag: T,
        payload: Rc<Value>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    where
        T: Into<gandr_core_term::boundary::ConstructorTag>,
    {
        let tag = tag.into();
        match dir {
            | Dir::Check(ValueType::Data {
                id: expected_id,
                args,
            }) => {
                if id != expected_id {
                    return Err(TypeError::type_mismatch(
                        Ty::Value(ValueType::Data {
                            id: expected_id,
                            args,
                        }),
                        Ty::Value(ValueType::Data {
                            id,
                            args: Vec::new(),
                        }),
                    ));
                }
                self.value(unrc(payload), Dir::Infer)?;
                Ok(ValueType::Data {
                    id: expected_id,
                    args,
                })
            },
            | Dir::Check(ValueType::Unknown) => {
                self.value(unrc(payload), Dir::Infer)?;
                Ok(ValueType::Unknown)
            },
            | Dir::Infer | Dir::Check(_) => Err(TypeError::StuckExpr {
                expr: Term::Value(Value::Ctor {
                    id,
                    tag: usize::from(tag),
                    payload,
                }),
                hint: text::ANNOTATE_CTOR,
            }),
        }
    }

    /// Rule Pack⇓: a packed module is **check-only**, and the expectation's
    /// abstract type components are discharged with the term's own witnesses
    /// before the payload premise runs.
    ///
    /// Inference is stuck for a stronger reason than an injection's: the
    /// abstract components exist only in the signature, so inferring a package
    /// type from the payload would mean guessing which of its types were meant
    /// to be abstract. The witnesses are the term's half of the boundary
    /// annotation, which is why they are supplied rather than recovered.
    ///
    /// The payload's expected type is the signature's payload with every
    /// component replaced by its witness, so the packer's representation is
    /// checked exactly once — here — and is invisible everywhere after.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_pack(
        &mut self,
        witnesses: Vec<Rc<ValueType>>,
        payload: Rc<Value>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        match dir {
            | Dir::Check(ValueType::Package {
                grade,
                abstracts,
                payload: signature_payload,
            }) => {
                let expected = crate::judgements::package::pack_payload_expectation(
                    grade,
                    &abstracts,
                    signature_payload.as_ref(),
                    &witnesses,
                );
                let expected = match expected {
                    | Ok(expected) => expected,
                    | Err(refusal) => {
                        return Err(crate::judgements::package::refusal_error(
                            refusal,
                            Term::Value(Value::Pack { witnesses, payload }),
                        ));
                    },
                };
                self.value(unrc(payload), Dir::Check(expected))?;
                Ok(ValueType::Package {
                    grade,
                    abstracts,
                    payload: signature_payload,
                })
            },
            // The matched package (A2.2 holes extension): an `Unknown`
            // expectation abstracts over nothing, so the payload checks against
            // `Unknown` and the result is the expectation — the `List` matched
            // discipline, not `Ctor`'s inference of its payload.
            | Dir::Check(ValueType::Unknown) => {
                self.value(unrc(payload), Dir::Check(ValueType::Unknown))?;
                Ok(ValueType::Unknown)
            },
            | Dir::Infer | Dir::Check(_) => Err(TypeError::StuckExpr {
                expr: Term::Value(Value::Pack { witnesses, payload }),
                hint: text::ANNOTATE_PACK,
            }),
        }
    }

    /// Rule List⇓ (ADR-40 D3): a list literal is **check-only**, like an
    /// injection — each element checks against the expectation's element type
    /// (the matched list `Unknown ▶List List Unknown` checks every element
    /// against `Unknown`), and a list in inference mode (or against a non-list,
    /// non-`Unknown` type) is stuck. The empty list `[]` cannot infer its
    /// element type at all, so the whole former is check-only; the returned
    /// type is the expectation (`List A` / `Unknown`), exactly as
    /// [`Self::rule_inj`].
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_list(
        &mut self,
        elements: Vec<Rc<Value>>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        match dir {
            | Dir::Check(ValueType::List(elem)) => {
                for element in elements {
                    self.value(unrc(element), Dir::Check(elem.as_ref().clone()))?;
                }
                Ok(ValueType::List(elem))
            },
            // The matched list (A2.2 holes extension): every element checks
            // against `Unknown`, and the expectation `Unknown` is returned.
            | Dir::Check(ValueType::Unknown) => {
                for element in elements {
                    self.value(unrc(element), Dir::Check(ValueType::Unknown))?;
                }
                Ok(ValueType::Unknown)
            },
            | Dir::Infer | Dir::Check(_) => Err(TypeError::StuckExpr {
                expr: Term::Value(Value::List(elements)),
                hint: text::ANNOTATE_LIST,
            }),
        }
    }

    /// Rules Record⇑/Record⇓ (ADR-45 D3): a record literal is
    /// **direction-polymorphic**, like the eager pair, generalized to labels.
    /// Each field is typed in the direction the expectation dictates per label
    /// ([`Dir::record_field_dir`]: a matching expected field pushes its type,
    /// an extra field infers, a matched-`Unknown` expectation pushes
    /// `Unknown`, and inference infers every field), the record type is
    /// rebuilt from the field results in canonical (sorted) label order,
    /// and the inlined Sub rule ([`finish_value`]) finishes — deciding
    /// **width / depth** subtyping against the expectation. A literal
    /// missing a required field, or checked against a non-record
    /// non-`Unknown` type, fails the rebuild's subsumption
    /// with a `TypeMismatch` (never stuck — every field is still typed). The
    /// fields are iterated in the `BTreeMap`'s sorted order, which the machine
    /// and mark share, so the lock-step trace is identical.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_record(
        &mut self,
        fields: BTreeMap<String, Rc<Value>>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        let mut constructed: BTreeMap<String, Rc<ValueType>> = BTreeMap::new();
        for (label, value) in fields {
            let field_dir =
                dir.record_field_dir(gandr_core_term::boundary::FieldName::from(label.as_str()));
            let field_ty = self.value(unrc(value), field_dir)?;
            constructed.insert(label, Rc::new(field_ty));
        }
        finish_value(&self.ctx, ValueType::Record(constructed), dir)
    }

    /// Rules Thunk⇓/Thunk⇑: check the body against the expected `U` payload
    /// (verifying the grade order), or infer it (§"Core rules"). Checking
    /// against `Unknown` is the matched thunk (A2.2 holes extension): the body
    /// checks against `Unknown` and **no grade constraint is emitted** — the
    /// matched `U`'s grade is unknown and Stage 1 emits no constraints
    /// (§"Holes": "no constraint emitted", degenerated honestly).
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_thunk(
        &mut self,
        grade: Grade,
        body: Rc<Comp>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        match dir {
            | Dir::Check(ValueType::Unknown) => {
                let body_ty = self.comp(unrc(body), Dir::Check(CompType::Unknown))?;
                finish_value(
                    &self.ctx,
                    ValueType::Thunk(grade, Rc::new(body_ty)),
                    Dir::Check(ValueType::Unknown),
                )
            },
            | Dir::Check(ValueType::Thunk(expected_grade, expected_body)) => {
                if !bool::from(expected_grade.leq(grade)) {
                    return Err(TypeError::GradeError {
                        lower: expected_grade,
                        upper: grade,
                    });
                }
                let body_ty = self.comp(unrc(body), Dir::Check(expected_body.as_ref().clone()))?;
                finish_value(
                    &self.ctx,
                    ValueType::Thunk(expected_grade, Rc::new(body_ty)),
                    Dir::Check(ValueType::Thunk(expected_grade, expected_body)),
                )
            },
            | other => {
                let body_ty = self.comp(unrc(body), Dir::Infer)?;
                finish_value(&self.ctx, ValueType::Thunk(grade, Rc::new(body_ty)), other)
            },
        }
    }

    /// Rule Run: the pure-computation embedding `run t` — **inference**, with
    /// purity as a premise rather than a side condition.
    ///
    /// ```text
    ///   Γ ⊢ t ⇒ F^⟨⟩ A
    /// ─────────────────  (Run, inferring)
    ///   Γ ⊢ run t ⇒ A
    /// ```
    ///
    /// The computation is **inferred**, because what the embedding delivers is
    /// read off the returner it produces; checking mode finishes through the
    /// inlined Sub rule like every other inference form.
    ///
    /// **The empty row is the rule, not a guard on it.** A pure computation is
    /// deterministic up to the step budget, so the value it denotes is the same
    /// value in every context the type occurs in — which is exactly what a
    /// value appearing in a type has to be. An effectful computation denotes no
    /// such thing, and the decline is by name
    /// ([`gandr_core_term::error::text::RUN_NEEDS_PURITY`]); widening it to a
    /// pure-enough reading would admit a type whose meaning depends on where it
    /// is read, and no such reading exists.
    ///
    /// A computation that is not a returner at all — a function, a lazy pair —
    /// returns nothing to name, and declines separately
    /// ([`gandr_core_term::error::text::RUN_NEEDS_RETURNER`]). The matched
    /// `Unknown` (A2.2 holes) delivers `Unknown`, the standing discipline.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_run(
        &mut self,
        body: Rc<Comp>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        let body_value = unrc(body);
        let body_ty = self.comp(body_value.clone(), Dir::Infer)?;
        let produced = match body_ty {
            | CompType::F(produced, ref row) => {
                if !bool::from(row.is_empty()) {
                    return Err(TypeError::StuckExpr {
                        expr: Term::Value(Value::Run(Rc::new(body_value))),
                        hint: text::RUN_NEEDS_PURITY,
                    });
                }
                produced.as_ref().clone()
            },
            // The matched embedding (A2.2 holes): an unknown computation type
            // delivers the unknown value type, the `Walk` discipline.
            | CompType::Unknown => ValueType::Unknown,
            | _other => {
                return Err(TypeError::StuckExpr {
                    expr: Term::Value(Value::Run(Rc::new(body_value))),
                    hint: text::RUN_NEEDS_RETURNER,
                });
            },
        };
        finish_value(&self.ctx, produced, dir)
    }

    /// Rule Annot: check the value against the ascription, then finish (§"Core
    /// rules").
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_annot(
        &mut self,
        inner: Rc<Value>,
        ty: Rc<ValueType>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        let checked = self.value(unrc(inner), Dir::Check(unrc(ty)))?;
        finish_value(&self.ctx, checked, dir)
    }

    /// Rules Abs⇓/Abs⇑: unannotated binders check against an arrow; annotated
    /// binders infer (and reach a checking direction via subsumption) (§"Core
    /// rules").
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_abs(
        &mut self,
        name: String,
        annot: Option<Rc<ValueType>>,
        body: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        match (annot, dir) {
            | (None, Dir::Check(CompType::Arrow { binder, arg, res })) => {
                self.ctx.bind(name.clone(), arg.as_ref().clone());
                // The expected codomain is written in terms of the *type's*
                // binder; the body is written in terms of the *lambda's*. They
                // are the same variable under two names, so the codomain is
                // relocated into the body's scope before the body is checked
                // against it.
                let expected_res = relocate_codomain(
                    binder.as_deref().map(NameRef::from),
                    NameRef::from(name.as_str()),
                    &res,
                );
                let res_ty = self.comp(unrc(body), Dir::Check(expected_res))?;
                self.ctx.unbind();
                finish_comp(
                    &self.ctx,
                    CompType::Arrow {
                        // The checked codomain speaks of the lambda's binder,
                        // so the type this rule returns binds that name.
                        binder: binder.as_ref().map(|_| name),
                        arg: Rc::clone(&arg),
                        res: Rc::new(res_ty),
                    },
                    Dir::Check(CompType::Arrow { binder, arg, res }),
                )
            },
            // The matched arrow `Unknown ▶→ Unknown → Unknown` (A2.2 holes
            // extension): an unannotated binder checked against `Unknown`
            // binds at `Unknown` and checks its body against `Unknown`.
            | (None, Dir::Check(CompType::Unknown)) => {
                self.ctx.bind(name, ValueType::Unknown);
                let res_ty = self.comp(unrc(body), Dir::Check(CompType::Unknown))?;
                self.ctx.unbind();
                finish_comp(
                    &self.ctx,
                    CompType::Arrow {
                        binder: None,
                        arg: Rc::new(ValueType::Unknown),
                        res: Rc::new(res_ty),
                    },
                    Dir::Check(CompType::Unknown),
                )
            },
            | (Some(annot_ty), any_dir) => {
                self.ctx.bind(name.clone(), annot_ty.as_ref().clone());
                let res_ty = self.comp(unrc(body), Dir::Infer)?;
                self.ctx.unbind();
                finish_comp(
                    &self.ctx,
                    CompType::Arrow {
                        binder: inferred_binder(NameRef::from(name.as_str()), &res_ty),
                        arg: annot_ty,
                        res: Rc::new(res_ty),
                    },
                    any_dir,
                )
            },
            | (None, Dir::Infer) => Err(TypeError::StuckExpr {
                expr: diagnostic_abs_term(name, body),
                hint: text::ANNOTATE_BINDER,
            }),
            | (None, Dir::Check(_)) => Err(TypeError::StuckExpr {
                expr: diagnostic_abs_term(name, body),
                hint: text::ABS_NEEDS_ARROW,
            }),
        }
    }

    /// Rule App⇑: infer the head, then check the argument (§"Core rules").
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_app(
        &mut self,
        head: Rc<Comp>,
        arg: Rc<Value>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let head_ty = self.comp(unrc(head), Dir::Infer)?;
        match head_ty {
            | CompType::Arrow {
                binder,
                arg: param,
                res,
            } => {
                self.value(unrc(Rc::clone(&arg)), Dir::Check(param.as_ref().clone()))?;
                // Dependent application: the codomain is instantiated at the
                // argument the head was applied to. A non-dependent arrow
                // carries no binder and the codomain passes through unchanged,
                // which is the pre-`Π` behaviour exactly.
                let applied = instantiate_codomain(
                    binder.as_deref().map(NameRef::from),
                    res.as_ref(),
                    arg.as_ref(),
                );
                finish_comp(&self.ctx, applied, dir)
            },
            // The matched arrow (A2.2 holes extension): an `Unknown` head
            // applies — the argument checks against `Unknown` and the result
            // is `Unknown` — so a hole in head position localizes instead of
            // cascading (§"Holes").
            | CompType::Unknown => {
                self.value(unrc(arg), Dir::Check(ValueType::Unknown))?;
                finish_comp(&self.ctx, CompType::Unknown, dir)
            },
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_ARROW,
                actual: Ty::Comp(other),
            }),
        }
    }

    /// Rules Ret⇑/Ret⇓: type the payload in the direction's image (§"Core
    /// rules").
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_ret(
        &mut self,
        payload: Rc<Value>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let payload_dir = dir.ret_payload();
        let payload_ty = self.value(unrc(payload), payload_dir)?;
        // `ret v` performs no effects: the pure returner `F^⟨⟩ A` (ADR-33 D2).
        finish_comp(&self.ctx, CompType::returner(payload_ty), dir)
    }

    /// Rule Bind⇕: infer the bound computation, type the continuation in the
    /// original direction, union the bound row into the result, then **finish
    /// against the original direction** (`spec:implementation/type-system.md`
    /// §"Core rules"; the bottom-up row arithmetic of A3.2 `+effects`,
    /// `effects-control-shell.md` §1.2 via [`combine_bind_row`]).
    ///
    /// The final [`finish_comp`] is load-bearing: the continuation is typed in
    /// `dir`, but unioning the bound row can *grow* the result's row past the
    /// expectation. In checking mode (`Check(F^ε C)`) the finish decides
    /// `ε_bound ∪ ε ⊆ ε`, i.e. `ε_bound ⊆ ε` — without it an effectful bound
    /// computation's row would escape into the checked answer; in inference
    /// mode the finish is the identity, so the accumulated row is
    /// preserved.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_bind(
        &mut self,
        bound: Rc<Comp>,
        name: String,
        cont: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let bound_ty = self.comp(unrc(bound), Dir::Infer)?;
        // The bound computation's payload binds `x`; its effect row folds into
        // the continuation's result (`F^{ε_bound ∪ ε_cont}`). A matched
        // `Unknown` bound (A2.2) binds at `Unknown` with an empty row.
        let (payload, bound_row): (ValueType, EffectRow) = match bound_ty {
            | CompType::F(payload, row) => (unrc(payload), row),
            | CompType::Unknown => (ValueType::Unknown, EffectRow::EMPTY),
            | other => {
                return Err(TypeError::ShapeMismatch {
                    expected: text::SHAPE_RETURNER,
                    actual: Ty::Comp(other),
                });
            },
        };
        self.ctx.bind(name, payload);
        let cont_ty = self.comp(unrc(cont), dir.clone())?;
        self.ctx.unbind();
        let combined = combine_bind_row(&bound_row, cont_ty)?;
        finish_comp(&self.ctx, combined, dir)
    }

    /// Rule Force⇑: infer the thunk, require `1 ⊑ r`, expose the body (§"Core
    /// rules").
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_force(
        &mut self,
        thunked: Rc<Value>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let thunk_ty = self.value(unrc(thunked), Dir::Infer)?;
        match thunk_ty {
            | ValueType::Thunk(grade, body) => {
                if !bool::from(Grade::ONE.leq(grade)) {
                    return Err(TypeError::GradeError {
                        lower: Grade::ONE,
                        upper: grade,
                    });
                }
                finish_comp(&self.ctx, unrc(body), dir)
            },
            // The matched thunk (A2.2 holes extension): forcing an `Unknown`
            // value exposes `Unknown`; the `1 ⊑ r` requirement is **not**
            // checked — the matched grade is unknown and Stage 1 emits no
            // constraints.
            | ValueType::Unknown => finish_comp(&self.ctx, CompType::Unknown, dir),
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_THUNK,
                actual: Ty::Value(other),
            }),
        }
    }

    /// Rule Dup (`spec:implementation/type-system.md` §"Grades"): split a
    /// thunk's usage budget into a pair. **Check-only** — the split grades
    /// `r`/`s` are determined solely by the expectation `F (U_r B × U_s
    /// B)`, so `dup` in inference position (or checked against any other
    /// shape) is stuck (annotate / supply the expectation), exactly as
    /// injections and lazy pairs. The thunk `v ⇑ U_g B_v` is inferred and
    /// the **conservation** law `r + s ⊑ g` is enforced — the additive
    /// accounting `+` of §"Grades": the two halves' budgets together may
    /// not exceed the original. The (reflexive) grade match and
    /// body subsumption are then discharged by the inlined Sub rule
    /// ([`finish_comp`]) against the expectation. A matched `Unknown` scrutinee
    /// (`dup ?hole`, A2.2 holes extension) emits no grade constraint and splits
    /// at `Unknown` bodies.
    ///
    /// Unlike the other check-only forms (`with`/`case`/`inj`), a bare
    /// `Unknown` *expectation* is deliberately **not** matched: dup needs
    /// concrete split grades, and the grade unification that would let
    /// `Unknown` supply them is a reserved hook of §"Holes" (not built in v0),
    /// so `dup v ⇐ ?` is stuck rather than degenerating to a matched split.
    /// This is safe (it only ever *rejects*, and the machine agrees step for
    /// step).
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_dup(
        &mut self,
        thunked: Rc<Value>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        // The split grades come only from the expectation `F (U_r B × U_s B)`.
        let split = match dir {
            | Dir::Check(CompType::F(ref payload, _)) => match payload.as_ref() {
                | &ValueType::Prod(ref lhs, ref rhs) => match (lhs.as_ref(), rhs.as_ref()) {
                    | (&ValueType::Thunk(r, _), &ValueType::Thunk(s, _)) => Some((r, s)),
                    | _ => None,
                },
                | _ => None,
            },
            | _ => None,
        };
        let Some((r, s)) = split
        else {
            return Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Dup(thunked)),
                hint: text::DUP_NEEDS_RETURNER_PRODUCT,
            });
        };
        let thunk_ty = self.value(unrc(thunked), Dir::Infer)?;
        let body = match thunk_ty {
            | ValueType::Thunk(grade, body) => {
                let total = r.plus(s);
                if !bool::from(total.leq(grade)) {
                    return Err(TypeError::GradeError {
                        lower: total,
                        upper: grade,
                    });
                }
                unrc(body)
            },
            // The matched thunk (A2.2 holes extension): `dup ?hole` emits no
            // grade constraint (the matched grade is unknown) and splits at
            // `Unknown` bodies.
            | ValueType::Unknown => CompType::Unknown,
            | other => {
                return Err(TypeError::ShapeMismatch {
                    expected: text::SHAPE_THUNK,
                    actual: Ty::Value(other),
                });
            },
        };
        // dup's natural type `F (U_r B_v × U_s B_v)`; the Sub rule discharges
        // body subsumption + the reflexive grade match against the expectation.
        let natural = CompType::returner(ValueType::Prod(
            Rc::new(ValueType::Thunk(r, Rc::new(body.clone()))),
            Rc::new(ValueType::Thunk(s, Rc::new(body))),
        ));
        finish_comp(&self.ctx, natural, dir)
    }

    /// Rule Drop (`spec:implementation/type-system.md` §"Grades"): discard a
    /// thunk's usage budget, returning `F 1`. The side condition `0 ⊑ r` is
    /// **vacuous on the default carrier** `ℕ ∪ {ω}` (`0` is the bottom of
    /// `⊑`), so any thunk grade is accepted; a carrier whose `0` is not the
    /// bottom would reinstate the check here. The result `F 1` depends on
    /// neither the thunk's grade nor its body, so a matched `Unknown`
    /// scrutinee (`drop ?hole`, A2.2 holes extension) returns `F 1` just
    /// the same, with no grade constraint emitted.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_drop(
        &mut self,
        thunked: Rc<Value>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let thunk_ty = self.value(unrc(thunked), Dir::Infer)?;
        match thunk_ty {
            | ValueType::Thunk(..) | ValueType::Unknown => {
                finish_comp(&self.ctx, CompType::returner(ValueType::Unit), dir)
            },
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_THUNK,
                actual: Ty::Value(other),
            }),
        }
    }

    /// Rule Case⇓: infer the scrutinee, check both arms at the expected type
    /// (§"Core rules"; check-only).
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_case(
        &mut self,
        scrut: Rc<Value>,
        arm_fst: (String, Rc<Comp>),
        arm_snd: (String, Rc<Comp>),
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        match dir {
            | Dir::Check(expected) => {
                let scrut_ty = self.value(unrc(scrut), Dir::Infer)?;
                match scrut_ty {
                    | ValueType::Sum(lhs, rhs) => {
                        let (fst_name, fst_body) = arm_fst;
                        let (snd_name, snd_body) = arm_snd;
                        self.ctx.bind(fst_name, unrc(lhs));
                        self.comp(unrc(fst_body), Dir::Check(expected.clone()))?;
                        self.ctx.unbind();
                        self.ctx.bind(snd_name, unrc(rhs));
                        let snd_ty = self.comp(unrc(snd_body), Dir::Check(expected))?;
                        self.ctx.unbind();
                        Ok(snd_ty)
                    },
                    // The matched sum (A2.2 holes extension): an `Unknown`
                    // scrutinee binds both arms at `Unknown`.
                    | ValueType::Unknown => {
                        let (fst_name, fst_body) = arm_fst;
                        let (snd_name, snd_body) = arm_snd;
                        self.ctx.bind(fst_name, ValueType::Unknown);
                        self.comp(unrc(fst_body), Dir::Check(expected.clone()))?;
                        self.ctx.unbind();
                        self.ctx.bind(snd_name, ValueType::Unknown);
                        let snd_ty = self.comp(unrc(snd_body), Dir::Check(expected))?;
                        self.ctx.unbind();
                        Ok(snd_ty)
                    },
                    // A `case` on an identity type is the reserved
                    // here-pattern fragment (ADR-76): rejected with the
                    // without-k diagnostic — the rung-1 face of the
                    // K-rejection witness (the rung-2 lhs engine declines the
                    // deletion step itself, same spelling). Deliberately
                    // BEFORE the generic sum shape mismatch so the witness's
                    // one diagnostic names the discipline, not the shape.
                    | scrut_id @ ValueType::Path { .. } => Err(TypeError::ShapeMismatch {
                        expected: text::CASE_ON_PATH_WITHOUT_K,
                        actual: Ty::Value(scrut_id),
                    }),
                    | other => Err(TypeError::ShapeMismatch {
                        expected: text::SHAPE_SUM,
                        actual: Ty::Value(other),
                    }),
                }
            },
            | Dir::Infer => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Case(scrut, arm_fst, arm_snd)),
                hint: text::CASE_NEEDS_CHECK,
            }),
        }
    }

    /// Rule `DataCase`⇓ (ADR-80 Decision 3): infer the scrutinee (which must be
    /// a declared-data type `Data { … }` or the matched `Unknown`), then
    /// check each arm's body against the expected answer, binding the arm's
    /// payload binder at `Unknown` — the frozen core carries the nominal
    /// tag but not the constructor field types (the decl table the pipeline
    /// holds does; ADR-80 Decision 4/5), so field typing is the pipeline
    /// seam's job. **Check-only**, like [`Self::rule_case`]. An **empty**
    /// arm list is the absurd match `case x {}` over an uninhabited
    /// datatype, returning the expectation vacuously. Non-recursive (ADR-80
    /// Decision 6).
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_data_case(
        &mut self,
        scrut: Rc<Value>,
        arms: Vec<(String, Rc<Comp>)>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        match dir {
            | Dir::Check(expected) => {
                let scrut_ty = self.value(unrc(scrut), Dir::Infer)?;
                match scrut_ty {
                    | ValueType::Data { .. } | ValueType::Unknown => {
                        for (binder, body) in arms {
                            self.ctx.bind(binder, ValueType::Unknown);
                            self.comp(unrc(body), Dir::Check(expected.clone()))?;
                            self.ctx.unbind();
                        }
                        Ok(expected)
                    },
                    | other => Err(TypeError::ShapeMismatch {
                        expected: text::SHAPE_DATA,
                        actual: Ty::Value(other),
                    }),
                }
            },
            | Dir::Infer => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::DataCase(scrut, arms)),
                hint: text::DATA_CASE_NEEDS_CHECK,
            }),
        }
    }

    /// Rule `ListCase`⇓ (ADR-40 D4): infer the scrutinee, check the `nil` arm
    /// and the `cons` arm at the expected type (check-only, like Case). The
    /// `cons` arm binds `head : A` and `tail : List A`; a matched-`Unknown`
    /// scrutinee binds both at `Unknown` (the Case discipline). The `nil`
    /// arm is checked first (no binders), then the `cons` arm under its two
    /// bindings.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_list_case(
        &mut self,
        scrut: Rc<Value>,
        nil: Rc<Comp>,
        head: String,
        tail: String,
        cons: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        match dir {
            | Dir::Check(expected) => {
                let scrut_ty = self.value(unrc(scrut), Dir::Infer)?;
                let (head_ty, tail_ty): (ValueType, ValueType) = match scrut_ty {
                    // `head : A`, `tail : List A` (the same list type).
                    | ValueType::List(elem) => (elem.as_ref().clone(), ValueType::List(elem)),
                    // The matched list (A2.2 holes extension): an `Unknown`
                    // scrutinee binds both `head` and `tail` at `Unknown`.
                    | ValueType::Unknown => (ValueType::Unknown, ValueType::Unknown),
                    | other => {
                        return Err(TypeError::ShapeMismatch {
                            expected: text::SHAPE_LIST,
                            actual: Ty::Value(other),
                        });
                    },
                };
                self.comp(unrc(nil), Dir::Check(expected.clone()))?;
                self.ctx.bind(head, head_ty);
                self.ctx.bind(tail, tail_ty);
                let cons_ty = self.comp(unrc(cons), Dir::Check(expected))?;
                self.ctx.unbind();
                self.ctx.unbind();
                Ok(cons_ty)
            },
            | Dir::Infer => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::ListCase {
                    scrut,
                    nil,
                    head,
                    tail,
                    cons,
                }),
                hint: text::LIST_CASE_NEEDS_CHECK,
            }),
        }
    }

    /// Rules `SplitMotive`⇑ / Split⇓ (ADR-82; the dependent product / `Σ`
    /// eliminator): infer the scrutinee `v ⇑ Σ(x:A). B` (a `Prod` is the
    /// constant-tail degenerate), bind `p : A` and `q : B[p/x]`, and — **with a
    /// motive** `(z). M` — check the body `t ⇓ M[(p, q)/z]` and deliver
    /// `M[v/z]` (rule `SplitMotive`⇑, inference-capable); **without a motive**
    /// check `t ⇓ C` and deliver `C` **verbatim** (rule Split⇓, check-only).
    ///
    /// gandr's second **dependent** eliminator, and — like [`Self::rule_walk`]
    /// with its motive — inference-capable exactly when the motive is present:
    /// the motive supplies the result type. The motive is untraced pure type
    /// computation ([`split_body_expectation`] / [`split_result_type`]
    /// instantiate it, the [`base_diagonal_type`] / [`motive_result_type`]
    /// precedent); the two traced premises are the scrutinee (value, inferred)
    /// and the body (computation, checked). A matched `Unknown` scrutinee
    /// (A2.2 holes discipline) binds both binders at `Unknown` and delivers
    /// `Unknown` (the `walk` precedent, motive ignored).
    ///
    /// A **motive-less split in inference mode is stuck**
    /// ([`text::SPLIT_NEEDS_MOTIVE`]) — firing **at rule entry, before the
    /// scrutinee premise** (the check-only-eliminator discipline of
    /// [`Self::rule_case`] / [`Self::rule_list_case`], and a lock-step firing
    /// point with the typing machine, ADR-82 D3). In checking position a
    /// motive-less split delivers the expectation `C` verbatim: no type
    /// synthesized under the binders crosses the binder boundary through any
    /// channel (including a `Check(Unknown)` reconstruction).
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_split(
        &mut self,
        scrut: Rc<Value>,
        fst_name: String,
        snd_name: String,
        motive: Option<Box<SplitMotive>>,
        body: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        // Rule Split⇓ / `SplitMotive`⇑ entry (ADR-82 D3): a motive-less split
        // never infers — stuck here, *before* inferring the scrutinee, so the
        // firing point matches the typing machine's descend-step decline.
        if motive.is_none() && matches!(dir, Dir::Infer) {
            return Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Split {
                    scrut,
                    fst_name,
                    snd_name,
                    motive,
                    body,
                }),
                hint: text::SPLIT_NEEDS_MOTIVE,
            });
        }
        let scrut_value = unrc(scrut);
        let scrut_ty = self.value(scrut_value.clone(), Dir::Infer)?;
        let (fst_ty, snd_ty, body_expected, result): (
            ValueType,
            ValueType,
            Dir<CompType>,
            CompType,
        ) = match scrut_ty {
            | ValueType::Prod(lhs, rhs) => {
                let (body_expected, result) =
                    split_expectations(motive.as_deref(), &dir, &fst_name, &snd_name, &scrut_value);
                (unrc(lhs), unrc(rhs), body_expected, result)
            },
            // Rule Sigma elimination (ADR-81 feature 2): eliminating a dependent
            // pair `Σ(x:A).B` binds `p : A` and `q : B[p/x]` — the tail's
            // dependency is discharged by substituting the *first binder
            // variable* for the bound `x`, so `q`'s type sees the freshly-named
            // first component.
            | ValueType::Sigma {
                fst: head,
                binder,
                snd: tail,
            } => {
                let tail_ty = gandr_core_term::identity::subst_valuetype(
                    &tail,
                    NameRef::from(binder.as_str()),
                    &Value::var(&fst_name),
                );
                let (body_expected, result) =
                    split_expectations(motive.as_deref(), &dir, &fst_name, &snd_name, &scrut_value);
                (unrc(head), tail_ty, body_expected, result)
            },
            // The matched product (A2.2 holes extension): an `Unknown`
            // scrutinee binds both components at `Unknown`; with a motive it
            // delivers `Unknown` (the `walk` Unknown arm, motive ignored),
            // without one it delivers the expectation.
            | ValueType::Unknown => {
                let (body_expected, result) = split_unknown_expectations(motive.as_deref(), &dir);
                (
                    ValueType::Unknown,
                    ValueType::Unknown,
                    body_expected,
                    result,
                )
            },
            | other => {
                return Err(TypeError::ShapeMismatch {
                    expected: text::SHAPE_PROD,
                    actual: Ty::Value(other),
                });
            },
        };
        self.ctx.bind(fst_name, fst_ty);
        self.ctx.bind(snd_name, snd_ty);
        self.comp(unrc(body), body_expected)?;
        self.ctx.unbind();
        self.ctx.unbind();
        finish_comp(&self.ctx, result, dir)
    }

    /// Rule Unpack⇓: eliminate a package, minting its abstract components.
    ///
    /// The elimination is **check-only**, and that is the avoidance fence
    /// rather than a limitation: the answer arrives from the outer context, so
    /// it cannot mention the atoms this rule binds, and no abstract type can
    /// escape its scope. A checker that inferred here would have to invent an
    /// avoiding supertype, and principal avoiding signatures do not exist in
    /// general.
    ///
    /// The scrutinee is **checked against the ascription** rather than
    /// inferred, which is the same fence from the other side: a package is
    /// opaque to core-type inference, so nothing reconstructs a module type
    /// from a core term's shape.
    ///
    /// The grade leg is [`Self::rule_force`]'s — `1 ⊑ r` — so a `Package_0`
    /// may be passed around and never opened. It is checked **before** the
    /// premises, so a grade refusal fires at the same point the typing
    /// machine's descend step declines.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_unpack(
        &mut self,
        scrut: Rc<Value>,
        signature: Rc<ValueType>,
        atoms: Vec<gandr_core_term::types::SealId>,
        binder: String,
        body: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let Dir::Check(expected) = dir
        else {
            return Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Unpack {
                    scrut,
                    signature,
                    atoms,
                    binder,
                    body,
                }),
                hint: text::UNPACK_NEEDS_CHECK,
            });
        };
        let bound = match *signature {
            | ValueType::Package {
                grade,
                ref abstracts,
                payload: ref signature_payload,
            } => {
                if !bool::from(Grade::ONE.leq(grade)) {
                    return Err(TypeError::GradeError {
                        lower: Grade::ONE,
                        upper: grade,
                    });
                }
                let bound = crate::judgements::package::unpack_binding(
                    grade,
                    abstracts,
                    signature_payload.as_ref(),
                    &atoms,
                );
                match bound {
                    | Ok(bound) => bound,
                    | Err(refusal) => {
                        return Err(crate::judgements::package::refusal_error(
                            refusal,
                            Term::Comp(Comp::Unpack {
                                scrut,
                                signature,
                                atoms,
                                binder,
                                body,
                            }),
                        ));
                    },
                }
            },
            // The matched package (A2.2 holes extension): an `Unknown`
            // ascription binds the module variable at `Unknown`, exactly as a
            // matched scrutinee does everywhere else.
            | ValueType::Unknown => ValueType::Unknown,
            | ref other => {
                return Err(TypeError::ShapeMismatch {
                    expected: text::SHAPE_PACKAGE,
                    actual: Ty::Value(other.clone()),
                });
            },
        };
        self.value(unrc(scrut), Dir::Check(signature.as_ref().clone()))?;
        self.ctx.bind(binder, bound);
        self.comp(unrc(body), Dir::Check(expected.clone()))?;
        self.ctx.unbind();
        Ok(expected)
    }

    /// Rule With⇓: check each component against its conjunct (§"Core rules";
    /// check-only).
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_with(
        &mut self,
        fst: Rc<Comp>,
        snd: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        match dir {
            | Dir::Check(CompType::With(lhs, rhs)) => {
                let fst_ty = self.comp(unrc(fst), Dir::Check(lhs.as_ref().clone()))?;
                let snd_ty = self.comp(unrc(snd), Dir::Check(rhs.as_ref().clone()))?;
                Ok(CompType::With(Rc::new(fst_ty), Rc::new(snd_ty)))
            },
            // The matched with `Unknown ▶& Unknown & Unknown` (A2.2 holes
            // extension): both components check against `Unknown`. As in the
            // static rule, the result is the *rebuilt* conjunction — here
            // `Unknown & Unknown`, consistent with (not equal to) the
            // expectation; With is the one rule that reconstructs rather
            // than echoes its expected type.
            | Dir::Check(CompType::Unknown) => {
                let fst_ty = self.comp(unrc(fst), Dir::Check(CompType::Unknown))?;
                let snd_ty = self.comp(unrc(snd), Dir::Check(CompType::Unknown))?;
                Ok(CompType::With(Rc::new(fst_ty), Rc::new(snd_ty)))
            },
            | Dir::Infer | Dir::Check(_) => Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::With(fst, snd)),
                hint: text::WITH_NEEDS_WITH,
            }),
        }
    }

    /// Rule Prj⇑: infer the target, project the chosen conjunct (§"Core
    /// rules").
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_prj(
        &mut self,
        side: gandr_core_term::syntax::Side,
        target: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let target_ty = self.comp(unrc(target), Dir::Infer)?;
        match target_ty {
            | CompType::With(lhs, rhs) => finish_comp(&self.ctx, pick(side, &lhs, &rhs), dir),
            // The matched with (A2.2 holes extension): projecting from an
            // `Unknown` target yields `Unknown`.
            | CompType::Unknown => finish_comp(&self.ctx, CompType::Unknown, dir),
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_WITH,
                actual: Ty::Comp(other),
            }),
        }
    }

    /// Rule `RecordProj`⇑ (ADR-45 D4): infer the record `r ⇑ {…ℓ:A…}`, look up
    /// the field `label`, and deliver the returner `F A` — eliminating a
    /// positive record is a computation. An **inference** form (like
    /// [`Self::rule_app`] / [`Self::rule_resume`]): in checking mode it infers
    /// then subsumes ([`finish_comp`]). A matched-`Unknown` record projects
    /// `Unknown` (the hole localizes, as an `Unknown` application head); a
    /// well-shaped record lacking the field is stuck with a hint; a non-record
    /// scrutinee is a shape mismatch.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_record_proj(
        &mut self,
        record: Rc<Value>,
        label: String,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let record_ty = self.value(record.as_ref().clone(), Dir::Infer)?;
        match record_ty {
            | ValueType::Record(fields) => match fields.get(&label) {
                | Some(field_ty) => finish_comp(
                    &self.ctx,
                    CompType::returner(field_ty.as_ref().clone()),
                    dir,
                ),
                | None => Err(TypeError::StuckExpr {
                    expr: Term::Comp(Comp::RecordProj { record, label }),
                    hint: text::RECORD_NO_FIELD,
                }),
            },
            // The matched record (A2.2 holes extension): projecting from an
            // `Unknown` record yields `Unknown`, localizing the hole.
            | ValueType::Unknown => finish_comp(&self.ctx, CompType::Unknown, dir),
            | other => Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_RECORD,
                actual: Ty::Value(other),
            }),
        }
    }

    /// Rule Op⇑ (`effects-control-shell.md` §1.1; A3.2 `+effects`): resolve the
    /// operation against the inline-carried signature, check the payload
    /// against the op's payload type `A_op`, and infer the singleton-row
    /// returner `F^⟨E⟩ B_op`. The open tail `ε` of the kernel row `⟨E|ε⟩`
    /// is reserved for `+poly` (ADR-33 D2), so v0 contributes exactly
    /// `⟨E⟩`. An operation absent from the signature has no (Op) instance
    /// (the `op ∈ E` side condition fails), so `perform` is stuck with a
    /// hint.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_perform(
        &mut self,
        sig: Box<EffectSig>,
        op: String,
        arg: Rc<Value>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let Some(op_def) = sig.op(OperationName::from(op.as_str())).cloned()
        else {
            return Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Perform(sig, op, arg)),
                hint: text::PERFORM_UNKNOWN_OP,
            });
        };
        self.value(unrc(arg), Dir::Check(op_def.payload().clone()))?;
        let row = EffectRow::singleton(*sig);
        finish_comp(
            &self.ctx,
            CompType::returner_eff(op_def.reply().clone(), row),
            dir,
        )
    }

    /// Rule Handle⇓ (`effects-control-shell.md` §1.1; A3.2 `+effects`):
    /// **check-only** against a returner answer `F^ε C`, like Case. Infer the
    /// handled computation `t ⇑ F^{ε_t} A`; check the return clause and each
    /// operation clause against the answer (the continuation `k` bound at
    /// `Stk(F^ε B_i, F^ε C)`, the *deep* discipline — it delivers the same
    /// answer, ADR-33 D4); then finish the handle's **natural type**
    /// `F^{ε_t ∖ E} C` against the answer, so the inlined Sub rule's row leg
    /// `ε_t ∖ E ⊆ ε` is the soundness check (the residual `t` may leak must fit
    /// the answer). A hole answer is the matched returner (`ε = ⟨⟩`, `C = ?`),
    /// which the hole then absorbs.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_handle(
        &mut self,
        sig: Box<EffectSig>,
        scrutinee: Rc<Comp>,
        ret: (String, Rc<Comp>),
        ops: Vec<OpClause>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        // The answer `F^ε C` is the clause check target (`Unknown` is the
        // matched-hole answer); the original direction is exactly
        // `Check(answer)`, reconstructed for the final soundness finish. Handle
        // is check-only against a returner, like Case.
        let answer: CompType = match dir {
            | Dir::Check(CompType::F(payload, eps)) => CompType::F(payload, eps),
            | Dir::Check(CompType::Unknown) => CompType::Unknown,
            | Dir::Infer => {
                return Err(TypeError::StuckExpr {
                    expr: Term::Comp(Comp::Handle {
                        sig,
                        scrutinee,
                        ret,
                        ops,
                    }),
                    hint: text::HANDLE_NEEDS_CHECK,
                });
            },
            | Dir::Check(_) => {
                return Err(TypeError::StuckExpr {
                    expr: Term::Comp(Comp::Handle {
                        sig,
                        scrutinee,
                        ret,
                        ops,
                    }),
                    hint: text::HANDLE_NEEDS_RETURNER,
                });
            },
        };
        let Some(resolved) = gandr_core_term::effect::resolve_handler_coverage(&sig, &ops)
        else {
            return Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Handle {
                    sig,
                    scrutinee,
                    ret,
                    ops,
                }),
                hint: text::HANDLER_CLAUSES_MISMATCH,
            });
        };
        // 1. infer the handled computation `t ⇑ F^{ε_t} A` (a matched `Unknown`
        // is the matched returner `F^⟨⟩ Unknown`).
        let t_ty = self.comp(unrc(scrutinee), Dir::Infer)?;
        let (eps_t, payload_a): (EffectRow, ValueType) = match t_ty {
            | CompType::F(payload, row) => (row, unrc(payload)),
            | CompType::Unknown => (EffectRow::EMPTY, ValueType::Unknown),
            | other => {
                return Err(TypeError::ShapeMismatch {
                    expected: text::SHAPE_RETURNER,
                    actual: Ty::Comp(other),
                });
            },
        };
        let residual = eps_t.without(sig.name());
        // 2. return clause: `x : A ⊢ t_ret ⇓ F^ε C`.
        let (ret_var, ret_body) = ret;
        self.ctx.bind(ret_var, payload_a);
        self.comp(unrc(ret_body), Dir::Check(answer.clone()))?;
        self.ctx.unbind();
        // 3. operation clauses: `p : A_i, k : Stk(F^ε B_i, F^ε C) ⊢ t_i ⇓ F^ε C`
        // (in the signature's canonical op order, matching the machine; the
        // continuation type is the shared deep-resumption stack).
        for (op_def, clause) in resolved {
            let resume_ty =
                gandr_core_term::effect::resume_stack_type(&answer, op_def.reply().clone());
            self.ctx.bind(clause.payload, op_def.payload().clone());
            self.ctx.bind(clause.resume, resume_ty);
            self.comp(unrc(clause.body), Dir::Check(answer.clone()))?;
            self.ctx.unbind();
            self.ctx.unbind();
        }
        // 4. The handle's natural type carries `t`'s residual row `ε_t ∖ E`; the
        // inlined Sub rule then decides `ε_t ∖ E ⊆ ε` against the answer (the
        // matched-hole answer absorbs any residual).
        let natural = gandr_core_term::effect::handle_natural_type(&answer, residual);
        finish_comp(&self.ctx, natural, Dir::Check(answer))
    }

    /// Rule Reify (`effects-control-shell.md` §2.1; A3.3 `+control`):
    /// **check-only** against a reified-stack type `Stk(B, C)` (like an
    /// injection — the consumed type `B` comes only from the expectation, since
    /// the empty stack is `B ⇒ B` for *any* `B`). The stack-typing judgment
    /// `K : B ⇒ C` runs forward from `B` ([`Self::stack_infer`]), synthesizing
    /// the delivered answer `C'`; the inlined Sub rule then fits `Stk(B, C')`
    /// to the expectation `Stk(B, C)`. Of the ADR-33 D6 contravariant-`B` /
    /// covariant-`C` variance, only the covariant-`C` leg `C' <: C` does real
    /// work here — the consumed `B` is taken verbatim from the expectation, so
    /// the contravariant-`B` leg is the reflexive `B <: B` (running the walk
    /// forward from the expectation's `B` *is* the rule); the full variance is
    /// exercised by the standalone `subtype_*` tests. A hole answer is the
    /// matched stack (`Unknown ▶Stk Stk(Unknown, Unknown)`), which the hole
    /// absorbs.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_stk(
        &mut self,
        stack: Rc<Stack>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        let consumed: CompType = match dir {
            | Dir::Check(ValueType::Stk(ref b, _)) => b.as_ref().clone(),
            | Dir::Check(ValueType::Unknown) => CompType::Unknown,
            | Dir::Infer | Dir::Check(_) => {
                return Err(TypeError::StuckExpr {
                    expr: Term::Value(Value::Stk(stack)),
                    hint: text::STK_NEEDS_STK_TYPE,
                });
            },
        };
        let delivered = self.stack_infer(unrc(stack), consumed.clone())?;
        finish_value(&self.ctx, ValueType::stk(consumed, delivered), dir)
    }

    /// The stack-typing judgment `K : B ⇒ C`, run **forward** from the consumed
    /// type `input` to synthesize the delivered answer
    /// (`effects-control-shell.md` §2.1; A3.3 `+control`). The empty stack
    /// delivers its input; an argument frame consumes a function and checks
    /// its argument; a bind frame consumes a returner, binds the payload,
    /// infers the continuation, and folds the consumed row in exactly as
    /// [`Self::rule_bind`] does (via [`combine_bind_row`]); a projection
    /// frame consumes a lazy product. The per-frame destructures are the
    /// `crate::judgements::stack` helpers shared with the typing machine, and
    /// only the *sub-terms* (an argument value, a bind continuation) log
    /// trace events — the structural walk does not — so the machine's frame
    /// walk produces the identical trace.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn stack_infer(
        &mut self,
        stack: Stack,
        input: CompType,
    ) -> Result<CompType, TypeError>
    {
        match stack {
            | Stack::Empty => Ok(input),
            | Stack::Arg(value, rest) => {
                let (binder, arg_ty, result_ty) = arrow_components(input)?;
                let value = unrc(value);
                self.value(value.clone(), Dir::Check(arg_ty))?;
                // The frame supplies the argument, so a dependent codomain is
                // closed here — the same instantiation the App rule performs,
                // reached through the stack instead of through the term.
                let result_ty =
                    instantiate_codomain(binder.as_deref().map(NameRef::from), &result_ty, &value);
                self.stack_infer(unrc(rest), result_ty)
            },
            | Stack::Bind(name, cont, rest) => {
                let (payload, consumed_row) = returner_components(input)?;
                self.ctx.bind(name, payload);
                let cont_ty = self.comp(unrc(cont), Dir::Infer)?;
                let sequenced = combine_bind_row(&consumed_row, cont_ty)?;
                self.ctx.unbind();
                self.stack_infer(unrc(rest), sequenced)
            },
            | Stack::Prj(side, rest) => {
                let projected = with_component(input, side)?;
                self.stack_infer(unrc(rest), projected)
            },
        }
    }

    /// Rule Resume (`effects-control-shell.md` §2.1; A3.3 `+control`): infer
    /// the reified stack `v ⇑ Stk(B, C)`, check the fed computation `t ⇓
    /// B`, and deliver the answer `C`. An **inference** form structurally
    /// identical to application — the "function" is the stack value, the
    /// "argument" the computation. A matched `Unknown` stack feeds `t`
    /// against `Unknown` and delivers `Unknown` (the hole localizes, as an
    /// `Unknown` application head).
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_resume(
        &mut self,
        stack: Rc<Value>,
        comp: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let stack_ty = self.value(unrc(stack), Dir::Infer)?;
        let (consumed, delivered) = stk_components(stack_ty)?;
        self.comp(unrc(comp), Dir::Check(consumed))?;
        finish_comp(&self.ctx, delivered, dir)
    }

    /// Rule Reset (`effects-control-shell.md` §2.2; A3.3 `+control`):
    /// **check-only** against the answer `C` (like a handler — in inference
    /// mode the answer is undetermined, so it is stuck). It establishes `C`
    /// as the ambient answer for the body's `shift`s — dynamically scoped:
    /// the prior ambient is saved and restored around the body — and is
    /// **transparent on the type**: it returns the body's *checked* type,
    /// not the answer `C` itself. The two coincide for a body that echoes
    /// its expectation, but a *matched* body (a `⟨_, _⟩` / `case` / `split`
    /// checked against `?`) reconstructs a type only consistent with `C`
    /// (e.g. `? & ?`, not `?`), so returning the body type — exactly as the
    /// machine's `gandr_core_machine::Frame::ResetBody` does — is what keeps
    /// the two faces
    /// lock-step (ADR-9; the bug a returned `C` would introduce). The
    /// restore runs only on success; on the error path the register is left
    /// as the failing derivation set it (matching the machine, whose
    /// `ResetBody` frame restores only at its pop), and the conformance
    /// suite compares errors, not the register.
    ///
    /// LIMITATION (v0 answer-type register; the full "control `C` effect" is
    /// reserved): the ambient answer is dynamically scoped along the
    /// *evaluation spine* only — it is **not** cleared when the body
    /// suspends a computation (a `thunk`, a `λ`, a reified-stack
    /// continuation, a handler clause), so a `shift` captured inside such a
    /// value reads this `reset`'s answer even though the value may run
    /// after this `reset` has returned. That permissive typing is sound
    /// against the literal §2.2 Shift rule (which requires no
    /// enclosing `reset` at all) but over-accepts against the runtime intent;
    /// tightening it (clear the register at suspension boundaries, or record
    /// the answer obligation in the row) is a control-soundness
    /// residual.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_reset(
        &mut self,
        body: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let Dir::Check(answer) = dir
        else {
            return Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Reset(body)),
                hint: text::RESET_NEEDS_CHECK,
            });
        };
        let saved = self.answer.replace(answer.clone());
        let body_ty = self.comp(unrc(body), Dir::Check(answer))?;
        self.answer = saved;
        Ok(body_ty)
    }

    /// Rule Shift (`effects-control-shell.md` §2.2; A3.3 `+control`):
    /// **check-only** against the captured type `B`. It reads the ambient
    /// answer `C` the nearest enclosing `reset` established (a `shift` with
    /// no enclosing `reset` — `answer = None` — is stuck), binds the
    /// continuation `k : Stk(B, C)`, checks the body `t ⇓ C`, and delivers
    /// `B`. The two stuck checks run in the machine's order (direction
    /// first, then the ambient answer) so the failure modes agree. The
    /// ambient answer is the lexical- spine register; see
    /// [`Self::rule_reset`] for the v0 suspension-boundary limitation it
    /// inherits (a `shift` captured inside a `thunk` / `λ` / reified stack
    /// reads the enclosing `reset`'s answer — over-accepting vs the runtime
    /// intent; a control-soundness residual).
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_shift(
        &mut self,
        k: String,
        body: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let Dir::Check(captured) = dir
        else {
            return Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Shift(k, body)),
                hint: text::SHIFT_NEEDS_CHECK,
            });
        };
        let Some(answer) = self.answer.clone()
        else {
            return Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Shift(k, body)),
                hint: text::SHIFT_NEEDS_RESET,
            });
        };
        self.ctx
            .bind(k, ValueType::stk(captured.clone(), answer.clone()));
        self.comp(unrc(body), Dir::Check(answer))?;
        self.ctx.unbind();
        Ok(captured)
    }

    /// Rule Fix⇓: the recursion former `fix x. t`, **check-primary**.
    ///
    /// The self-binding needs the fixpoint's own computation type in order to
    /// state the self-reference's type, so that type must arrive from the
    /// context rather than be synthesized from the body: with the expectation
    /// `B` in hand the rule binds `x : U_ω B`, checks `t ⇓ B`, and delivers
    /// `B`.
    ///
    /// **Inference is stuck**, and that is the rule rather than a gap. There is
    /// nothing in `fix x. t` to synthesize `B` from — the body is checked
    /// against it, and the body's own type is what is being defined. The
    /// ascription coercion is the inference route, and a recursive definition's
    /// declared signature is what supplies it in practice.
    ///
    /// **The grade is `ω`** because a recursive call forces the knot an
    /// unbounded number of times. The rule generalizes to any grade above one
    /// without changing shape, so a grade-`1` tail-recursion refinement is
    /// growth on this rule rather than a second form.
    ///
    /// The former adds no subtyping seam: it introduces no atom relation, no
    /// row relaxation, and no grade relaxation, so it carries no coherence
    /// obligation of its own.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_fix(
        &mut self,
        x: String,
        body: Rc<Comp>,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let Dir::Check(expected) = dir
        else {
            return Err(TypeError::StuckExpr {
                expr: Term::Comp(Comp::Fix(x, body)),
                hint: text::FIX_NEEDS_CHECK,
            });
        };
        self.ctx
            .bind(x, ValueType::thunk(Grade::OMEGA, expected.clone()));
        let result = self.comp(unrc(body), Dir::Check(expected));
        self.ctx.unbind();
        result
    }

    /// Rules Here⇑/Here⇓ (ADR-76; rule `Here`): `here(v) : Path A v v`.
    ///
    /// An introduction form, like [`Self::rule_inj`]: infer the witness `v ⇑
    /// A`, construct the natural type `Path A v v` (both endpoints are the
    /// witness), and finish through the inlined Sub rule. In checking mode
    /// against an expected `Path A′ x y` the subsumption
    /// ([`crate::discipline::subtype::value_subtype`]'s `Path` arm) enforces
    /// carrier invariance `A ≡ A′` and endpoint equality `x ≡ᵥ v`, `y ≡ᵥ v`
    /// — the design's `Here⇓` premises — while an `Unknown` expectation
    /// degenerates to consistency and any non-`Path` expectation is a
    /// type mismatch (only `Path` types are inhabited by `here`). Rung 1 infers
    /// the witness (rather than checking it against the expected carrier),
    /// so the integer-literal widening of `Here⇓` awaits the `NbE` era; the
    /// day-one surface carriers match the witness's inferred atom.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_here(
        &mut self,
        witness: Rc<Value>,
        dir: Dir<ValueType>,
    ) -> Result<ValueType, TypeError>
    {
        let witness_value = unrc(witness);
        let witness_ty = self.value(witness_value.clone(), Dir::Infer)?;
        let natural = ValueType::path(witness_ty, witness_value.clone(), witness_value);
        finish_value(&self.ctx, natural, dir)
    }

    /// Rule `Walk`⇑/`Walk`⇓ (ADR-76; the full Martin-Löf dinatural eliminator):
    /// infer the scrutinee `p ⇑ Path A a b`, check the diagonal base `c ⇓
    /// C[x/y][here(x)/q]` under `x : A`, and deliver `C[a/x][b/y][p/q]`.
    ///
    /// gandr's first **dependent** eliminator, and — unlike [`Self::rule_case`]
    /// — inference-capable: the explicit motive supplies the result type. The
    /// motive is untraced pure type computation
    /// ([`gandr_core_term::identity::subst_comptype`] instantiates it); the
    /// two traced premises are the scrutinee (value, inferred) and the base
    /// body (computation, checked), so the control trace is exactly
    /// [`Self::rule_split`]'s shape with a checked body. A matched
    /// `Unknown` scrutinee (A2.2 holes discipline) binds the base at `Unknown`
    /// and delivers `Unknown`.
    /// # Termination
    /// - reason: mirrors finite typing-rule derivations.
    /// - measure: remaining checked syntax, type, or stack premises.
    /// - boundedness: inputs are finite Rust values allocated before checking.
    /// - input recursion: structurally finite checked-term descent.
    fn rule_walk(
        &mut self,
        scrut: Rc<Value>,
        motive: &WalkMotive,
        base: WalkBase,
        dir: Dir<CompType>,
    ) -> Result<CompType, TypeError>
    {
        let scrut_value = unrc(scrut);
        let scrut_ty = self.value(scrut_value.clone(), Dir::Infer)?;
        let (carrier, base_expected, result): (ValueType, CompType, CompType) = match scrut_ty {
            | ValueType::Path { ty, lhs, rhs } => {
                let carrier = unrc(ty);
                let diagonal = base_diagonal_type(motive, &base.x);
                let result = motive_result_type(motive, &lhs, &rhs, &scrut_value);
                (carrier, diagonal, result)
            },
            // The matched identity (A2.2 holes extension): an `Unknown`
            // scrutinee binds the base at `Unknown` and delivers `Unknown`.
            | ValueType::Unknown => (ValueType::Unknown, CompType::Unknown, CompType::Unknown),
            | other => {
                return Err(TypeError::ShapeMismatch {
                    expected: text::SHAPE_PATH,
                    actual: Ty::Value(other),
                });
            },
        };
        self.ctx.bind(base.x.clone(), carrier);
        self.comp(unrc(base.body), Dir::Check(base_expected))?;
        self.ctx.unbind();
        finish_comp(&self.ctx, result, dir)
    }
}

/// Builds the identity eliminator base's expected type `C[x/y][here(x)/q]`.
///
/// This is the motive's **diagonal** instance under the base binder `x`
/// (ADR-76): the motive's endpoint binders `y` and `x` both map to the base
/// binder, and the path binder `q` maps to `here(x)`.
///
/// # Contract
/// - ensures: returns `motive.body` with `motive.x` and `motive.y` replaced by
///   `base_binder` and `motive.q` replaced by `here(base_binder)`.
/// - panics: none.
#[inline]
#[must_use]
pub fn base_diagonal_type<'source, N>(
    motive: &WalkMotive,
    base_binder: N,
) -> CompType
where
    N: Into<NameRef<'source>>,
{
    let base_binder = base_binder.into();
    let base_var = Value::var(base_binder);
    let here_var = Value::here(base_var.clone());
    let step = subst_comptype(&motive.body, NameRef::from(motive.x.as_str()), &base_var);
    let step = subst_comptype(&step, NameRef::from(motive.y.as_str()), &base_var);
    subst_comptype(&step, NameRef::from(motive.q.as_str()), &here_var)
}

/// Relocates a `Π` codomain from the type's binder into the lambda's.
///
/// The checking rule for an unannotated lambda meets a codomain written in
/// terms of the *type's* binder and a body written in terms of the *lambda's*.
/// They denote one variable, so one of the two names has to move before the
/// body can be checked against the codomain, and moving the type is what keeps
/// the source term untouched.
///
/// A non-dependent arrow carries no binder and needs no relocation; identical
/// names need none either, which is the common case when the elaborator names
/// the type's binder after the source's.
///
/// # Contract
/// - ensures: returns a codomain in which the type's binder is spelled `name`.
/// - panics: none.
#[inline]
#[must_use]
pub fn relocate_codomain(
    binder: Option<NameRef<'_>>,
    name: NameRef<'_>,
    res: &Rc<CompType>,
) -> CompType
{
    match binder {
        | Some(bound) if bound.as_ref() != name.as_ref() => subst_comptype(
            res.as_ref(),
            bound,
            &Value::Var(alloc::string::String::from(name.as_ref())),
        ),
        | Some(_) | None => res.as_ref().clone(),
    }
}

/// Instantiates a function type's codomain at the argument it was applied to.
///
/// The one operation that makes a dependent function type *dependent*, and the
/// single place it happens — the term-level application rule and the
/// argument-frame stack rule both arrive here, so the two cannot drift.
///
/// A non-dependent arrow carries no binder and its codomain passes through
/// unchanged, which is the pre-`Π` behaviour byte for byte.
///
/// # Contract
/// - ensures: returns `res` with the binder replaced by `arg`, or `res`
///   unchanged when there is no binder.
/// - panics: none.
#[inline]
#[must_use]
pub fn instantiate_codomain(
    binder: Option<NameRef<'_>>,
    res: &CompType,
    arg: &Value,
) -> CompType
{
    match binder {
        | Some(bound) => subst_comptype(res, bound, arg),
        | None => res.clone(),
    }
}

/// The binder an **inferred** function type carries: the bound name when the
/// inferred codomain actually mentions it, and none when it does not.
///
/// # Why inference may derive a binder where checking may not
///
/// The written binder [`CompType::Arrow`] documents is about *checking*: a
/// codomain the elaborator wrote may later be instantiated, so deriving its
/// binder from an occurrence would be deriving it from an incomplete type.
/// Inference is the opposite situation. The codomain here is the type the body
/// was just *checked to have* — it is complete, nothing further substitutes
/// into it, and the occurrence question therefore has a stable answer.
///
/// Deriving it is what keeps an ordinary non-dependent lambda inferring the
/// ordinary non-dependent arrow it always did, rather than a vacuous `Π` that
/// every downstream expectation would then have to see through.
///
/// # Contract
/// - ensures: returns `Some(name)` iff `name` occurs free in `res`.
/// - panics: none.
#[inline]
#[must_use]
pub fn inferred_binder(
    name: NameRef<'_>,
    res: &CompType,
) -> Option<alloc::string::String>
{
    bool::from(occurs_free_comptype(res, name)).then(|| alloc::string::String::from(name.as_ref()))
}

/// Builds the identity eliminator's result type `C[a/x][b/y][p/q]` (ADR-76):
/// the motive instantiated at the inferred endpoints `a`, `b` and the scrutinee
/// `p`.
///
/// # Contract
/// - ensures: returns `motive.body` with `motive.x`/`motive.y`/`motive.q`
///   replaced by `lhs`/`rhs`/`scrut` respectively.
/// - panics: none.
#[inline]
#[must_use]
pub fn motive_result_type(
    motive: &WalkMotive,
    lhs: &Value,
    rhs: &Value,
    scrut: &Value,
) -> CompType
{
    let step = subst_comptype(&motive.body, NameRef::from(motive.x.as_str()), lhs);
    let step = subst_comptype(&step, NameRef::from(motive.y.as_str()), rhs);
    subst_comptype(&step, NameRef::from(motive.q.as_str()), scrut)
}

/// The split body's checked-against type and the split's delivered answer for a
/// `Prod` / `Σ` scrutinee (rules `SplitMotive`⇑ / Split⇓, ADR-82 D2/D3).
///
/// # Contract
/// - ensures: **with** a motive `(z). M` — the body is checked against `M[(p,
///   q)/z]` ([`split_body_expectation`]) and the split delivers `M[v/z]`
///   ([`split_result_type`]); **without** a motive `dir` is `Check(C)` (the
///   entry guard rejected `Infer`), and both the body expectation and the
///   delivered answer are `C` verbatim — no type synthesized under the binders
///   crosses the binder boundary (D3).
/// - panics: none — a `None` motive is only reached under a `Check` direction
///   (the motive-less `Infer` case is declined at rule entry, ADR-82 D3).
#[inline]
#[must_use]
pub fn split_expectations<'source, F, S>(
    motive: Option<&SplitMotive>,
    dir: &Dir<CompType>,
    fst_name: F,
    snd_name: S,
    scrut: &Value,
) -> (Dir<CompType>, CompType)
where
    F: Into<NameRef<'source>> + Copy,
    S: Into<NameRef<'source>> + Copy,
{
    match motive {
        | Some(motive) => (
            Dir::Check(split_body_expectation(motive, fst_name, snd_name)),
            split_result_type(motive, scrut),
        ),
        // Rule Split⇓: the motive-less check-only split delivers the expectation
        // verbatim (the `Infer` case never reaches here — declined at entry).
        | None => match *dir {
            | Dir::Check(ref expected) => (Dir::Check(expected.clone()), expected.clone()),
            | Dir::Infer => (Dir::Infer, CompType::Unknown),
        },
    }
}

/// The split body's checked-against type `M[(p, q)/z]` — the motive
/// instantiated at the **pair** of the two freshly-named components (ADR-82 D2;
/// the [`base_diagonal_type`] precedent, with the pair substituted for the
/// scrutinee binder `z`).
///
/// # Contract
/// - ensures: returns `motive.body` with `motive.binder` replaced by the eager
///   pair `(fst_name, snd_name)`.
/// - panics: none.
#[must_use]
pub(crate) fn split_body_expectation<'source, F, S>(
    motive: &SplitMotive,
    fst_name: F,
    snd_name: S,
) -> CompType
where
    F: Into<NameRef<'source>>,
    S: Into<NameRef<'source>>,
{
    let pair = Value::pair(Value::var(fst_name), Value::var(snd_name));
    subst_comptype(&motive.body, NameRef::from(motive.binder.as_str()), &pair)
}

/// The split's delivered answer `M[v/z]` — the motive instantiated at the
/// scrutinee value `v` (ADR-82 D2; the [`motive_result_type`] precedent). The
/// answer is built from the outer-scoped motive and the scrutinee, so **no
/// binder can occur in it**.
///
/// # Contract
/// - ensures: returns `motive.body` with `motive.binder` replaced by `scrut`.
/// - panics: none.
#[must_use]
pub(crate) fn split_result_type(
    motive: &SplitMotive,
    scrut: &Value,
) -> CompType
{
    subst_comptype(&motive.body, NameRef::from(motive.binder.as_str()), scrut)
}

/// The split body's checked-against type and the split's delivered answer for a
/// **matched `Unknown`** scrutinee (A2.2 holes; ADR-82 D2/D3).
///
/// # Contract
/// - ensures: **with** a motive the body is checked against `Unknown` and the
///   split delivers `Unknown` (the `walk` Unknown arm, motive ignored);
///   **without** a motive the direction is `Check(C)` and both are `C`
///   verbatim.
/// - panics: none — see [`split_expectations`].
#[inline]
#[must_use]
pub fn split_unknown_expectations(
    motive: Option<&SplitMotive>,
    dir: &Dir<CompType>,
) -> (Dir<CompType>, CompType)
{
    match motive {
        | Some(_) => (Dir::Check(CompType::Unknown), CompType::Unknown),
        | None => match *dir {
            | Dir::Check(ref expected) => (Dir::Check(expected.clone()), expected.clone()),
            | Dir::Infer => (Dir::Infer, CompType::Unknown),
        },
    }
}

#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;

    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Term;
    use gandr_core_term::syntax::Value;

    use super::diagnostic_abs_term;

    /// The unannotated-abstraction stuck diagnostic is a public compatibility
    /// readback boundary: it reconstructs the legacy term only after routing
    /// the lambda body through `FlatArena`.
    #[test]
    fn abs_stuck_diagnostic_reads_back_through_arena_bridge()
    {
        let body = Rc::new(Comp::ret(Value::Unit));
        let term = diagnostic_abs_term("x".to_owned(), Rc::clone(&body));

        assert_eq!(term, Term::Comp(Comp::Abs("x".to_owned(), None, body)));
    }
}
