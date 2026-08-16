//! Readback: a semantic value back into syntax, iteratively, in one of three
//! modes.
//!
//! # Readback chooses a face
//!
//! The value domain carries two faces and readback is where the choice is
//! visible:
//!
//! * [`QuoteMode::Retained`] prefers the **term face** — a value that never
//!   reduced hands back the source term it came from, so quoting an unreduced
//!   subterm costs one reference-count clone and never rebuilds anything;
//! * [`QuoteMode::Canonical`] ignores the term face and rebuilds from the
//!   semantic value, so binders come out in **canonical form** — a de Bruijn
//!   level rendered into a name — which is what makes two readback results
//!   comparable by structure alone;
//! * [`QuoteMode::Unfolding`] rebuilds and additionally spends the definitional
//!   environment, so a definition's body appears in the result.
//!
//! The "which face" policy here and the "which side unfolds" policy in
//! [`conv`] read the **same** unfolding face and the same transparency table.
//! That is the whole of the discipline that keeps them from drifting.
//!
//! # Under a binder
//!
//! Going under a binder mints a fresh **de Bruijn level** and binds the source
//! name to it, so alpha-equivalence becomes identity and the binder rendered
//! into the result depends only on how many binders are open — never on an
//! address, an allocation order, or a hash-map iteration order. Readback is
//! deterministic for that reason and not by accident.
//!
//! [`conv`]: crate::nbe::conv

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::boundary::VariableLevel;
use crate::grade::Grade;
use crate::nbe::Normalizer;
use crate::nbe::eval::ForceMode;
use crate::nbe::eval::eval_comp;
use crate::nbe::eval::force_value;
use crate::nbe::eval::value;
use crate::nbe::sem::ClosureId;
use crate::nbe::sem::Elim;
use crate::nbe::sem::NeutralHead;
use crate::nbe::sem::Rigid;
use crate::nbe::sem::SemCompId;
use crate::nbe::sem::SemCompNode;
use crate::nbe::sem::SemError;
use crate::nbe::sem::SemValueId;
use crate::nbe::sem::SemValueNode;
use crate::nbe::sem::ValueUnfold;
use crate::syntax::Comp;
use crate::syntax::OpClause;
use crate::syntax::Side;
use crate::syntax::Value;
use crate::syntax::WalkBase;
use crate::types::DataId;

/// How readback treats the two faces and the definitional environment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum QuoteMode
{
    /// Prefer the term face and unfold nothing: the cheapest readback, and the
    /// one whose result carries the source's own binder names wherever nothing
    /// reduced.
    #[default]
    Retained,
    /// Ignore the term face and unfold nothing: rebuild from the semantic
    /// value, so binders come out canonical and two results are comparable by
    /// structure alone.
    Canonical,
    /// Rebuild and spend the definitional environment, bounded by the
    /// normalizer's fuel.
    Unfolding,
}

impl QuoteMode
{
    /// The force mode readback drives its heads with.
    #[inline]
    #[must_use]
    fn force(self) -> ForceMode
    {
        match self {
            | Self::Retained | Self::Canonical => ForceMode::WeakHead,
            | Self::Unfolding => ForceMode::Unfold,
        }
    }
}

/// The name a de Bruijn level is rendered into.
///
/// The bracket characters are outside the identifier grammar the surface
/// parses, so a generated binder can never collide with a source name.
#[inline]
#[must_use]
fn level_name(level: VariableLevel) -> String
{
    format!("\u{ab}{}\u{bb}", u32::from(level))
}

/// Opens a closure under fresh levels: binds each binder to a rigid variable
/// and drives the body to weak-head normal form.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn open(
    nbe: &mut Normalizer,
    cell: ClosureId,
    mode: QuoteMode,
) -> Result<(Vec<String>, SemCompId), SemError>
{
    let (mut env, binders, body) = {
        let closure = nbe.arena().closure(cell)?;
        (
            closure.env(),
            closure.binders().to_vec(),
            Rc::clone(closure.body()),
        )
    };
    let mut names = Vec::with_capacity(binders.len());
    for binder in binders {
        let level = nbe.fresh_level();
        let node = SemValueNode::Rigid(Rigid::Level(level), ValueUnfold::Rigid);
        let rigid = value(nbe, node)?;
        env = nbe.arena_mut().bind(env, binder, rigid)?;
        names.push(level_name(level));
    }
    let whnf = eval_comp(nbe, env, &body, mode.force())?;
    Ok((names, whnf))
}

/// What a value-side finish frame reassembles.
enum ValueFinish
{
    /// Rebuild an eager pair.
    Pair,
    /// Rebuild a sum injection.
    Inj(Side),
    /// Rebuild a list of this many elements.
    List(usize),
    /// Rebuild a record over these labels.
    Record(Vec<String>),
    /// Rebuild a reflexivity witness.
    Here,
    /// Rebuild a constructor value.
    Ctor(DataId, usize),
    /// Rebuild a thunk from the computation on the computation stack.
    Thunk(Grade),
}

/// What a computation-side finish frame reassembles.
enum CompFinish
{
    /// Rebuild a lambda over this binder.
    Abs(String),
    /// Rebuild a returner.
    Ret,
    /// Rebuild a lazy pair.
    With,
    /// Rebuild a force.
    Force,
    /// Rebuild a sum elimination over these branch binders.
    Case(String, String),
    /// Rebuild a declared-data elimination over these arm names and binders.
    DataCase(Vec<(String, String)>),
    /// Rebuild a list elimination over the cons binders.
    ListCase(String, String),
    /// Rebuild a pair elimination over its two binders.
    Split(String, String),
    /// Rebuild a record projection at this label.
    Project(String),
    /// Rebuild an identity elimination over the diagonal binder.
    Walk(String),
    /// Rebuild a native application over this many arguments.
    Native(crate::prim::NativePrim, usize),
    /// Rebuild a grade duplication.
    Dup,
    /// Rebuild a grade discard.
    Drop,
    /// Rebuild an effect performance.
    Perform(Rc<crate::effect::EffectSig>, String),
    /// Rebuild a handler over its return binder and its operation clauses.
    Handle(
        Rc<crate::effect::EffectSig>,
        String,
        Vec<(String, String, String)>,
    ),
    /// Rebuild a resumption.
    Resume,
    /// Rebuild a delimiter.
    Reset,
    /// Rebuild a capture over its binder.
    Shift(String),
    /// Rebuild a computation hole.
    Hole(crate::syntax::HoleId),
    /// Re-apply one frustrated application to the computation beneath it.
    Apply,
    /// Re-apply one frustrated projection.
    Prj(Side),
    /// Re-apply one frustrated sequencing continuation over its binder.
    Sequence(String),
}

/// One pending step of the readback walk.
enum Task
{
    /// Read back a value.
    Value(SemValueId),
    /// Read back a computation.
    Comp(SemCompId),
    /// Reassemble a value from the results already on the stacks.
    FinishValue(ValueFinish),
    /// Reassemble a computation from the results already on the stacks.
    FinishComp(CompFinish),
}

/// Reads a semantic **value** back into syntax.
///
/// # Contract
/// - requires: `id` was minted by this normalizer's arena.
/// - ensures: the result is a term whose evaluation is convertible to `id`; in
///   [`QuoteMode::Retained`] an unreduced value hands back the very term it was
///   evaluated from, and in the other two modes every binder is rendered from a
///   de Bruijn level, so the result is in canonical binder form.
/// - provides: the syntax half of normalization — the normal form a caller
///   asked for.
/// - fails: the arena's error when an id fails to resolve or the id space is
///   exhausted.
/// - panics: none.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Adequacy
/// - hypothesis: L2 against evaluation — reading back and re-evaluating must
///   land on a convertible value for every generated term — plus L3 for the
///   three modes, separated pointwise by one value that has a term face, one
///   that does not, and one whose head is a definition.
/// - witness: `nbe::tests::retained_readback_hands_back_the_source_term`
/// - witness: `nbe::tests::canonical_readback_renames_binders_to_levels`
/// - witness: `nbe::tests::unfolding_readback_spends_the_definition`
///
/// # Termination
/// - reason: the walk drains an explicit task stack; going under a binder
///   drives one weak-head step and queues its result rather than recursing.
/// - measure: pending tasks on the stack.
/// - boundedness: each task queues tasks only for the node's own children, and
///   unfolding is bounded by the normalizer's fuel.
/// - input recursion: none.
#[inline]
pub fn quote_value(
    nbe: &mut Normalizer,
    id: SemValueId,
    mode: QuoteMode,
) -> Result<Rc<Value>, SemError>
{
    let mut work = alloc::vec![Task::Value(id)];
    let mut values: Vec<Rc<Value>> = Vec::new();
    let mut comps: Vec<Rc<Comp>> = Vec::new();
    run(nbe, mode, &mut work, &mut values, &mut comps)?;
    Ok(pop_value(&mut values))
}

/// Reads a semantic **computation** back into syntax.
///
/// # Contract
/// - requires: `id` was minted by this normalizer's arena.
/// - ensures: the result is a computation whose evaluation is convertible to
///   `id`, with binders in canonical form in the rebuilding modes.
/// - fails: the arena's error when an id fails to resolve.
/// - panics: none.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
#[inline]
pub fn quote_comp(
    nbe: &mut Normalizer,
    id: SemCompId,
    mode: QuoteMode,
) -> Result<Rc<Comp>, SemError>
{
    let mut work = alloc::vec![Task::Comp(id)];
    let mut values: Vec<Rc<Value>> = Vec::new();
    let mut comps: Vec<Rc<Comp>> = Vec::new();
    run(nbe, mode, &mut work, &mut values, &mut comps)?;
    Ok(pop_comp(&mut comps))
}

/// Drains the readback worklist.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Termination
/// - reason: the driver is a loop over an explicit task stack, not recursion.
/// - measure: pending tasks on the stack.
/// - boundedness: every task pushes tasks only for its own node's children.
/// - input recursion: none.
fn run(
    nbe: &mut Normalizer,
    mode: QuoteMode,
    work: &mut Vec<Task>,
    values: &mut Vec<Rc<Value>>,
    comps: &mut Vec<Rc<Comp>>,
) -> Result<(), SemError>
{
    while let Some(task) = work.pop() {
        match task {
            | Task::Value(id) => quote_value_task(nbe, id, mode, work, values)?,
            | Task::Comp(id) => quote_comp_task(nbe, id, mode, work)?,
            | Task::FinishValue(finish) => finish_value(finish, values, comps),
            | Task::FinishComp(finish) => finish_comp(finish, values, comps),
        }
    }
    Ok(())
}

/// Queues the readback of one value node.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn quote_value_task(
    nbe: &mut Normalizer,
    id: SemValueId,
    mode: QuoteMode,
    work: &mut Vec<Task>,
    values: &mut Vec<Rc<Value>>,
) -> Result<(), SemError>
{
    if matches!(mode, QuoteMode::Retained)
        && let Some(term) = nbe.arena().value(id)?.face().retained()
    {
        values.push(Rc::clone(term));
        return Ok(());
    }
    let id = match mode {
        | QuoteMode::Unfolding => force_value(nbe, id, ForceMode::Unfold)?,
        | QuoteMode::Retained | QuoteMode::Canonical => id,
    };
    let node = nbe.arena().value(id)?.node().clone();
    match node {
        | SemValueNode::Unit => values.push(Rc::new(Value::Unit)),
        | SemValueNode::Int(literal) => values.push(Rc::new(Value::Int(literal))),
        | SemValueNode::Str(literal) => values.push(Rc::new(Value::Str(literal))),
        | SemValueNode::Num(literal) => values.push(Rc::new(Value::Num(literal))),
        | SemValueNode::Reified(stack) => values.push(Rc::new(Value::Stk(stack))),
        | SemValueNode::Rigid(head, _) => {
            let term = match head {
                | Rigid::Level(level) => Value::Var(level_name(level)),
                | Rigid::Free(name) => Value::Var(name),
                | Rigid::Hole(hole) => Value::Hole(hole),
            };
            values.push(Rc::new(term));
        },
        | SemValueNode::Pair(fst, snd) => {
            work.push(Task::FinishValue(ValueFinish::Pair));
            work.push(Task::Value(snd));
            work.push(Task::Value(fst));
        },
        | SemValueNode::Inj(side, payload) => {
            work.push(Task::FinishValue(ValueFinish::Inj(side)));
            work.push(Task::Value(payload));
        },
        | SemValueNode::Here(witness) => {
            work.push(Task::FinishValue(ValueFinish::Here));
            work.push(Task::Value(witness));
        },
        | SemValueNode::Ctor { id, tag, payload } => {
            work.push(Task::FinishValue(ValueFinish::Ctor(id, usize::from(tag))));
            work.push(Task::Value(payload));
        },
        | SemValueNode::List(elements) => {
            work.push(Task::FinishValue(ValueFinish::List(elements.len())));
            for element in elements.iter().rev() {
                work.push(Task::Value(*element));
            }
        },
        | SemValueNode::Record(fields) => {
            let labels = fields.keys().cloned().collect::<Vec<_>>();
            work.push(Task::FinishValue(ValueFinish::Record(labels)));
            for field in fields.values().rev() {
                work.push(Task::Value(*field));
            }
        },
        | SemValueNode::Thunk(grade, cell) => {
            let (_, whnf) = open(nbe, cell, mode)?;
            work.push(Task::FinishValue(ValueFinish::Thunk(grade)));
            work.push(Task::Comp(whnf));
        },
    }
    Ok(())
}

/// Queues the readback of one computation node.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn quote_comp_task(
    nbe: &mut Normalizer,
    id: SemCompId,
    mode: QuoteMode,
    work: &mut Vec<Task>,
) -> Result<(), SemError>
{
    let node = nbe.arena().comp(id)?.node().clone();
    match node {
        | SemCompNode::Lambda(cell) => {
            let (names, whnf) = open(nbe, cell, mode)?;
            let binder = names.first().cloned().unwrap_or_default();
            work.push(Task::FinishComp(CompFinish::Abs(binder)));
            work.push(Task::Comp(whnf));
        },
        | SemCompNode::Return(carried) => {
            work.push(Task::FinishComp(CompFinish::Ret));
            work.push(Task::Value(carried));
        },
        | SemCompNode::LazyPair(fst, snd) => {
            let (_, fst) = open(nbe, fst, mode)?;
            let (_, snd) = open(nbe, snd, mode)?;
            work.push(Task::FinishComp(CompFinish::With));
            work.push(Task::Comp(snd));
            work.push(Task::Comp(fst));
        },
        | SemCompNode::Neutral(stuck) => {
            let (head, spine) = {
                let neutral = nbe.arena().neutral(stuck)?;
                (neutral.head().clone(), neutral.spine().to_vec())
            };
            // The spine is applied outermost last, so the finish frames are
            // queued in reverse: each pops the computation beneath it.
            for elim in spine.iter().rev() {
                match *elim {
                    | Elim::Apply(arg) => {
                        work.push(Task::FinishComp(CompFinish::Apply));
                        work.push(Task::Value(arg));
                    },
                    | Elim::Project(side) => {
                        work.push(Task::FinishComp(CompFinish::Prj(side)));
                    },
                    | Elim::Sequence(cell) => {
                        let (names, whnf) = open(nbe, cell, mode)?;
                        let binder = names.first().cloned().unwrap_or_default();
                        work.push(Task::FinishComp(CompFinish::Sequence(binder)));
                        work.push(Task::Comp(whnf));
                    },
                }
            }
            queue_head(nbe, head, mode, work)?;
        },
    }
    Ok(())
}

/// Queues the readback of one neutral head.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn queue_head(
    nbe: &mut Normalizer,
    head: NeutralHead,
    mode: QuoteMode,
    work: &mut Vec<Task>,
) -> Result<(), SemError>
{
    match head {
        | NeutralHead::Force(carried) => {
            work.push(Task::FinishComp(CompFinish::Force));
            work.push(Task::Value(carried));
        },
        | NeutralHead::Case {
            scrutinee,
            on_left,
            on_right,
        } => {
            let (left_names, left) = open(nbe, on_left, mode)?;
            let (right_names, right) = open(nbe, on_right, mode)?;
            let finish = CompFinish::Case(
                left_names.first().cloned().unwrap_or_default(),
                right_names.first().cloned().unwrap_or_default(),
            );
            work.push(Task::FinishComp(finish));
            work.push(Task::Comp(right));
            work.push(Task::Comp(left));
            work.push(Task::Value(scrutinee));
        },
        | NeutralHead::DataCase { scrutinee, arms } => {
            let mut names = Vec::with_capacity(arms.len());
            let mut bodies = Vec::with_capacity(arms.len());
            for (label, cell) in arms {
                let (binders, body) = open(nbe, cell, mode)?;
                names.push((label, binders.first().cloned().unwrap_or_default()));
                bodies.push(body);
            }
            work.push(Task::FinishComp(CompFinish::DataCase(names)));
            for body in bodies.iter().rev() {
                work.push(Task::Comp(*body));
            }
            work.push(Task::Value(scrutinee));
        },
        | NeutralHead::ListCase {
            scrutinee,
            nil,
            cons,
        } => {
            let (_, nil) = open(nbe, nil, mode)?;
            let (cons_names, cons) = open(nbe, cons, mode)?;
            let mut cons_names = cons_names.into_iter();
            let finish = CompFinish::ListCase(
                cons_names.next().unwrap_or_default(),
                cons_names.next().unwrap_or_default(),
            );
            work.push(Task::FinishComp(finish));
            work.push(Task::Comp(cons));
            work.push(Task::Comp(nil));
            work.push(Task::Value(scrutinee));
        },
        | NeutralHead::Split { scrutinee, body } => {
            let (names, body) = open(nbe, body, mode)?;
            let mut names = names.into_iter();
            let finish = CompFinish::Split(
                names.next().unwrap_or_default(),
                names.next().unwrap_or_default(),
            );
            work.push(Task::FinishComp(finish));
            work.push(Task::Comp(body));
            work.push(Task::Value(scrutinee));
        },
        | NeutralHead::Project { record, label } => {
            work.push(Task::FinishComp(CompFinish::Project(label)));
            work.push(Task::Value(record));
        },
        | NeutralHead::Walk { scrutinee, base } => {
            let (names, body) = open(nbe, base, mode)?;
            let finish = CompFinish::Walk(names.first().cloned().unwrap_or_default());
            work.push(Task::FinishComp(finish));
            work.push(Task::Comp(body));
            work.push(Task::Value(scrutinee));
        },
        | NeutralHead::Native { prim, args } => {
            work.push(Task::FinishComp(CompFinish::Native(prim, args.len())));
            for arg in args.iter().rev() {
                work.push(Task::Value(*arg));
            }
        },
        | NeutralHead::Dup(carried) => {
            work.push(Task::FinishComp(CompFinish::Dup));
            work.push(Task::Value(carried));
        },
        | NeutralHead::Drop(carried) => {
            work.push(Task::FinishComp(CompFinish::Drop));
            work.push(Task::Value(carried));
        },
        | NeutralHead::Perform { sig, op, payload } => {
            work.push(Task::FinishComp(CompFinish::Perform(sig, op)));
            work.push(Task::Value(payload));
        },
        | NeutralHead::Handle {
            sig,
            scrutinee,
            ret,
            ops,
        } => {
            let (_, scrutinee) = open(nbe, scrutinee, mode)?;
            let (ret_names, ret) = open(nbe, ret, mode)?;
            let mut clauses = Vec::with_capacity(ops.len());
            let mut bodies = Vec::with_capacity(ops.len());
            for (label, cell) in ops {
                let (binders, body) = open(nbe, cell, mode)?;
                let mut binders = binders.into_iter();
                clauses.push((
                    label,
                    binders.next().unwrap_or_default(),
                    binders.next().unwrap_or_default(),
                ));
                bodies.push(body);
            }
            let finish =
                CompFinish::Handle(sig, ret_names.first().cloned().unwrap_or_default(), clauses);
            work.push(Task::FinishComp(finish));
            for body in bodies.iter().rev() {
                work.push(Task::Comp(*body));
            }
            work.push(Task::Comp(ret));
            work.push(Task::Comp(scrutinee));
        },
        | NeutralHead::Resume {
            value: carried,
            body,
        } => {
            let (_, body) = open(nbe, body, mode)?;
            work.push(Task::FinishComp(CompFinish::Resume));
            work.push(Task::Comp(body));
            work.push(Task::Value(carried));
        },
        | NeutralHead::Reset(body) => {
            let (_, body) = open(nbe, body, mode)?;
            work.push(Task::FinishComp(CompFinish::Reset));
            work.push(Task::Comp(body));
        },
        | NeutralHead::Shift(body) => {
            let (names, body) = open(nbe, body, mode)?;
            let finish = CompFinish::Shift(names.first().cloned().unwrap_or_default());
            work.push(Task::FinishComp(finish));
            work.push(Task::Comp(body));
        },
        | NeutralHead::Hole(hole) => {
            // A computation hole has no children, so it reassembles with
            // nothing queued beneath it.
            work.push(Task::FinishComp(CompFinish::Hole(hole)));
        },
        | NeutralHead::Mismatch(inner) => {
            work.push(Task::Comp(inner));
        },
    }
    Ok(())
}

/// Reassembles one value from the results already on the stacks.
fn finish_value(
    finish: ValueFinish,
    values: &mut Vec<Rc<Value>>,
    comps: &mut Vec<Rc<Comp>>,
)
{
    let term = match finish {
        | ValueFinish::Pair => {
            let snd = pop_value(values);
            let fst = pop_value(values);
            Value::Pair(fst, snd)
        },
        | ValueFinish::Inj(side) => Value::Inj(side, pop_value(values)),
        | ValueFinish::Here => Value::Here(pop_value(values)),
        | ValueFinish::Ctor(id, tag) => Value::Ctor {
            id,
            tag,
            payload: pop_value(values),
        },
        | ValueFinish::List(count) => {
            let mut elements = Vec::with_capacity(count);
            for _ in 0 .. count {
                elements.push(pop_value(values));
            }
            elements.reverse();
            Value::List(elements)
        },
        | ValueFinish::Record(labels) => {
            let mut fields = Vec::with_capacity(labels.len());
            for _ in 0 .. labels.len() {
                fields.push(pop_value(values));
            }
            fields.reverse();
            Value::Record(labels.into_iter().zip(fields).collect::<BTreeMap<_, _>>())
        },
        | ValueFinish::Thunk(grade) => Value::Thunk(grade, pop_comp(comps)),
    };
    values.push(Rc::new(term));
}

/// Reassembles one computation from the results already on the stacks.
fn finish_comp(
    finish: CompFinish,
    values: &mut Vec<Rc<Value>>,
    comps: &mut Vec<Rc<Comp>>,
)
{
    let term = match finish {
        | CompFinish::Abs(binder) => Comp::Abs(binder, None, pop_comp(comps)),
        | CompFinish::Ret => Comp::Ret(pop_value(values)),
        | CompFinish::With => {
            let snd = pop_comp(comps);
            let fst = pop_comp(comps);
            Comp::With(fst, snd)
        },
        | CompFinish::Force => Comp::Force(pop_value(values)),
        | CompFinish::Case(left, right) => {
            let right_body = pop_comp(comps);
            let left_body = pop_comp(comps);
            Comp::Case(pop_value(values), (left, left_body), (right, right_body))
        },
        | CompFinish::DataCase(names) => {
            let mut bodies = Vec::with_capacity(names.len());
            for _ in 0 .. names.len() {
                bodies.push(pop_comp(comps));
            }
            bodies.reverse();
            let scrutinee = pop_value(values);
            let arms = names
                .into_iter()
                .map(|(_, binder)| binder)
                .zip(bodies)
                .collect::<Vec<_>>();
            Comp::DataCase(scrutinee, arms)
        },
        | CompFinish::ListCase(head, tail) => {
            let cons = pop_comp(comps);
            let nil = pop_comp(comps);
            Comp::ListCase {
                scrut: pop_value(values),
                nil,
                head,
                tail,
                cons,
            }
        },
        | CompFinish::Split(fst_name, snd_name) => {
            let body = pop_comp(comps);
            Comp::Split {
                scrut: pop_value(values),
                fst_name,
                snd_name,
                motive: None,
                body,
            }
        },
        | CompFinish::Project(label) => Comp::RecordProj {
            record: pop_value(values),
            label,
        },
        | CompFinish::Walk(binder) => {
            let body = pop_comp(comps);
            let scrut = pop_value(values);
            Comp::Walk {
                scrut,
                motive: alloc::boxed::Box::new(crate::syntax::WalkMotive::new(
                    "\u{ab}x\u{bb}",
                    "\u{ab}y\u{bb}",
                    "\u{ab}q\u{bb}",
                    crate::types::CompType::Unknown,
                )),
                base: WalkBase { x: binder, body },
            }
        },
        | CompFinish::Native(prim, count) => {
            let mut args = Vec::with_capacity(count);
            for _ in 0 .. count {
                args.push(pop_value(values));
            }
            args.reverse();
            Comp::Native { prim, args }
        },
        | CompFinish::Dup => Comp::Dup(pop_value(values)),
        | CompFinish::Drop => Comp::Drop(pop_value(values)),
        | CompFinish::Perform(sig, op) => Comp::Perform(
            alloc::boxed::Box::new(sig.as_ref().clone()),
            op,
            pop_value(values),
        ),
        | CompFinish::Handle(sig, ret_binder, clauses) => {
            let mut bodies = Vec::with_capacity(clauses.len());
            for _ in 0 .. clauses.len() {
                bodies.push(pop_comp(comps));
            }
            bodies.reverse();
            let ret_body = pop_comp(comps);
            let scrutinee = pop_comp(comps);
            let ops = clauses
                .into_iter()
                .zip(bodies)
                .map(|((op, payload, resume), body)| OpClause {
                    op,
                    payload,
                    resume,
                    body,
                })
                .collect::<Vec<_>>();
            Comp::Handle {
                sig: alloc::boxed::Box::new(sig.as_ref().clone()),
                scrutinee,
                ret: (ret_binder, ret_body),
                ops,
            }
        },
        | CompFinish::Resume => {
            let body = pop_comp(comps);
            Comp::Resume(pop_value(values), body)
        },
        | CompFinish::Reset => Comp::Reset(pop_comp(comps)),
        | CompFinish::Shift(binder) => Comp::Shift(binder, pop_comp(comps)),
        | CompFinish::Hole(hole) => Comp::Hole(hole),
        | CompFinish::Apply => {
            let head = pop_comp(comps);
            Comp::App(head, pop_value(values))
        },
        | CompFinish::Prj(side) => Comp::Prj(side, pop_comp(comps)),
        | CompFinish::Sequence(binder) => {
            let cont = pop_comp(comps);
            Comp::Bind(pop_comp(comps), binder, cont)
        },
    };
    comps.push(Rc::new(term));
}

/// Pops one finished value off the readback stack.
///
/// The fallback is unreachable under the post-order balance invariant every
/// finish frame maintains; it degenerates to the unit value rather than a
/// panic, because indexing and unwrapping are banned in shipping code.
fn pop_value(values: &mut Vec<Rc<Value>>) -> Rc<Value>
{
    debug_assert!(
        !values.is_empty(),
        "readback worklist underflow (post-order balance)"
    );
    values.pop().unwrap_or_else(|| Rc::new(Value::Unit))
}

/// Pops one finished computation off the readback stack (see [`pop_value`]).
fn pop_comp(comps: &mut Vec<Rc<Comp>>) -> Rc<Comp>
{
    debug_assert!(
        !comps.is_empty(),
        "readback worklist underflow (post-order balance)"
    );
    comps
        .pop()
        .unwrap_or_else(|| Rc::new(Comp::Ret(Rc::new(Value::Unit))))
}
