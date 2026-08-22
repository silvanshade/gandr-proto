//! Identity-type support: the value-into-type substitution that instantiates
//! the [`Comp::Walk`](crate::syntax::Comp::Walk) motive (ADR-76; the
//! identity and univalence design's §4).
//!
//! `Path A x y` is gandr's first dependent former — terms occur inside a type —
//! and it needs two pieces of machinery the non-dependent core never did. One
//! of them lives here.
//!
//! * **Value-into-type substitution** ([`subst_valuetype`] /
//!   [`subst_comptype`]) is this module's: the identity eliminator's typing
//!   rule (`gandr_core_checker::judgements::checker` / `gandr_core_machine`)
//!   drives it to form the base's expected type `C[x/y][here(x)/q]` and the
//!   result type `C[a/x][b/y][p/q]`. The dependent-application rule drives the
//!   same entry to instantiate a `Π` codomain at its argument.
//! * **Definitional equality on the endpoints** is not. The `≡ᵥ` the identity
//!   subtyping arm decides its endpoints with is now
//!   `gandr_core_nbe::conv::converts`, the normalizer's own relation. The
//!   structural, no-reduction equality that stood in for it at rung 1 is
//!   **retired**: it decided a strictly weaker relation than the language's,
//!   and keeping both would have meant two definitional equalities in one
//!   checker.
//!
//! # What binds inside a type
//!
//! Most formers bind nothing: `Path A x y` does not bind `x`/`y` — they are
//! value *occurrences* — and every structural former simply rebuilds its
//! children. Two formers do bind a **value** variable, and this engine's
//! shadowing discipline is what handles them:
//!
//! * [`ValueType::Sigma`] binds its head variable in its tail.
//! * [`CompType::Arrow`] binds its argument variable in its codomain when it
//!   carries a `Π` binder; the non-dependent arrow carries none and always
//!   descends.
//!
//! In both cases a binder of the substituted name **shadows** it, so the body
//! is reused untouched rather than substituted into. [`ValueType::Package`]
//! binds **type** names, which this engine never substitutes for, so its
//! payload is always traversed.
//!
//! The remaining value occurrences are reached inside `Path` endpoints and
//! under a thunk, both of which delegate to the shadowing-aware engine
//! [`crate::subst::subst_value`]. Every binder-bearing case therefore shares
//! one discipline rather than growing a second.
//!
//! The substitution engine runs as an explicit heap worklist (the ADR-47
//! iterative discipline), so even an adversarially deep type substitutes
//! without overflowing the host call stack.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use crate::boundary::FreeOccurrence;
use crate::boundary::NameRef;
use crate::effect::EffectRow;
use crate::grade::Grade;
use crate::static_term::FamilyApp;
use crate::subst::subst_value;
use crate::syntax::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// Substitutes the value `repl` for the free value variable `name` inside a
/// **value type**, the type-level half of the identity eliminator's motive
/// instantiation (ADR-76).
///
/// Structural recursion under the module's shadowing discipline: a value
/// occurs inside a type at an [`ValueType::Path`] endpoint (and its carrier),
/// where the substitution delegates to [`crate::subst::subst_value`], and a
/// binder of the substituted name at a [`ValueType::Sigma`] tail or a `Π`
/// codomain blocks the descent. Every other former simply rebuilds its
/// children.
///
/// # Contract
/// - ensures: returns `ty` with every free `name` in a `Path` endpoint (at any
///   depth) replaced by `repl`, leaving occurrences under a `Σ` tail or `Π`
///   codomain that rebinds `name` untouched; identical to `ty` when `name` does
///   not occur free.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — each value-type former is rebuilt without changing its
///   structure, only `Path` endpoints substitute, and a binder of the
///   substituted name blocks the descent into its body.
/// - witness: the identity conformance tests for endpoint substitution
/// - witness: `tests::stack_types_rebuild_both_computation_children`
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
/// [`Comp::Walk`](crate::syntax::Comp::Walk) motive is instantiated
/// through (ADR-76).
///
/// The computation-type analogue of [`subst_valuetype`]: structural recursion
/// under the same shadowing discipline, with value occurrences reached through
/// the value-type children (`F`'s payload, a function type's argument) and a
/// `Π` binder of the substituted name blocking the descent into its codomain.
///
/// # Contract
/// - ensures: returns `ty` with every free `name` in a nested `Path` endpoint
///   replaced by `repl`, leaving occurrences under a `Σ` tail or `Π` codomain
///   that rebinds `name` untouched; identical to `ty` when `name` does not
///   occur free.
/// - ensures: **no free variable of `repl` is captured** by a binder the
///   substitution descends under; a binder that would capture one is renamed
///   apart first. Shadowing and capture are different obligations and only the
///   first was met: shadowing asks whether the binder rebinds the substituted
///   NAME, capture asks whether it rebinds a name the substituted VALUE
///   mentions. `gandr-ijdw`.
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

/// Whether the value variable `name` occurs **free** in a computation type.
///
/// The occurrence question conversion asks when it meets a `Π` on one side and
/// a plain arrow on the other: the two classify the same functions exactly when
/// the `Π` binder is unused, and that is the only place in the tree where the
/// question is posed. It is deliberately **not** asked at construction — the
/// [`CompType::Arrow`] docs say why a written binder beats a derived one.
///
/// Free means free: a `Σ` tail or `Π` codomain that rebinds `name` hides every
/// occurrence beneath it, exactly as the substitution engine's shadowing
/// discipline does, so the two agree by construction.
///
/// # Contract
/// - ensures: returns `true` iff `name` occurs in a value position of `ty` that
///   no enclosing binder of `name` shadows.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the walk reaches exactly the value positions
///   [`subst_comptype`] would rewrite, and stops at exactly the binders it
///   would decline to descend under.
/// - witness: `tests::pi_binder_occurrence_is_shadowed_by_an_inner_binder`
/// - witness: `tests::pi_binder_occurrence_is_seen_through_a_path_endpoint`
#[inline]
#[must_use]
/// # Termination
/// - reason: the walk drains an explicit worklist over finite type trees.
/// - measure: pending type tasks.
/// - boundedness: source types are finite Rust values.
/// - input recursion: none.
pub fn occurs_free_comptype<'source, N>(
    ty: &CompType,
    name: N,
) -> FreeOccurrence
where
    N: Into<NameRef<'source>> + Copy,
{
    // Substituting a *distinct* fresh variable for `name` changes the type
    // exactly when `name` occurred free in it. Reusing the substitution engine
    // rather than writing a second traversal is what keeps the two answers from
    // drifting apart: the shadowing discipline is stated once.
    let probe = Value::Var(alloc::string::String::from(OCCURRENCE_PROBE));
    FreeOccurrence::from(subst_comptype(ty, name, &probe) != *ty)
}

/// The variable name [`occurs_free_comptype`] substitutes in to detect an
/// occurrence.
///
/// It must be a name no source can write, or the probe would compare equal to a
/// genuine occurrence of itself and report `false` for a variable that does
/// occur. The surface grammar admits no space in an identifier, so this name is
/// unwritable rather than merely unlikely.
const OCCURRENCE_PROBE: &str = "occurrence probe";

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
    FinishComp(CompFinish<'type_>),
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
    /// Rebuild a classifier-bearing family application, substituting value
    /// indices through its static neutral spine.
    Family(FamilyApp),
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
    /// Rebuild a package `Package_r ⟨ᾱ⟩ A` from its grade, its binder labels,
    /// and its rebuilt payload.
    ///
    /// No shadowing test is needed and that is worth stating rather than
    /// leaving to inference: a package binds **type** names, and this engine
    /// substitutes a **value** for a value variable, so the two binder spaces
    /// do not meet. The payload is therefore always traversed.
    Package(Grade, &'type_ [String]),
}

/// A pending computation-type reassembly — everything a
/// [`TypeTask::FinishComp`] frame needs to rebuild one [`CompType`] node once
/// its substituted children sit on the result stacks.
enum CompFinish<'type_>
{
    /// Rebuild a returner from its rebuilt payload and effect row.
    F(EffectRow),
    /// Rebuild a function type from its rebuilt argument and result,
    /// substituting into the result only when the binder does not shadow the
    /// substituted name.
    ///
    /// The non-dependent arrow carries `binder: None` and always substitutes,
    /// which is the pre-`Π` behaviour unchanged.
    Arrow
    {
        /// The bound variable, kept verbatim in the rebuild.
        binder: Option<String>,
        /// The original result, reused untouched when `binder` shadows the
        /// substituted name.
        original_res: &'type_ Rc<CompType>,
        /// Whether the result substitutes.
        substitute_res: bool,
    },
    /// Rebuild a `with` type from its rebuilt components.
    With,
    /// Rebuild a computation-family application after substituting its value
    /// arguments.
    Family(FamilyApp),
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
                | ValueType::Atom(ref atom) if atom == name.as_ref() => {
                    match *repl {
                        | Value::Var(ref replacement) => {
                            values.push(ValueType::Atom(replacement.clone()));
                        },
                        | _ => values.push(ty.clone()),
                    }
                },
                | ValueType::Atom(_)
                | ValueType::Unit
                | ValueType::Unknown
                | ValueType::Universe { .. }
                // A sealed atom is a leaf with no children to substitute into:
                // it carries an identity and nothing else, so substitution
                // passes it through unchanged. That is the same shape opacity
                // takes everywhere — an absence of structure rather than a
                // guard against reaching it.
                | ValueType::Sealed(_) => {
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
                | ValueType::Family(ref application) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Family(
                        application.clone(),
                    )));
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
                | ValueType::Package {
                    grade,
                    ref abstracts,
                    ref payload,
                } => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Package(grade, abstracts)));
                    tasks.push(TypeTask::Value(payload));
                },
            },
            | TypeTask::Comp(ty) => match *ty {
                | CompType::Unknown => comps.push(CompType::Unknown),
                | CompType::F(ref of, ref row) => {
                    tasks.push(TypeTask::FinishComp(CompFinish::F(row.clone())));
                    tasks.push(TypeTask::Value(of));
                },
                | CompType::Arrow {
                    ref binder,
                    ref arg,
                    ref res,
                } => {
                    // gandr-ijdw: this handles SHADOWING and not CAPTURE. The
                    // binder blocks the descent when it rebinds the substituted
                    // NAME, which is right; nothing here asks whether `repl`'s
                    // own free variables would be captured by the binder, and
                    // that is the defect. Applying a dependent spine walks one
                    // argument at a time, so a caller whose type-variable names
                    // collide with the callee's binders substitutes a name that
                    // the next binder then rebinds — and the following argument
                    // rewrites it again. The witness pair is on the bead.
                    //
                    // The repair is capture-avoiding descent: rename the binder
                    // to a name free in both the body and `repl` before
                    // descending. A rename is the only correct move here; a
                    // refusal would reject correct programs, and simultaneous
                    // substitution at the application site would fix this spine
                    // and leave every other caller of `subst_comptype` wrong.
                    //
                    // A `Π` binder of the substituted name shadows it in the
                    // codomain, exactly as `Σ`'s does in its tail; a `None`
                    // binder shadows nothing and always descends.
                    let substitute_res =
                        binder.as_deref().is_none_or(|bound| bound != name.as_ref());
                    tasks.push(TypeTask::FinishComp(CompFinish::Arrow {
                        binder: binder.clone(),
                        original_res: res,
                        substitute_res,
                    }));
                    if substitute_res {
                        tasks.push(TypeTask::Comp(res));
                    }
                    tasks.push(TypeTask::Value(arg));
                },
                | CompType::With(ref fst, ref snd) => {
                    tasks.push(TypeTask::FinishComp(CompFinish::With));
                    tasks.push(TypeTask::Comp(snd));
                    tasks.push(TypeTask::Comp(fst));
                },
                | CompType::Family(ref application) => {
                    tasks.push(TypeTask::FinishComp(CompFinish::Family(
                        application.clone(),
                    )));
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
                | ValueFinish::Family(application) => {
                    values.push(ValueType::Family(application.substitute_value(name, repl)));
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
                | ValueFinish::Package(grade, abstracts) => {
                    let payload = pop_value_type(&mut values);
                    values.push(ValueType::Package {
                        grade,
                        abstracts: abstracts.to_vec(),
                        payload: Rc::new(payload),
                    });
                },
            },
            | TypeTask::FinishComp(finish) => match finish {
                | CompFinish::F(row) => {
                    let of = pop_value_type(&mut values);
                    comps.push(CompType::F(Rc::new(of), row));
                },
                | CompFinish::Arrow {
                    binder,
                    original_res,
                    substitute_res,
                } => {
                    let res = if substitute_res {
                        Rc::new(pop_comp_type(&mut comps))
                    }
                    else {
                        Rc::clone(original_res)
                    };
                    let arg = pop_value_type(&mut values);
                    comps.push(CompType::Arrow {
                        binder,
                        arg: Rc::new(arg),
                        res,
                    });
                },
                | CompFinish::With => {
                    let snd = pop_comp_type(&mut comps);
                    let fst = pop_comp_type(&mut comps);
                    comps.push(CompType::With(Rc::new(fst), Rc::new(snd)));
                },
                | CompFinish::Family(application) => {
                    comps.push(CompType::Family(application.substitute_value(name, repl)));
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

#[cfg(test)]
mod tests
{
    use super::*;

    /// Substituting a value whose free variable a binder below would rebind
    /// renames that binder apart rather than capturing.
    ///
    /// Shadowing and capture are different obligations, and the engine met only
    /// the first: shadowing asks whether the binder rebinds the substituted
    /// name, capture asks whether it rebinds a name the substituted value
    /// mentions. `gandr-ijdw`.
    #[ignore = "gandr-ijdw: scaffold; the body is owed with this rung"]
    #[test]
    fn substitution_renames_a_binder_that_would_capture_the_replacement()
    {
        todo!("gandr-ijdw")
    }

    /// The **wrong-acceptance** direction: two types that must not agree, which
    /// capturing substitution makes coincide.
    ///
    /// This is the witness that matters. A repair verified only against the
    /// program capture broke proves the capture stopped rejecting something
    /// correct; it says nothing about whether capture was also *accepting*
    /// something wrong, and capture can make two distinct types agree exactly
    /// as easily as it makes two equal ones diverge.
    ///
    /// The separating source, in surface spelling: applying
    /// `comp(a, b, c, f, g, x)` at indices `(a, c, d)` should demand `g` at
    /// `U[ω] (c -> F d)`. Capturing substitution rewrites that expectation to
    /// `U[ω] (d -> F d)`, so an argument written at the captured type is
    /// accepted. The fixed engine must refuse it.
    #[ignore = "gandr-ijdw: scaffold; the body is owed with this rung"]
    #[test]
    fn a_captured_expectation_does_not_accept_the_wrong_argument()
    {
        todo!("gandr-ijdw")
    }

    /// A `Π` binder shadowing the queried name hides every occurrence beneath
    /// it, so the occurrence walk and the substitution engine agree.
    #[test]
    fn pi_binder_occurrence_is_shadowed_by_an_inner_binder()
    {
        let endpoint = Value::Var(String::from("x"));
        let inner = CompType::F(
            Rc::new(ValueType::Path {
                ty: Rc::new(ValueType::Unit),
                lhs: Rc::new(endpoint.clone()),
                rhs: Rc::new(endpoint),
            }),
            EffectRow::EMPTY,
        );
        // `Π(x : 1). F (Path 1 x x)` binds the very name being queried.
        let shadowed = CompType::pi("x", ValueType::Unit, inner);

        assert!(
            !bool::from(occurs_free_comptype(&shadowed, NameRef::from("x"))),
            "a Pi binder of the queried name shadows every occurrence in its codomain"
        );
    }

    /// The walk reaches a value position inside a type — the `Path` endpoint
    /// case the substitution engine also reaches.
    #[test]
    fn pi_binder_occurrence_is_seen_through_a_path_endpoint()
    {
        let endpoint = Value::Var(String::from("a"));
        let codomain = CompType::F(
            Rc::new(ValueType::Path {
                ty: Rc::new(ValueType::Unit),
                lhs: Rc::new(endpoint.clone()),
                rhs: Rc::new(endpoint),
            }),
            EffectRow::EMPTY,
        );
        // `Π(b : 1). F (Path 1 a a)` leaves `a` free under a binder for `b`.
        let dependent = CompType::pi("b", ValueType::Unit, codomain);

        assert!(
            bool::from(occurs_free_comptype(&dependent, NameRef::from("a"))),
            "a free endpoint occurrence is visible through an unrelated binder"
        );
        assert!(
            !bool::from(occurs_free_comptype(&dependent, NameRef::from("b"))),
            "the bound variable itself does not occur in this codomain"
        );
    }

    #[test]
    fn stack_types_rebuild_both_computation_children()
    {
        let consumes = CompType::Unknown;
        let delivers = CompType::F(Rc::new(ValueType::Unit), EffectRow::EMPTY);
        let stack = ValueType::Stk(Rc::new(consumes), Rc::new(delivers));

        assert_eq!(
            subst_valuetype(&stack, NameRef::from("absent"), &Value::Unit),
            stack
        );
    }
    #[test]
    fn atom_substitution_replaces_matching_variable()
    {
        let atom = ValueType::Atom(String::from("a"));

        assert_eq!(
            subst_valuetype(&atom, NameRef::from("a"), &Value::Var(String::from("b"))),
            ValueType::Atom(String::from("b"))
        );
    }
}
