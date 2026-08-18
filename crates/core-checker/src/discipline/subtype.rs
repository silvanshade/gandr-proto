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

use crate::discipline::boundary::IntegerLiteral;
use crate::discipline::boundary::NameRef;
use crate::discipline::boundary::PackageArity;
use crate::discipline::boundary::SubtypeDecision;
use crate::error::TypeError;
use crate::machine::control::Dir;
use crate::nbe::Normalizer;
use crate::nbe::conv::converts;
use crate::nbe::conv::type_converts;
use crate::term::types::CompType;
use crate::term::types::Ty;
use crate::term::types::ValueType;

/// Completes the integer-literal rule under a direction (ADR-39 D4) — the
/// checking-mode-polymorphic counterpart of [`finish_value`] for
/// [`crate::term::syntax::Value::Int`].
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
                finish_value(ValueType::integer(), Dir::Check(expected))
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
    constructed: ValueType,
    dir: Dir<ValueType>,
) -> Result<ValueType, TypeError>
{
    match dir {
        | Dir::Infer => Ok(constructed),
        | Dir::Check(expected) => {
            if bool::from(value_subtype(&constructed, &expected)) {
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
    sub: &ValueType,
    sup: &ValueType,
) -> SubtypeDecision
{
    subtype_goals(vec![SubtypeGoal::Value(
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
    constructed: CompType,
    dir: Dir<CompType>,
) -> Result<CompType, TypeError>
{
    match dir {
        | Dir::Infer => Ok(constructed),
        | Dir::Check(expected) => {
            if bool::from(comp_subtype(&constructed, &expected)) {
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
    sub: &CompType,
    sup: &CompType,
) -> SubtypeDecision
{
    subtype_goals(vec![SubtypeGoal::Comp(
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
/// ([`crate::nbe::conv::type_converts`]) in one call each, because invariance
/// is what conversion decides and a two-way subtyping pass was only ever
/// spelling that out. Endpoints go the same way
/// ([`crate::nbe::conv::converts`]), so they now relate up to beta.
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
fn subtype_goals(mut goals: Vec<SubtypeGoal>) -> SubtypeDecision
{
    // The normalizer is minted only if an invariant position is actually met,
    // and it is then reused for every one in this run. It carries the **empty**
    // definitional environment, and that is correct rather than convenient: the
    // core term language has no definition form at all, so nothing the checker
    // compares can have an unfolding rule. A populated environment belongs to
    // the elaborator, which reaches conversion through the normalizer's own
    // entry points rather than through this relation.
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
                        let nbe = nbe.get_or_insert_with(Normalizer::new);
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
                            Rc::new(crate::judgements::identity::subst_valuetype(
                                lo_snd,
                                NameRef::from(lo_binder.as_str()),
                                &crate::term::syntax::Value::var(NameRef::from(hi_binder.as_str())),
                            ))
                        };
                        let nbe = nbe.get_or_insert_with(Normalizer::new);
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
                    | (
                        &CompType::Arrow(ref lo_arg, ref lo_res),
                        &CompType::Arrow(ref hi_arg, ref hi_res),
                    ) => {
                        goals.push(SubtypeGoal::Comp(Rc::clone(lo_res), Rc::clone(hi_res)));
                        goals.push(SubtypeGoal::Value(Rc::clone(hi_arg), Rc::clone(lo_arg)));
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
pub(crate) fn pick<T>(
    side: crate::term::syntax::Side,
    fst: &Rc<T>,
    snd: &Rc<T>,
) -> T
where
    T: Clone,
{
    match side {
        | crate::term::syntax::Side::Fst => fst.as_ref().clone(),
        | crate::term::syntax::Side::Snd => snd.as_ref().clone(),
    }
}

#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;

    use super::value_subtype;
    use crate::term::types::SealId;
    use crate::term::types::ValueType;

    /// The top-level reflexive entry `value_subtype(&x, &x)` short-circuits
    /// through the `core::ptr::eq` fast-path (ADR-50 Decision B).
    #[test]
    fn value_subtype_reflexive_entry_short_circuits()
    {
        let value_type = ValueType::atom("Foo");
        assert!(bool::from(value_subtype(&value_type, &value_type)));
    }

    /// **The conversion migration, observed through the relation that consumes
    /// it.** Identity endpoints used to be compared by a structural,
    /// no-reduction equality, so two beta-equal endpoints read as distinct and
    /// the two path types did not relate. They now go through the normalizer,
    /// which is the deliberate coarsening the rung was scheduled for.
    #[test]
    fn path_endpoints_relate_up_to_beta_since_the_normalizer_decides_them()
    {
        let redex = Rc::new(crate::term::syntax::Value::Thunk(
            crate::discipline::grade::Grade::ONE,
            Rc::new(crate::term::syntax::Comp::app(
                crate::term::syntax::Comp::lam(
                    "x",
                    crate::term::syntax::Comp::ret(crate::term::syntax::Value::var(
                        crate::discipline::boundary::NameRef::from("x"),
                    )),
                ),
                crate::term::syntax::Value::Int(3),
            )),
        ));
        let contractum = Rc::new(crate::term::syntax::Value::Thunk(
            crate::discipline::grade::Grade::ONE,
            Rc::new(crate::term::syntax::Comp::ret(
                crate::term::syntax::Value::Int(3),
            )),
        ));
        let path = |lhs: &Rc<crate::term::syntax::Value>, rhs: &Rc<crate::term::syntax::Value>| {
            ValueType::Path {
                ty: Rc::new(ValueType::integer()),
                lhs: Rc::clone(lhs),
                rhs: Rc::clone(rhs),
            }
        };
        let reduced = path(&contractum, &contractum);
        let unreduced = path(&redex, &redex);
        assert!(bool::from(value_subtype(&unreduced, &reduced)));
        assert!(bool::from(value_subtype(&reduced, &unreduced)));
        // Endpoints that are genuinely apart still do not relate.
        let apart = path(
            &contractum,
            &Rc::new(crate::term::syntax::Value::Thunk(
                crate::discipline::grade::Grade::ONE,
                Rc::new(crate::term::syntax::Comp::ret(
                    crate::term::syntax::Value::Int(4),
                )),
            )),
        );
        assert!(!bool::from(value_subtype(&reduced, &apart)));
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
        assert!(bool::from(value_subtype(&sigma("x"), &sigma("y"))));
        let other = ValueType::Sigma {
            fst: Rc::new(ValueType::integer()),
            binder: alloc::string::String::from("x"),
            snd: Rc::new(ValueType::integer()),
        };
        assert!(!bool::from(value_subtype(&sigma("x"), &other)));
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
                !bool::from(value_subtype(&sealed, &representation)),
                "a sealed atom is not a subtype of a candidate representation"
            );
            assert!(
                !bool::from(value_subtype(&representation, &sealed)),
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
            !bool::from(value_subtype(&counter, &gauge)),
            "two sealings do not interchange, however alike their implementations"
        );
        let same = ValueType::Sealed(SealId::new(0_u64, "Counter", "t"));
        assert!(
            !core::ptr::eq(&raw const counter, &raw const same),
            "the two references are distinct allocations"
        );
        assert!(
            bool::from(value_subtype(&counter, &same)),
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
        assert!(bool::from(value_subtype(shared.as_ref(), clone.as_ref())));
    }
}
