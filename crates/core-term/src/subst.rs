//! Shadowing-aware substitution (ADR-47 T1) — the iterative worklist engine
//! shared by motive instantiation and by hole substitution.
//!
//! **Shadowing, not capture.** Every binder here asks whether it rebinds the
//! substituted `name`. None asks whether it rebinds a free name of the
//! substituted value, so a thunk binder spelled like one still captures it.
//! `gandr-j078`.
//!
//! One traversal, two rules. [`subst_value`] replaces a free value variable
//! inside a source value, respecting binder shadowing; it is what the identity
//! former's motive instantiation drives ([`crate::identity`] calls
//! it at each [`crate::types::ValueType::Path`] endpoint).
//! [`subst_holes_value`] and [`subst_holes_comp`] replace **solved holes** by
//! their solutions, which is how a unifier's certificate is applied to a term
//! before the ordinary conversion engine re-checks it
//! (`gandr_core_unify`).
//!
//! The two rules differ in exactly two places and share everything else. A
//! variable substitution is blocked by a binder of the same name; a hole
//! substitution is blocked by nothing, because a hole is not a binder-bound
//! name and every solution is a closed term. Sharing the traversal is what
//! keeps one grammar-complete descent in the crate rather than two that drift.
//!
//! The engine ([`Subst`]) owns an explicit LIFO work stack and one result stack
//! per syntactic sort, so substitution depth follows the heap, not the host
//! call stack — the iterative shadow of the recursive specification (the Agda
//! metatheory stays the oracle, ADR-47). It is a **durable** helper: it
//! outlived the CEK evaluator that once co-hosted it (its computation-level
//! companion, the CEK's `subst_comp`, retired with that machine).

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::Vec;

use crate::boundary::HoleId;
use crate::boundary::HoleOccurrence;
use crate::boundary::NameRef;
use crate::syntax::Comp;
use crate::syntax::OpClause;
use crate::syntax::Stack;
use crate::syntax::Value;
use crate::syntax::WalkBase;

/// Shadowing-aware substitution of `repl` for the free value variable `name`
/// inside a **value**.
///
/// The value-into-value entry of the iterative [`Subst`] engine (the ADR-47
/// traversal the CEK's computation-level substitution once shared, reusing its
/// binder-shadowing discipline).
///
/// This is the substitution the identity former's motive instantiation drives
/// ([`crate::identity`] calls it at each [`crate::types::ValueType::Path`]
/// endpoint), and the one a solver's certificate is re-checked through. It is
/// public precisely so every caller shares one substitution engine rather than
/// reimplementing the descent. **It avoids shadowing, not capture**: a binder
/// blocks the descent when it rebinds the substituted `name`, and nothing here
/// asks whether it rebinds a free name of `repl`. `gandr-j078`.
///
/// # Contract
/// - ensures: returns `value` with every free `name` replaced by `repl`,
///   leaving occurrences under a rebinding of `name` (a thunked computation's
///   binders) untouched; structurally identical to the direct recursive
///   definition.
/// - panics: none (the worklist's post-order balance keeps every result pop
///   defined; a `debug_assert` guards the invariant in test / debug builds).
#[inline]
#[must_use]
pub fn subst_value<'source, N>(
    value: &Value,
    name: N,
    repl: &Value,
) -> Value
where
    N: Into<NameRef<'source>>,
{
    let name = name.into();
    let mut engine = Subst::new(Rule::Variable(repl), Some(name));
    engine.work.push(Task::DescendValue(value));
    engine.run();
    engine.take_value()
}

/// Replaces every solved hole inside a **value** by its solution, reporting
/// whether any hole survived.
///
/// The survivor report is the point of the return pair. Conversion treats a
/// hole as consistent with every value, so a term that still carries one makes
/// a conversion verdict **vacuous** rather than conclusive, and a certificate
/// validator has to know which it got. Computing it here costs nothing: the
/// traversal already visits every hole.
///
/// # Contract
/// - requires: every solution in `solutions` is a closed term, which is what
///   the closed-metavariable discipline of `gandr_core_unify` guarantees.
/// - ensures: returns `value` with each hole bound in `solutions` replaced by
///   its solution and every other node untouched, together with whether the
///   result still carries a hole. No binder blocks the replacement, because a
///   hole is not a bound name and a closed solution captures nothing.
/// - provides: the substitution half of the substitute-and-re-check evidence a
///   unification certificate offers.
/// - panics: none (the worklist's post-order balance keeps every result pop
///   defined; a `debug_assert` guards the invariant in test / debug builds).
///
/// # Adequacy
/// - hypothesis: L3 — three decision surfaces separated pointwise: a bound hole
///   is replaced, an unbound hole survives and sets the report, and a hole-free
///   term reports no survivor.
/// - witness: `subst::tests::substituting_a_bound_hole_replaces_it_and_reports_a_hole_free_result`
/// - witness: `subst::tests::substituting_leaves_an_unbound_hole_and_reports_it`
/// - witness: `subst::tests::substituting_a_hole_free_term_reports_no_survivor`
#[inline]
#[must_use]
pub fn subst_holes_value(
    value: &Value,
    solutions: &HoleSubstitution,
) -> (Value, HoleOccurrence)
{
    let mut engine = Subst::new(Rule::Holes(solutions), None);
    engine.work.push(Task::DescendValue(value));
    engine.run();
    let residual = engine.residual;
    (engine.take_value(), residual)
}

/// Replaces every solved hole inside a **computation** by its solution,
/// reporting whether any hole survived (the computation-sorted companion of
/// [`subst_holes_value`], with the identical contract).
///
/// # Contract
/// - requires: every solution in `solutions` is a closed term.
/// - ensures: returns `comp` with each bound hole replaced and whether the
///   result still carries a hole.
/// - provides: hole substitution at computation sort, which is where a
///   higher-order metavariable solution lands.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the computation-sorted image of the value entry,
///   separated by a solved computation hole under an application spine.
/// - witness: `subst::tests::substituting_a_computation_hole_replaces_it_under_an_application_spine`
#[inline]
#[must_use]
pub fn subst_holes_comp(
    comp: &Comp,
    solutions: &HoleSubstitution,
) -> (Comp, HoleOccurrence)
{
    let mut engine = Subst::new(Rule::Holes(solutions), None);
    engine.work.push(Task::DescendComp(comp));
    engine.run();
    let residual = engine.residual;
    (engine.take_comp(), residual)
}

/// What a solved hole is replaced by, at the sort the hole occupies.
///
/// The two sorts share one identifier space, so the sort is what decides which
/// occurrence a solution answers: a value solution replaces
/// [`Value::Hole`] and never [`Comp::Hole`], and the reverse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HoleRepl
{
    /// A value-sorted solution.
    Value(Rc<Value>),
    /// A computation-sorted solution.
    Comp(Rc<Comp>),
}

/// A finished map from hole identity to solution — the substitution a
/// unification certificate carries.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HoleSubstitution
{
    /// The solutions, keyed by hole identity in canonical order.
    entries: BTreeMap<HoleId, HoleRepl>,
}

impl HoleSubstitution
{
    /// An empty substitution.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Binds `hole` to `repl`, replacing any earlier binding.
    ///
    /// # Contract
    /// - requires: `repl` is a closed term at the sort `hole` occupies.
    /// - ensures: a later lookup of `hole` at that sort returns `repl`.
    /// - panics: none.
    #[inline]
    pub fn bind(
        &mut self,
        hole: HoleId,
        repl: HoleRepl,
    )
    {
        self.entries.insert(hole, repl);
    }

    /// The value-sorted solution of `hole`, when it has one.
    #[inline]
    #[must_use]
    pub fn value(
        &self,
        hole: HoleId,
    ) -> Option<&Rc<Value>>
    {
        match self.entries.get(&hole) {
            | Some(&HoleRepl::Value(ref term)) => Some(term),
            | Some(&HoleRepl::Comp(_)) | None => None,
        }
    }

    /// The computation-sorted solution of `hole`, when it has one.
    #[inline]
    #[must_use]
    pub fn comp(
        &self,
        hole: HoleId,
    ) -> Option<&Rc<Comp>>
    {
        match self.entries.get(&hole) {
            | Some(&HoleRepl::Comp(ref term)) => Some(term),
            | Some(&HoleRepl::Value(_)) | None => None,
        }
    }

    /// The bindings, in canonical hole order.
    #[inline]
    pub fn entries(&self) -> impl Iterator<Item = (HoleId, &HoleRepl)>
    {
        self.entries.iter().map(|(&hole, repl)| (hole, repl))
    }
}

/// Which substitution the engine is performing.
///
/// The engine descends one grammar and the rule decides only what a leaf
/// occurrence becomes and whether a binder blocks it.
#[derive(Clone, Copy, Debug)]
enum Rule<'src>
{
    /// Replace every free occurrence of the shadowable name by this value.
    Variable(&'src Value),
    /// Replace every solved hole by its solution; nothing shadows a hole.
    Holes(&'src HoleSubstitution),
}

/// One pending task on the substitution worklist — the defunctionalised image
/// of a recursive `subst_*` call (ADR-47 T1). A `Descend*` visits a source
/// node, rebuilding a leaf / whole-node-shadow directly or pushing a `Combine*`
/// followed by its substituted children; a `Combine*` re-reads the **same**
/// source node and reassembles it from those children, now on the result
/// stacks. Every task borrows the immutable input for the whole run, so a
/// `Descend`/`Combine` pair reads one source and recomputes the identical
/// shadowing decision — the two halves cannot desync.
enum Task<'src>
{
    /// Visit a computation.
    DescendComp(&'src Comp),
    /// Reassemble a computation from its substituted children.
    CombineComp(&'src Comp),
    /// Visit a value.
    DescendValue(&'src Value),
    /// Reassemble a value from its substituted children.
    CombineValue(&'src Value),
    /// Visit a reified stack.
    DescendStack(&'src Stack),
    /// Reassemble a reified stack from its substituted children.
    CombineStack(&'src Stack),
}

/// The iterative shadowing-aware substitution engine (ADR-47 T1): the driver
/// behind [`subst_value`] (and the computation / stack sub-substitutions it
/// inlines to reach thunk bodies and reified stacks). It owns an explicit LIFO
/// work stack and one result stack per syntactic sort, so substitution depth
/// follows the heap, not the host call stack — the iterative shadow of the
/// recursive specification (the Agda metatheory stays the oracle, ADR-47).
///
/// # Contract
/// - ensures: after [`Self::run`] drains a work stack seeded by exactly one
///   `Descend*`, the matching result stack holds exactly one rebuilt node and
///   the other result stacks are empty (the post-order balance invariant).
struct Subst<'src>
{
    /// Which substitution is being performed.
    rule: Rule<'src>,
    /// The binder name that blocks the rule, for a rule a binder can block.
    ///
    /// A variable substitution is blocked by a binder of the substituted name;
    /// a hole substitution is blocked by nothing, so this is `None` and every
    /// `Descend`/`Combine` pair takes the unshadowed branch.
    shadowed: Option<NameRef<'src>>,
    /// Whether a hole reached a leaf without a solution to replace it.
    residual: HoleOccurrence,
    /// Pending tasks, processed last-in-first-out (post order).
    work: Vec<Task<'src>>,
    /// Rebuilt computations, most-recent last.
    comps: Vec<Comp>,
    /// Rebuilt values, most-recent last.
    values: Vec<Value>,
    /// Rebuilt reified stacks, most-recent last.
    stacks: Vec<Stack>,
}

impl<'src> Subst<'src>
{
    /// Builds an empty engine applying `rule`, blocked by `shadowed`.
    fn new(
        rule: Rule<'src>,
        shadowed: Option<NameRef<'src>>,
    ) -> Self
    {
        Self {
            rule,
            shadowed,
            residual: HoleOccurrence::from(false),
            work: Vec::new(),
            comps: Vec::new(),
            values: Vec::new(),
            stacks: Vec::new(),
        }
    }

    /// Drains the work stack to completion (post-order rebuild).
    fn run(&mut self)
    {
        while let Some(task) = self.work.pop() {
            match task {
                | Task::DescendComp(node) => self.descend_comp(node),
                | Task::CombineComp(node) => self.combine_comp(node),
                | Task::DescendValue(node) => self.descend_value(node),
                | Task::CombineValue(node) => self.combine_value(node),
                | Task::DescendStack(node) => self.descend_stack(node),
                | Task::CombineStack(node) => self.combine_stack(node),
            }
        }
    }

    /// Pops the most-recent rebuilt computation.
    ///
    /// The post-order balance invariant guarantees a result is present at every
    /// pop; the `debug_assert` surfaces a broken invariant in test / debug
    /// builds, and the fallback keeps the pop total under the no-`unwrap` /
    /// no-`panic` lint wall — it is never reached (a desync would fail the
    /// substitution's own tests first).
    fn take_comp(&mut self) -> Comp
    {
        debug_assert!(
            !self.comps.is_empty(),
            "subst worklist underflow: a rebuilt computation must be present (ADR-47 post-order balance)"
        );
        self.comps.pop().unwrap_or_else(|| Comp::ret(Value::Unit))
    }

    /// Pops the most-recent rebuilt value (see [`Self::take_comp`]).
    fn take_value(&mut self) -> Value
    {
        debug_assert!(
            !self.values.is_empty(),
            "subst worklist underflow: a rebuilt value must be present (ADR-47 post-order balance)"
        );
        self.values.pop().unwrap_or(Value::Unit)
    }

    /// Pops the most-recent rebuilt reified stack (see [`Self::take_comp`]).
    fn take_stack(&mut self) -> Stack
    {
        debug_assert!(
            !self.stacks.is_empty(),
            "subst worklist underflow: a rebuilt stack must be present (ADR-47 post-order balance)"
        );
        self.stacks.pop().unwrap_or(Stack::Empty)
    }

    /// Visits a computation: rebuilds a leaf / whole-node-shadow directly, or
    /// pushes a [`Task::CombineComp`] and descends its substituted children in
    /// source order (mirrors the recursive substitution specification arm-for-
    /// arm; a shadowed child is not descended and is shared by `combine_comp`).
    fn descend_comp(
        &mut self,
        comp: &'src Comp,
    )
    {
        match *comp {
            | Comp::Abs(ref binder, _, ref body) => {
                if self.shadowed == Some(NameRef::from(binder.as_str())) {
                    self.comps.push(comp.clone());
                }
                else {
                    self.work.push(Task::CombineComp(comp));
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            | Comp::App(ref head, ref arg) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(head.as_ref()));
                self.work.push(Task::DescendValue(arg.as_ref()));
            },
            // `ret v`, `force v`, `dup v`, `drop v`, and `perform … v` each
            // descend a single value child; only the reassembly differs, so the
            // descent is shared here (`combine_comp` re-matches to rebuild each).
            | Comp::Ret(ref value)
            | Comp::Force(ref value)
            | Comp::Dup(ref value)
            | Comp::Drop(ref value)
            | Comp::Perform(_, _, ref value) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(value.as_ref()));
            },
            | Comp::Bind(ref bound, ref binder, ref body) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(bound.as_ref()));
                if self.shadowed != Some(NameRef::from(binder.as_str())) {
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            | Comp::Case(ref scrut, ref arm_fst, ref arm_snd) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                if self.shadowed != Some(NameRef::from(arm_fst.0.as_str())) {
                    self.work.push(Task::DescendComp(arm_fst.1.as_ref()));
                }
                if self.shadowed != Some(NameRef::from(arm_snd.0.as_str())) {
                    self.work.push(Task::DescendComp(arm_snd.1.as_ref()));
                }
            },
            | Comp::ListCase {
                ref scrut,
                ref nil,
                ref head,
                ref tail,
                ref cons,
            } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                self.work.push(Task::DescendComp(nil.as_ref()));
                // The `cons` body is under `head`/`tail`; descend it only when
                // neither binder rebinds `name` (the `nil` body always descends).
                if self.shadowed != Some(NameRef::from(head.as_str()))
                    && self.shadowed != Some(NameRef::from(tail.as_str()))
                {
                    self.work.push(Task::DescendComp(cons.as_ref()));
                }
            },
            // The motive is a *type* (runtime-erased, untraced) carried verbatim
            // by the combine arm — exactly as an `Abs` binder annotation and the
            // `Walk` motive are not substituted here (ADR-82 D4); only the
            // scrutinee value and the body (under `p`/`q`) are descended.
            | Comp::Split {
                ref scrut,
                ref fst_name,
                ref snd_name,
                ref body,
                ..
            } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                if self.shadowed != Some(NameRef::from(fst_name.as_str()))
                    && self.shadowed != Some(NameRef::from(snd_name.as_str()))
                {
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            // Each arm body is under its own payload binder; descend only the
            // arms that binder does not rebind `name` (ADR-80), in source order.
            | Comp::DataCase(ref scrut, ref arms) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                for arm in arms {
                    if self.shadowed != Some(NameRef::from(arm.0.as_str())) {
                        self.work.push(Task::DescendComp(arm.1.as_ref()));
                    }
                }
            },
            | Comp::With(ref fst, ref snd) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(fst.as_ref()));
                self.work.push(Task::DescendComp(snd.as_ref()));
            },
            | Comp::Prj(_, ref target) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(target.as_ref()));
            },
            // The label is not a binder, so substitution descends into the
            // record value unconditionally (ADR-45 D4).
            | Comp::RecordProj { ref record, .. } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(record.as_ref()));
            },
            | Comp::Handle {
                ref scrutinee,
                ret: (ref ret_var, ref ret_body),
                ref ops,
                ..
            } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(scrutinee.as_ref()));
                if self.shadowed != Some(NameRef::from(ret_var.as_str())) {
                    self.work.push(Task::DescendComp(ret_body.as_ref()));
                }
                // Each clause body is under its own payload / resume binders;
                // descend only the clauses neither rebinds `name`, in order.
                for clause in ops {
                    if self.shadowed != Some(NameRef::from(clause.payload.as_str()))
                        && self.shadowed != Some(NameRef::from(clause.resume.as_str()))
                    {
                        self.work.push(Task::DescendComp(clause.body.as_ref()));
                    }
                }
            },
            | Comp::Resume(ref reified, ref fed) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(reified.as_ref()));
                self.work.push(Task::DescendComp(fed.as_ref()));
            },
            | Comp::Reset(ref body) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendComp(body.as_ref()));
            },
            | Comp::Shift(ref k, ref body) => {
                if self.shadowed == Some(NameRef::from(k.as_str())) {
                    self.comps.push(comp.clone());
                }
                else {
                    self.work.push(Task::CombineComp(comp));
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            // The fixpoint binds its self-reference over the body, so it
            // shadows exactly as a `Shift` binds its continuation variable.
            | Comp::Fix(ref x, ref body) => {
                if self.shadowed == Some(NameRef::from(x.as_str())) {
                    self.comps.push(comp.clone());
                }
                else {
                    self.work.push(Task::CombineComp(comp));
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            // The computation-sorted half of the hole rule, with the same
            // survivor report as its value-sorted twin.
            | Comp::Hole(hole) => {
                let solution = match self.rule {
                    | Rule::Holes(solutions) => solutions.comp(HoleId::from(hole)),
                    | Rule::Variable(_) => None,
                };
                match solution {
                    | Some(term) => self.comps.push(term.as_ref().clone()),
                    | None => {
                        self.residual = HoleOccurrence::from(true);
                        self.comps.push(comp.clone());
                    },
                }
            },
            // A native builtin carries its accumulated argument values (closed in
            // a closed program, and empty in a source term) — descend into each
            // so substitution stays total (ADR-42); the opaque `prim` is
            // unaffected.
            | Comp::Native { ref args, .. } => {
                self.work.push(Task::CombineComp(comp));
                for arg in args {
                    self.work.push(Task::DescendValue(arg.as_ref()));
                }
            },
            // The identity eliminator (ADR-76): descend into the scrutinee value
            // and the base body (under the diagonal binder `x`, so descend it
            // only when `x` does not rebind `name`). The motive is a *type*
            // (runtime-erased, untraced) and is carried verbatim, exactly as an
            // `Abs` binder annotation is not substituted here.
            | Comp::Walk {
                ref scrut,
                ref base,
                ..
            } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                if self.shadowed != Some(NameRef::from(base.x.as_str())) {
                    self.work.push(Task::DescendComp(base.body.as_ref()));
                }
            },
            // The module variable binds a **value**, so it shadows exactly as
            // `Bind`'s does. The ascribed signature is a type and is shared
            // through, the same treatment an ascription's type gets here.
            | Comp::Unpack {
                ref scrut,
                ref binder,
                ref body,
                ..
            } => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                if self.shadowed != Some(NameRef::from(binder.as_str())) {
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
        }
    }

    /// Reassembles a computation from its substituted children on the result
    /// stacks (re-reads the same source node as [`Self::descend_comp`], so the
    /// shadowing decision is recomputed identically; children pop in source
    /// order — the two LIFO reversals of work and result stacks cancel).
    fn combine_comp(
        &mut self,
        comp: &'src Comp,
    )
    {
        let rebuilt = match *comp {
            | Comp::Abs(ref binder, ref annot, _) => {
                Comp::Abs(binder.clone(), annot.clone(), Rc::new(self.take_comp()))
            },
            | Comp::App(..) => {
                let head = self.take_comp();
                let arg = self.take_value();
                Comp::App(Rc::new(head), Rc::new(arg))
            },
            | Comp::Ret(_) => Comp::Ret(Rc::new(self.take_value())),
            | Comp::Bind(_, ref binder, ref body) => {
                let bound = self.take_comp();
                let body_sub = if self.shadowed == Some(NameRef::from(binder.as_str())) {
                    Rc::clone(body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::Bind(Rc::new(bound), binder.clone(), body_sub)
            },
            | Comp::Force(_) => Comp::Force(Rc::new(self.take_value())),
            | Comp::Case(_, ref arm_fst, ref arm_snd) => {
                let scrut = self.take_value();
                let fst_body = if self.shadowed == Some(NameRef::from(arm_fst.0.as_str())) {
                    Rc::clone(&arm_fst.1)
                }
                else {
                    Rc::new(self.take_comp())
                };
                let snd_body = if self.shadowed == Some(NameRef::from(arm_snd.0.as_str())) {
                    Rc::clone(&arm_snd.1)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::Case(
                    Rc::new(scrut),
                    (arm_fst.0.clone(), fst_body),
                    (arm_snd.0.clone(), snd_body),
                )
            },
            | Comp::ListCase {
                ref head,
                ref tail,
                ref cons,
                ..
            } => {
                let scrut = self.take_value();
                let nil = self.take_comp();
                let cons_sub = if self.shadowed == Some(NameRef::from(head.as_str()))
                    || self.shadowed == Some(NameRef::from(tail.as_str()))
                {
                    Rc::clone(cons)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::ListCase {
                    scrut: Rc::new(scrut),
                    nil: Rc::new(nil),
                    head: head.clone(),
                    tail: tail.clone(),
                    cons: cons_sub,
                }
            },
            // The motive is carried verbatim (a runtime-erased type child, the
            // `Walk` precedent; ADR-82 D4) — only the scrutinee value and the
            // body (unless `p`/`q` shadow `name`) are substituted.
            | Comp::Split {
                ref fst_name,
                ref snd_name,
                ref motive,
                ref body,
                ..
            } => {
                let scrut = self.take_value();
                let body_sub = if self.shadowed == Some(NameRef::from(fst_name.as_str()))
                    || self.shadowed == Some(NameRef::from(snd_name.as_str()))
                {
                    Rc::clone(body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::Split {
                    scrut: Rc::new(scrut),
                    fst_name: fst_name.clone(),
                    snd_name: snd_name.clone(),
                    motive: motive.clone(),
                    body: body_sub,
                }
            },
            | Comp::DataCase(_, ref arms) => {
                let scrut = self.take_value();
                let mut arms_sub = Vec::with_capacity(arms.len());
                for arm in arms {
                    if self.shadowed == Some(NameRef::from(arm.0.as_str())) {
                        arms_sub.push((arm.0.clone(), Rc::clone(&arm.1)));
                    }
                    else {
                        arms_sub.push((arm.0.clone(), Rc::new(self.take_comp())));
                    }
                }
                Comp::DataCase(Rc::new(scrut), arms_sub)
            },
            | Comp::With(..) => {
                let fst = self.take_comp();
                let snd = self.take_comp();
                Comp::With(Rc::new(fst), Rc::new(snd))
            },
            | Comp::Prj(side, _) => Comp::Prj(side, Rc::new(self.take_comp())),
            | Comp::RecordProj { ref label, .. } => Comp::RecordProj {
                record: Rc::new(self.take_value()),
                label: label.clone(),
            },
            | Comp::Dup(_) => Comp::Dup(Rc::new(self.take_value())),
            | Comp::Drop(_) => Comp::Drop(Rc::new(self.take_value())),
            | Comp::Perform(ref sig, ref op, _) => {
                Comp::Perform(sig.clone(), op.clone(), Rc::new(self.take_value()))
            },
            | Comp::Handle {
                ref sig,
                ret: (ref ret_var, ref ret_body),
                ref ops,
                ..
            } => {
                let scrutinee = self.take_comp();
                let ret_sub = if self.shadowed == Some(NameRef::from(ret_var.as_str())) {
                    Rc::clone(ret_body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                let mut ops_sub = Vec::with_capacity(ops.len());
                for clause in ops {
                    if self.shadowed == Some(NameRef::from(clause.payload.as_str()))
                        || self.shadowed == Some(NameRef::from(clause.resume.as_str()))
                    {
                        ops_sub.push(clause.clone());
                    }
                    else {
                        ops_sub.push(OpClause {
                            op: clause.op.clone(),
                            payload: clause.payload.clone(),
                            resume: clause.resume.clone(),
                            body: Rc::new(self.take_comp()),
                        });
                    }
                }
                Comp::Handle {
                    sig: sig.clone(),
                    scrutinee: Rc::new(scrutinee),
                    ret: (ret_var.clone(), ret_sub),
                    ops: ops_sub,
                }
            },
            | Comp::Resume(..) => {
                let reified = self.take_value();
                let fed = self.take_comp();
                Comp::Resume(Rc::new(reified), Rc::new(fed))
            },
            | Comp::Reset(_) => Comp::Reset(Rc::new(self.take_comp())),
            | Comp::Shift(ref k, _) => Comp::Shift(k.clone(), Rc::new(self.take_comp())),
            | Comp::Fix(ref x, _) => Comp::Fix(x.clone(), Rc::new(self.take_comp())),
            // A leaf / whole-node-shadow is rebuilt in `descend_comp` and never
            // reaches a combine; the arm is required only for exhaustiveness.
            | Comp::Hole(_) => comp.clone(),
            | Comp::Native { prim, ref args } => {
                let mut built = Vec::with_capacity(args.len());
                for _ in args {
                    built.push(Rc::new(self.take_value()));
                }
                Comp::Native { prim, args: built }
            },
            | Comp::Walk {
                ref motive,
                ref base,
                ..
            } => {
                let scrut = self.take_value();
                let base_body = if self.shadowed == Some(NameRef::from(base.x.as_str())) {
                    Rc::clone(&base.body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::Walk {
                    scrut: Rc::new(scrut),
                    motive: motive.clone(),
                    base: WalkBase {
                        x: base.x.clone(),
                        body: base_body,
                    },
                }
            },
            | Comp::Unpack {
                ref signature,
                ref atoms,
                ref binder,
                ref body,
                ..
            } => {
                let scrut = self.take_value();
                let body_sub = if self.shadowed == Some(NameRef::from(binder.as_str())) {
                    Rc::clone(body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                Comp::Unpack {
                    scrut: Rc::new(scrut),
                    signature: Rc::clone(signature),
                    atoms: atoms.clone(),
                    binder: binder.clone(),
                    body: body_sub,
                }
            },
        };
        self.comps.push(rebuilt);
    }

    /// Visits a value (mirrors the recursive `subst_value`).
    fn descend_value(
        &mut self,
        value: &'src Value,
    )
    {
        match *value {
            | Value::Var(ref var) => {
                let substituted = match self.rule {
                    | Rule::Variable(repl) if self.shadowed == Some(NameRef::from(var.as_str())) => {
                        repl.clone()
                    },
                    | Rule::Variable(_) | Rule::Holes(_) => value.clone(),
                };
                self.values.push(substituted);
            },
            // The value-sorted half of the hole rule. An unsolved hole is
            // rebuilt and recorded, because a survivor is what makes a later
            // conversion verdict vacuous rather than conclusive.
            | Value::Hole(hole) => {
                let solution = match self.rule {
                    | Rule::Holes(solutions) => solutions.value(HoleId::from(hole)),
                    | Rule::Variable(_) => None,
                };
                match solution {
                    | Some(term) => self.values.push(term.as_ref().clone()),
                    | None => {
                        self.residual = HoleOccurrence::from(true);
                        self.values.push(value.clone());
                    },
                }
            },
            | Value::Unit | Value::Int(_) | Value::Str(_) | Value::Num(_) => {
                self.values.push(value.clone());
            },
            | Value::Pair(ref fst, ref snd) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(fst.as_ref()));
                self.work.push(Task::DescendValue(snd.as_ref()));
            },
            | Value::Inj(_, ref payload) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(payload.as_ref()));
            },
            // A list's elements are ordinary values: descend into each (ADR-40
            // D2).
            | Value::List(ref elements) => {
                self.work.push(Task::CombineValue(value));
                for element in elements {
                    self.work.push(Task::DescendValue(element.as_ref()));
                }
            },
            // A record's field values are ordinary values: descend into each,
            // preserving the labels (ADR-45 D2). Iteration is canonical key
            // order (`Value::Record` is a `BTreeMap`), matched by `combine_value`.
            | Value::Record(ref fields) => {
                self.work.push(Task::CombineValue(value));
                for field in fields.values() {
                    self.work.push(Task::DescendValue(field.as_ref()));
                }
            },
            | Value::Thunk(_, ref body) | Value::Run(ref body) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendComp(body.as_ref()));
            },
            | Value::Annot(ref inner, _) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(inner.as_ref()));
            },
            | Value::Stk(ref stack) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendStack(stack.as_ref()));
            },
            // A reflexivity proof carries an ordinary value witness (ADR-76);
            // descend into it exactly as an injection payload.
            | Value::Here(ref witness) => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(witness.as_ref()));
            },
            // A declared-data constructor carries an ordinary field-tuple
            // payload (ADR-80); descend into it exactly as an injection payload.
            // A declared-data constructor carries a field-tuple payload and a
            // pack carries its module payload; both descend that one value
            // child. A pack's witnesses are types and are shared through, the
            // treatment an ascription's type gets here.
            | Value::Ctor {
                payload: ref field, ..
            }
            | Value::Pack {
                payload: ref field, ..
            } => {
                self.work.push(Task::CombineValue(value));
                self.work.push(Task::DescendValue(field.as_ref()));
            },
        }
    }

    /// Reassembles a value from its substituted children (re-reads the source
    /// node; children pop in source order).
    fn combine_value(
        &mut self,
        value: &'src Value,
    )
    {
        let rebuilt = match *value {
            | Value::Pair(..) => {
                let fst = self.take_value();
                let snd = self.take_value();
                Value::Pair(Rc::new(fst), Rc::new(snd))
            },
            | Value::Inj(side, _) => Value::Inj(side, Rc::new(self.take_value())),
            | Value::Ctor { ref id, tag, .. } => Value::Ctor {
                id: id.clone(),
                tag,
                payload: Rc::new(self.take_value()),
            },
            | Value::List(ref elements) => {
                let mut built = Vec::with_capacity(elements.len());
                for _ in elements {
                    built.push(Rc::new(self.take_value()));
                }
                Value::List(built)
            },
            | Value::Record(ref fields) => {
                let mut built = BTreeMap::new();
                // `fields.values()` (descend) and this key iteration walk the
                // BTreeMap in the same canonical order, so `take_value` returns
                // each field's substituted value in the matching order.
                for label in fields.keys() {
                    let field = self.take_value();
                    built.insert(label.clone(), Rc::new(field));
                }
                Value::Record(built)
            },
            | Value::Thunk(grade, _) => Value::Thunk(grade, Rc::new(self.take_comp())),
            | Value::Run(_) => Value::Run(Rc::new(self.take_comp())),
            | Value::Annot(_, ref ty) => Value::Annot(Rc::new(self.take_value()), Rc::clone(ty)),
            | Value::Stk(_) => Value::Stk(Rc::new(self.take_stack())),
            | Value::Here(_) => Value::Here(Rc::new(self.take_value())),
            | Value::Pack { ref witnesses, .. } => Value::Pack {
                witnesses: witnesses.clone(),
                payload: Rc::new(self.take_value()),
            },
            // Leaves are rebuilt in `descend_value` and never reach a combine;
            // the arm is required only for exhaustiveness.
            | Value::Var(_)
            | Value::Unit
            | Value::Int(_)
            | Value::Str(_)
            | Value::Num(_)
            | Value::Hole(_) => value.clone(),
        };
        self.values.push(rebuilt);
    }

    /// Visits a reified stack (mirrors the recursive `subst_stack`).
    fn descend_stack(
        &mut self,
        stack: &'src Stack,
    )
    {
        match *stack {
            | Stack::Empty => self.stacks.push(Stack::Empty),
            | Stack::Arg(ref value, ref rest) => {
                self.work.push(Task::CombineStack(stack));
                self.work.push(Task::DescendValue(value.as_ref()));
                self.work.push(Task::DescendStack(rest.as_ref()));
            },
            | Stack::Bind(ref binder, ref body, ref rest) => {
                self.work.push(Task::CombineStack(stack));
                if self.shadowed != Some(NameRef::from(binder.as_str())) {
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
                self.work.push(Task::DescendStack(rest.as_ref()));
            },
            | Stack::Prj(_, ref rest) => {
                self.work.push(Task::CombineStack(stack));
                self.work.push(Task::DescendStack(rest.as_ref()));
            },
        }
    }

    /// Reassembles a reified stack from its substituted children (re-reads the
    /// source node; the value / bind-body and the rest live on distinct result
    /// stacks, so their pop order is independent).
    fn combine_stack(
        &mut self,
        stack: &'src Stack,
    )
    {
        let rebuilt = match *stack {
            | Stack::Arg(..) => {
                let value = self.take_value();
                let rest = self.take_stack();
                Stack::Arg(Rc::new(value), Rc::new(rest))
            },
            | Stack::Bind(ref binder, ref body, _) => {
                let body_sub = if self.shadowed == Some(NameRef::from(binder.as_str())) {
                    Rc::clone(body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                let rest = self.take_stack();
                Stack::Bind(binder.clone(), body_sub, Rc::new(rest))
            },
            | Stack::Prj(side, _) => Stack::Prj(side, Rc::new(self.take_stack())),
            // `Stack::Empty` is rebuilt in `descend_stack` and never reaches a
            // combine; the arm is required only for exhaustiveness.
            | Stack::Empty => Stack::Empty,
        };
        self.stacks.push(rebuilt);
    }
}

#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;

    use crate::boundary::HoleId;
    use crate::subst::HoleRepl;
    use crate::subst::HoleSubstitution;
    use crate::subst::subst_holes_comp;
    use crate::subst::subst_holes_value;
    use crate::syntax::Comp;
    use crate::syntax::Value;

    /// An integer literal, as a shared value node.
    fn int(literal: IntegerLiteral) -> Rc<Value>
    {
        Rc::new(Value::Int(i64::from(literal)))
    }

    /// An integer literal a fixture writes down.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct IntegerLiteral(i64);

    impl From<IntegerLiteral> for i64
    {
        #[inline]
        fn from(value: IntegerLiteral) -> Self
        {
            value.0
        }
    }

    /// A value-sorted hole, as a shared value node.
    fn value_hole(hole: HoleId) -> Rc<Value>
    {
        Rc::new(Value::Hole(u32::from(hole)))
    }

    #[test]
    fn substituting_a_bound_hole_replaces_it_and_reports_a_hole_free_result()
    {
        // The first decision surface: a hole the substitution binds is replaced
        // by its solution, and the result reports no survivor. The pair's other
        // component is asserted unchanged, so a mutant that rewrites the whole
        // term rather than the bound occurrence is separated too.
        let mut solutions = HoleSubstitution::new();
        solutions.bind(HoleId::from(0), HoleRepl::Value(int(IntegerLiteral(3))));
        let (term, holes) = subst_holes_value(
            &Value::Pair(value_hole(HoleId::from(0)), int(IntegerLiteral(1))),
            &solutions,
        );
        assert_eq!(
            Value::Pair(int(IntegerLiteral(3)), int(IntegerLiteral(1))),
            term,
            "the bound hole is replaced and every other node is untouched"
        );
        assert!(
            !bool::from(holes),
            "and nothing hole-shaped survives, so the report is negative"
        );
    }

    #[test]
    fn substituting_leaves_an_unbound_hole_and_reports_it()
    {
        // The second: a hole the substitution does not bind survives verbatim,
        // and the report goes positive. Same term as above against an empty
        // substitution, so the report is the only thing that can differ.
        let solutions = HoleSubstitution::new();
        let (term, holes) = subst_holes_value(
            &Value::Pair(value_hole(HoleId::from(0)), int(IntegerLiteral(1))),
            &solutions,
        );
        assert_eq!(
            Value::Pair(value_hole(HoleId::from(0)), int(IntegerLiteral(1))),
            term,
            "an unbound hole is left exactly as it was"
        );
        assert!(
            bool::from(holes),
            "and the survivor is reported, which is what the caller re-checks on"
        );
    }

    #[test]
    fn substituting_a_hole_free_term_reports_no_survivor()
    {
        // The third: a term with no hole at all reports nothing, which
        // separates a mutant that sets the residual unconditionally from one
        // that sets it on a surviving occurrence.
        let mut solutions = HoleSubstitution::new();
        solutions.bind(HoleId::from(0), HoleRepl::Value(int(IntegerLiteral(3))));
        let (term, holes) = subst_holes_value(
            &Value::Pair(int(IntegerLiteral(2)), int(IntegerLiteral(1))),
            &solutions,
        );
        assert_eq!(
            Value::Pair(int(IntegerLiteral(2)), int(IntegerLiteral(1))),
            term,
            "a hole-free term is returned unchanged"
        );
        assert!(
            !bool::from(holes),
            "and reports no survivor, though the substitution was non-empty"
        );
    }

    #[test]
    fn substituting_a_computation_hole_replaces_it_under_an_application_spine()
    {
        // The computation-sorted image of the value entry, at the position the
        // hypothesis names: a solved hole in head position under an
        // application. The assertion is structural rather than by conversion,
        // because what this function does is replace the node — reducing the
        // redex afterwards is the evaluator's job, and a witness that ran it
        // would pass on a substitution that landed the solution elsewhere.
        let solution = Comp::lam("z", Comp::ret(Value::var("z")));
        let mut solutions = HoleSubstitution::new();
        solutions.bind(HoleId::from(0), HoleRepl::Comp(Rc::new(solution.clone())));
        let (term, holes) = subst_holes_comp(&Comp::app(Comp::Hole(0), Value::Int(4)), &solutions);
        assert_eq!(
            Comp::app(solution, Value::Int(4)),
            term,
            "the solution lands in head position and the argument is untouched"
        );
        assert!(
            !bool::from(holes),
            "and the spine carries no surviving hole"
        );
    }
}
