//! Structural subtyping and the subsumption rule (`type-system.md`
//! §"Notation and judgment forms" rule Sub, §"Subtyping decomposition" core
//! decompositions).
//!
//! Stage 1 has no unification variables, unions, or intersections, so the
//! §"Algorithmic subtyping and the worklist solver" degenerates to a direct
//! structural decision procedure: goals are decomposed by the §"Subtyping
//! decomposition" rules for the core constructors (covariant `×`, `+`, `F`;
//! contravariant arrow argument; `s ⊑ r` on `U`), and reflexivity/transitivity
//! are admissible, never rules. Goals are decided depth-first from an explicit
//! worklist ([`SubtypeGoal`]), which with no metavariables in play is
//! observationally the in-order structural recursion (ADR-27 decision 1 records
//! this timing and its reversal when the solver lands).
//!
//! # `Unknown` and consistent subtyping (A2.2 holes extension)
//!
//! With the hole type ([`ValueType::Unknown`] / [`CompType::Unknown`], D5 of
//! `A2-PLAN.md`; `incremental-pipeline.md` §"Holes"), the relation decided here
//! is **consistent subtyping** (Siek–Taha gradual typing): `Unknown` relates to
//! every type *in both directions*. On `Unknown`-free ("static") types the
//! relation is exactly the old structural subtyping.
//!
//! **Decision tree** for `Unknown`'s relationship to other types
//! [SPECULATIVE DECISION, recorded per the A2.2 mandate]:
//!
//! - *Bidirectional wildcard (consistent subtyping)* — **adopted**. D5 says it
//!   verbatim ("subsumption treats `Unknown` as consistent in both
//!   directions"), and §"Holes": "a hole checks against any `A` / a hole's type
//!   flows anywhere without spurious errors" needs flow in both directions: a
//!   hole-typed head must feed argument positions of known type *and* known
//!   terms must check against elided (`Unknown`) ascriptions.
//! - *`Unknown` as top* — rejected: an `Unknown`-typed result could never
//!   subsume to a concrete expectation, so every use of a hole's output would
//!   cascade a `TypeMismatch` — exactly the spurious-error cascade §"Holes"
//!   exists to remove.
//! - *`Unknown` as bottom* — rejected, symmetric: nothing would check against
//!   an elided ascription.
//! - *A separate consistency relation alongside subtyping* — rejected for Stage
//!   1: subsumption is the only consumer, so a second relation buys nothing but
//!   surface. **Reversal triggers**: (a) the Stage 3 solver lands — "fresh α,
//!   no constraints" un-degenerates into real metavariables and consistency
//!   must be split from subtyping before σ exists (the AGT/Siek–Taha shape);
//!   (b) ADR-17 marks promotion — marked terms carry their own consistency
//!   discipline.
//!
//! **Cost, recorded**: with `Unknown` in play the relation is reflexive but
//! **not transitive** (`Int ≲ Unknown ≲ Str` yet `Int ⋦ Str`) — the
//! well-known price of gradual consistency. The conformance suite pins
//! transitivity on static types only, plus an explicit non-transitivity
//! witness.

use alloc::rc::Rc;

use gandr_core_nbe::Normalizer;
use gandr_core_nbe::conv::comp_type_converts;
use gandr_core_nbe::conv::converts;
use gandr_core_nbe::conv::type_converts;
use gandr_core_term::boundary::IntegerLiteral;
use gandr_core_term::boundary::NameRef;
use gandr_core_term::boundary::PackageArity;
use gandr_core_term::boundary::SubtypeDecision;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::error::TypeError;
use gandr_core_term::intern::TypeId;
use gandr_core_term::intern::TypeInterner;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

use crate::judgements::control::Dir;

/// Completes the integer-literal rule under a direction (ADR-39 D4) — the
/// checking-mode-polymorphic counterpart of [`finish_value`] for
/// [`gandr_core_term::syntax::Value::Int`].
///
/// In inference mode an integer literal is the rigid `Integer` atom (frozen,
/// A2.1). In checking mode it additionally accepts any integer numeric atom it
/// is representable in ([`int_literal_fits`]) — the Rust `{integer}` literal
/// rule — and otherwise falls back to the ordinary `Integer` subsumption, so a
/// `: Integer` check, an `Unknown` goal, and a genuine mismatch behave exactly
/// as before. The checker, machine, and mark all complete the integer literal
/// through this one function (mark through its own marking variant), keeping
/// the three passes lock-step.
///
/// # Contract
/// - ensures: `Dir::Infer` returns `Integer`; `Dir::Check(expected)` returns
///   `expected` when `int_literal_fits(n, &expected)`, else the
///   [`finish_value`] result for `Integer` against `expected`.
/// - fails: `TypeError::TypeMismatch` exactly when `finish_value(Integer,
///   Check(expected))` would (i.e. `expected` is neither a fitting integer atom
///   nor a supertype of `Integer`).
/// - panics: none.
///
/// # Errors
///
/// Returns [`TypeError::TypeMismatch`] when neither the literal fit nor the
/// `Integer` subsumption holds.
#[inline]
pub fn finish_int_literal(
    ctx: &Ctx,
    n: IntegerLiteral,
    dir: Dir<ValueType>,
) -> Result<ValueType, TypeError>
{
    match dir {
        | Dir::Infer => Ok(ValueType::integer()),
        | Dir::Check(expected) => {
            if bool::from(int_literal_fits(n, &expected)) {
                Ok(expected)
            }
            else {
                finish_value(ctx, ValueType::integer(), Dir::Check(expected))
            }
        },
    }
}

/// True when the rigid atom `expected` is one of the four *integer* numeric
/// primitives (ADR-39) and the integer literal value `n` is representable in
/// it — the Rust `{integer}` literal rule (ADR-39 D4).
///
/// This is a **literal-only** relation, not subtyping: a *variable* of type
/// `Integer` never widens to `u32`, only an integer literal does, so it is
/// consulted from the integer-literal rule ([`finish_int_literal`]) and **not**
/// from [`value_subtype`] (whose atoms relate only by equality).
#[must_use]
pub(crate) fn int_literal_fits(
    n: IntegerLiteral,
    expected: &ValueType,
) -> SubtypeDecision
{
    let ValueType::Atom(ref name) = *expected
    else {
        return false.into();
    };
    match name.as_str() {
        | "u32" => u32::try_from(i64::from(n)).is_ok().into(),
        | "u64" => u64::try_from(i64::from(n)).is_ok().into(),
        | "i32" => i32::try_from(i64::from(n)).is_ok().into(),
        | "i64" => true.into(),
        | _ => false.into(),
    }
}

/// Completes a value rule under a direction (the Sub rule, inlined).
///
/// In inference mode the constructed type is the result; in checking mode the
/// constructed type must be a subtype of the expected type, and the expected
/// type is the result (matching the machine, whose check-mode `Return`
/// carries the expected type).
///
/// # Contract
/// - ensures: in `Dir::Infer` returns `constructed` unchanged (infallible); in
///   `Dir::Check(expected)` returns `expected` when `value_subtype(constructed,
///   expected)` holds.
/// - fails: `TypeError::TypeMismatch { expected, actual }` (with `actual` =
///   `constructed`) when the check-mode subsumption fails.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TypeError::TypeMismatch`] when subsumption fails.
#[inline]
pub fn finish_value(
    ctx: &Ctx,
    constructed: ValueType,
    dir: Dir<ValueType>,
) -> Result<ValueType, TypeError>
{
    match dir {
        | Dir::Infer => Ok(constructed),
        | Dir::Check(expected) => {
            if bool::from(value_subtype(ctx, &constructed, &expected)) {
                Ok(expected)
            }
            else {
                Err(TypeError::TypeMismatch {
                    expected: Ty::Value(expected),
                    actual: Ty::Value(constructed),
                })
            }
        },
    }
}

/// Decides `sub ≲ sup` on value types: consistent subtyping (see the module
/// doc) — structural subtyping with [`ValueType::Unknown`] consistent in
/// both directions.
///
/// # Contract
/// - ensures: returns `true` iff `sub` is a consistent subtype of `sup` —
///   structural on the constructors (covariant `Prod` and `Sum`, contravariant
///   grade plus covariant body on `Thunk`, where the grade leg is
///   `hi_grade.leq(lo_grade)`), with `ValueType::Unknown` consistent with every
///   type in both directions. The relation is reflexive (`value_subtype(t, t)`
///   for every `t`) but NOT transitive once `Unknown` participates (`Int ≲
///   Unknown` and `Unknown ≲ Str`, yet `Int ⋦ Str`); transitivity holds only on
///   `Unknown`-free types.
/// - panics: none.
#[inline]
#[must_use]
/// # Termination
/// - reason: subtyping follows finite type-tree structure.
/// - measure: remaining pair of type children under comparison.
/// - boundedness: compared types are finite Rust values.
/// - input recursion: none.
pub fn value_subtype(
    ctx: &Ctx,
    sub: &ValueType,
    sup: &ValueType,
) -> SubtypeDecision
{
    subtype_goals(ctx, vec![SubtypeGoal::Value(
        Rc::new(sub.clone()),
        Rc::new(sup.clone()),
    )])
}

/// A pending subtyping obligation: one pair of types to relate (`sub ≲ sup`).
///
/// The goal unit of the degenerate §"Algorithmic subtyping and the worklist
/// solver" (see the module doc): goals sit in the worklist until popped,
/// decided against the §"Subtyping decomposition" decompositions, and replaced
/// by their child goals — with no metavariables in play a LIFO queue is
/// observationally the in-order structural recursion.
enum SubtypeGoal
{
    /// Relates two value types (`sub ≲ sup`).
    Value(Rc<ValueType>, Rc<ValueType>),
    /// Relates two computation types (`sub ≲ sup`).
    Comp(Rc<CompType>, Rc<CompType>),
}

/// Completes a computation rule under a direction (the Sub rule, inlined).
///
/// # Contract
/// - ensures: in `Dir::Infer` returns `constructed` unchanged (infallible); in
///   `Dir::Check(expected)` returns `expected` when `comp_subtype(constructed,
///   expected)` holds.
/// - fails: `TypeError::TypeMismatch { expected, actual }` (with `actual` =
///   `constructed`) when the check-mode subsumption fails.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TypeError::TypeMismatch`] when subsumption fails.
#[inline]
pub fn finish_comp(
    ctx: &Ctx,
    constructed: CompType,
    dir: Dir<CompType>,
) -> Result<CompType, TypeError>
{
    match dir {
        | Dir::Infer => Ok(constructed),
        | Dir::Check(expected) => {
            if bool::from(comp_subtype(ctx, &constructed, &expected)) {
                Ok(expected)
            }
            else {
                Err(TypeError::TypeMismatch {
                    expected: Ty::Comp(expected),
                    actual: Ty::Comp(constructed),
                })
            }
        },
    }
}

/// Decides `sub ≲ sup` on computation types: consistent subtyping, as
/// [`value_subtype`].
///
/// # Contract
/// - ensures: returns `true` iff `sub` is a consistent subtype of `sup` —
///   structural on the constructors (covariant `F`, contravariant argument plus
///   covariant result on `Arrow`, covariant `With`), with `CompType::Unknown`
///   consistent with every type in both directions. Reflexive but NOT
///   transitive once `Unknown` participates, exactly as `value_subtype`.
/// - panics: none.
#[inline]
#[must_use]
/// # Termination
/// - reason: subtyping follows finite type-tree structure.
/// - measure: remaining pair of type children under comparison.
/// - boundedness: compared types are finite Rust values.
/// - input recursion: none.
pub fn comp_subtype(
    ctx: &Ctx,
    sub: &CompType,
    sup: &CompType,
) -> SubtypeDecision
{
    subtype_goals(ctx, vec![SubtypeGoal::Comp(
        Rc::new(sub.clone()),
        Rc::new(sup.clone()),
    )])
}

/// Decides a conjunction of subtyping goals from an explicit worklist — the
/// degenerate §"Algorithmic subtyping and the worklist solver" of the module
/// doc, made iterative so the decision procedure carries no input recursion.
///
/// Each pop short-circuits on pointer equality (ADR-50 Decision B) or an
/// `Unknown` side (the consistent-subtyping wildcard, D5), then decomposes
/// the pair by the §"Subtyping decomposition" rules — covariant `×`, `+`,
/// `List`, record, `F`, and `With`; contravariant arrow argument, `Stk` answer,
/// and `Thunk` grade — and pushes the child goals.
///
/// The two **invariant** formers are decided rather than decomposed: `Path` and
/// `Sigma` go to the normalizer's definitional equality
/// ([`gandr_core_nbe::conv::type_converts`]) in one call each, because
/// invariance is what conversion decides and a two-way subtyping pass was only
/// ever spelling that out. Endpoints go the same way
/// ([`gandr_core_nbe::conv::converts`]), so they now relate up to beta.
///
/// # Contract
/// - ensures: returns `true` iff every goal the initial goals decompose into
///   holds; `false.into()` at the first unsatisfiable goal.
/// - panics: none.
///
/// # Termination
/// - reason: each popped goal decomposes a type pair into strictly smaller
///   child pairs, or succeeds/fails without pushing.
/// - measure: the summed type-tree size of the queued goals.
/// - boundedness: compared types are finite Rust values.
/// - input recursion: none.
fn subtype_goals(
    ctx: &Ctx,
    mut goals: Vec<SubtypeGoal>,
) -> SubtypeDecision
{
    // One mint function for every normalizer this run builds. **Every mint
    // takes the same environment or none does**: two normalizers minted from
    // one crate with different environments would be a site-dependent
    // definitional equality — the same defect a strategy deciding a judgement
    // would be, arriving as a mint instead of a policy. Routing all four
    // invariant positions through one closure is what makes that structural
    // rather than a rule someone has to remember.
    //
    // The environment is the one the typing context carries. An empty context
    // reproduces the pre-unfolding relation exactly, so nothing that does not
    // populate one changes behaviour.
    let mint = || {
        let mut nbe = Normalizer::new();
        for entry in ctx.definition_chain() {
            // A definition that fails to lower contributes no unfolding rule,
            // which is the same state as a sealed atom: conversion is finer
            // than it could have been, never wrong. Unfolding only merges
            // equivalence classes, so a missing rule can cost a refusal and can
            // never produce an acceptance the full environment would refute.
            let _defined = nbe.define(NameRef::from(entry.0.as_str()), entry.1.as_ref());
        }
        nbe
    };
    // The normalizer is minted only if an invariant position is actually met,
    // and it is then reused for every one in this run.
    //
    // **It carries the definition chain the typing context holds**, so a
    // definition mentioned in an identity endpoint or a family index unfolds
    // here rather than standing as a free variable. That is what lets a law
    // field whose endpoint computes across a definition be *proved* rather than
    // merely stated. An empty context reproduces the pre-unfolding relation
    // exactly, so nothing that does not populate one changes behaviour.
    let mut nbe: Option<Normalizer> = None;
    while let Some(goal) = goals.pop() {
        match goal {
            | SubtypeGoal::Value(sub, sup) => {
                if core::ptr::eq(sub.as_ref(), sup.as_ref())
                    || matches!(sub.as_ref(), ValueType::Unknown)
                    || matches!(sup.as_ref(), ValueType::Unknown)
                {
                    continue;
                }
                match (sub.as_ref(), sup.as_ref()) {
                    | (&ValueType::Atom(ref lhs), &ValueType::Atom(ref rhs)) if lhs == rhs => {},
                    | (&ValueType::Unit, &ValueType::Unit)
                    | (&ValueType::Universe, &ValueType::Universe) => {},
                    // A sealed atom relates to itself by identity and to nothing
                    // else — the same nominal-before-structural discipline
                    // `Data` takes, minus the carrier. The catch-all below is
                    // what refuses a seal against its representation, so opacity
                    // needs no arm of its own; this arm exists only so two
                    // *references* to one atom relate, which pointer equality
                    // alone would miss.
                    | (&ValueType::Sealed(ref lhs), &ValueType::Sealed(ref rhs)) => {
                        if lhs != rhs {
                            return false.into();
                        }
                    },
                    | (
                        &ValueType::Prod(ref lo_fst, ref lo_snd),
                        &ValueType::Prod(ref hi_fst, ref hi_snd),
                    )
                    | (
                        &ValueType::Sum(ref lo_fst, ref lo_snd),
                        &ValueType::Sum(ref hi_fst, ref hi_snd),
                    ) => {
                        goals.push(SubtypeGoal::Value(Rc::clone(lo_snd), Rc::clone(hi_snd)));
                        goals.push(SubtypeGoal::Value(Rc::clone(lo_fst), Rc::clone(hi_fst)));
                    },
                    | (&ValueType::List(ref lo_elem), &ValueType::List(ref hi_elem)) => {
                        goals.push(SubtypeGoal::Value(Rc::clone(lo_elem), Rc::clone(hi_elem)));
                    },
                    | (&ValueType::Record(ref lo_fields), &ValueType::Record(ref hi_fields)) => {
                        for (label, hi_ty) in hi_fields {
                            let Some(lo_ty) = lo_fields.get(label)
                            else {
                                return false.into();
                            };
                            goals.push(SubtypeGoal::Value(Rc::clone(lo_ty), Rc::clone(hi_ty)));
                        }
                    },
                    | (
                        &ValueType::Thunk(ref lo_grade, ref lo_body),
                        &ValueType::Thunk(ref hi_grade, ref hi_body),
                    ) => {
                        if !bool::from(hi_grade.leq(*lo_grade)) {
                            return false.into();
                        }
                        goals.push(SubtypeGoal::Comp(Rc::clone(lo_body), Rc::clone(hi_body)));
                    },
                    | (
                        &ValueType::Stk(ref lo_b, ref lo_c),
                        &ValueType::Stk(ref hi_b, ref hi_c),
                    ) => {
                        goals.push(SubtypeGoal::Comp(Rc::clone(lo_c), Rc::clone(hi_c)));
                        goals.push(SubtypeGoal::Comp(Rc::clone(hi_b), Rc::clone(lo_b)));
                    },
                    // A type-family application is compared **invariantly**,
                    // the `Path` precedent exactly: variance in an index is a
                    // refinement this rung does not make, and conversion is the
                    // relation that decides index equality. Delegating the whole
                    // pair keeps that decision in one place rather than
                    // re-deriving the spine comparison here.
                    | (&ValueType::Family { .. }, &ValueType::Family { .. }) => {
                        let nbe = nbe.get_or_insert_with(mint);
                        if !bool::from(type_converts(nbe, sub.as_ref(), sup.as_ref())) {
                            return false.into();
                        }
                    },
                    | (
                        &ValueType::Path {
                            ty: ref lo_ty,
                            lhs: ref lo_lhs,
                            rhs: ref lo_rhs,
                        },
                        &ValueType::Path {
                            ty: ref hi_ty,
                            lhs: ref hi_lhs,
                            rhs: ref hi_rhs,
                        },
                    ) => {
                        // Invariance in the carrier and in both endpoints: the
                        // identity type's arm is decided by *conversion*, not by
                        // a two-way subtyping pass, because widening a path
                        // without transport is unsound and conversion is the
                        // relation that says so directly.
                        let nbe = nbe.get_or_insert_with(mint);
                        if !bool::from(converts(nbe, lo_lhs, hi_lhs))
                            || !bool::from(converts(nbe, lo_rhs, hi_rhs))
                            || !bool::from(type_converts(nbe, lo_ty, hi_ty))
                        {
                            return false.into();
                        }
                    },
                    | (
                        &ValueType::Data {
                            id: ref lo_id,
                            args: ref lo_args,
                        },
                        &ValueType::Data {
                            id: ref hi_id,
                            args: ref hi_args,
                        },
                    ) => {
                        if lo_id != hi_id {
                            return false.into();
                        }
                        if !lo_args.is_empty() && !hi_args.is_empty() {
                            if lo_args.len() != hi_args.len() {
                                return false.into();
                            }
                            for (lo, hi) in lo_args.iter().zip(hi_args.iter()) {
                                goals.push(SubtypeGoal::Value(Rc::clone(lo), Rc::clone(hi)));
                            }
                        }
                    },
                    | (
                        &ValueType::Sigma {
                            fst: ref lo_fst,
                            binder: ref lo_binder,
                            snd: ref lo_snd,
                        },
                        &ValueType::Sigma {
                            fst: ref hi_fst,
                            binder: ref hi_binder,
                            snd: ref hi_snd,
                        },
                    ) => {
                        // Invariance again, and for the same reason: covariant
                        // subtyping under a dependent binder is a refinement
                        // this rung does not make, so the two sides convert or
                        // they do not relate. The binder alignment stays here
                        // because it is the caller's obligation, not the
                        // conversion relation's.
                        let lo_snd_aligned = if lo_binder == hi_binder {
                            Rc::clone(lo_snd)
                        }
                        else {
                            Rc::new(gandr_core_term::identity::subst_valuetype(
                                lo_snd,
                                NameRef::from(lo_binder.as_str()),
                                &gandr_core_term::syntax::Value::var(NameRef::from(
                                    hi_binder.as_str(),
                                )),
                            ))
                        };
                        let nbe = nbe.get_or_insert_with(mint);
                        if !bool::from(type_converts(nbe, lo_fst, hi_fst))
                            || !bool::from(type_converts(nbe, &lo_snd_aligned, hi_snd))
                        {
                            return false.into();
                        }
                    },
                    // A package relates to a package and to nothing else — the
                    // catch-all below is what refuses it against a `Thunk`,
                    // which is the coercion the module design forbids in both
                    // directions rather than merely declines to provide.
                    //
                    // The grade leg is `Thunk`'s, contravariantly: a package
                    // openable more often is a subtype of one openable less
                    // often. The payload is compared **invariantly** (the
                    // `Sigma` precedent) after both sides are α-aligned at one
                    // canonical binder per position, so neither side's spelling
                    // of its abstract components can decide the answer.
                    | (
                        &ValueType::Package {
                            grade: lo_grade,
                            abstracts: ref lo_abstracts,
                            payload: ref lo_payload,
                        },
                        &ValueType::Package {
                            grade: hi_grade,
                            abstracts: ref hi_abstracts,
                            payload: ref hi_payload,
                        },
                    ) => {
                        if lo_abstracts.len() != hi_abstracts.len()
                            || !bool::from(hi_grade.leq(lo_grade))
                        {
                            return false.into();
                        }
                        let arity = PackageArity::from(lo_abstracts.len());
                        let witnesses = crate::judgements::package::canonical_witnesses(arity);
                        // A refused alignment is not a relation that failed but
                        // one that could not be decided, and an undecided
                        // relation is refused rather than assumed: the
                        // permissive direction here would relate an abstraction
                        // to a representation.
                        let Ok(lo_aligned) = crate::judgements::package::instantiate(
                            lo_abstracts,
                            lo_payload,
                            &witnesses,
                        )
                        else {
                            return false.into();
                        };
                        let Ok(hi_aligned) = crate::judgements::package::instantiate(
                            hi_abstracts,
                            hi_payload,
                            &witnesses,
                        )
                        else {
                            return false.into();
                        };
                        // The payload's own thunk grade IS the package's grade,
                        // so it is normalized away here and decided once by the
                        // leg above rather than twice — invariance would
                        // otherwise win and the contravariant leg would never
                        // fire (`crate::judgements::package::comparable_payload`).
                        let lo_aligned = Rc::new(crate::judgements::package::comparable_payload(
                            lo_grade,
                            &lo_aligned,
                        ));
                        let hi_aligned = Rc::new(crate::judgements::package::comparable_payload(
                            hi_grade,
                            &hi_aligned,
                        ));
                        goals.push(SubtypeGoal::Value(
                            Rc::clone(&hi_aligned),
                            Rc::clone(&lo_aligned),
                        ));
                        goals.push(SubtypeGoal::Value(lo_aligned, hi_aligned));
                    },
                    | _ => return false.into(),
                }
            },
            | SubtypeGoal::Comp(sub, sup) => {
                if core::ptr::eq(sub.as_ref(), sup.as_ref())
                    || matches!(sub.as_ref(), CompType::Unknown)
                    || matches!(sup.as_ref(), CompType::Unknown)
                {
                    continue;
                }
                match (sub.as_ref(), sup.as_ref()) {
                    | (
                        &CompType::F(ref lo_of, ref lo_row),
                        &CompType::F(ref hi_of, ref hi_row),
                    ) => {
                        if !bool::from(lo_row.is_subset(hi_row)) {
                            return false.into();
                        }
                        goals.push(SubtypeGoal::Value(Rc::clone(lo_of), Rc::clone(hi_of)));
                    },
                    // The **non-dependent** function type keeps the variance it
                    // always had: contravariant in the argument, covariant in
                    // the result.
                    | (
                        &CompType::Arrow {
                            binder: None,
                            arg: ref lo_arg,
                            res: ref lo_res,
                        },
                        &CompType::Arrow {
                            binder: None,
                            arg: ref hi_arg,
                            res: ref hi_res,
                        },
                    ) => {
                        goals.push(SubtypeGoal::Comp(Rc::clone(lo_res), Rc::clone(hi_res)));
                        goals.push(SubtypeGoal::Value(Rc::clone(hi_arg), Rc::clone(lo_arg)));
                    },
                    // A **dependent** function type is compared invariantly, the
                    // `Sigma` precedent exactly: variance under a dependent
                    // binder is a refinement this rung does not make, so the two
                    // sides convert or they do not relate.
                    //
                    // The whole pair goes to `type_converts` rather than being
                    // decomposed here, because the binder alignment a dependent
                    // comparison needs already lives there — in one function,
                    // stated once. Decomposing here would be a second place
                    // deciding when two function types are the same type, which
                    // is the duplication the identity rung already paid for
                    // once.
                    | (&CompType::Arrow { .. }, &CompType::Arrow { .. }) => {
                        let nbe = nbe.get_or_insert_with(mint);
                        if !bool::from(comp_type_converts(nbe, sub.as_ref(), sup.as_ref())) {
                            return false.into();
                        }
                    },
                    | (
                        &CompType::With(ref lo_fst, ref lo_snd),
                        &CompType::With(ref hi_fst, ref hi_snd),
                    ) => {
                        goals.push(SubtypeGoal::Comp(Rc::clone(lo_snd), Rc::clone(hi_snd)));
                        goals.push(SubtypeGoal::Comp(Rc::clone(lo_fst), Rc::clone(hi_fst)));
                    },
                    | _ => return false.into(),
                }
            },
        }
    }
    true.into()
}

/// Picks a summand or component by side, cloning out of the pair of children.
#[inline]
#[must_use]
pub fn pick<T>(
    side: gandr_core_term::syntax::Side,
    fst: &Rc<T>,
    snd: &Rc<T>,
) -> T
where
    T: Clone,
{
    match side {
        | gandr_core_term::syntax::Side::Fst => fst.as_ref().clone(),
        | gandr_core_term::syntax::Side::Snd => snd.as_ref().clone(),
    }
}

/// Decides `sub ≲ sup` on two ids minted by `interner`.
///
/// Intern identity is an O(1) reflexive short-circuit taken before any
/// structural descent — the interned analogue of the `core::ptr::eq` fast path
/// inside [`value_subtype`], but catching *structurally* equal
/// address-distinct types too, since those share an id.
///
/// The relation lives here rather than on the interner because the interner is
/// substrate: it knows type identity and nothing about subsumption, and a
/// substrate that named this relation would depend on the crate that decides
/// it.
///
/// # Contract
/// - requires: `sub` and `sup` were both minted by `interner`; an id from
///   another interner, or one never minted, resolves to nothing and so returns
///   `false`.
/// - ensures: returns `true` iff the type at `sub` is a consistent subtype of
///   the type at `sup` — same-sort structural subtyping via [`value_subtype`] /
///   [`comp_subtype`]. Identical ids return `true` in O(1) (reflexivity,
///   admissible per `type-system.md` §"Algorithmic subtyping and the worklist
///   solver"); a value id against a computation id returns `false`, because no
///   value type is a subtype of a computation type or conversely. The verdict
///   equals the structural relation on the resolved types, so the id
///   short-circuit is a pure optimization.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 — the id-equality short-circuit never disagrees with
///   structural subtyping, because it only pre-empts the reflexive case, which
///   the structural relation also accepts.
/// - witness: `gandr_core_checker::interning::interned_subtype_agrees_with_structural_value`
/// - witness: `gandr_core_checker::interning::interned_subtype_agrees_with_structural_comp`
/// - witness: `tests::interned_subtype_takes_the_reflexive_hit`
/// - witness: `tests::interned_subtype_falls_back_to_structural_descent`
/// - witness: `tests::interned_subtype_refuses_cross_sort_and_foreign_ids`
#[inline]
#[must_use]
pub fn interned_subtype(
    ctx: &Ctx,
    interner: &TypeInterner,
    sub: TypeId,
    sup: TypeId,
) -> SubtypeDecision
{
    // Reflexive intern hit: equal ids ⟹ structurally identical types ⟹
    // (consistent) subtypes, decided in O(1) without any descent.
    if sub == sup {
        return true.into();
    }
    match (interner.resolve(sub), interner.resolve(sup)) {
        | (Some(&Ty::Value(ref lo)), Some(&Ty::Value(ref hi))) => value_subtype(ctx, lo, hi),
        | (Some(&Ty::Comp(ref lo)), Some(&Ty::Comp(ref hi))) => comp_subtype(ctx, lo, hi),
        // Cross-sort (a value type and a computation type never relate) or an id
        // this interner never minted.
        | _ => false.into(),
    }
}

#[cfg(test)]
mod tests
{

    /// **A law field that computes across a definition.** This is the target
    /// the whole definitional-environment slice exists for.
    ///
    /// `idf` is a top-level definition — a thunked identity function. The law
    /// says `idf(f)` equals `f`, so its type is `Path A (force idf applied to
    /// f) f` and its witness is `here(f)`, whose natural type is `Path A f f`.
    /// Accepting the witness means the checker reduced the endpoint **through
    /// the definition**: delta on `idf`, then force, then beta.
    ///
    /// **The pair is the evidence, not the acceptance.** The same two types
    /// under a context with an EMPTY chain must be refused, because there `idf`
    /// is a free variable and the endpoint is stuck. A test showing only the
    /// acceptance would pass equally for a relation that accepted everything.
    #[test]
    fn a_law_field_computes_across_a_definition_and_only_with_the_chain()
    {
        use alloc::rc::Rc;

        use gandr_core_term::boundary::NameRef;
        use gandr_core_term::syntax::Comp;
        use gandr_core_term::syntax::Value;

        // `idf = thunk(λx. ret x)`, the definition the endpoint reduces through.
        let identity = Rc::new(Value::thunk(
            gandr_core_term::grade::Grade::OMEGA,
            Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
        ));
        // The endpoint as written: `force idf` applied to `f`, thunked so it
        // sits in value position.
        let applied = Rc::new(Value::thunk(
            gandr_core_term::grade::Grade::OMEGA,
            Comp::app(
                Comp::force(Value::var(NameRef::from("idf"))),
                Value::var(NameRef::from("f")),
            ),
        ));
        let plain = Rc::new(Value::thunk(
            gandr_core_term::grade::Grade::OMEGA,
            Comp::ret(Value::var(NameRef::from("f"))),
        ));
        let path = |lhs: &Rc<Value>, rhs: &Rc<Value>| ValueType::Path {
            ty: Rc::new(ValueType::integer()),
            lhs: Rc::clone(lhs),
            rhs: Rc::clone(rhs),
        };
        // The law's stated type, and the type its reflexivity witness has.
        let stated = path(&applied, &plain);
        let witnessed = path(&plain, &plain);

        // With the definition in scope, the endpoint reduces and the law holds.
        let mut with_chain = Ctx::new();
        with_chain.define(NameRef::from("idf"), Rc::clone(&identity));
        assert!(
            bool::from(value_subtype(&with_chain, &witnessed, &stated)),
            "the reflexivity witness was refused against a law whose endpoint \
             reduces through a definition in scope"
        );

        // Without it, `idf` is free, the endpoint is stuck, and the same law is
        // refused. This is the separating half.
        let empty = Ctx::new();
        assert!(
            !bool::from(value_subtype(&empty, &witnessed, &stated)),
            "the law was accepted with an empty definition chain, so the \
             acceptance above is not evidence that anything unfolded"
        );
    }
    use alloc::rc::Rc;

    use gandr_core_term::ctx::Ctx;
    use gandr_core_term::grade::Grade;
    use gandr_core_term::intern::TypeInterner;
    use gandr_core_term::types::CompType;
    use gandr_core_term::types::SealId;
    use gandr_core_term::types::Ty;
    use gandr_core_term::types::ValueType;

    use super::interned_subtype;
    use super::value_subtype;

    /// The top-level reflexive entry `value_subtype(&x, &x)` short-circuits
    /// through the `core::ptr::eq` fast-path (ADR-50 Decision B).
    #[test]
    fn value_subtype_reflexive_entry_short_circuits()
    {
        let value_type = ValueType::atom("Foo");
        assert!(bool::from(value_subtype(
            &Ctx::new(),
            &value_type,
            &value_type
        )));
    }

    /// **The conversion migration, observed through the relation that consumes
    /// it.** Identity endpoints used to be compared by a structural,
    /// no-reduction equality, so two beta-equal endpoints read as distinct and
    /// the two path types did not relate. They now go through the normalizer,
    /// which is the deliberate coarsening the rung was scheduled for.
    #[test]
    fn path_endpoints_relate_up_to_beta_since_the_normalizer_decides_them()
    {
        let redex = Rc::new(gandr_core_term::syntax::Value::Thunk(
            gandr_core_term::grade::Grade::ONE,
            Rc::new(gandr_core_term::syntax::Comp::app(
                gandr_core_term::syntax::Comp::lam(
                    "x",
                    gandr_core_term::syntax::Comp::ret(gandr_core_term::syntax::Value::var(
                        gandr_core_term::boundary::NameRef::from("x"),
                    )),
                ),
                gandr_core_term::syntax::Value::Int(3),
            )),
        ));
        let contractum = Rc::new(gandr_core_term::syntax::Value::Thunk(
            gandr_core_term::grade::Grade::ONE,
            Rc::new(gandr_core_term::syntax::Comp::ret(
                gandr_core_term::syntax::Value::Int(3),
            )),
        ));
        let path = |lhs: &Rc<gandr_core_term::syntax::Value>,
                    rhs: &Rc<gandr_core_term::syntax::Value>| {
            ValueType::Path {
                ty: Rc::new(ValueType::integer()),
                lhs: Rc::clone(lhs),
                rhs: Rc::clone(rhs),
            }
        };
        let reduced = path(&contractum, &contractum);
        let unreduced = path(&redex, &redex);
        assert!(bool::from(value_subtype(&Ctx::new(), &unreduced, &reduced)));
        assert!(bool::from(value_subtype(&Ctx::new(), &reduced, &unreduced)));
        // Endpoints that are genuinely apart still do not relate.
        let apart = path(
            &contractum,
            &Rc::new(gandr_core_term::syntax::Value::Thunk(
                gandr_core_term::grade::Grade::ONE,
                Rc::new(gandr_core_term::syntax::Comp::ret(
                    gandr_core_term::syntax::Value::Int(4),
                )),
            )),
        );
        assert!(!bool::from(value_subtype(&Ctx::new(), &reduced, &apart)));
    }

    /// A dependent pair is invariant, and the normalizer decides it: two sigmas
    /// differing only in their binder name relate, and one differing in a
    /// component does not.
    #[test]
    fn sigma_is_invariant_and_alpha_insensitive()
    {
        let sigma = |binder: &str| ValueType::Sigma {
            fst: Rc::new(ValueType::integer()),
            binder: alloc::string::String::from(binder),
            snd: Rc::new(ValueType::Unit),
        };
        assert!(bool::from(value_subtype(
            &Ctx::new(),
            &sigma("x"),
            &sigma("y")
        )));
        let other = ValueType::Sigma {
            fst: Rc::new(ValueType::integer()),
            binder: alloc::string::String::from("x"),
            snd: Rc::new(ValueType::integer()),
        };
        assert!(!bool::from(value_subtype(&Ctx::new(), &sigma("x"), &other)));
    }

    /// **The abstraction-leak refutation at the checked core.** A sealed atom
    /// relates to no structural type, in either direction, so a client cannot
    /// substitute the representation for the abstraction or the reverse.
    ///
    /// There is no unfolding to try: the seal carries no carrier, so this is a
    /// relation that cannot be established rather than one that failed.
    #[test]
    fn a_sealed_atom_relates_to_no_representation()
    {
        let sealed = ValueType::Sealed(SealId::new(0_u64, "Counter", "t"));
        for representation in [ValueType::atom("Integer"), ValueType::Unit] {
            assert!(
                !bool::from(value_subtype(&Ctx::new(), &sealed, &representation)),
                "a sealed atom is not a subtype of a candidate representation"
            );
            assert!(
                !bool::from(value_subtype(&Ctx::new(), &representation, &sealed)),
                "nor is a candidate representation a subtype of the sealed atom"
            );
        }
    }

    /// Two sealings stay apart, and one seal relates to itself across distinct
    /// allocations — generativity, decided on the minted identity rather than
    /// on the address.
    #[test]
    fn sealed_atoms_relate_by_identity_alone()
    {
        let counter = ValueType::Sealed(SealId::new(0_u64, "Counter", "t"));
        let gauge = ValueType::Sealed(SealId::new(1_u64, "Gauge", "t"));
        assert!(
            !bool::from(value_subtype(&Ctx::new(), &counter, &gauge)),
            "two sealings do not interchange, however alike their implementations"
        );
        let same = ValueType::Sealed(SealId::new(0_u64, "Counter", "t"));
        assert!(
            !core::ptr::eq(&raw const counter, &raw const same),
            "the two references are distinct allocations"
        );
        assert!(
            bool::from(value_subtype(&Ctx::new(), &counter, &same)),
            "one atom relates to itself through two references"
        );
    }

    /// Two clones of the SAME `Rc` deref to a common address, so the
    /// pointer-equality fast-path fires on aliased children too — pinning the
    /// aliased-`Rc` case the recursive descent exploits.
    #[test]
    fn value_subtype_aliased_rc_short_circuits()
    {
        let shared = Rc::new(ValueType::atom("Foo"));
        let clone = Rc::clone(&shared);
        // Shared allocation ⇒ the two inner references have equal addresses.
        assert!(core::ptr::eq(shared.as_ref(), clone.as_ref()));
        assert!(bool::from(value_subtype(
            &Ctx::new(),
            shared.as_ref(),
            clone.as_ref()
        )));
    }

    /// A small nested value type, rebuilt from fresh allocations on each call
    /// so two invocations are structurally equal and share no interior address
    /// — which is what makes the interner's dedup a content decision and keeps
    /// the structural descent below genuinely exercised.
    fn fresh_nested() -> ValueType
    {
        ValueType::prod(
            ValueType::list(ValueType::atom("A")),
            ValueType::thunk(Grade::OMEGA, CompType::returner(ValueType::integer())),
        )
    }

    /// Reflexive hit: an address-distinct rebuild interns to the same id, so
    /// [`interned_subtype`] answers in O(1) — and it agrees with the structural
    /// [`value_subtype`] over the two address-distinct values, which is the
    /// non-vacuous reflexive descent.
    #[test]
    fn interned_subtype_takes_the_reflexive_hit()
    {
        let lhs = fresh_nested();
        let rhs = fresh_nested();
        // The addresses differ, so `value_subtype`'s `ptr::eq` fast path cannot
        // pre-empt: the relation holds by structural reflexivity.
        assert!(
            bool::from(value_subtype(&Ctx::new(), &lhs, &rhs)),
            "reflexivity holds structurally"
        );

        let mut interner = TypeInterner::new();
        let lhs_id = interner.intern(&Ty::Value(lhs));
        let rhs_id = interner.intern(&Ty::Value(rhs));
        assert_eq!(lhs_id, rhs_id, "the rebuild deduped to the same id");
        assert!(
            bool::from(interned_subtype(&Ctx::new(), &interner, lhs_id, rhs_id)),
            "same id gives the O(1) reflexive hit"
        );
    }

    /// Structural fallback on distinct ids: `U_ω B <: U_1 B` because `1 ⊑ ω`,
    /// and not conversely. The two thunks intern to distinct ids, so
    /// [`interned_subtype`] takes the structural path and matches
    /// [`value_subtype`] in both directions.
    #[test]
    fn interned_subtype_falls_back_to_structural_descent()
    {
        let omega = ValueType::thunk(Grade::OMEGA, CompType::returner(ValueType::integer()));
        let one = ValueType::thunk(Grade::ONE, CompType::returner(ValueType::integer()));

        let mut interner = TypeInterner::new();
        let omega_id = interner.intern(&Ty::Value(omega.clone()));
        let one_id = interner.intern(&Ty::Value(one.clone()));
        assert_ne!(omega_id, one_id, "different grades give different ids");

        assert_eq!(
            interned_subtype(&Ctx::new(), &interner, omega_id, one_id),
            value_subtype(&Ctx::new(), &omega, &one),
            "the interned verdict matches structural subtyping"
        );
        assert!(
            bool::from(interned_subtype(&Ctx::new(), &interner, omega_id, one_id)),
            "U_ω B <: U_1 B, since 1 ⊑ ω"
        );
        assert_eq!(
            interned_subtype(&Ctx::new(), &interner, one_id, omega_id),
            value_subtype(&Ctx::new(), &one, &omega),
            "the interned negative verdict matches structural subtyping"
        );
        assert!(
            !bool::from(interned_subtype(&Ctx::new(), &interner, one_id, omega_id)),
            "U_1 B is not below U_ω B, so the relation stays directional"
        );
    }

    /// The two refusals the resolve guard buys: a value id never relates to a
    /// computation id, and an id this interner never minted resolves to
    /// nothing. The foreign id is minted by a second interner, which is how a
    /// caller reaches the unresolvable case through the public API.
    #[test]
    fn interned_subtype_refuses_cross_sort_and_foreign_ids()
    {
        let mut interner = TypeInterner::new();
        let value_id = interner.intern(&Ty::Value(ValueType::integer()));
        let comp_id = interner.intern(&Ty::Comp(CompType::returner(ValueType::integer())));
        assert!(
            !bool::from(interned_subtype(&Ctx::new(), &interner, value_id, comp_id)),
            "a value type is not a subtype of a computation type"
        );
        assert!(
            !bool::from(interned_subtype(&Ctx::new(), &interner, comp_id, value_id)),
            "nor the converse"
        );

        let mut foreign = TypeInterner::new();
        let _first = foreign.intern(&Ty::Value(ValueType::integer()));
        let _second = foreign.intern(&Ty::Value(ValueType::atom("A")));
        let third = foreign.intern(&Ty::Value(ValueType::atom("B")));
        assert!(
            !bool::from(interned_subtype(&Ctx::new(), &interner, value_id, third)),
            "an id minted by another interner never relates"
        );
    }
}
