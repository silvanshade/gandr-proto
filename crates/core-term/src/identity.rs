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
//! # Shadowing and capture are different obligations
//!
//! Shadowing asks whether the binder rebinds the substituted **name**. Capture
//! asks whether it rebinds a name the substituted **value** mentions, and the
//! answer decides whether descending under the binder is sound at all: a
//! replacement carrying a free `c` written under a binder spelled `c` stops
//! meaning what the caller wrote. So a binder that would capture is renamed
//! apart before the descent, to a name carrying a space and therefore outside
//! the identifiers any source can write.
//!
//! This is not a corner. Applying a dependent spine instantiates one parameter
//! at a time, so a caller whose type variables are spelled like the callee's
//! binders substitutes a name the next binder rebinds and the following
//! argument rewrites again — and every category law written the obvious way
//! spells its objects the way the composition operation spells its own.
//!
//! # Who chooses the fresh name, and why it is not the engine
//!
//! Renaming needs a name the type does not already use, and asking whether a
//! candidate occurs in a type means running the substitution engine on it. An
//! engine that answered that question for itself would recurse on data its own
//! caller supplied, which is the one thing this crate's walks refuse.
//!
//! So the engine **asks**: it reports the binder it cannot open and stops, the
//! entry point chooses a name and records it, and the pass runs again with the
//! choice in hand. The number of restarts is the number of distinct binder
//! names that collide, which is bounded by the replacement's own free names.
//! Nothing grows a second descent over the type formers, and the engine stays a
//! leaf.
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

use crate::boundary::BinderInScope;
use crate::boundary::FreeOccurrence;
use crate::boundary::NameAbsence;
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
///   codomain that rebinds `name` untouched; **alpha-equivalent** to `ty` when
///   `name` does not occur free — not identical, because a binder spelled like
///   a free name of `repl` is renamed apart whether or not the substitution
///   changes anything beneath it. Both clauses of this contract are therefore
///   stated up to alpha, and neither is a syntactic guarantee.
/// - ensures: **no free variable of `repl` is captured by a TYPE binder** the
///   substitution descends under — a `Σ` tail or `Π` codomain binder that
///   rebinds a free name of `repl` is renamed apart first, so the result is
///   alpha-equivalent to the capture-avoiding specification **at the type
///   sort**. Value binders reached through a `Path` endpoint or a `Family`
///   argument are substituted by [`crate::subst::subst_value`], which
///   implements **shadowing only and does not rename**; capture at the value
///   sort is `gandr-j078`.
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
/// - reason: each pass rebuilds finite type trees from an explicit worklist;
///   the pass restarts only to add an alias, and each restart inserts a binder
///   name the plan did not already hold.
/// - measure: **lexicographic** — first the aliases still owed, then the
///   pending type tasks and result frames within a pass. A pass's own measure
///   resets to full on a restart, so it bounds nothing across them.
/// - boundedness: source types are finite Rust values; every plan key is a free
///   name of `repl`, because the `mentions_free` guard precedes the only `Err`
///   return in [`open_binder`]; `repl` does not change across restarts; and
///   [`fresh_alias`] never returns a name free in `repl`, so no alias can cause
///   a later request. Restarts are therefore bounded by the number of free
///   names of `repl`.
/// - input recursion: none.
pub fn subst_valuetype<'source, N>(
    ty: &ValueType,
    name: N,
    repl: &Value,
) -> ValueType
where
    N: Into<NameRef<'source>> + Copy,
{
    let root = TypeRoot::Value(Rc::new(ty.clone()));
    let name = name.into();
    let mut plan = AliasPlan::new();
    loop {
        match subst_type(&root, name, repl, &plan) {
            | Rewrite::Done(mut values, _) => return pop_value_type(&mut values),
            | Rewrite::NeedsAlias(binder) => {
                let alias = fresh_alias(NameRef::from(binder.as_str()), &root, repl, name, &plan);
                plan.insert(binder, alias);
            },
        }
    }
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
///   that rebinds `name` untouched; **alpha-equivalent** to `ty` when `name`
///   does not occur free — not identical, because a binder spelled like a free
///   name of `repl` is renamed apart whether or not the substitution changes
///   anything beneath it. Both clauses of this contract are therefore stated up
///   to alpha, and neither is a syntactic guarantee.
/// - ensures: **no free variable of `repl` is captured by a TYPE binder** the
///   substitution descends under — a `Σ` tail or `Π` codomain binder that
///   rebinds a free name of `repl` is renamed apart first, so the result is
///   alpha-equivalent to the capture-avoiding specification **at the type
///   sort**. Value binders reached through a `Path` endpoint or a `Family`
///   argument are substituted by [`crate::subst::subst_value`], which
///   implements **shadowing only and does not rename**; capture at the value
///   sort is `gandr-j078`.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the rename fires on a binder that rebinds a free variable
///   of the replacement, and the sequential instantiation of a dependent spine
///   is separated from it by a caller whose indices collide with the callee's.
/// - witness: `tests::substitution_renames_a_binder_that_would_capture_the_replacement`
/// - witness: `tests::a_captured_expectation_does_not_accept_the_wrong_argument`
#[inline]
#[must_use]
/// # Termination
/// - reason: each pass rebuilds finite type trees from an explicit worklist;
///   the pass restarts only to add an alias, and each restart inserts a binder
///   name the plan did not already hold.
/// - measure: **lexicographic** — first the aliases still owed, then the
///   pending type tasks and result frames within a pass. A pass's own measure
///   resets to full on a restart, so it bounds nothing across them.
/// - boundedness: source types are finite Rust values; every plan key is a free
///   name of `repl`, because the `mentions_free` guard precedes the only `Err`
///   return in [`open_binder`]; `repl` does not change across restarts; and
///   [`fresh_alias`] never returns a name free in `repl`, so no alias can cause
///   a later request. Restarts are therefore bounded by the number of free
///   names of `repl`.
/// - input recursion: none.
pub fn subst_comptype<'source, N>(
    ty: &CompType,
    name: N,
    repl: &Value,
) -> CompType
where
    N: Into<NameRef<'source>> + Copy,
{
    let root = TypeRoot::Comp(Rc::new(ty.clone()));
    let name = name.into();
    let mut plan = AliasPlan::new();
    loop {
        match subst_type(&root, name, repl, &plan) {
            | Rewrite::Done(_, mut comps) => return pop_comp_type(&mut comps),
            | Rewrite::NeedsAlias(binder) => {
                let alias = fresh_alias(NameRef::from(binder.as_str()), &root, repl, name, &plan);
                plan.insert(binder, alias);
            },
        }
    }
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
/// - witness: `identity::tests::pi_binder_occurrence_is_shadowed_by_an_inner_binder`
/// - witness: `identity::tests::pi_binder_occurrence_is_seen_through_a_path_endpoint`
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
enum TypeRoot
{
    /// A value-type root: the run leaves exactly one rebuilt [`ValueType`].
    Value(Rc<ValueType>),
    /// A computation-type root: the run leaves exactly one rebuilt
    /// [`CompType`].
    Comp(Rc<CompType>),
}

/// One task on the [`subst_type`] worklist — the defunctionalized image of a
/// recursive `subst_valuetype` / `subst_comptype` call (ADR-47 T1). A
/// `Value` / `Comp` task visits a source type, rebuilding a leaf directly or
/// pushing a `Finish*` frame followed by its children; a `Finish*` frame
/// reassembles one node from the rebuilt children on the result stacks.
enum TypeTask
{
    /// Visit a value type under the aliases in force over it.
    Value(Rc<ValueType>, Rc<AliasScope>),
    /// Visit a computation type under the aliases in force over it.
    Comp(Rc<CompType>, Rc<AliasScope>),
    /// Reassemble a value type from its rebuilt children.
    FinishValue(ValueFinish),
    /// Reassemble a computation type from its rebuilt children.
    FinishComp(CompFinish),
}

/// A pending value-type reassembly — everything a [`TypeTask::FinishValue`]
/// frame needs to rebuild one [`ValueType`] node once its substituted children
/// sit on the result stack.
enum ValueFinish
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
    Path(Rc<Value>, Rc<Value>, Rc<AliasScope>),
    /// Rebuild a declared-data application from its id and rebuilt arguments
    /// (the `usize` is the argument count).
    Data(crate::types::DataId, usize),
    /// Rebuild a classifier-bearing family application, substituting value
    /// indices through its static neutral spine.
    Family(FamilyApp, Rc<AliasScope>),
    /// Rebuild a dependent pair `Σ(x : A). B`, substituting into the tail only
    /// when the binder does not shadow the substituted name.
    Sigma
    {
        /// The bound variable, kept verbatim in the rebuild.
        binder: String,
        /// The original tail, reused untouched when `binder` shadows the
        /// substituted name.
        original_snd: Rc<ValueType>,
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
    Package(Grade, Vec<String>),
}

/// A pending computation-type reassembly — everything a
/// [`TypeTask::FinishComp`] frame needs to rebuild one [`CompType`] node once
/// its substituted children sit on the result stacks.
enum CompFinish
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
        original_res: Rc<CompType>,
        /// Whether the result substitutes.
        substitute_res: bool,
    },
    /// Rebuild a `with` type from its rebuilt components.
    With,
    /// Rebuild a computation-family application after substituting its value
    /// arguments.
    Family(FamilyApp, Rc<AliasScope>),
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
/// - ensures: a binder that would capture a free variable of `repl` carries the
///   alias `plan` holds for its name, and the body's occurrences of that name
///   resolve to the alias beneath it.
/// - fails: [`Rewrite::NeedsAlias`] when such a binder has no alias yet; the
///   engine asks rather than choosing, because choosing means asking whether a
///   candidate occurs in the type, and that question is answered by running
///   this engine — which it may not do to itself.
/// - panics: none (a `debug_assert` in the `pop_*` helpers guards the balance
///   invariant in debug / test builds).
///
/// # Termination
/// - reason: the engine drains a finite task stack.
/// - measure: pending type tasks and result frames.
/// - boundedness: source types are finite Rust values.
/// - input recursion: none.
fn subst_type(
    root: &TypeRoot,
    name: NameRef<'_>,
    repl: &Value,
    plan: &AliasPlan,
) -> Rewrite
{
    let empty = Rc::new(AliasScope::Empty);
    let mut tasks = match *root {
        | TypeRoot::Value(ref ty) => vec![TypeTask::Value(Rc::clone(ty), empty)],
        | TypeRoot::Comp(ref ty) => vec![TypeTask::Comp(Rc::clone(ty), empty)],
    };
    let mut values = Vec::new();
    let mut comps = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            | TypeTask::Value(ty, scope) => match *ty {
                | ValueType::Atom(ref atom) if atom == name.as_ref() => {
                    match *repl {
                        | Value::Var(ref replacement) => {
                            values.push(ValueType::Atom(replacement.clone()));
                        },
                        | _ => values.push((*ty).clone()),
                    }
                },
                // An occurrence of a binder this run renamed apart. The alias is
                // in force only under that binder, which is what the scope
                // records; a free occurrence of the same name above it is the
                // caller's own variable and keeps its spelling.
                | ValueType::Atom(ref atom) if bool::from(scope.holds(NameRef::from(atom.as_str()))) => {
                    let aliased = plan.get(atom).cloned().unwrap_or_else(|| atom.clone());
                    values.push(ValueType::Atom(aliased));
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
                    values.push((*ty).clone());
                },
                | ValueType::Prod(ref fst, ref snd) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Prod));
                    tasks.push(TypeTask::Value(Rc::clone(snd), Rc::clone(&scope)));
                    tasks.push(TypeTask::Value(Rc::clone(fst), Rc::clone(&scope)));
                },
                | ValueType::Sum(ref lhs, ref rhs) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Sum));
                    tasks.push(TypeTask::Value(Rc::clone(rhs), Rc::clone(&scope)));
                    tasks.push(TypeTask::Value(Rc::clone(lhs), Rc::clone(&scope)));
                },
                | ValueType::List(ref element) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::List));
                    tasks.push(TypeTask::Value(Rc::clone(element), Rc::clone(&scope)));
                },
                | ValueType::Record(ref fields) => {
                    let labels = fields.keys().cloned().collect::<Vec<_>>();
                    tasks.push(TypeTask::FinishValue(ValueFinish::Record(labels)));
                    for field in fields.values().rev() {
                        tasks.push(TypeTask::Value(Rc::clone(field), Rc::clone(&scope)));
                    }
                },
                | ValueType::Thunk(grade, ref body) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Thunk(grade)));
                    tasks.push(TypeTask::Comp(Rc::clone(body), Rc::clone(&scope)));
                },
                | ValueType::Stk(ref consumes, ref delivers) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Stk));
                    tasks.push(TypeTask::Comp(Rc::clone(delivers), Rc::clone(&scope)));
                    tasks.push(TypeTask::Comp(Rc::clone(consumes), Rc::clone(&scope)));
                },
                | ValueType::Path {
                    ty: ref carrier,
                    ref lhs,
                    ref rhs,
                } => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Path(
                        Rc::clone(lhs),
                        Rc::clone(rhs),
                        Rc::clone(&scope),
                    )));
                    tasks.push(TypeTask::Value(Rc::clone(carrier), Rc::clone(&scope)));
                },
                | ValueType::Family(ref application) => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Family(
                        application.clone(),
                        Rc::clone(&scope),
                    )));
                },
                | ValueType::Data { ref id, ref args } => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Data(
                        id.clone(),
                        args.len(),
                    )));
                    for arg in args.iter().rev() {
                        tasks.push(TypeTask::Value(Rc::clone(arg), Rc::clone(&scope)));
                    }
                },
                | ValueType::Sigma {
                    ref fst,
                    ref binder,
                    ref snd,
                } => {
                    let opened =
                        match open_binder(NameRef::from(binder.as_str()), name, repl, &scope, plan) {
                        | Ok(opened) => opened,
                        | Err(wanted) => return Rewrite::NeedsAlias(wanted),
                    };
                    tasks.push(TypeTask::FinishValue(ValueFinish::Sigma {
                        binder: opened.binder,
                        original_snd: Rc::clone(snd),
                        substitute_snd: opened.substitute,
                    }));
                    if opened.substitute {
                        tasks.push(TypeTask::Value(Rc::clone(snd), opened.scope));
                    }
                    tasks.push(TypeTask::Value(Rc::clone(fst), Rc::clone(&scope)));
                },
                | ValueType::Package {
                    grade,
                    ref abstracts,
                    ref payload,
                } => {
                    tasks.push(TypeTask::FinishValue(ValueFinish::Package(
                        grade,
                        abstracts.clone(),
                    )));
                    tasks.push(TypeTask::Value(Rc::clone(payload), Rc::clone(&scope)));
                },
            },
            | TypeTask::Comp(ty, scope) => match *ty {
                | CompType::Unknown => comps.push(CompType::Unknown),
                | CompType::F(ref of, ref row) => {
                    tasks.push(TypeTask::FinishComp(CompFinish::F(row.clone())));
                    tasks.push(TypeTask::Value(Rc::clone(of), Rc::clone(&scope)));
                },
                | CompType::Arrow {
                    ref binder,
                    ref arg,
                    ref res,
                } => {
                    // A `Pi` binder of the substituted name shadows it in the
                    // codomain, exactly as `Sigma`'s does in its tail; a `None`
                    // binder shadows nothing and always descends. A binder that
                    // would capture a free variable of `repl` is renamed apart,
                    // to the alias this run committed to for that name.
                    let Some(bound) = binder.as_deref()
                    else {
                        tasks.push(TypeTask::FinishComp(CompFinish::Arrow {
                            binder: None,
                            original_res: Rc::clone(res),
                            substitute_res: true,
                        }));
                        tasks.push(TypeTask::Comp(Rc::clone(res), Rc::clone(&scope)));
                        tasks.push(TypeTask::Value(Rc::clone(arg), Rc::clone(&scope)));
                        continue;
                    };
                    let opened = match open_binder(NameRef::from(bound), name, repl, &scope, plan) {
                        | Ok(opened) => opened,
                        | Err(wanted) => return Rewrite::NeedsAlias(wanted),
                    };
                    tasks.push(TypeTask::FinishComp(CompFinish::Arrow {
                        binder: Some(opened.binder),
                        original_res: Rc::clone(res),
                        substitute_res: opened.substitute,
                    }));
                    if opened.substitute {
                        tasks.push(TypeTask::Comp(Rc::clone(res), opened.scope));
                    }
                    tasks.push(TypeTask::Value(Rc::clone(arg), Rc::clone(&scope)));
                },
                | CompType::With(ref fst, ref snd) => {
                    tasks.push(TypeTask::FinishComp(CompFinish::With));
                    tasks.push(TypeTask::Comp(Rc::clone(snd), Rc::clone(&scope)));
                    tasks.push(TypeTask::Comp(Rc::clone(fst), Rc::clone(&scope)));
                },
                | CompType::Family(ref application) => {
                    tasks.push(TypeTask::FinishComp(CompFinish::Family(
                        application.clone(),
                        Rc::clone(&scope),
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
                | ValueFinish::Family(application, scope) => {
                    let aliased = alias_family(application, &scope, plan);
                    values.push(ValueType::Family(aliased.substitute_value(name, repl)));
                },
                | ValueFinish::Path(lhs, rhs, scope) => {
                    let carrier = pop_value_type(&mut values);
                    values.push(ValueType::Path {
                        ty: Rc::new(carrier),
                        lhs: Rc::new(subst_value(&alias_value(&lhs, &scope, plan), name, repl)),
                        rhs: Rc::new(subst_value(&alias_value(&rhs, &scope, plan), name, repl)),
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
                        original_snd
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
                        abstracts,
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
                        original_res
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
                | CompFinish::Family(application, scope) => {
                    let aliased = alias_family(application, &scope, plan);
                    comps.push(CompType::Family(aliased.substitute_value(name, repl)));
                },
            },
        }
    }
    Rewrite::Done(values, comps)
}

/// The fresh binder names a capture-avoiding run has committed to, keyed by the
/// name each one replaces.
///
/// Keying by name rather than by position is what makes one alias enough for
/// every binder that shares a name: an inner binder shadowed an outer one of
/// the same name before the rename and shadows it after, because both moved to
/// the same place.
type AliasPlan = BTreeMap<String, String>;

/// What one pass of [`subst_type`] produced.
enum Rewrite
{
    /// The rebuilt result stacks, one of which holds the root.
    Done(Vec<ValueType>, Vec<CompType>),
    /// The pass met a binder that would capture and has no alias for it yet.
    ///
    /// The engine asks rather than choosing, because choosing a fresh name
    /// means asking whether a candidate occurs in the type, and that
    /// question is answered by running the engine — which the engine may
    /// not do to itself. The entry point owns freshness and re-runs the
    /// pass with the alias added.
    NeedsAlias(String),
}

/// The aliases in force over one branch of the walk.
///
/// An alias holds only under the binder it renamed; a free occurrence of the
/// same name above that binder is the caller's own variable and keeps its
/// spelling. A shared cons list, so extending it costs a pointer.
enum AliasScope
{
    /// No alias in force.
    Empty,
    /// One renamed binder over an enclosing scope.
    Bound(String, Rc<Self>),
}

impl AliasScope
{
    /// Whether an alias for `name` is in force here.
    fn holds(
        self: &Rc<Self>,
        name: NameRef<'_>,
    ) -> BinderInScope
    {
        let mut scope = self;
        loop {
            match **scope {
                | Self::Empty => return BinderInScope::from(false),
                | Self::Bound(ref bound, ref rest) => {
                    if bound == name.as_ref() {
                        return BinderInScope::from(true);
                    }
                    scope = rest;
                },
            }
        }
    }

    /// This scope extended by one renamed binder.
    fn with(
        self: &Rc<Self>,
        binder: NameRef<'_>,
    ) -> Rc<Self>
    {
        Rc::new(Self::Bound(String::from(binder.as_ref()), Rc::clone(self)))
    }
}

/// One binder opened for the descent.
struct OpenedBinder
{
    /// The name the rebuilt binder carries, renamed when it would have
    /// captured.
    binder: String,
    /// The alias scope in force over the body.
    scope: Rc<AliasScope>,
    /// Whether the substitution descends into the body at all.
    substitute: bool,
}

/// Opens a binder for the descent into its body.
///
/// Three cases, and the middle one is the whole point. A binder of the
/// substituted **name** shadows it and blocks the descent. A binder that
/// rebinds a name the substituted **value** mentions would capture it, so it
/// takes the run's alias for that name — and asks for one when the run has none
/// yet. Everything else descends unchanged.
///
/// # Contract
/// - ensures: `Ok` carries a binder no free variable of `repl` is spelled like,
///   together with the scope under which the body's occurrences of the original
///   name resolve to it.
/// - fails: `Err(name)` when the binder needs an alias the plan does not hold.
/// - panics: none.
fn open_binder(
    binder: NameRef<'_>,
    name: NameRef<'_>,
    repl: &Value,
    scope: &Rc<AliasScope>,
    plan: &AliasPlan,
) -> Result<OpenedBinder, String>
{
    if binder.as_ref() == name.as_ref() {
        return Ok(OpenedBinder {
            binder: String::from(binder.as_ref()),
            scope: Rc::clone(scope),
            substitute: false,
        });
    }
    if !bool::from(mentions_free(repl, binder)) {
        return Ok(OpenedBinder {
            binder: String::from(binder.as_ref()),
            scope: Rc::clone(scope),
            substitute: true,
        });
    }
    let Some(alias) = plan.get(binder.as_ref())
    else {
        return Err(String::from(binder.as_ref()));
    };
    Ok(OpenedBinder {
        binder: alias.clone(),
        scope: scope.with(binder),
        substitute: true,
    })
}

/// Applies the aliases in force to a value reached inside a type.
///
/// **Every target is fresh, so no step can feed another** — that, and not the
/// sources being distinct, is why the order the scope is walked in cannot
/// change the answer. Sources are *not* distinct when two nested binders share
/// a spelling: the scope then carries the same name twice and the second
/// application finds nothing left to rewrite.
fn alias_value(
    value: &Rc<Value>,
    scope: &Rc<AliasScope>,
    plan: &AliasPlan,
) -> Value
{
    let mut current = (**value).clone();
    let mut cursor = scope;
    loop {
        match **cursor {
            | AliasScope::Empty => return current,
            | AliasScope::Bound(ref bound, ref rest) => {
                if let Some(alias) = plan.get(bound) {
                    current = subst_value(
                        &current,
                        NameRef::from(bound.as_str()),
                        &Value::Var(alias.clone()),
                    );
                }
                cursor = rest;
            },
        }
    }
}

/// Applies the aliases in force to a family application's value indices.
fn alias_family(
    application: FamilyApp,
    scope: &Rc<AliasScope>,
    plan: &AliasPlan,
) -> FamilyApp
{
    let mut current = application;
    let mut cursor = scope;
    loop {
        match **cursor {
            | AliasScope::Empty => return current,
            | AliasScope::Bound(ref bound, ref rest) => {
                if let Some(alias) = plan.get(bound) {
                    current = current.substitute_value(
                        NameRef::from(bound.as_str()),
                        &Value::Var(alias.clone()),
                    );
                }
                cursor = rest;
            },
        }
    }
}

/// A name for `base` that occurs nowhere in the type and free nowhere in
/// `repl`.
///
/// The generated name carries a space, which the surface grammar admits in no
/// identifier, so it can collide only with a name an earlier run generated —
/// and [`name_is_absent`] rejects exactly that.
///
/// # Termination
/// - reason: the type and `repl` are finite, so only finitely many candidates
///   can be rejected and the first surviving tag is reached.
/// - measure: the number of names occurring in the type and in `repl`.
/// - boundedness: each iteration tests one new candidate.
/// - input recursion: none.
fn fresh_alias(
    base: NameRef<'_>,
    root: &TypeRoot,
    repl: &Value,
    name: NameRef<'_>,
    plan: &AliasPlan,
) -> String
{
    let mut tag = 1_u32;
    loop {
        let candidate = alloc::format!("{} {tag}", base.as_ref());
        let reference = NameRef::from(candidate.as_str());
        if candidate != name.as_ref()
            && !bool::from(mentions_free(repl, reference))
            && !plan.values().any(|taken| *taken == candidate)
            && bool::from(name_is_absent(root, reference))
        {
            return candidate;
        }
        tag = tag.wrapping_add(1);
    }
}

/// Whether a name occurs nowhere in a type, neither free nor as a binder.
///
/// Both halves are asked of the substitution engine rather than of a second
/// traversal, so the answer cannot drift from what the rewrite will do.
///
/// A **binder** spelled like the candidate is what makes the engine ask for an
/// alias, so running a rewrite whose replacement is the candidate — for a name
/// no source can write, which therefore rewrites nothing — reports one exactly
/// when a binder carries it. A **free occurrence** is what changes a type when
/// it is substituted away, so a rewrite that leaves the type equal reports
/// none.
fn name_is_absent(
    root: &TypeRoot,
    candidate: NameRef<'_>,
) -> NameAbsence
{
    let probe = NameRef::from(OCCURRENCE_PROBE);
    let named = Value::Var(String::from(candidate.as_ref()));
    if matches!(
        subst_type(root, probe, &named, &AliasPlan::new()),
        Rewrite::NeedsAlias(_)
    ) {
        return NameAbsence::from(false);
    }
    let erased = Value::Var(String::from(OCCURRENCE_PROBE));
    match subst_type(root, candidate, &erased, &AliasPlan::new()) {
        | Rewrite::Done(mut values, mut comps) => match *root {
            | TypeRoot::Value(ref ty) => NameAbsence::from(pop_value_type(&mut values) == **ty),
            | TypeRoot::Comp(ref ty) => NameAbsence::from(pop_comp_type(&mut comps) == **ty),
        },
        | Rewrite::NeedsAlias(_) => NameAbsence::from(false),
    }
}

/// Whether `binder` occurs free in the substituted value.
///
/// The [`OCCURRENCE_PROBE`] idiom at the value sort: substituting a name no
/// source can write changes the value exactly when the queried name occurred
/// free in it, so the answer comes from the substitution engine rather than
/// from a second traversal that could disagree with it.
fn mentions_free(
    repl: &Value,
    binder: NameRef<'_>,
) -> FreeOccurrence
{
    let probe = Value::Var(String::from(OCCURRENCE_PROBE));
    FreeOccurrence::from(subst_value(repl, binder, &probe) != *repl)
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
    use crate::syntax::Comp;

    /// Substituting a value whose free variable a binder below would rebind
    /// renames that binder apart rather than capturing.
    ///
    /// Shadowing and capture are different obligations: shadowing asks whether
    /// the binder rebinds the substituted NAME, capture asks whether it rebinds
    /// a name the substituted VALUE mentions.
    #[test]
    fn substitution_renames_a_binder_that_would_capture_the_replacement()
    {
        // `Π(c : 1). F b` — the codomain mentions `b`, and `c` is the binder.
        let source = CompType::pi(
            "c",
            ValueType::Unit,
            CompType::F(
                Rc::new(ValueType::Atom(String::from("b"))),
                EffectRow::EMPTY,
            ),
        );
        // Instantiating `b := c` writes a `c` into the codomain, where the `Π`
        // binder would bind it. The caller meant its own `c`, so the binder
        // moves instead.
        let instantiated =
            subst_comptype(&source, NameRef::from("b"), &Value::Var(String::from("c")));
        let CompType::Arrow {
            ref binder,
            ref res,
            ..
        } = instantiated
        else {
            panic!("instantiating a Pi rebuilt something else");
        };
        let bound = binder.as_deref().expect("the Pi keeps a binder");
        assert_ne!(
            bound, "c",
            "the Pi binder captured the replacement: the caller variable `c` is              now bound by the callee binder that happened to share its name"
        );
        assert_eq!(
            **res,
            CompType::F(
                Rc::new(ValueType::Atom(String::from("c"))),
                EffectRow::EMPTY
            ),
            "the codomain lost the replacement the substitution wrote into it"
        );
    }

    /// A bare type atom.
    fn atom(name: NameRef<'_>) -> ValueType
    {
        ValueType::Atom(String::from(name.as_ref()))
    }

    /// The returner over an atom.
    fn returns(name: NameRef<'_>) -> CompType
    {
        CompType::F(Rc::new(atom(name)), EffectRow::EMPTY)
    }

    /// A thunked function type `U[ω] (from -> F to)`.
    fn thunked(
        from: NameRef<'_>,
        to: NameRef<'_>,
    ) -> ValueType
    {
        ValueType::Thunk(
            Grade::OMEGA,
            Rc::new(CompType::Arrow {
                binder: None,
                arg: Rc::new(atom(from)),
                res: Rc::new(returns(to)),
            }),
        )
    }

    /// The separating construction, shared by the two capture witnesses.
    ///
    /// Returns the instantiated `comp` type and the `thunked` builder, so each
    /// witness asserts on its own argument position and **fails for its own
    /// reason** — a single test asserting both would report only whichever
    /// assertion it reached first.
    fn captured_composition() -> CompType
    {
        /// Peels one `Π` and instantiates it at a type variable, which is what
        /// applying a dependent spine does one argument at a time.
        fn instantiate(
            ty: &CompType,
            arg: NameRef<'_>,
        ) -> CompType
        {
            let CompType::Arrow {
                ref binder,
                ref res,
                ..
            } = *ty
            else {
                panic!("expected a Pi to instantiate");
            };
            let bound = binder.as_deref().expect("expected a dependent binder");
            subst_comptype(
                res.as_ref(),
                NameRef::from(bound),
                &Value::Var(String::from(arg.as_ref())),
            )
        }
        let arrow = |arg: ValueType, res: CompType| CompType::Arrow {
            binder: None,
            arg: Rc::new(arg),
            res: Rc::new(res),
        };
        // `comp : Π(a). Π(b). Π(c). U[ω] (a -> F b) -> U[ω] (b -> F c) -> a -> F c`
        // (the index domains are irrelevant here and stand as `1`).
        let comp = CompType::pi(
            "a",
            ValueType::Unit,
            CompType::pi(
                "b",
                ValueType::Unit,
                CompType::pi(
                    "c",
                    ValueType::Unit,
                    arrow(
                        thunked(NameRef::from("a"), NameRef::from("b")),
                        arrow(
                            thunked(NameRef::from("b"), NameRef::from("c")),
                            arrow(atom(NameRef::from("a")), returns(NameRef::from("c"))),
                        ),
                    ),
                ),
            ),
        );
        // Applied at `(a, c, d)` — the caller spells its own indices with names
        // the callee also binds, which is what every category law written the
        // obvious way does.
        instantiate(
            &instantiate(&instantiate(&comp, NameRef::from("a")), NameRef::from("c")),
            NameRef::from("d"),
        )
    }

    /// The **second** function's expectation, `g` — the **wrong-acceptance**
    /// direction: two types that must not agree, which capturing substitution
    /// makes coincide.
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
    #[test]
    fn a_captured_expectation_does_not_accept_the_wrong_argument()
    {
        let applied = captured_composition();
        let CompType::Arrow { res: ref rest, .. } = applied
        else {
            panic!("the instantiated type lost its first argument");
        };
        let CompType::Arrow {
            arg: ref second, ..
        } = **rest
        else {
            panic!("the instantiated type lost its second argument");
        };

        // The wrong-acceptance direction FIRST, because it is the reason this
        // witness exists. A repair verified only against the refused program
        // proves capture stopped rejecting something correct and says nothing
        // about what it was accepting.
        assert_ne!(
            **second,
            thunked(NameRef::from("d"), NameRef::from("d")),
            "the expectation collapsed to the captured type, so an argument \
             written at it would be accepted"
        );
        assert_eq!(
            **second,
            thunked(NameRef::from("c"), NameRef::from("d")),
            "the expectation for the second function is not the one the caller wrote"
        );
    }

    /// The **first** function's expectation, `f` — the other expectation the
    /// unfixed engine captured, and the one this sibling test exists to keep
    /// reportable.
    ///
    /// Sequential instantiation rewrites `f`'s expectation too: `a -> F b`
    /// becomes `a -> F c` and then `a -> F d`. `gandr-ijdw`'s prose explains
    /// the second function's capture while its pasted diagnostic shows this
    /// one; both are real, and a witness keyed only to the second leaves this
    /// one resting on a single corpus file.
    #[test]
    fn the_first_functions_expectation_is_not_captured_either()
    {
        let applied = captured_composition();
        let CompType::Arrow { arg: ref first, .. } = applied
        else {
            panic!("the instantiated type lost its first argument");
        };

        assert_ne!(
            **first,
            thunked(NameRef::from("a"), NameRef::from("d")),
            "the first function's expectation collapsed to the captured type"
        );
        assert_eq!(
            **first,
            thunked(NameRef::from("a"), NameRef::from("c")),
            "the expectation for the first function is not the one the caller wrote"
        );
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

    /// A binder that **cannot** capture is renamed apart anyway, so the result
    /// is alpha-equivalent to the source rather than identical to it.
    ///
    /// [`open_binder`] decides to rename on `mentions_free(repl, binder)`
    /// alone. That test consults the replacement and the binder spelling and
    /// **never asks whether the substituted name occurs beneath that binder**,
    /// and the walk reaches every binder unconditionally. So a substitution
    /// that changes nothing still moves a binder spelled like a free name of
    /// the replacement.
    ///
    /// The rename therefore fires on a **superset** of the binders that could
    /// capture — exact for collision, wider than necessary for effect. That is
    /// why this module's first `ensures` clause promises alpha-equivalence
    /// rather than identity: identity-when-absent is the kind of property
    /// callers build fast paths on, and it is not true here.
    ///
    /// `crates/core-nbe/src/conv.rs` is the caller that would notice, because
    /// it alpha-renames one side's binder to match the other and then compares
    /// structurally. It does not break:
    /// `tests::a_spurious_rename_does_not_refuse_a_convertible_pair` measures
    /// the exact shape conv produces.
    #[test]
    fn a_binder_that_cannot_capture_is_renamed_apart_anyway()
    {
        // `Π(d : 1). F z` — `x` occurs nowhere in it, so substituting for `x`
        // is a no-op by the first clause.
        let source = CompType::pi(
            "d",
            ValueType::Unit,
            CompType::F(
                Rc::new(ValueType::Atom(String::from("z"))),
                EffectRow::EMPTY,
            ),
        );

        let substituted =
            subst_comptype(&source, NameRef::from("x"), &Value::Var(String::from("d")));

        // **The clause-level claim first, the mechanism second.** An assertion
        // that can only fire in cases the one above it also catches has no
        // voice: it never reports. Non-identity can fail for reasons other than
        // the binder moving, and the binder can stay put while the type changes
        // for another reason, so each of these two can be the one that speaks.
        assert_ne!(
            substituted, source,
            "the result is identical to the source, so the weakened clause is             weaker than it needs to be"
        );
        let CompType::Arrow { ref binder, .. } = substituted
        else {
            panic!("substituting a Pi rebuilt something else");
        };
        let bound = binder.as_deref().expect("the Pi keeps a binder");
        assert_ne!(
            bound, "d",
            "the type changed but the binder did not, so this witness no longer             names the mechanism it was written for"
        );
    }

    /// A **value** binder inside a `Path` endpoint captures the replacement.
    ///
    /// The rename this module added is a type-sort repair. Values inside an
    /// endpoint are substituted by [`crate::subst::subst_value`], which decides
    /// every binder by shadowing alone — it blocks the descent when the binder
    /// rebinds the substituted NAME, and never asks whether the binder rebinds
    /// a name the substituted VALUE mentions. That is the same distinction one
    /// sort down, and it is `gandr-j078`.
    ///
    /// **This test pins the current behaviour rather than the intended one**,
    /// so the bead carries a measured counterexample instead of a read one.
    /// When the value sort renames, both assertions below flip and this becomes
    /// that repair's witness.
    #[test]
    fn a_value_binder_inside_an_endpoint_captures_the_replacement()
    {
        // `Path(1, thunk_ω (λd. ret x), ())` — the endpoint's binder is `d`
        // and its body mentions `x` free.
        let endpoint = Value::Thunk(
            Grade::OMEGA,
            Rc::new(Comp::Abs(
                String::from("d"),
                None,
                Rc::new(Comp::Ret(Rc::new(Value::Var(String::from("x"))))),
            )),
        );
        let source = ValueType::Path {
            ty: Rc::new(ValueType::Unit),
            lhs: Rc::new(endpoint),
            rhs: Rc::new(Value::Unit),
        };

        // Substituting `x := d` writes a `d` into the body, where `λd` binds
        // it. The capture-avoiding specification renames the `λ` apart and
        // leaves the substituted `d` free.
        let substituted =
            subst_valuetype(&source, NameRef::from("x"), &Value::Var(String::from("d")));

        let ValueType::Path { ref lhs, .. } = substituted
        else {
            panic!("substituting a Path rebuilt something else");
        };
        let Value::Thunk(_, ref produced) = **lhs
        else {
            panic!("the endpoint stopped being a thunk");
        };
        let Comp::Abs(ref binder, _, ref inner) = **produced
        else {
            panic!("the thunk stopped holding an abstraction");
        };

        assert_eq!(
            binder, "d",
            "gandr-j078: the value binder is NOT renamed apart, which is the             defect this test pins"
        );
        assert_eq!(
            **inner,
            Comp::Ret(Rc::new(Value::Var(String::from("d")))),
            "gandr-j078: the caller's `d` is now bound by the callee's `λd` —             that is capture, and it is what the type-sort rename does not reach"
        );
    }
}
