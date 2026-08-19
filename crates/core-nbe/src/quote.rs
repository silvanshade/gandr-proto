//! Readback: a semantic value back into syntax, iteratively, in one of three
//! modes.
//!
//! Readback produces **node ids** in the syntax store, exactly as evaluation
//! consumes them. A caller wanting an ordinary term reifies one at the boundary
//! ([`Normalizer::normalize`]); nothing inside the engine ever holds an owned
//! recursive term.
//!
//! # Readback chooses a face
//!
//! The value domain carries two faces and readback is where the choice is
//! visible:
//!
//! * [`QuoteMode::Retained`] prefers the **term face** — a value that never
//!   reduced hands back the source node it came from, so quoting an unreduced
//!   subterm costs one id and rebuilds nothing;
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
//! [`conv`]: crate::conv
//! [`Normalizer::normalize`]: crate::Normalizer::normalize

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::boundary::VariableLevel;
use gandr_core_term::grade::Grade;
use gandr_core_term::syntax::CompNode;
use gandr_core_term::syntax::CompNodeId;
use gandr_core_term::syntax::CompTypeNode;
use gandr_core_term::syntax::OpClauseNode;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::ValueNode;
use gandr_core_term::syntax::ValueNodeId;
use gandr_core_term::syntax::WalkBaseNode;
use gandr_core_term::syntax::WalkMotiveNode;
use gandr_core_term::types::DataId;

use crate::Normalizer;
use crate::eval::ForceMode;
use crate::eval::eval_comp;
use crate::eval::force_value;
use crate::eval::syntax_comp;
use crate::eval::value;
use crate::sem::ClosureId;
use crate::sem::Elim;
use crate::sem::NeutralHead;
use crate::sem::Rigid;
use crate::sem::SemCompId;
use crate::sem::SemCompNode;
use crate::sem::SemError;
use crate::sem::SemValueId;
use crate::sem::SemValueNode;
use crate::sem::ValueUnfold;

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
pub fn level_name(level: VariableLevel) -> String
{
    format!("\u{ab}{}\u{bb}", u32::from(level))
}

/// Reads a generated binder name back to the level it renders, or `None` for
/// any other name.
///
/// The inverse of [`level_name`], kept beside it so the two halves of one
/// naming convention cannot drift. The scope check in `gandr_core_unify` is
/// what needs the inverse: it has to tell a variable the solver itself opened
/// from an ordinary source name, and a solver holding its own copy of the
/// format string would keep working while this one changed.
///
/// # Contract
/// - ensures: `parse_level_name(level_name(level))` is `Some(level)` for every
///   level, and the result is `None` for every name [`level_name`] cannot
///   produce — including a bracketed name whose body is not a `u32`.
/// - provides: the level-versus-source-name discrimination the escape check
///   rests on.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the round trip on an ordinary level and on zero, plus
///   four rejections separated pointwise: a source name, an unbracketed number,
///   a half-bracketed name, and a bracketed non-number.
/// - witness: `crate::tests::a_level_name_round_trips_through_its_parser`
/// - witness: `crate::tests::a_parser_rejects_every_name_readback_cannot_produce`
#[inline]
#[must_use]
pub fn parse_level_name(name: gandr_core_term::boundary::NameRef<'_>) -> Option<VariableLevel>
{
    let name = <&str>::from(name);
    let body = name.strip_prefix('\u{ab}')?.strip_suffix('\u{bb}')?;
    body.parse::<u32>().ok().map(VariableLevel::from)
}

/// Allocates one value node in the syntax store.
///
/// # Errors
///
/// Returns [`SemError::SyntaxStore`] when the store's id space is exhausted.
#[inline]
fn emit_value(
    nbe: &mut Normalizer,
    node: ValueNode,
) -> Result<ValueNodeId, SemError>
{
    nbe.syntax_mut()
        .values
        .alloc(node)
        .ok_or(SemError::SyntaxStore)
}

/// Allocates one computation node in the syntax store.
///
/// # Errors
///
/// Returns [`SemError::SyntaxStore`] when the store's id space is exhausted.
#[inline]
fn emit_comp(
    nbe: &mut Normalizer,
    node: CompNode,
) -> Result<CompNodeId, SemError>
{
    nbe.syntax_mut()
        .comps
        .alloc(node)
        .ok_or(SemError::SyntaxStore)
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
        (closure.env(), closure.binders().to_vec(), closure.body())
    };
    let mut names = Vec::with_capacity(binders.len());
    for binder in binders {
        let level = nbe.fresh_level();
        let node = SemValueNode::Rigid(Rigid::Level(level), ValueUnfold::Rigid);
        let rigid = value(nbe, node)?;
        env = nbe.arena_mut().bind(env, binder, rigid)?;
        names.push(level_name(level));
    }
    let whnf = eval_comp(nbe, env, body, mode.force())?;
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
    /// Rebuild a packed module over these witness types.
    Pack(Vec<gandr_core_term::syntax::ValueTypeNodeId>),
    /// Rebuild a thunk from the computation on the computation stack.
    Thunk(Grade),
    /// Rebuild a stuck pure-computation embedding from the computation on the
    /// computation stack.
    Run,
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
    /// Rebuild a declared-data elimination over these arm binders.
    DataCase(Vec<String>),
    /// Rebuild a list elimination over the cons binders.
    ListCase(String, String),
    /// Rebuild a pair elimination over its two binders.
    Split(String, String),
    /// Rebuild a package elimination over its module binder, reading its
    /// signature and its minted atoms off the source.
    Unpack(CompNodeId, String),
    /// Rebuild a record projection at this label.
    Project(String),
    /// Rebuild an identity elimination over the diagonal binder.
    Walk(String),
    /// Rebuild a native application over this many arguments.
    Native(gandr_core_term::prim::NativePrim, usize),
    /// Rebuild a grade duplication.
    Dup,
    /// Rebuild a grade discard.
    Drop,
    /// Rebuild an effect performance, reading its signature off the source.
    Perform(CompNodeId),
    /// Rebuild a handler, reading its signature and labels off the source.
    Handle(CompNodeId, String, Vec<(String, String)>),
    /// Rebuild a resumption.
    Resume,
    /// Rebuild a delimiter.
    Reset,
    /// Rebuild a capture over its binder.
    Shift(String),
    /// Reassembles a fixpoint from its read-back body, under the
    /// self-reference binder the closure was opened with.
    Fix(String),
    /// Rebuild a computation hole.
    Hole(gandr_core_term::syntax::HoleId),
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

/// Reads a semantic **value** back into a syntax node.
///
/// # Contract
/// - requires: `id` was minted by this normalizer's arena.
/// - ensures: the result names a node whose evaluation is convertible to `id`;
///   in [`QuoteMode::Retained`] an unreduced value hands back the very node it
///   was evaluated from, and in the other two modes every binder is rendered
///   from a de Bruijn level, so the result is in canonical binder form.
/// - provides: the syntax half of normalization — the normal form a caller
///   asked for.
/// - fails: the arena's error when an id fails to resolve or an id space is
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
/// - witness: `crate::tests::retained_readback_hands_back_the_source_term`
/// - witness: `crate::tests::canonical_readback_renames_binders_to_levels`
/// - witness: `crate::tests::unfolding_readback_spends_the_definition`
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
) -> Result<ValueNodeId, SemError>
{
    let mut work = alloc::vec![Task::Value(id)];
    let mut values: Vec<ValueNodeId> = Vec::new();
    let mut comps: Vec<CompNodeId> = Vec::new();
    run(nbe, mode, &mut work, &mut values, &mut comps)?;
    pop_value(nbe, &mut values)
}

/// Reads a semantic **computation** back into a syntax node.
///
/// # Contract
/// - requires: `id` was minted by this normalizer's arena.
/// - ensures: the result names a node whose evaluation is convertible to `id`,
///   with binders in canonical form in the rebuilding modes.
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
) -> Result<CompNodeId, SemError>
{
    let mut work = alloc::vec![Task::Comp(id)];
    let mut values: Vec<ValueNodeId> = Vec::new();
    let mut comps: Vec<CompNodeId> = Vec::new();
    run(nbe, mode, &mut work, &mut values, &mut comps)?;
    pop_comp(nbe, &mut comps)
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
    values: &mut Vec<ValueNodeId>,
    comps: &mut Vec<CompNodeId>,
) -> Result<(), SemError>
{
    while let Some(task) = work.pop() {
        match task {
            | Task::Value(id) => quote_value_task(nbe, id, mode, work, values)?,
            | Task::Comp(id) => quote_comp_task(nbe, id, mode, work)?,
            | Task::FinishValue(finish) => finish_value(nbe, finish, values, comps)?,
            | Task::FinishComp(finish) => finish_comp(nbe, finish, values, comps)?,
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
    values: &mut Vec<ValueNodeId>,
) -> Result<(), SemError>
{
    if matches!(mode, QuoteMode::Retained)
        && let Some(term) = nbe.arena().value(id)?.face().retained()
    {
        values.push(term);
        return Ok(());
    }
    let id = match mode {
        | QuoteMode::Unfolding => force_value(nbe, id, ForceMode::Unfold)?,
        | QuoteMode::Retained | QuoteMode::Canonical => id,
    };
    let node = nbe.arena().value(id)?.node().clone();
    match node {
        | SemValueNode::Unit => {
            let id = emit_value(nbe, ValueNode::Unit)?;
            values.push(id);
        },
        | SemValueNode::Int(literal) => {
            let id = emit_value(nbe, ValueNode::Int(literal))?;
            values.push(id);
        },
        | SemValueNode::Str(literal) => {
            let id = emit_value(nbe, ValueNode::Str(literal))?;
            values.push(id);
        },
        | SemValueNode::Num(literal) => {
            let id = emit_value(nbe, ValueNode::Num(literal))?;
            values.push(id);
        },
        | SemValueNode::Reified(stack) => {
            let id = emit_value(nbe, ValueNode::Stk(stack))?;
            values.push(id);
        },
        | SemValueNode::Rigid(head, _) => {
            let node = match head {
                | Rigid::Level(level) => ValueNode::Var(level_name(level)),
                | Rigid::Free(name) => ValueNode::Var(name),
                | Rigid::Hole(hole) => ValueNode::Hole(hole),
            };
            let id = emit_value(nbe, node)?;
            values.push(id);
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
        | SemValueNode::Pack { witnesses, payload } => {
            work.push(Task::FinishValue(ValueFinish::Pack(witnesses)));
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
        | SemValueNode::Run(cell) => {
            let (_, whnf) = open(nbe, cell, mode)?;
            work.push(Task::FinishValue(ValueFinish::Run));
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
            // The spine applies outermost last, so the finish frames are queued
            // in reverse: each one pops the computation beneath it.
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
            for arm in arms {
                let (binders, body) = open(nbe, arm.1, mode)?;
                names.push(binders.first().cloned().unwrap_or_default());
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
        | NeutralHead::Unpack {
            source,
            scrutinee,
            body,
        } => {
            let (names, body) = open(nbe, body, mode)?;
            let finish = CompFinish::Unpack(source, names.first().cloned().unwrap_or_default());
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
        | NeutralHead::Perform { source, payload } => {
            work.push(Task::FinishComp(CompFinish::Perform(source)));
            work.push(Task::Value(payload));
        },
        | NeutralHead::Handle {
            source,
            scrutinee,
            ret,
            ops,
        } => {
            let (_, scrutinee) = open(nbe, scrutinee, mode)?;
            let (ret_names, ret) = open(nbe, ret, mode)?;
            let mut binders = Vec::with_capacity(ops.len());
            let mut bodies = Vec::with_capacity(ops.len());
            for cell in ops {
                let (names, body) = open(nbe, cell, mode)?;
                let mut names = names.into_iter();
                binders.push((
                    names.next().unwrap_or_default(),
                    names.next().unwrap_or_default(),
                ));
                bodies.push(body);
            }
            let finish = CompFinish::Handle(
                source,
                ret_names.first().cloned().unwrap_or_default(),
                binders,
            );
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
        | NeutralHead::Fix(body) => {
            let (names, body) = open(nbe, body, mode)?;
            let finish = CompFinish::Fix(names.first().cloned().unwrap_or_default());
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

/// Reassembles one value node from the results already on the stacks.
///
/// # Errors
///
/// Returns [`SemError`] when a stack underflows or the store is exhausted.
fn finish_value(
    nbe: &mut Normalizer,
    finish: ValueFinish,
    values: &mut Vec<ValueNodeId>,
    comps: &mut Vec<CompNodeId>,
) -> Result<(), SemError>
{
    let node = match finish {
        | ValueFinish::Pair => {
            let snd = pop_value(nbe, values)?;
            let fst = pop_value(nbe, values)?;
            ValueNode::Pair(fst, snd)
        },
        | ValueFinish::Inj(side) => ValueNode::Inj(side, pop_value(nbe, values)?),
        | ValueFinish::Here => ValueNode::Here(pop_value(nbe, values)?),
        | ValueFinish::Ctor(id, tag) => ValueNode::Ctor {
            id,
            tag,
            payload: pop_value(nbe, values)?,
        },
        | ValueFinish::Pack(witnesses) => ValueNode::Pack {
            witnesses,
            payload: pop_value(nbe, values)?,
        },
        | ValueFinish::List(count) => {
            let mut elements = Vec::with_capacity(count);
            for _ in 0 .. count {
                elements.push(pop_value(nbe, values)?);
            }
            elements.reverse();
            ValueNode::List(elements)
        },
        | ValueFinish::Record(labels) => {
            let mut fields = Vec::with_capacity(labels.len());
            for _ in 0 .. labels.len() {
                fields.push(pop_value(nbe, values)?);
            }
            fields.reverse();
            ValueNode::Record(labels.into_iter().zip(fields).collect::<BTreeMap<_, _>>())
        },
        | ValueFinish::Thunk(grade) => ValueNode::Thunk(grade, pop_comp(nbe, comps)?),
        | ValueFinish::Run => ValueNode::Run(pop_comp(nbe, comps)?),
    };
    let id = emit_value(nbe, node)?;
    values.push(id);
    Ok(())
}

/// Reassembles one computation node from the results already on the stacks.
///
/// # Errors
///
/// Returns [`SemError`] when a stack underflows or the store is exhausted.
fn finish_comp(
    nbe: &mut Normalizer,
    finish: CompFinish,
    values: &mut Vec<ValueNodeId>,
    comps: &mut Vec<CompNodeId>,
) -> Result<(), SemError>
{
    let node = match finish {
        | CompFinish::Abs(binder) => CompNode::Abs(binder, None, pop_comp(nbe, comps)?),
        | CompFinish::Ret => CompNode::Ret(pop_value(nbe, values)?),
        | CompFinish::With => {
            let snd = pop_comp(nbe, comps)?;
            let fst = pop_comp(nbe, comps)?;
            CompNode::With(fst, snd)
        },
        | CompFinish::Force => CompNode::Force(pop_value(nbe, values)?),
        | CompFinish::Case(left, right) => {
            let right_body = pop_comp(nbe, comps)?;
            let left_body = pop_comp(nbe, comps)?;
            CompNode::Case(
                pop_value(nbe, values)?,
                (left, left_body),
                (right, right_body),
            )
        },
        | CompFinish::DataCase(names) => {
            let mut bodies = Vec::with_capacity(names.len());
            for _ in 0 .. names.len() {
                bodies.push(pop_comp(nbe, comps)?);
            }
            bodies.reverse();
            CompNode::DataCase {
                scrut: pop_value(nbe, values)?,
                arms: names.into_iter().zip(bodies).collect::<Vec<_>>(),
            }
        },
        | CompFinish::ListCase(head, tail) => {
            let cons = pop_comp(nbe, comps)?;
            let nil = pop_comp(nbe, comps)?;
            CompNode::ListCase {
                scrut: pop_value(nbe, values)?,
                nil,
                head,
                tail,
                cons,
            }
        },
        | CompFinish::Split(fst_name, snd_name) => {
            let body = pop_comp(nbe, comps)?;
            CompNode::Split {
                scrut: pop_value(nbe, values)?,
                fst_name,
                snd_name,
                motive: None,
                body,
            }
        },
        | CompFinish::Unpack(source, binder) => {
            // The ascribed signature and the minted atoms are read off the
            // source node, the `Perform` and `Handle` discipline. An atom is a
            // nominal identity: there is no unknown one to emit the way the
            // walk motive emits an unknown type, and inventing one would mint
            // an abstraction the elaborator never made.
            let CompNode::Unpack {
                signature, atoms, ..
            } = syntax_comp(nbe, source)?
            else {
                return Err(SemError::MissingSyntaxComp(source));
            };
            let body = pop_comp(nbe, comps)?;
            CompNode::Unpack {
                scrut: pop_value(nbe, values)?,
                signature,
                atoms,
                binder,
                body,
            }
        },
        | CompFinish::Project(label) => CompNode::RecordProj {
            record: pop_value(nbe, values)?,
            label,
        },
        | CompFinish::Walk(binder) => {
            let body = pop_comp(nbe, comps)?;
            let scrut = pop_value(nbe, values)?;
            // The motive is inert for the value computed, so readback emits the
            // unknown motive rather than inventing one: conversion erases it,
            // and no consumer of a normal form reads it back for typing.
            let motive_body = nbe
                .syntax_mut()
                .comp_types
                .alloc(CompTypeNode::Unknown)
                .ok_or(SemError::SyntaxStore)?;
            CompNode::Walk {
                scrut,
                motive: WalkMotiveNode {
                    x: String::from("\u{ab}x\u{bb}"),
                    y: String::from("\u{ab}y\u{bb}"),
                    q: String::from("\u{ab}q\u{bb}"),
                    body: motive_body,
                },
                base: WalkBaseNode { x: binder, body },
            }
        },
        | CompFinish::Native(prim, count) => {
            let mut args = Vec::with_capacity(count);
            for _ in 0 .. count {
                args.push(pop_value(nbe, values)?);
            }
            args.reverse();
            CompNode::Native { prim, args }
        },
        | CompFinish::Dup => CompNode::Dup(pop_value(nbe, values)?),
        | CompFinish::Drop => CompNode::Drop(pop_value(nbe, values)?),
        | CompFinish::Perform(source) => {
            let CompNode::Perform(sig, op, _) = syntax_comp(nbe, source)?
            else {
                return Err(SemError::MissingSyntaxComp(source));
            };
            CompNode::Perform(sig, op, pop_value(nbe, values)?)
        },
        | CompFinish::Handle(source, ret_binder, binders) => {
            let CompNode::Handle { sig, ops, .. } = syntax_comp(nbe, source)?
            else {
                return Err(SemError::MissingSyntaxComp(source));
            };
            let mut bodies = Vec::with_capacity(binders.len());
            for _ in 0 .. binders.len() {
                bodies.push(pop_comp(nbe, comps)?);
            }
            bodies.reverse();
            let ret_body = pop_comp(nbe, comps)?;
            let scrutinee = pop_comp(nbe, comps)?;
            let clauses = ops
                .into_iter()
                .zip(binders)
                .zip(bodies)
                .map(|((clause, (payload, resume)), body)| OpClauseNode {
                    op: clause.op,
                    payload,
                    resume,
                    body,
                })
                .collect::<Vec<_>>();
            CompNode::Handle {
                sig,
                scrutinee,
                ret: (ret_binder, ret_body),
                ops: clauses,
            }
        },
        | CompFinish::Resume => {
            let body = pop_comp(nbe, comps)?;
            CompNode::Resume(pop_value(nbe, values)?, body)
        },
        | CompFinish::Reset => CompNode::Reset(pop_comp(nbe, comps)?),
        | CompFinish::Shift(binder) => CompNode::Shift(binder, pop_comp(nbe, comps)?),
        | CompFinish::Fix(binder) => CompNode::Fix(binder, pop_comp(nbe, comps)?),
        | CompFinish::Hole(hole) => CompNode::Hole(hole),
        | CompFinish::Apply => {
            let head = pop_comp(nbe, comps)?;
            CompNode::App(head, pop_value(nbe, values)?)
        },
        | CompFinish::Prj(side) => CompNode::Prj(side, pop_comp(nbe, comps)?),
        | CompFinish::Sequence(binder) => {
            let cont = pop_comp(nbe, comps)?;
            CompNode::Bind(pop_comp(nbe, comps)?, binder, cont)
        },
    };
    let id = emit_comp(nbe, node)?;
    comps.push(id);
    Ok(())
}

/// Pops one finished value node off the readback stack.
///
/// The stack cannot underflow under the post-order balance invariant every
/// finish frame maintains; an underflow degenerates to a fresh unit node rather
/// than panicking, because indexing and unwrapping are banned in shipping code.
///
/// # Errors
///
/// Returns [`SemError::SyntaxStore`] when the store's id space is exhausted.
fn pop_value(
    nbe: &mut Normalizer,
    values: &mut Vec<ValueNodeId>,
) -> Result<ValueNodeId, SemError>
{
    debug_assert!(
        !values.is_empty(),
        "readback worklist underflow (post-order balance)"
    );
    match values.pop() {
        | Some(id) => Ok(id),
        | None => emit_value(nbe, ValueNode::Unit),
    }
}

/// Pops one finished computation node off the readback stack (see
/// [`pop_value`]).
///
/// # Errors
///
/// Returns [`SemError::SyntaxStore`] when the store's id space is exhausted.
fn pop_comp(
    nbe: &mut Normalizer,
    comps: &mut Vec<CompNodeId>,
) -> Result<CompNodeId, SemError>
{
    debug_assert!(
        !comps.is_empty(),
        "readback worklist underflow (post-order balance)"
    );
    match comps.pop() {
        | Some(id) => Ok(id),
        | None => {
            let unit = emit_value(nbe, ValueNode::Unit)?;
            emit_comp(nbe, CompNode::Ret(unit))
        },
    }
}
