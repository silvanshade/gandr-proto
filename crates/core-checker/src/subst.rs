//! Capture-avoiding value substitution (ADR-47 T1) — the iterative worklist
//! engine `𝓕`'s type-side motive instantiation shares.
//!
//! [`subst_value`] replaces a free value variable inside a source value,
//! respecting binder shadowing. It is the substitution the identity former's
//! motive instantiation drives ([`crate::identity`] calls it at each
//! [`crate::types::ValueType::Path`] endpoint). The engine ([`Subst`]) owns an
//! explicit LIFO work stack and one result stack per syntactic sort, so
//! substitution depth follows the heap, not the host call stack — the iterative
//! shadow of the recursive specification (the Agda metatheory stays the oracle,
//! ADR-47). It is a **durable** helper: it outlived the CEK evaluator that once
//! co-hosted it (its computation-level companion, the CEK's `subst_comp`,
//! retired with that machine).

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::Vec;

use crate::boundary::NameRef;
use crate::syntax::Comp;
use crate::syntax::OpClause;
use crate::syntax::Stack;
use crate::syntax::Value;
use crate::syntax::WalkBase;

/// Capture-avoiding substitution of `repl` for the free value variable `name`
/// inside a **value** — the value-into-value entry of the iterative [`Subst`]
/// engine (the ADR-47 traversal the CEK's computation-level substitution once
/// shared, reusing its binder-shadowing discipline).
///
/// This is the substitution the identity former's motive instantiation drives
/// (`crate::identity` calls it at each [`crate::types::ValueType::Path`]
/// endpoint). Exposed to the crate (`pub(crate)`) precisely so motive
/// instantiation shares one proven substitution rather than reimplementing
/// capture-avoidance.
///
/// # Contract
/// - ensures: returns `value` with every free `name` replaced by `repl`,
///   leaving occurrences under a rebinding of `name` (a thunked computation's
///   binders) untouched; structurally identical to the direct recursive
///   definition.
/// - panics: none (the worklist's post-order balance keeps every result pop
///   defined; a `debug_assert` guards the invariant in test / debug builds).
#[must_use]
pub(crate) fn subst_value<'source, N>(
    value: &Value,
    name: N,
    repl: &Value,
) -> Value
where
    N: Into<NameRef<'source>>,
{
    let mut engine = Subst::new(name.into(), repl);
    engine.work.push(Task::DescendValue(value));
    engine.run();
    engine.take_value()
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

/// The iterative capture-avoiding substitution engine (ADR-47 T1): the driver
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
    /// The variable being replaced.
    name: &'src str,
    /// The replacement value (cloned at each matching free [`Value::Var`]).
    repl: &'src Value,
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
    /// Builds an empty engine substituting `repl` for `name`.
    fn new(
        name: NameRef<'src>,
        repl: &'src Value,
    ) -> Self
    {
        Self {
            name: <&str>::from(name),
            repl,
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
                if binder == self.name {
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
                if binder != self.name {
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            | Comp::Case(ref scrut, ref arm_fst, ref arm_snd) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                if arm_fst.0 != self.name {
                    self.work.push(Task::DescendComp(arm_fst.1.as_ref()));
                }
                if arm_snd.0 != self.name {
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
                if head != self.name && tail != self.name {
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
                if fst_name != self.name && snd_name != self.name {
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            // Each arm body is under its own payload binder; descend only the
            // arms that binder does not rebind `name` (ADR-80), in source order.
            | Comp::DataCase(ref scrut, ref arms) => {
                self.work.push(Task::CombineComp(comp));
                self.work.push(Task::DescendValue(scrut.as_ref()));
                for arm in arms {
                    if arm.0 != self.name {
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
                if ret_var != self.name {
                    self.work.push(Task::DescendComp(ret_body.as_ref()));
                }
                // Each clause body is under its own payload / resume binders;
                // descend only the clauses neither rebinds `name`, in order.
                for clause in ops {
                    if clause.payload != self.name && clause.resume != self.name {
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
                if k == self.name {
                    self.comps.push(comp.clone());
                }
                else {
                    self.work.push(Task::CombineComp(comp));
                    self.work.push(Task::DescendComp(body.as_ref()));
                }
            },
            | Comp::Hole(_) => self.comps.push(comp.clone()),
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
                if base.x != self.name {
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
                if binder != self.name {
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
                let body_sub = if binder == self.name {
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
                let fst_body = if arm_fst.0 == self.name {
                    Rc::clone(&arm_fst.1)
                }
                else {
                    Rc::new(self.take_comp())
                };
                let snd_body = if arm_snd.0 == self.name {
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
                let cons_sub = if head == self.name || tail == self.name {
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
                let body_sub = if fst_name == self.name || snd_name == self.name {
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
                    if arm.0 == self.name {
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
                let ret_sub = if ret_var == self.name {
                    Rc::clone(ret_body)
                }
                else {
                    Rc::new(self.take_comp())
                };
                let mut ops_sub = Vec::with_capacity(ops.len());
                for clause in ops {
                    if clause.payload == self.name || clause.resume == self.name {
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
                let base_body = if base.x == self.name {
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
                let body_sub = if binder == self.name {
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
                let substituted = if var == self.name {
                    self.repl.clone()
                }
                else {
                    value.clone()
                };
                self.values.push(substituted);
            },
            | Value::Unit | Value::Int(_) | Value::Str(_) | Value::Num(_) | Value::Hole(_) => {
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
            | Value::Thunk(_, ref body) => {
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
                if binder != self.name {
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
                let body_sub = if binder == self.name {
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
