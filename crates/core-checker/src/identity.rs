//! Identity-type support.
//!
//! Structural value equality and the value-into-type substitution that
//! instantiates the [`Comp::Walk`](crate::syntax::Comp::Walk) motive (ADR-76;
//! `proposal-identity-univalence.md` §4).
//!
//! `Path A x y` is gandr's first dependent former — terms occur inside a type —
//! so two pieces of machinery the non-dependent core never needed arrive here:
//!
//! * **structural value equality** [`value_eq`], the `≡ᵥ` the identity
//!   subtyping arm ([`crate::subtype::value_subtype`]) decides its endpoints
//!   with; and
//! * **value-into-type substitution** ([`subst_valuetype`] /
//!   [`subst_comptype`]), which the identity eliminator's typing rule
//!   ([`crate::checker`] / [`crate::machine`]) drives to form the base's
//!   expected type `C[x/y][here(x)/q]` and the result type `C[a/x][b/y][p/q]`.
//!
//! At rung 1 (`proposal-identity-univalence.md` §4.3) there is **no reduction
//! inside a type**: `value_eq` is α-respecting structural equality with a
//! pointer-equality fast path and hole/annotation consistency, nothing more;
//! the NbE-era definitional equality that adds Walk-β,
//! congruence, and substitution laws is future work. Types carry **no binders**
//! (`Path A x y` does not bind `x`/`y` — they are value *occurrences*), so type
//! substitution is capture-free structural recursion that delegates the one
//! binder-bearing case — a value under a thunk — to the proven capture-avoiding
//! engine [`crate::eval::subst_value`]. Both `value_eq` and the type
//! substitution engine run as explicit heap worklists (the ADR-47 iterative
//! discipline), so even an adversarially deep value or type compares and
//! substitutes without overflowing the host call stack.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use crate::boundary::NameRef;
use crate::boundary::ValueEquality;
use crate::effect::EffectRow;
use crate::eval::subst_value;
use crate::grade::Grade;
use crate::syntax::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// Decides the structural value equality `x ≡ᵥ y` the identity type's endpoints
/// are compared with (ADR-76; the `≡ᵥ` of [`crate::types::ValueType::Path`]).
///
/// This is **not** subtyping: it is a symmetric, reflexive equality on values,
/// the rung-1 (no-reduction) shadow of the NbE-era definitional equality on
/// `Path` endpoints. It composes with the gradual discipline exactly as
/// [`crate::subtype::value_subtype`] does: a [`Value::Hole`] is consistent with
/// any value (both directions), and an annotation `(v : A)` is transparent.
/// Otherwise it descends structurally, with a [`core::ptr::eq`] reflexive
/// fast-path so an aliased `Rc` short-circuits (the ADR-50 Decision B idiom).
///
/// # Contract
/// - ensures: returns `true` iff `lhs` and `rhs` are structurally equal after
///   annotation-peeling, treating any [`Value::Hole`] as consistent with every
///   value. Reflexive (`value_eq(v, v)` for every `v`) and symmetric; NOT
///   transitive once a hole participates (the gradual-consistency price,
///   exactly as `value_subtype`).
/// - panics: none.
#[inline]
#[must_use]
/// # Termination
/// - reason: structural equality drains an explicit finite comparison stack.
/// - measure: remaining queued child pairs under comparison.
/// - boundedness: compared terms and types are finite Rust values.
/// - input recursion: none.
pub fn value_eq(
    lhs: &Value,
    rhs: &Value,
) -> ValueEquality
{
    let mut work = vec![(lhs, rhs)];
    while let Some((left, right)) = work.pop() {
        if core::ptr::eq(left, right) {
            continue;
        }
        let left = peel(left);
        let right = peel(right);
        if matches!(*left, Value::Hole(_)) || matches!(*right, Value::Hole(_)) {
            continue;
        }
        match (left, right) {
            | (&Value::Pair(ref a_fst, ref a_snd), &Value::Pair(ref b_fst, ref b_snd)) => {
                work.push((a_fst, b_fst));
                work.push((a_snd, b_snd));
            },
            | (&Value::Inj(a_side, ref a_payload), &Value::Inj(b_side, ref b_payload)) => {
                if a_side != b_side {
                    return false.into();
                }
                work.push((a_payload, b_payload));
            },
            | (&Value::List(ref a_elems), &Value::List(ref b_elems)) => {
                if a_elems.len() != b_elems.len() {
                    return false.into();
                }
                work.extend(
                    a_elems
                        .iter()
                        .zip(b_elems.iter())
                        .map(|(a_elem, b_elem)| (a_elem.as_ref(), b_elem.as_ref())),
                );
            },
            | (&Value::Record(ref a_fields), &Value::Record(ref b_fields)) => {
                if a_fields.len() != b_fields.len() {
                    return false.into();
                }
                for (label, a_field) in a_fields {
                    let Some(b_field) = b_fields.get(label)
                    else {
                        return false.into();
                    };
                    work.push((a_field.as_ref(), b_field.as_ref()));
                }
            },
            | (&Value::Here(ref a_witness), &Value::Here(ref b_witness)) => {
                work.push((a_witness, b_witness));
            },
            | (
                &Value::Ctor {
                    id: ref a_id,
                    tag: a_tag,
                    payload: ref a_payload,
                },
                &Value::Ctor {
                    id: ref b_id,
                    tag: b_tag,
                    payload: ref b_payload,
                },
            ) => {
                if a_id != b_id || a_tag != b_tag {
                    return false.into();
                }
                work.push((a_payload, b_payload));
            },
            | (other_lhs, other_rhs) if other_lhs == other_rhs => {},
            | _ => return false.into(),
        }
    }
    true.into()
}

/// Peels type-annotation layers `(v : A)` off a value, returning the innermost
/// non-[`Value::Annot`] value — an ascription is transparent to identity, so
/// `value_eq` compares through it (mirroring the private `Value::peeled`).
#[inline]
#[must_use]
fn peel(value: &Value) -> &Value
{
    let mut current = value;
    while let Value::Annot(ref inner, _) = *current {
        current = inner;
    }
    current
}

/// Substitutes the value `repl` for the free value variable `name` inside a
/// **value type**, the type-level half of the identity eliminator's motive
/// instantiation (ADR-76).
///
/// Types carry no binders, so this is capture-free structural recursion; the
/// only place a value occurs inside a type is an [`ValueType::Path`] endpoint
/// (and its carrier), where the substitution delegates to the capture-avoiding
/// value engine [`crate::eval::subst_value`]. Every non-`Path` former simply
/// rebuilds its children.
///
/// # Contract
/// - ensures: returns `ty` with every free `name` in an `Path` endpoint (at any
///   depth) replaced by `repl`; identical to `ty` when `name` does not occur.
/// - panics: none.
#[inline]
#[must_use]
/// # Termination
/// - reason: substitution rebuilds finite type trees from an explicit worklist.
/// - measure: pending type tasks and result frames.
/// - boundedness: source types are finite Rust values.
/// - input recursion: none.
pub fn subst_valuetype<'source, N>(
    ty: &ValueType,
    name: N,
    repl: &Value,
) -> ValueType
where
    N: Into<NameRef<'source>> + Copy,
{
    let (mut values, _) = subst_type(&TypeRoot::Value(ty), name.into(), repl);
    pop_value_type(&mut values)
}

/// Substitutes the value `repl` for the free value variable `name` inside a
/// **computation type** — the entry the
/// [`Comp::Walk`](crate::syntax::Comp::Walk) motive is instantiated through
/// (ADR-76).
///
/// The computation-type analogue of [`subst_valuetype`]: capture-free
/// structural recursion, with value occurrences reached only through the
/// value-type children (`F`'s payload, `Arrow`'s argument).
///
/// # Contract
/// - ensures: returns `ty` with every free `name` in a nested `Path` endpoint
///   replaced by `repl`; identical to `ty` when `name` does not occur.
/// - panics: none.
#[inline]
#[must_use]
/// # Termination
/// - reason: substitution rebuilds finite type trees from an explicit worklist.
/// - measure: pending type tasks and result frames.
/// - boundedness: source types are finite Rust values.
/// - input recursion: none.
pub fn subst_comptype<'source, N>(
    ty: &CompType,
    name: N,
    repl: &Value,
) -> CompType
where
    N: Into<NameRef<'source>> + Copy,
{
    let (_, mut comps) = subst_type(&TypeRoot::Comp(ty), name.into(), repl);
    pop_comp_type(&mut comps)
}

/// The root sort of a [`subst_type`] run — the entry-point tag selecting which
/// result stack the single rebuilt root type lands on.
enum TypeRoot<'type_>
{
    /// A value-type root: the run leaves exactly one rebuilt [`ValueType`].
    Value(&'type_ ValueType),
    /// A computation-type root: the run leaves exactly one rebuilt
    /// [`CompType`].
    Comp(&'type_ CompType),
}

/// One task on the [`subst_type`] worklist — the defunctionalized image of a
/// recursive `subst_valuetype` / `subst_comptype` call (ADR-47 T1). A
/// `Value` / `Comp` task visits a source type, rebuilding a leaf directly or
/// pushing a `Finish*` frame followed by its children; a `Finish*` frame
/// reassembles one node from the rebuilt children on the result stacks.
enum TypeTask<'type_>
{
    /// Visit a value type.
    Value(&'type_ ValueType),
    /// Visit a computation type.
    Comp(&'type_ CompType),
    /// Reassemble a value type from its rebuilt children.
    FinishValue(ValueFinish<'type_>),
    /// Reassemble a computation type from its rebuilt children.
    FinishComp(CompFinish),
}

/// A pending value-type reassembly — everything a [`TypeTask::FinishValue`]
/// frame needs to rebuild one [`ValueType`] node once its substituted children
/// sit on the result stack.
enum ValueFinish<'type_>
{
    /// Rebuild a product from its rebuilt `fst` and `snd`.
    Prod,
    /// Rebuild a sum from its rebuilt `lhs` and `rhs`.
    Sum,
    /// Rebuild a list type from its rebuilt element type.
    List,
    /// Rebuild a record type from its rebuilt fields, in the order of the
    /// saved labels.
    Record(Vec<String>),
    /// Rebuild a thunk type from its grade and rebuilt body.
    Thunk(Grade),
    /// Rebuild a stack type from its rebuilt consumes and delivers.
    Stk,
    /// Rebuild an identity type from its rebuilt carrier, substituting the
    /// borrowed endpoint values through [`subst_value`].
    Path(&'type_ Value, &'type_ Value),
    /// Rebuild a declared-data application from its id and rebuilt arguments
    /// (the `usize` is the argument count).
    Data(crate::types::DataId, usize),
    /// Rebuild a dependent pair `Σ(x : A). B`, substituting into the tail only
    /// when the binder does not shadow the substituted name.
    Sigma
    {
        /// The bound variable, kept verbatim in the rebuild.
        binder: String,
        /// The original tail, reused untouched when `binder` shadows the
        /// substituted name.
        original_snd: &'type_ Rc<ValueType>,
        /// Whether the tail substitutes (`binder` differs from the substituted
        /// name).
        substitute_snd: bool,
    },
}

/// A pending computation-type reassembly — everything a
/// [`TypeTask::FinishComp`] frame needs to rebuild one [`CompType`] node once
/// its substituted children sit on the result stacks.
enum CompFinish
{
    /// Rebuild a returner from its rebuilt payload and effect row.
    F(EffectRow),
    /// Rebuild an arrow from its rebuilt argument and result.
    Arrow,
    /// Rebuild a `with` type from its rebuilt components.
    With,
}

/// The iterative value-into-type substitution engine (ADR-47 T1): the shared
/// driver behind [`subst_valuetype`] / [`subst_comptype`]. It drains an
/// explicit LIFO task stack onto one result stack per type sort, so
/// substitution depth follows the heap, not the host call stack — the
/// iterative shadow of the mutually-recursive map this module specifies.
///
/// # Contract
/// - ensures: after the worklist drains, the result stack matching `root`'s
///   sort holds exactly one rebuilt type and the other stack is empty (the
///   post-order balance invariant); every free `name` in a `Path` endpoint of
///   the input has been replaced by `repl`.
/// - panics: none (a `debug_assert` in the `pop_*` helpers guards the balance
///   invariant in debug / test builds).
///
/// # Termination
/// - reason: the engine drains a finite task stack.
/// - measure: pending type tasks and result frames.
/// - boundedness: source types are finite Rust values.
/// - input recursion: none.
fn subst_type(
    root: &TypeRoot<'_>,
    name: NameRef<'_>,
    repl: &Value,
) -> (Vec<ValueType>, Vec<CompType>)
{
    let mut tasks = match *root {
        | TypeRoot::Value(ty) => vec![TypeTask::Value(ty)],
        | TypeRoot::Comp(ty) => vec![TypeTask::Comp(ty)],
    };
    let mut values = Vec::new();
    let mut comps = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            | TypeTask::Value(ty) => match *ty {
                | ValueType::Atom(_)
                | ValueType::Unit
                | ValueType::Unknown
                | ValueType::Universe => {
                    values.push(ty.clone());
                },
                | ValueType::Prod(ref fst, ref snd) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Prod));
                    tasks.push(TypeTask::Value(snd));
                    tasks.push(TypeTask::Value(fst));
                },
                | ValueType::Sum(ref lhs, ref rhs) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Sum));
                    tasks.push(TypeTask::Value(rhs));
                    tasks.push(TypeTask::Value(lhs));
                },
                | ValueType::List(ref element) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::List));
                    tasks.push(TypeTask::Value(element));
                },
                | ValueType::Record(ref fields) => {
                    let labels = fields.keys().cloned().collect::<Vec<_>>();
                    tasks.push(TypeTask::FinishValue(ValueFinish::Record(labels)));
                    for field in fields.values().rev() {
                        tasks.push(TypeTask::Value(field));
                    }
                },
                | ValueType::Thunk(grade, ref body) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Thunk(grade)));
                    tasks.push(TypeTask::Comp(body));
                },
                | ValueType::Stk(ref consumes, ref delivers) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Stk));
                    tasks.push(TypeTask::Comp(delivers));
                    tasks.push(TypeTask::Comp(consumes));
                },
                | ValueType::Path {
                    ty: ref carrier,
                    ref lhs,
                    ref rhs,
                } => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Path(lhs, rhs)));
                    tasks.push(TypeTask::Value(carrier));
                },
                | ValueType::Data { ref id, ref args } => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Data(
                        id.clone(),
                        args.len(),
                    )));
                    for arg in args.iter().rev() {
                        tasks.push(TypeTask::Value(arg));
                    }
                },
                | ValueType::Sigma {
                    ref fst,
                    ref binder,
                    ref snd,
                } => {
                    let substitute_snd = binder != name.as_ref();
                    tasks.push(TypeTask::FinishValue(ValueFinish::Sigma {
                        binder: binder.clone(),
                        original_snd: snd,
                        substitute_snd,
                    }));
                    if substitute_snd {
                        tasks.push(TypeTask::Value(snd));
                    }
                    tasks.push(TypeTask::Value(fst));
                },
            },
            | TypeTask::Comp(ty) => match *ty {
                | CompType::Unknown => comps.push(CompType::Unknown),
                | CompType::F(ref of, ref row) => {
                    tasks.push(TypeTask::FinishComp(CompFinish::F(row.clone())));
                    tasks.push(TypeTask::Value(of));
                },
                | CompType::Arrow(ref arg, ref res) => {
                    tasks.push(TypeTask::FinishComp(CompFinish::Arrow));
                    tasks.push(TypeTask::Comp(res));
                    tasks.push(TypeTask::Value(arg));
                },
                | CompType::With(ref fst, ref snd) => {
                    tasks.push(TypeTask::FinishComp(CompFinish::With));
                    tasks.push(TypeTask::Comp(snd));
                    tasks.push(TypeTask::Comp(fst));
                },
            },
            | TypeTask::FinishValue(finish) => match finish {
                | ValueFinish::Prod => {
                    let snd = pop_value_type(&mut values);
                    let fst = pop_value_type(&mut values);
                    values.push(ValueType::Prod(Rc::new(fst), Rc::new(snd)));
                },
                | ValueFinish::Sum => {
                    let rhs = pop_value_type(&mut values);
                    let lhs = pop_value_type(&mut values);
                    values.push(ValueType::Sum(Rc::new(lhs), Rc::new(rhs)));
                },
                | ValueFinish::List => {
                    let element = pop_value_type(&mut values);
                    values.push(ValueType::List(Rc::new(element)));
                },
                | ValueFinish::Record(labels) => {
                    let mut fields = BTreeMap::new();
                    let mut rebuilt = Vec::with_capacity(labels.len());
                    for _ in 0 .. labels.len() {
                        rebuilt.push(pop_value_type(&mut values));
                    }
                    rebuilt.reverse();
                    for (label, field) in labels.into_iter().zip(rebuilt) {
                        fields.insert(label, Rc::new(field));
                    }
                    values.push(ValueType::Record(fields));
                },
                | ValueFinish::Thunk(grade) => {
                    let body = pop_comp_type(&mut comps);
                    values.push(ValueType::Thunk(grade, Rc::new(body)));
                },
                | ValueFinish::Stk => {
                    let delivers = pop_comp_type(&mut comps);
                    let consumes = pop_comp_type(&mut comps);
                    values.push(ValueType::Stk(Rc::new(consumes), Rc::new(delivers)));
                },
                | ValueFinish::Path(lhs, rhs) => {
                    let carrier = pop_value_type(&mut values);
                    values.push(ValueType::Path {
                        ty: Rc::new(carrier),
                        lhs: Rc::new(subst_value(lhs, name, repl)),
                        rhs: Rc::new(subst_value(rhs, name, repl)),
                    });
                },
                | ValueFinish::Data(id, count) => {
                    let mut args = Vec::with_capacity(count);
                    for _ in 0 .. count {
                        args.push(pop_value_type(&mut values));
                    }
                    args.reverse();
                    values.push(ValueType::Data {
                        id,
                        args: args.into_iter().map(Rc::new).collect(),
                    });
                },
                | ValueFinish::Sigma {
                    binder,
                    original_snd,
                    substitute_snd,
                } => {
                    let fst = pop_value_type(&mut values);
                    let snd = if substitute_snd {
                        Rc::new(pop_value_type(&mut values))
                    }
                    else {
                        Rc::clone(original_snd)
                    };
                    values.push(ValueType::Sigma {
                        fst: Rc::new(fst),
                        binder,
                        snd,
                    });
                },
            },
            | TypeTask::FinishComp(finish) => match finish {
                | CompFinish::F(row) => {
                    let of = pop_value_type(&mut values);
                    comps.push(CompType::F(Rc::new(of), row));
                },
                | CompFinish::Arrow => {
                    let res = pop_comp_type(&mut comps);
                    let arg = pop_value_type(&mut values);
                    comps.push(CompType::Arrow(Rc::new(arg), Rc::new(res)));
                },
                | CompFinish::With => {
                    let snd = pop_comp_type(&mut comps);
                    let fst = pop_comp_type(&mut comps);
                    comps.push(CompType::With(Rc::new(fst), Rc::new(snd)));
                },
            },
        }
    }
    (values, comps)
}

/// Pops the most-recent rebuilt value type from the result stack, with a
/// balance-invariant guard (the fallback is never reached — a desync would
/// fail the substitution tests first).
fn pop_value_type(values: &mut Vec<ValueType>) -> ValueType
{
    debug_assert!(
        !values.is_empty(),
        "subst_type worklist underflow (post-order balance)"
    );
    values.pop().unwrap_or(ValueType::Unknown)
}

/// Pops the most-recent rebuilt computation type from the result stack (see
/// [`pop_value_type`]).
fn pop_comp_type(comps: &mut Vec<CompType>) -> CompType
{
    debug_assert!(
        !comps.is_empty(),
        "subst_type worklist underflow (post-order balance)"
    );
    comps.pop().unwrap_or(CompType::Unknown)
}
