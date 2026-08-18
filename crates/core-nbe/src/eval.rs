//! Evaluation and forcing: syntax into the glued semantic domain, and the
//! three force modes that decide how far a weak head is driven.
//!
//! # Syntax arrives as node ids, never as owned terms
//!
//! Every term this module reads is a **node id** into the flat carrier
//! ([`FlatArena`]), and every term it hands to a semantic record is the same.
//! Nothing here clones a reference-counted term into the semantic arena, which
//! is what makes that arena's teardown flat in any drop order rather than only
//! when the caller still owns the syntax.
//!
//! # Weak-head normalization is spine-local
//!
//! A projection or an application drives its **head** to a value form and no
//! further — never the siblings. The glued representation's laziness is what
//! makes that pay: the unfolded face is built only along the spine actually
//! forced, so projecting one label of a structure never evaluates the others.
//!
//! # The machine is two-phase and iterative
//!
//! Computation evaluation is an explicit machine over a frame stack, in two
//! phases: **descend** carries an environment and a syntax node toward the
//! head, pushing the eliminators it passes; **unwind** carries a weak-head
//! normal form back out, consuming those eliminators. A canonical form meeting
//! its own eliminator reduces; a neutral meeting one grows its spine; and a
//! canonical form meeting an eliminator of the wrong polarity — which the
//! checker rejects, but which this engine must still answer for — becomes a
//! mismatch neutral rather than silently dropping the eliminator.
//!
//! Nothing here recurses on term depth. The frame stack is the recursion, and
//! it lives on the heap.
//!
//! # What does not reduce
//!
//! Effects, handlers, resumptions, delimiters, captures, and the grade
//! structural operations evaluate their operands and then **stop**. That is the
//! conversion quarantine, and it is now a property of this module's match arms
//! rather than a property of there being nothing to evaluate.
//!
//! Native primitives stop for a different reason: they are pure, but firing one
//! inside conversion would run a computation, so a native is a rigid head whose
//! arguments are compared by congruence. Two natives with the same primitive
//! and convertible arguments are convertible, which is strictly more than
//! syntactic equality would give.
//!
//! [`FlatArena`]: gandr_core_term::syntax::FlatArena

use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::boundary::ConstructorTag;
use gandr_core_term::boundary::NameRef;
use gandr_core_term::boundary::SemanticHash;
use gandr_core_term::boundary::TermRetention;
use gandr_core_term::boundary::UnfoldPermission;
use gandr_core_term::syntax::CompNode;
use gandr_core_term::syntax::CompNodeId;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::ValueNode;
use gandr_core_term::syntax::ValueNodeId;
use gandr_core_term::syntax::ValueTypeNodeId;

use crate::Normalizer;
use crate::sem::Closure;
use crate::sem::ClosureId;
use crate::sem::CompUnfold;
use crate::sem::Elim;
use crate::sem::EnvId;
use crate::sem::Guard;
use crate::sem::Neutral;
use crate::sem::NeutralHead;
use crate::sem::Rigid;
use crate::sem::SemArena;
use crate::sem::SemComp;
use crate::sem::SemCompId;
use crate::sem::SemCompNode;
use crate::sem::SemError;
use crate::sem::SemValue;
use crate::sem::SemValueId;
use crate::sem::SemValueNode;
use crate::sem::ValueUnfold;
use crate::sem::mix_hashable;
use crate::sem::mix_str;
use crate::sem::mix_word;
use crate::sem::seed;

/// How far a force drives its subject.
///
/// The three modes are the whole of the unfolding policy the engine layer
/// exposes, and conversion reads exactly this enum: the pipeline's structural
/// step forces [`Self::WeakHead`], its smart-unfolding step forces
/// [`Self::Speculative`], and its height and speculation steps force
/// [`Self::Unfold`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ForceMode
{
    /// Drive to weak-head normal form using beta alone. A head with an
    /// unfolding rule keeps its neutral face, so the answer is what the term
    /// says before any definition is consulted.
    #[default]
    WeakHead,
    /// Drive to weak-head normal form, unfolding definitional heads until the
    /// head is canonical or genuinely rigid.
    Unfold,
    /// Drive to weak-head normal form, unfolding only **reducible**
    /// definitions. An irreducible definition keeps its neutral face, which is
    /// what the reserved opt-out buys.
    Speculative,
}

impl ForceMode
{
    /// Whether this mode may spend an unfolding on a definition with the given
    /// transparency.
    #[inline]
    #[must_use]
    fn unfolds(
        self,
        transparency: crate::defs::Transparency,
    ) -> UnfoldPermission
    {
        UnfoldPermission::from(match self {
            | Self::WeakHead => false,
            | Self::Unfold => true,
            | Self::Speculative => {
                matches!(transparency, crate::defs::Transparency::Reducible)
            },
        })
    }
}

/// One frame on the evaluation machine's stack.
enum Frame
{
    /// An eliminator waiting for a weak-head normal form to consume it.
    Elim(Elim),
    /// A thunk cell waiting to memoize the weak-head normal form beneath it.
    Memoize(ClosureId),
}

/// The machine's phase.
enum Phase
{
    /// Carrying an environment and a syntax node toward the head.
    Descend(EnvId, CompNodeId),
    /// Carrying a weak-head normal form back out through the frames.
    Unwind(SemCompId),
}

/// The node-kind tags the guard word's hash is seeded with.
mod kind
{
    /// The tag of the unit value.
    pub const UNIT: u64 = 1;
    /// The tag of an integer literal.
    pub const INT: u64 = 2;
    /// The tag of a string literal.
    pub const STR: u64 = 3;
    /// The tag of a typed numeric literal.
    pub const NUM: u64 = 4;
    /// The tag of an eager pair.
    pub const PAIR: u64 = 5;
    /// The tag of a sum injection.
    pub const INJ: u64 = 6;
    /// The tag of a list literal.
    pub const LIST: u64 = 7;
    /// The tag of a record literal.
    pub const RECORD: u64 = 8;
    /// The tag of a thunk.
    pub const THUNK: u64 = 9;
    /// The tag of a reflexivity witness.
    pub const HERE: u64 = 10;
    /// The tag of a constructor value.
    pub const CTOR: u64 = 11;
    /// The tag of a reified stack.
    pub const REIFIED: u64 = 12;
    /// The tag of a rigid head.
    pub const RIGID: u64 = 13;
    /// The tag of a lambda.
    pub const LAMBDA: u64 = 14;
    /// The tag of a returner.
    pub const RETURN: u64 = 15;
    /// The tag of a lazy pair.
    pub const LAZY_PAIR: u64 = 16;
    /// The tag of a neutral computation.
    pub const NEUTRAL: u64 = 17;
    /// The tag of a packed module.
    pub const PACK: u64 = 18;
}

/// Reads one syntax value node out of the store.
///
/// The node is cloned rather than borrowed because minting needs the store's
/// owner mutably; a node's children are ids, so the clone is shallow.
///
/// # Errors
///
/// Returns [`SemError::MissingSyntaxValue`] when the id does not resolve.
#[inline]
pub fn syntax_value(
    nbe: &Normalizer,
    id: ValueNodeId,
) -> Result<ValueNode, SemError>
{
    nbe.syntax()
        .values
        .get(id)
        .cloned()
        .ok_or(SemError::MissingSyntaxValue(id))
}

/// Reads one syntax computation node out of the store (see [`syntax_value`]).
///
/// # Errors
///
/// Returns [`SemError::MissingSyntaxComp`] when the id does not resolve.
#[inline]
pub fn syntax_comp(
    nbe: &Normalizer,
    id: CompNodeId,
) -> Result<CompNode, SemError>
{
    nbe.syntax()
        .comps
        .get(id)
        .cloned()
        .ok_or(SemError::MissingSyntaxComp(id))
}

/// Computes the guard word of a value node from its children's cached guards.
///
/// # Contract
/// - ensures: the guard's hash is a function of the node's content alone; a
///   node containing a closure or a reified stack is **not** rigid, because the
///   guard does not walk the syntax inside one and must never claim a
///   distinctness it cannot see.
/// - fails: a missing-child error when a child id does not resolve.
/// - panics: none.
///
/// # Errors
///
/// Returns the arena's missing-node error when a child does not resolve.
fn value_guard(
    arena: &SemArena,
    node: &SemValueNode,
) -> Result<Guard, SemError>
{
    let guard = match *node {
        | SemValueNode::Unit => Guard::leaf(seed(SemanticHash::from(kind::UNIT))),
        | SemValueNode::Int(literal) => Guard::leaf(mix_word(
            seed(SemanticHash::from(kind::INT)),
            SemanticHash::from(u64::from_le_bytes(literal.to_le_bytes())),
        )),
        | SemValueNode::Str(ref literal) => Guard::leaf(mix_str(
            seed(SemanticHash::from(kind::STR)),
            NameRef::from(literal.as_str()),
        )),
        | SemValueNode::Num(literal) => {
            Guard::leaf(mix_hashable(seed(SemanticHash::from(kind::NUM)), &literal))
        },
        | SemValueNode::Pair(fst, snd) => {
            let base = Guard::leaf(seed(SemanticHash::from(kind::PAIR)));
            let base = base.fold(arena.value(fst)?.guard());
            base.fold(arena.value(snd)?.guard())
        },
        | SemValueNode::Inj(side, payload) => {
            let base = Guard::leaf(mix_hashable(seed(SemanticHash::from(kind::INJ)), &side));
            base.fold(arena.value(payload)?.guard())
        },
        | SemValueNode::List(ref elements) => {
            let mut base = Guard::leaf(mix_word(
                seed(SemanticHash::from(kind::LIST)),
                SemanticHash::from(u64::try_from(elements.len()).unwrap_or(u64::MAX)),
            ));
            for element in elements {
                base = base.fold(arena.value(*element)?.guard());
            }
            base
        },
        | SemValueNode::Record(ref fields) => {
            let mut base = Guard::leaf(mix_word(
                seed(SemanticHash::from(kind::RECORD)),
                SemanticHash::from(u64::try_from(fields.len()).unwrap_or(u64::MAX)),
            ));
            for (label, field) in fields {
                base = Guard::leaf(mix_str(base.hash(), NameRef::from(label.as_str())))
                    .fold(base)
                    .fold(arena.value(*field)?.guard());
            }
            base
        },
        | SemValueNode::Thunk(grade, _) => {
            Guard::leaf(mix_hashable(seed(SemanticHash::from(kind::THUNK)), &grade))
                .with_unfolding()
        },
        | SemValueNode::Here(witness) => {
            let base = Guard::leaf(seed(SemanticHash::from(kind::HERE)));
            base.fold(arena.value(witness)?.guard())
        },
        | SemValueNode::Ctor {
            ref id,
            tag,
            payload,
        } => {
            let base = Guard::leaf(mix_word(
                mix_hashable(seed(SemanticHash::from(kind::CTOR)), id),
                SemanticHash::from(u64::try_from(usize::from(tag)).unwrap_or(u64::MAX)),
            ));
            base.fold(arena.value(payload)?.guard())
        },
        | SemValueNode::Pack {
            ref witnesses,
            payload,
        } => {
            // The witness types are syntax this guard does not walk, so the word
            // hashes the arity it can see and then declines rigidity: claiming a
            // distinctness it cannot see is the one thing a guard may never do,
            // and a reified stack declines it for the same reason.
            let base = Guard::leaf(mix_word(
                seed(SemanticHash::from(kind::PACK)),
                SemanticHash::from(u64::try_from(witnesses.len()).unwrap_or(u64::MAX)),
            ));
            base.fold(arena.value(payload)?.guard()).with_unfolding()
        },
        | SemValueNode::Reified(_) => {
            Guard::leaf(seed(SemanticHash::from(kind::REIFIED))).with_unfolding()
        },
        | SemValueNode::Rigid(ref head, face) => {
            let hash = match *head {
                | Rigid::Level(level) => mix_word(
                    seed(SemanticHash::from(kind::RIGID)),
                    SemanticHash::from(u64::from(u32::from(level))),
                ),
                | Rigid::Free(ref name) => mix_str(
                    seed(SemanticHash::from(kind::RIGID)),
                    NameRef::from(name.as_str()),
                ),
                | Rigid::Hole(id) => mix_hashable(seed(SemanticHash::from(kind::RIGID)), &id),
            };
            let guard = Guard::leaf(hash);
            let guard = match *head {
                | Rigid::Hole(_) => guard.with_hole(),
                | Rigid::Level(_) | Rigid::Free(_) => guard,
            };
            if matches!(face, ValueUnfold::Rigid) {
                guard
            }
            else {
                guard.with_unfolding()
            }
        },
    };
    Ok(guard)
}

/// Computes the guard word of a computation node.
///
/// # Errors
///
/// Returns the arena's missing-node error when a child does not resolve.
fn comp_guard(
    arena: &SemArena,
    node: &SemCompNode,
) -> Result<Guard, SemError>
{
    let guard = match *node {
        | SemCompNode::Lambda(_) => {
            Guard::leaf(seed(SemanticHash::from(kind::LAMBDA))).with_unfolding()
        },
        | SemCompNode::Return(value) => {
            Guard::leaf(seed(SemanticHash::from(kind::RETURN))).fold(arena.value(value)?.guard())
        },
        | SemCompNode::LazyPair(..) => {
            Guard::leaf(seed(SemanticHash::from(kind::LAZY_PAIR))).with_unfolding()
        },
        | SemCompNode::Neutral(_) => {
            Guard::leaf(seed(SemanticHash::from(kind::NEUTRAL))).with_unfolding()
        },
    };
    Ok(guard)
}

/// Mints a value node with its computed guard word.
///
/// # Errors
///
/// Returns the arena's error when a child does not resolve or the id space is
/// exhausted.
#[inline]
pub fn value(
    nbe: &mut Normalizer,
    node: SemValueNode,
) -> Result<SemValueId, SemError>
{
    let guard = value_guard(nbe.arena(), &node)?;
    nbe.arena_mut().mint_value(SemValue::new(node, guard))
}

/// Mints a value node with its computed guard word, retaining `term` as its
/// term face.
///
/// # Errors
///
/// Returns the arena's error when a child does not resolve or the id space is
/// exhausted.
fn value_retaining(
    nbe: &mut Normalizer,
    node: SemValueNode,
    term: ValueNodeId,
) -> Result<SemValueId, SemError>
{
    let guard = value_guard(nbe.arena(), &node)?;
    nbe.arena_mut()
        .mint_value(SemValue::new(node, guard).retaining(term))
}

/// Mints a computation node with its computed guard word.
///
/// # Errors
///
/// Returns the arena's error when a child does not resolve or the id space is
/// exhausted.
#[inline]
pub fn comp(
    nbe: &mut Normalizer,
    node: SemCompNode,
) -> Result<SemCompId, SemError>
{
    let guard = comp_guard(nbe.arena(), &node)?;
    nbe.arena_mut().mint_comp(SemComp::new(node, guard))
}

/// Mints a neutral computation with the given head and unfolding face.
///
/// # Errors
///
/// Returns the arena's error when the id space is exhausted.
fn neutral(
    nbe: &mut Normalizer,
    head: NeutralHead,
    unfold: CompUnfold,
) -> Result<SemCompId, SemError>
{
    let id = nbe.arena_mut().mint_neutral(Neutral::new(head, unfold))?;
    comp(nbe, SemCompNode::Neutral(id))
}

/// Mints a closure over `env` abstracting `binders` in `body`.
///
/// # Errors
///
/// Returns the arena's error when the id space is exhausted.
fn closure(
    nbe: &mut Normalizer,
    env: EnvId,
    binders: Vec<String>,
    body: CompNodeId,
) -> Result<ClosureId, SemError>
{
    nbe.arena_mut()
        .mint_closure(Closure::new(env, binders, body))
}

/// Binds a closure's binders to `args` over its captured environment.
///
/// # Errors
///
/// Returns [`SemError::ClosureArity`] when the counts disagree, or the arena's
/// error when an id does not resolve.
fn enter(
    nbe: &mut Normalizer,
    id: ClosureId,
    args: &[SemValueId],
) -> Result<(EnvId, CompNodeId), SemError>
{
    let (mut env, binders, body) = {
        let closure = nbe.arena().closure(id)?;
        (closure.env(), closure.binders().to_vec(), closure.body())
    };
    if binders.len() != args.len() {
        return Err(SemError::ClosureArity);
    }
    for (binder, arg) in binders.into_iter().zip(args.iter()) {
        env = nbe.arena_mut().bind(env, binder, *arg)?;
    }
    Ok((env, body))
}

/// One finished child on the value walk's result stack: the semantic value,
/// together with whether its evaluation touched the environment at all.
#[derive(Clone, Copy, Debug)]
struct Evaluated
{
    /// The semantic value.
    id: SemValueId,
    /// Whether the source term is still equal to this value, so the term face
    /// may retain it.
    retained: TermRetention,
}

/// What a value-walk finish frame reassembles.
enum ValueFinish
{
    /// Rebuild a pair.
    Pair,
    /// Rebuild an injection.
    Inj(Side),
    /// Rebuild a list of this many elements.
    List(usize),
    /// Rebuild a record over these labels, in order.
    Record(Vec<String>),
    /// Rebuild a reflexivity witness.
    Here,
    /// Rebuild a constructor value.
    Ctor(gandr_core_term::types::DataId, ConstructorTag),
    /// Rebuild a packed module over these witness types.
    Pack(Vec<ValueTypeNodeId>),
}

/// One pending step of the value walk.
enum ValueTask
{
    /// Evaluate a value node.
    Visit(ValueNodeId),
    /// Reassemble a node from the results already on the stack.
    Finish(ValueNodeId, ValueFinish),
}

/// Evaluates a **value** under `env` into the semantic domain.
///
/// Values are inert: nothing here reduces, so evaluation is a structural
/// rebuild that resolves variables and suspends thunks. What it also does is
/// decide the **term face**: a value whose evaluation consulted no environment
/// binding and captured no environment into a closure is equal to the node it
/// came from, so that node id is retained and readback can hand it straight
/// back.
///
/// # Contract
/// - requires: `env` was minted by this normalizer's arena and `term` names a
///   node in its syntax store.
/// - ensures: the result is the semantic value of `term` under `env`; a
///   variable bound in `env` **aliases** the bound id rather than copying it,
///   which is how the sharing the caller handed in is preserved without any
///   being created; an ascription is erased, exactly as conversion has always
///   treated one.
/// - provides: the domain every other entry point in this module consumes.
/// - fails: the arena's error when the id space is exhausted or an id fails to
///   resolve.
/// - panics: none.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Adequacy
/// - hypothesis: L3 for the term face's two decision surfaces — a term with no
///   environment interaction must retain, and a term with one must not — plus
///   L2 for the rebuild, whose oracle is readback: quoting the evaluation of
///   any closed value returns that value.
/// - witness: `crate::tests::evaluating_an_unreduced_value_retains_its_term_face`
/// - witness: `crate::tests::evaluating_through_the_environment_drops_the_term_face`
/// - witness: `crate::tests::quote_after_eval_is_the_identity_on_inert_values`
///
/// # Termination
/// - reason: the walk drains an explicit task stack over one finite term.
/// - measure: pending tasks on the stack.
/// - boundedness: terms are finite node graphs and thunk bodies are suspended
///   rather than entered.
/// - input recursion: none.
#[inline]
pub fn eval_value(
    nbe: &mut Normalizer,
    env: EnvId,
    term: ValueNodeId,
) -> Result<SemValueId, SemError>
{
    let mut work = alloc::vec![ValueTask::Visit(term)];
    let mut done: Vec<Evaluated> = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            | ValueTask::Visit(term) => visit_value(nbe, env, term, &mut work, &mut done)?,
            | ValueTask::Finish(term, finish) => {
                finish_value(nbe, term, finish, &mut done)?;
            },
        }
    }
    Ok(pop(&mut done).id)
}

/// Evaluates one value node, queueing its children.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn visit_value(
    nbe: &mut Normalizer,
    env: EnvId,
    term: ValueNodeId,
    work: &mut Vec<ValueTask>,
    done: &mut Vec<Evaluated>,
) -> Result<(), SemError>
{
    /// Records one finished leaf, with its term face retained.
    fn leaf(
        nbe: &mut Normalizer,
        node: SemValueNode,
        term: ValueNodeId,
        done: &mut Vec<Evaluated>,
    ) -> Result<(), SemError>
    {
        let id = value_retaining(nbe, node, term)?;
        done.push(Evaluated {
            id,
            retained: TermRetention::from(true),
        });
        Ok(())
    }

    match syntax_value(nbe, term)? {
        | ValueNode::Var(name) => {
            let bound = nbe.arena().lookup(env, NameRef::from(name.as_str()))?;
            match bound {
                | Some(id) => done.push(Evaluated {
                    id,
                    retained: TermRetention::from(false),
                }),
                | None => {
                    let face = match nbe.definitions().lookup(NameRef::from(name.as_str())) {
                        | Some(definition) => ValueUnfold::Pending(definition.height()),
                        | None => ValueUnfold::Rigid,
                    };
                    leaf(
                        nbe,
                        SemValueNode::Rigid(Rigid::Free(name), face),
                        term,
                        done,
                    )?;
                },
            }
        },
        | ValueNode::Unit => leaf(nbe, SemValueNode::Unit, term, done)?,
        | ValueNode::Int(literal) => leaf(nbe, SemValueNode::Int(literal), term, done)?,
        | ValueNode::Str(literal) => leaf(nbe, SemValueNode::Str(literal), term, done)?,
        | ValueNode::Num(literal) => leaf(nbe, SemValueNode::Num(literal), term, done)?,
        | ValueNode::Hole(hole) => {
            leaf(
                nbe,
                SemValueNode::Rigid(Rigid::Hole(hole), ValueUnfold::Rigid),
                term,
                done,
            )?;
        },
        | ValueNode::Thunk(grade, body) => {
            let cell = closure(nbe, env, Vec::new(), body)?;
            suspend(nbe, SemValueNode::Thunk(grade, cell), env, term, done)?;
        },
        | ValueNode::Stk(stack) => {
            suspend(nbe, SemValueNode::Reified(stack), env, term, done)?;
        },
        | ValueNode::Annot(inner, _) => work.push(ValueTask::Visit(inner)),
        | ValueNode::Pair(fst, snd) => {
            work.push(ValueTask::Finish(term, ValueFinish::Pair));
            work.push(ValueTask::Visit(snd));
            work.push(ValueTask::Visit(fst));
        },
        | ValueNode::Inj(side, payload) => {
            work.push(ValueTask::Finish(term, ValueFinish::Inj(side)));
            work.push(ValueTask::Visit(payload));
        },
        | ValueNode::Here(witness) => {
            work.push(ValueTask::Finish(term, ValueFinish::Here));
            work.push(ValueTask::Visit(witness));
        },
        | ValueNode::Ctor { id, tag, payload } => {
            let finish = ValueFinish::Ctor(id, ConstructorTag::from(tag));
            work.push(ValueTask::Finish(term, finish));
            work.push(ValueTask::Visit(payload));
        },
        | ValueNode::Pack { witnesses, payload } => {
            // A packed module is inert, exactly as a constructor value is: the
            // payload evaluates and the witness types ride along as the syntax
            // they are. Nothing here discharges a binder — instantiation is a
            // typing step, and it leaves no residue in the term.
            work.push(ValueTask::Finish(term, ValueFinish::Pack(witnesses)));
            work.push(ValueTask::Visit(payload));
        },
        | ValueNode::List(elements) => {
            work.push(ValueTask::Finish(term, ValueFinish::List(elements.len())));
            for element in elements.iter().rev() {
                work.push(ValueTask::Visit(*element));
            }
        },
        | ValueNode::Record(fields) => {
            let labels = fields.keys().cloned().collect::<Vec<_>>();
            work.push(ValueTask::Finish(term, ValueFinish::Record(labels)));
            for field in fields.values().rev() {
                work.push(ValueTask::Visit(*field));
            }
        },
    }
    Ok(())
}

/// Records a node that suspends its body over the environment — a thunk or a
/// reified stack. Its term face survives only in the empty environment, where
/// the capture cannot have changed anything.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn suspend(
    nbe: &mut Normalizer,
    node: SemValueNode,
    env: EnvId,
    term: ValueNodeId,
    done: &mut Vec<Evaluated>,
) -> Result<(), SemError>
{
    let inert = env == SemArena::EMPTY_ENV;
    let id = if inert {
        value_retaining(nbe, node, term)?
    }
    else {
        value(nbe, node)?
    };
    done.push(Evaluated {
        id,
        retained: TermRetention::from(inert),
    });
    Ok(())
}

/// Reassembles one value node from the children already on the result stack.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn finish_value(
    nbe: &mut Normalizer,
    term: ValueNodeId,
    finish: ValueFinish,
    done: &mut Vec<Evaluated>,
) -> Result<(), SemError>
{
    let (node, unchanged) = match finish {
        | ValueFinish::Pair => {
            let snd = pop(done);
            let fst = pop(done);
            (
                SemValueNode::Pair(fst.id, snd.id),
                bool::from(fst.retained) && bool::from(snd.retained),
            )
        },
        | ValueFinish::Inj(side) => {
            let payload = pop(done);
            (
                SemValueNode::Inj(side, payload.id),
                bool::from(payload.retained),
            )
        },
        | ValueFinish::Here => {
            let witness = pop(done);
            (SemValueNode::Here(witness.id), bool::from(witness.retained))
        },
        | ValueFinish::Ctor(id, tag) => {
            let payload = pop(done);
            (
                SemValueNode::Ctor {
                    id,
                    tag,
                    payload: payload.id,
                },
                bool::from(payload.retained),
            )
        },
        | ValueFinish::Pack(witnesses) => {
            let payload = pop(done);
            (
                SemValueNode::Pack {
                    witnesses,
                    payload: payload.id,
                },
                bool::from(payload.retained),
            )
        },
        | ValueFinish::List(count) => {
            let mut elements = Vec::with_capacity(count);
            let mut unchanged = true;
            for _ in 0 .. count {
                let child = pop(done);
                elements.push(child.id);
                unchanged = unchanged && bool::from(child.retained);
            }
            elements.reverse();
            (SemValueNode::List(elements), unchanged)
        },
        | ValueFinish::Record(labels) => {
            let mut fields = Vec::with_capacity(labels.len());
            let mut unchanged = true;
            for _ in 0 .. labels.len() {
                let child = pop(done);
                fields.push(child.id);
                unchanged = unchanged && bool::from(child.retained);
            }
            fields.reverse();
            (
                SemValueNode::Record(labels.into_iter().zip(fields).collect()),
                unchanged,
            )
        },
    };
    let id = if unchanged {
        value_retaining(nbe, node, term)?
    }
    else {
        value(nbe, node)?
    };
    done.push(Evaluated {
        id,
        retained: TermRetention::from(unchanged),
    });
    Ok(())
}

/// Pops one finished child off the value walk's result stack.
///
/// The fallback is unreachable under the post-order balance invariant every
/// finish frame maintains; it exists because indexing and unwrapping are
/// banned, and it degenerates rather than panicking.
fn pop(done: &mut Vec<Evaluated>) -> Evaluated
{
    debug_assert!(
        !done.is_empty(),
        "value evaluation worklist underflow (post-order balance)"
    );
    done.pop().unwrap_or_else(|| Evaluated {
        id: SemValueId::from(0),
        retained: TermRetention::from(false),
    })
}

/// Forces a **value** to canonical form, unfolding its head's definition when
/// `mode` allows.
///
/// # Contract
/// - requires: `id` was minted by this normalizer's arena.
/// - ensures: the result is convertible to `id` and is either canonical or a
///   rigid head this mode declined to unfold; the unfolded face is memoized on
///   the node that owns it, so a second force of the same value costs one
///   lookup.
/// - provides: the canonical scrutinee every eliminator in [`eval_comp`] needs.
/// - fails: the arena's error, or a silent stop at the fuel bound when a
///   definition unfolds to itself.
/// - panics: none.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Adequacy
/// - hypothesis: L3 — four decision surfaces separated pointwise: a canonical
///   value is returned unchanged, a rigid value with no definition is returned
///   unchanged, a reducible definition unfolds under every unfolding mode, and
///   an irreducible one unfolds under the full mode but not the speculative
///   one. The fuel guard is separated by a self-referential definition.
/// - witness: `crate::tests::forcing_unfolds_a_reducible_definition`
/// - witness: `crate::tests::speculative_forcing_leaves_an_irreducible_definition_alone`
/// - witness: `crate::tests::forcing_a_self_referential_definition_terminates`
#[inline]
pub fn force_value(
    nbe: &mut Normalizer,
    id: SemValueId,
    mode: ForceMode,
) -> Result<SemValueId, SemError>
{
    let mut current = id;
    let mut fuel = u32::from(nbe.fuel());
    while fuel > 0 {
        fuel = fuel.saturating_sub(1);
        let (name, face) = {
            let node = nbe.arena().value(current)?;
            match *node.node() {
                | SemValueNode::Rigid(Rigid::Free(ref name), face) => (name.clone(), face),
                | _ => return Ok(current),
            }
        };
        match face {
            | ValueUnfold::Rigid | ValueUnfold::Blocked(_) => return Ok(current),
            | ValueUnfold::Forced(unfolded) => {
                current = unfolded;
            },
            | ValueUnfold::Pending(_) => {
                let Some(definition) = nbe.definitions().lookup(NameRef::from(name.as_str()))
                else {
                    return Ok(current);
                };
                if !bool::from(mode.unfolds(definition.transparency())) {
                    return Ok(current);
                }
                let body = definition.body();
                let unfolded = eval_value(nbe, SemArena::EMPTY_ENV, body)?;
                nbe.arena_mut()
                    .set_value_unfold(current, ValueUnfold::Forced(unfolded))?;
                if unfolded == current {
                    return Ok(current);
                }
                current = unfolded;
            },
        }
    }
    Ok(current)
}

/// The unfolding face a neutral computation inherits from the value at its
/// head.
fn head_face(
    nbe: &Normalizer,
    head: SemValueId,
) -> Result<CompUnfold, SemError>
{
    let node = nbe.arena().value(head)?;
    let face = match *node.node() {
        | SemValueNode::Rigid(_, face) => face,
        | _ => ValueUnfold::Rigid,
    };
    Ok(match face.height() {
        | Some(height) => CompUnfold::Pending(height),
        | None => CompUnfold::Rigid,
    })
}

/// Evaluates a **computation** under `env` to weak-head normal form.
///
/// # Contract
/// - requires: `env` was minted by this normalizer's arena and `term` names a
///   node in its syntax store.
/// - ensures: the result is the weak-head normal form of `term` under `env` in
///   `mode`; every reduction that fires is a beta law of the calculus, and no
///   effect, handler, resumption, delimiter, capture, grade operation, or
///   native primitive is ever run.
/// - provides: the head every eliminator and every conversion step consumes.
/// - fails: the arena's error when the id space is exhausted or an id fails to
///   resolve.
/// - panics: none.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Adequacy
/// - hypothesis: L2 against readback for the reduction rules — quoting the
///   evaluation of a redex must equal quoting the evaluation of its contractum
///   — plus L3 for the neutral arms, each separated by one term whose head is a
///   rigid variable, and for the quarantine, separated by an effectful term
///   whose weak head must stay neutral.
/// - witness: `crate::tests::beta_fires_for_every_positive_eliminator`
/// - witness: `crate::tests::record_projection_reduces_and_stays_spine_local`
/// - witness: `crate::tests::walk_beta_fires_on_here`
/// - witness: `crate::tests::unpacking_a_packed_module_binds_the_payload`
/// - witness: `crate::tests::unpacking_a_neutral_package_stays_stuck_and_keeps_its_annotation_half`
/// - witness: `crate::tests::the_quarantine_leaves_effects_neutral`
#[inline]
pub fn eval_comp(
    nbe: &mut Normalizer,
    env: EnvId,
    term: CompNodeId,
    mode: ForceMode,
) -> Result<SemCompId, SemError>
{
    run_machine(nbe, Phase::Descend(env, term), Vec::new(), mode)
}

/// Drives the machine from `phase` over `stack` until the stack drains.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Termination
/// - reason: the driver is a loop over an explicit frame stack, not recursion.
/// - measure: the frame stack together with the node being descended into.
/// - boundedness: each step consumes one syntax node or one frame, and
///   unfolding is bounded by the normalizer's fuel.
/// - input recursion: none.
fn run_machine(
    nbe: &mut Normalizer,
    mut phase: Phase,
    mut stack: Vec<Frame>,
    mode: ForceMode,
) -> Result<SemCompId, SemError>
{
    loop {
        phase = match phase {
            | Phase::Descend(env, node) => descend(nbe, env, node, mode, &mut stack)?,
            | Phase::Unwind(id) => match unwind(nbe, id, &mut stack)? {
                | Some(next) => next,
                | None => return Ok(id),
            },
        };
    }
}

/// Applies a weak-head normal computation to one argument.
///
/// This is how eta is decided: a function and a neutral are compared by
/// applying **both** to one fresh variable, and applying a neutral is what
/// grows its spine.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
#[inline]
pub fn apply(
    nbe: &mut Normalizer,
    id: SemCompId,
    arg: SemValueId,
    mode: ForceMode,
) -> Result<SemCompId, SemError>
{
    run_machine(
        nbe,
        Phase::Unwind(id),
        alloc::vec![Frame::Elim(Elim::Apply(arg))],
        mode,
    )
}

/// Projects one component out of a weak-head normal computation.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
#[inline]
pub fn project(
    nbe: &mut Normalizer,
    id: SemCompId,
    side: Side,
    mode: ForceMode,
) -> Result<SemCompId, SemError>
{
    run_machine(
        nbe,
        Phase::Unwind(id),
        alloc::vec![Frame::Elim(Elim::Project(side))],
        mode,
    )
}

/// Forces a nullary closure to weak-head normal form, memoizing the result in
/// its own thunk cell.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
#[inline]
pub fn enter_nullary(
    nbe: &mut Normalizer,
    cell: ClosureId,
    mode: ForceMode,
) -> Result<SemCompId, SemError>
{
    if let Some(memo) = nbe.arena().closure(cell)?.memo() {
        return Ok(memo);
    }
    let (env, body) = enter(nbe, cell, &[])?;
    run_machine(
        nbe,
        Phase::Descend(env, body),
        alloc::vec![Frame::Memoize(cell)],
        mode,
    )
}

/// Enters a closure on `args` and drives its body to weak-head normal form.
///
/// # Errors
///
/// Returns [`SemError::ClosureArity`] on a count mismatch, or the arena's
/// error.
#[inline]
pub fn enter_with(
    nbe: &mut Normalizer,
    cell: ClosureId,
    args: &[SemValueId],
    mode: ForceMode,
) -> Result<SemCompId, SemError>
{
    if args.is_empty() {
        return enter_nullary(nbe, cell, mode);
    }
    let (env, body) = enter(nbe, cell, args)?;
    run_machine(nbe, Phase::Descend(env, body), Vec::new(), mode)
}

/// Forces an already-evaluated value as a computation — the semantic image of
/// `force v`.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
#[inline]
pub fn force_head(
    nbe: &mut Normalizer,
    id: SemValueId,
    mode: ForceMode,
) -> Result<SemCompId, SemError>
{
    let cell = match *nbe.arena().value(id)?.node() {
        | SemValueNode::Thunk(_, cell) => Some(cell),
        | _ => None,
    };
    match cell {
        | Some(cell) => enter_nullary(nbe, cell, mode),
        | None => {
            let face = head_face(nbe, id)?;
            neutral(nbe, NeutralHead::Force(id), face)
        },
    }
}

/// Re-applies a recorded spine to a fresh head — how the unfolding face is
/// rebuilt after its head unfolds.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
#[inline]
pub fn rerun_spine(
    nbe: &mut Normalizer,
    base: SemCompId,
    spine: &[Elim],
    mode: ForceMode,
) -> Result<SemCompId, SemError>
{
    let stack = spine
        .iter()
        .rev()
        .map(|elim| Frame::Elim(*elim))
        .collect::<Vec<_>>();
    run_machine(nbe, Phase::Unwind(base), stack, mode)
}

/// One descend step: consume a syntax node, pushing the eliminators it passes.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn descend(
    nbe: &mut Normalizer,
    env: EnvId,
    node: CompNodeId,
    mode: ForceMode,
    stack: &mut Vec<Frame>,
) -> Result<Phase, SemError>
{
    match syntax_comp(nbe, node)? {
        | CompNode::Abs(binder, _, body) => {
            if let Some(&Frame::Elim(Elim::Apply(arg))) = stack.last() {
                stack.pop();
                let env = nbe.arena_mut().bind(env, binder, arg)?;
                Ok(Phase::Descend(env, body))
            }
            else {
                let cell = closure(nbe, env, alloc::vec![binder], body)?;
                Ok(Phase::Unwind(comp(nbe, SemCompNode::Lambda(cell))?))
            }
        },
        | CompNode::App(head, arg) => {
            let arg = eval_value(nbe, env, arg)?;
            stack.push(Frame::Elim(Elim::Apply(arg)));
            Ok(Phase::Descend(env, head))
        },
        | CompNode::Prj(side, body) => {
            stack.push(Frame::Elim(Elim::Project(side)));
            Ok(Phase::Descend(env, body))
        },
        | CompNode::Bind(bound, binder, cont) => {
            let cell = closure(nbe, env, alloc::vec![binder], cont)?;
            stack.push(Frame::Elim(Elim::Sequence(cell)));
            Ok(Phase::Descend(env, bound))
        },
        | CompNode::Ret(carried) => {
            let carried = eval_value(nbe, env, carried)?;
            if let Some(&Frame::Elim(Elim::Sequence(cont))) = stack.last() {
                stack.pop();
                let (env, body) = enter(nbe, cont, &[carried])?;
                Ok(Phase::Descend(env, body))
            }
            else {
                Ok(Phase::Unwind(comp(nbe, SemCompNode::Return(carried))?))
            }
        },
        | CompNode::With(fst, snd) => {
            let fst = closure(nbe, env, Vec::new(), fst)?;
            let snd = closure(nbe, env, Vec::new(), snd)?;
            Ok(Phase::Unwind(comp(nbe, SemCompNode::LazyPair(fst, snd))?))
        },
        | CompNode::Force(thunked) => {
            let thunked = eval_value(nbe, env, thunked)?;
            let thunked = force_value(nbe, thunked, mode)?;
            let cell = match *nbe.arena().value(thunked)?.node() {
                | SemValueNode::Thunk(_, cell) => Some(cell),
                | _ => None,
            };
            match cell {
                | Some(cell) => {
                    if let Some(memo) = nbe.arena().closure(cell)?.memo() {
                        return Ok(Phase::Unwind(memo));
                    }
                    stack.push(Frame::Memoize(cell));
                    let (env, body) = enter(nbe, cell, &[])?;
                    Ok(Phase::Descend(env, body))
                },
                | None => {
                    let face = head_face(nbe, thunked)?;
                    Ok(Phase::Unwind(neutral(
                        nbe,
                        NeutralHead::Force(thunked),
                        face,
                    )?))
                },
            }
        },
        | CompNode::Case(scrut, on_left, on_right) => {
            let scrut = eval_value(nbe, env, scrut)?;
            let scrut = force_value(nbe, scrut, mode)?;
            let injected = match *nbe.arena().value(scrut)?.node() {
                | SemValueNode::Inj(side, payload) => Some((side, payload)),
                | _ => None,
            };
            match injected {
                | Some((Side::Fst, payload)) => {
                    let cell = closure(nbe, env, alloc::vec![on_left.0], on_left.1)?;
                    let (env, body) = enter(nbe, cell, &[payload])?;
                    Ok(Phase::Descend(env, body))
                },
                | Some((Side::Snd, payload)) => {
                    let cell = closure(nbe, env, alloc::vec![on_right.0], on_right.1)?;
                    let (env, body) = enter(nbe, cell, &[payload])?;
                    Ok(Phase::Descend(env, body))
                },
                | None => {
                    let left = closure(nbe, env, alloc::vec![on_left.0], on_left.1)?;
                    let right = closure(nbe, env, alloc::vec![on_right.0], on_right.1)?;
                    let face = head_face(nbe, scrut)?;
                    let head = NeutralHead::Case {
                        scrutinee: scrut,
                        on_left: left,
                        on_right: right,
                    };
                    Ok(Phase::Unwind(neutral(nbe, head, face)?))
                },
            }
        },
        | CompNode::DataCase { scrut, arms } => {
            let scrut = eval_value(nbe, env, scrut)?;
            let scrut = force_value(nbe, scrut, mode)?;
            let constructed = match *nbe.arena().value(scrut)?.node() {
                | SemValueNode::Ctor { tag, payload, .. } => Some((usize::from(tag), payload)),
                | _ => None,
            };
            let selected = constructed
                .and_then(|(tag, payload)| arms.get(tag).map(|arm| (arm.clone(), payload)));
            match selected {
                | Some(((binder, body), payload)) => {
                    let cell = closure(nbe, env, alloc::vec![binder], body)?;
                    let (env, body) = enter(nbe, cell, &[payload])?;
                    Ok(Phase::Descend(env, body))
                },
                | None => {
                    let mut cells = Vec::with_capacity(arms.len());
                    for arm in arms {
                        let cell = closure(nbe, env, alloc::vec![arm.0.clone()], arm.1)?;
                        cells.push((arm.0, cell));
                    }
                    let face = head_face(nbe, scrut)?;
                    let head = NeutralHead::DataCase {
                        scrutinee: scrut,
                        arms: cells,
                    };
                    Ok(Phase::Unwind(neutral(nbe, head, face)?))
                },
            }
        },
        | CompNode::ListCase {
            scrut,
            nil,
            head,
            tail,
            cons,
        } => {
            let scrut = eval_value(nbe, env, scrut)?;
            let scrut = force_value(nbe, scrut, mode)?;
            let listed = match *nbe.arena().value(scrut)?.node() {
                | SemValueNode::List(ref elements) => Some(elements.clone()),
                | _ => None,
            };
            match listed {
                | Some(elements) => match elements.split_first() {
                    | None => {
                        let cell = closure(nbe, env, Vec::new(), nil)?;
                        let (env, body) = enter(nbe, cell, &[])?;
                        Ok(Phase::Descend(env, body))
                    },
                    | Some((first, rest)) => {
                        let rest = value(nbe, SemValueNode::List(rest.to_vec()))?;
                        let cell = closure(nbe, env, alloc::vec![head, tail], cons)?;
                        let (env, body) = enter(nbe, cell, &[*first, rest])?;
                        Ok(Phase::Descend(env, body))
                    },
                },
                | None => {
                    let nil = closure(nbe, env, Vec::new(), nil)?;
                    let cons = closure(nbe, env, alloc::vec![head, tail], cons)?;
                    let face = head_face(nbe, scrut)?;
                    let head = NeutralHead::ListCase {
                        scrutinee: scrut,
                        nil,
                        cons,
                    };
                    Ok(Phase::Unwind(neutral(nbe, head, face)?))
                },
            }
        },
        | CompNode::Split {
            scrut,
            fst_name,
            snd_name,
            body,
            ..
        } => {
            // The motive is a type ascription on the eliminator: it names the
            // result type and contributes nothing to the value computed, so it
            // is erased here for the same reason an ascription is.
            let scrut = eval_value(nbe, env, scrut)?;
            let scrut = force_value(nbe, scrut, mode)?;
            let paired = match *nbe.arena().value(scrut)?.node() {
                | SemValueNode::Pair(fst, snd) => Some([fst, snd]),
                | _ => None,
            };
            let cell = closure(nbe, env, alloc::vec![fst_name, snd_name], body)?;
            match paired {
                | Some(components) => {
                    let (env, body) = enter(nbe, cell, &components)?;
                    Ok(Phase::Descend(env, body))
                },
                | None => {
                    let face = head_face(nbe, scrut)?;
                    let head = NeutralHead::Split {
                        scrutinee: scrut,
                        body: cell,
                    };
                    Ok(Phase::Unwind(neutral(nbe, head, face)?))
                },
            }
        },
        | CompNode::Unpack {
            scrut,
            binder,
            body,
            ..
        } => {
            // The package eliminator, on the `Split` shape: stuck on its
            // scrutinee, firing the moment that scrutinee is a packed module.
            //
            // **What fires is the term half of the rule, and that is the whole
            // of it here.** The typing rule discharges the signature's binders
            // at freshly minted atoms, and the term rule binds the module
            // variable to the payload; the atom substitution reaches types
            // alone, and this engine erases every type it meets — an
            // ascription, an abstraction's annotation, and both eliminator
            // motives. So no witness is consulted and no atom is substituted,
            // and the residue of the typing half is exactly nothing.
            //
            // The signature and the atoms are not dropped: they stay on this
            // source node, which the neutral head names, so readback rebuilds
            // the elimination it came from rather than inventing one.
            let scrut = eval_value(nbe, env, scrut)?;
            let scrut = force_value(nbe, scrut, mode)?;
            let packed = match *nbe.arena().value(scrut)?.node() {
                | SemValueNode::Pack { payload, .. } => Some(payload),
                | _ => None,
            };
            let cell = closure(nbe, env, alloc::vec![binder], body)?;
            match packed {
                | Some(payload) => {
                    let (env, body) = enter(nbe, cell, &[payload])?;
                    Ok(Phase::Descend(env, body))
                },
                | None => {
                    let face = head_face(nbe, scrut)?;
                    let head = NeutralHead::Unpack {
                        source: node,
                        scrutinee: scrut,
                        body: cell,
                    };
                    Ok(Phase::Unwind(neutral(nbe, head, face)?))
                },
            }
        },
        | CompNode::RecordProj { record, label } => {
            // Structure projection, weak-head and spine-local: the record's
            // head is driven to a structure and the sibling components are
            // never touched.
            let record = eval_value(nbe, env, record)?;
            let record = force_value(nbe, record, mode)?;
            let selected = match *nbe.arena().value(record)?.node() {
                | SemValueNode::Record(ref fields) => fields.get(label.as_str()).copied(),
                | _ => None,
            };
            match selected {
                | Some(field) => Ok(Phase::Unwind(comp(nbe, SemCompNode::Return(field))?)),
                | None => {
                    let face = head_face(nbe, record)?;
                    let head = NeutralHead::Project { record, label };
                    Ok(Phase::Unwind(neutral(nbe, head, face)?))
                },
            }
        },
        | CompNode::Walk { scrut, base, .. } => {
            let scrut = eval_value(nbe, env, scrut)?;
            let scrut = force_value(nbe, scrut, mode)?;
            let witnessed = match *nbe.arena().value(scrut)?.node() {
                | SemValueNode::Here(witness) => Some(witness),
                | _ => None,
            };
            let cell = closure(nbe, env, alloc::vec![base.x], base.body)?;
            match witnessed {
                | Some(witness) => {
                    let (env, body) = enter(nbe, cell, &[witness])?;
                    Ok(Phase::Descend(env, body))
                },
                | None => {
                    let face = head_face(nbe, scrut)?;
                    let head = NeutralHead::Walk {
                        scrutinee: scrut,
                        base: cell,
                    };
                    Ok(Phase::Unwind(neutral(nbe, head, face)?))
                },
            }
        },
        | CompNode::Native { prim, args } => {
            let mut evaluated = Vec::with_capacity(args.len());
            for arg in args {
                evaluated.push(eval_value(nbe, env, arg)?);
            }
            let head = NeutralHead::Native {
                prim,
                args: evaluated,
            };
            Ok(Phase::Unwind(neutral(nbe, head, CompUnfold::Rigid)?))
        },
        | CompNode::Dup(carried) => {
            let carried = eval_value(nbe, env, carried)?;
            let head = NeutralHead::Dup(carried);
            Ok(Phase::Unwind(neutral(nbe, head, CompUnfold::Rigid)?))
        },
        | CompNode::Drop(carried) => {
            let carried = eval_value(nbe, env, carried)?;
            let head = NeutralHead::Drop(carried);
            Ok(Phase::Unwind(neutral(nbe, head, CompUnfold::Rigid)?))
        },
        | CompNode::Perform(_, _, carried) => {
            // The signature and the operation name stay on the source node,
            // which the head names rather than copies.
            let carried = eval_value(nbe, env, carried)?;
            let head = NeutralHead::Perform {
                source: node,
                payload: carried,
            };
            Ok(Phase::Unwind(neutral(nbe, head, CompUnfold::Rigid)?))
        },
        | CompNode::Handle {
            scrutinee,
            ret,
            ops,
            ..
        } => {
            let scrutinee = closure(nbe, env, Vec::new(), scrutinee)?;
            let ret_cell = closure(nbe, env, alloc::vec![ret.0], ret.1)?;
            let mut clauses = Vec::with_capacity(ops.len());
            for clause in ops {
                let binders = alloc::vec![clause.payload, clause.resume];
                clauses.push(closure(nbe, env, binders, clause.body)?);
            }
            let head = NeutralHead::Handle {
                source: node,
                scrutinee,
                ret: ret_cell,
                ops: clauses,
            };
            Ok(Phase::Unwind(neutral(nbe, head, CompUnfold::Rigid)?))
        },
        | CompNode::Resume(carried, body) => {
            let carried = eval_value(nbe, env, carried)?;
            let body = closure(nbe, env, Vec::new(), body)?;
            let head = NeutralHead::Resume {
                value: carried,
                body,
            };
            Ok(Phase::Unwind(neutral(nbe, head, CompUnfold::Rigid)?))
        },
        | CompNode::Reset(body) => {
            let body = closure(nbe, env, Vec::new(), body)?;
            let head = NeutralHead::Reset(body);
            Ok(Phase::Unwind(neutral(nbe, head, CompUnfold::Rigid)?))
        },
        | CompNode::Shift(binder, body) => {
            let body = closure(nbe, env, alloc::vec![binder], body)?;
            let head = NeutralHead::Shift(body);
            Ok(Phase::Unwind(neutral(nbe, head, CompUnfold::Rigid)?))
        },
        | CompNode::Hole(hole) => {
            let head = NeutralHead::Hole(hole);
            Ok(Phase::Unwind(neutral(nbe, head, CompUnfold::Rigid)?))
        },
    }
}

/// One unwind step: consume a frame with a weak-head normal form in hand.
///
/// Returns `None` when the frame stack is empty and the form is the answer.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn unwind(
    nbe: &mut Normalizer,
    id: SemCompId,
    stack: &mut Vec<Frame>,
) -> Result<Option<Phase>, SemError>
{
    let Some(frame) = stack.pop()
    else {
        return Ok(None);
    };
    let elim = match frame {
        | Frame::Memoize(cell) => {
            nbe.arena_mut().set_closure_memo(cell, id)?;
            return Ok(Some(Phase::Unwind(id)));
        },
        | Frame::Elim(elim) => elim,
    };
    let node = nbe.arena().comp(id)?.node().clone();
    let phase = match (node, elim) {
        | (SemCompNode::Lambda(cell), Elim::Apply(arg)) => {
            let (env, body) = enter(nbe, cell, &[arg])?;
            Phase::Descend(env, body)
        },
        | (SemCompNode::Return(carried), Elim::Sequence(cont)) => {
            let (env, body) = enter(nbe, cont, &[carried])?;
            Phase::Descend(env, body)
        },
        | (SemCompNode::LazyPair(fst, snd), Elim::Project(side)) => {
            let cell = match side {
                | Side::Fst => fst,
                | Side::Snd => snd,
            };
            if let Some(memo) = nbe.arena().closure(cell)?.memo() {
                Phase::Unwind(memo)
            }
            else {
                stack.push(Frame::Memoize(cell));
                let (env, body) = enter(nbe, cell, &[])?;
                Phase::Descend(env, body)
            }
        },
        | (SemCompNode::Neutral(stuck), elim) => {
            let extended = {
                let neutral = nbe.arena().neutral(stuck)?;
                neutral.extended(elim, neutral.unfold().height())
            };
            let stuck = nbe.arena_mut().mint_neutral(extended)?;
            Phase::Unwind(comp(nbe, SemCompNode::Neutral(stuck))?)
        },
        | (_, elim) => {
            let stuck =
                Neutral::new(NeutralHead::Mismatch(id), CompUnfold::Rigid).extended(elim, None);
            let stuck = nbe.arena_mut().mint_neutral(stuck)?;
            Phase::Unwind(comp(nbe, SemCompNode::Neutral(stuck))?)
        },
    };
    Ok(Some(phase))
}
