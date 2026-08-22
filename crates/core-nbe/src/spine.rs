//! The canonical shape of a neutral computation's spine.
//!
//! A neutral computation is a stuck head under a list of frustrated
//! eliminators. Two identifications of the calculus are facts about that list
//! rather than about either term being compared, so both live here and both are
//! computed **one spine at a time, independently of whatever the spine will be
//! compared against**:
//!
//! * **Eta for the returner** — `M >>= ret` is `M`, so a sequence entry whose
//!   continuation returns its own binder is not part of the spine at all.
//! * **Associativity of the bind** — `(M >>= f) >>= g` is `M >>= \x. f x >>=
//!   g`, so a sequence entry whose continuation *is itself a bind* carries
//!   eliminators that belong to the spine beneath it.
//!
//! # Why associativity cannot be settled where the spine is extended
//!
//! Extending a neutral's spine is the one place a sequence eliminator meets an
//! existing spine, and the syntactic half of both rules is decided there (see
//! [`crate::eval`]). But the continuation reaching that point is *stored
//! syntax*: for a composite built out of a defined composition operator, the
//! continuation's stored body is an **application**, and it is a bind only in
//! its normal form. Seeing that at the extension site would mean evaluating a
//! body of caller-chosen depth from inside the machine's own step, which is the
//! host-recursion hazard the walk discipline refuses.
//!
//! So the residue is computed at the first site where the normal form *is*
//! computable — where a continuation is opened — and the peel itself is
//! [`peel_bind_chain`], **one function shared with the extension site** so the
//! two cannot drift apart.
//!
//! # The freshness side condition
//!
//! Re-association is sound only when the trailing computations are independent
//! of the bound variable: `M >>= \x. f x >>= g` is `(M >>= f) >>= g` only when
//! `g` does not mention `x`, or the re-association leaves `x` free where a
//! binder stood. That is an ordinary occurs check on the continuation body, and
//! [`free_names`] answers it — conservatively, reporting every name as possibly
//! occurring for any node whose variable occurrences it cannot account for, so
//! an unhandled shape costs a refusal and never an acceptance.
//!
//! Re-association also never reorders effects: both associations perform `M`,
//! then `f`, then `g`, in that order. The rule moves a bind's bracketing; it
//! never commutes one bind past another.

use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString as _;
use alloc::vec::Vec;

use gandr_core_term::boundary::TrivialContinuation;
use gandr_core_term::syntax::CompNode;
use gandr_core_term::syntax::CompNodeId;
use gandr_core_term::syntax::FlatArena;
use gandr_core_term::syntax::ValueNode;
use gandr_core_term::syntax::ValueNodeId;

use crate::Normalizer;
use crate::eval::ForceMode;
use crate::eval::enter_with;
use crate::eval::value;
use crate::quote::QuoteMode;
use crate::quote::level_name;
use crate::quote::parse_level_name;
use crate::quote::quote_comp;
use crate::sem::Closure;
use crate::sem::ClosureId;
use crate::sem::Elim;
use crate::sem::Rigid;
use crate::sem::SemArena;
use crate::sem::SemCompNode;
use crate::sem::SemError;
use crate::sem::SemValueNode;
use crate::sem::ValueUnfold;

/// One continuation body split into the computation it sequences and the
/// trailing continuations that may be lifted out of it.
///
/// `head` still abstracts the original binder; each `tails` entry is a binder
/// and a body that the occurs check proved independent of it, in the order the
/// spine takes them.
pub(crate) struct BindChain
{
    /// The computation left under the original binder.
    pub(crate) head: CompNodeId,
    /// The lifted continuations, innermost first.
    pub(crate) tails: Vec<(String, CompNodeId)>,
}

/// Peels the trailing binds of a continuation body abstracting `binder`.
///
/// Returns `None` when nothing may be lifted — either the body is not a bind,
/// or the freshness side condition fails on the first tail that would move.
///
/// # Contract
/// - ensures: a returned chain reassembles to `body` up to the associativity of
///   the bind, and every lifted tail is free of `binder`.
/// - panics: none.
///
/// # Termination
/// - reason: the loop walks strictly inward through `Bind` nodes.
/// - measure: the number of `Bind` nodes on the body's leftmost spine.
/// - boundedness: each iteration replaces the body by its own bound child.
/// - input recursion: none.
pub(crate) fn peel_bind_chain(
    store: &FlatArena,
    body: CompNodeId,
    binder: &str,
) -> Option<BindChain>
{
    let mut head = body;
    let mut tails: Vec<(String, CompNodeId)> = Vec::new();
    while let Some(&CompNode::Bind(bound, ref tail_binder, tail)) = store.comps.get(head) {
        // The side condition, and the whole difficulty: lifting the tail out of
        // the continuation is sound only when the tail cannot see the binder.
        match free_names(store, tail) {
            | Some(free) if !free.contains(binder) => {},
            | _ => break,
        }
        tails.push((tail_binder.clone(), tail));
        head = bound;
    }
    if tails.is_empty() {
        return None;
    }
    tails.reverse();
    Some(BindChain { head, tails })
}

/// Splits a sequence continuation whose **stored** body is already a bind into
/// the spine entries the flattened form carries — the construction-site fast
/// path, taken while the spine is being extended.
///
/// This is the half of the rule that fires without evaluating anything, and it
/// shares [`peel_bind_chain`] with the normal-form half so the two cannot
/// disagree about what may be lifted or about the freshness side condition.
/// What it cannot see is a continuation that is a *call*: its stored body is an
/// application and it is a bind only in its normal form, which is why
/// [`canonical_spine`] exists.
///
/// The lifted closures keep the continuation's own environment. The side
/// condition is what makes that sound: a tail free of the binder resolves every
/// name it mentions in the enclosing environment either way.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
pub(crate) fn stored_reassociation(
    nbe: &mut Normalizer,
    cont: ClosureId,
) -> Result<Option<Vec<Elim>>, SemError>
{
    let (env, binder, body) = {
        let closure = nbe.arena().closure(cont)?;
        let [ref binder] = *closure.binders()
        else {
            return Ok(None);
        };
        (closure.env(), binder.clone(), closure.body())
    };
    let Some(chain) = peel_bind_chain(nbe.syntax(), body, &binder)
    else {
        return Ok(None);
    };
    let mut parts = Vec::with_capacity(chain.tails.len() + 1);
    let head = nbe
        .arena_mut()
        .mint_closure(Closure::new(env, alloc::vec![binder], chain.head))?;
    parts.push(Elim::Sequence(head));
    for (tail_binder, tail) in chain.tails {
        let lifted =
            nbe.arena_mut()
                .mint_closure(Closure::new(env, alloc::vec![tail_binder], tail))?;
        parts.push(Elim::Sequence(lifted));
    }
    Ok(Some(parts))
}

/// Canonicalizes one spine: drops every identity eliminator and flattens every
/// sequence entry whose normal form is itself a bind.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Termination
/// - reason: the worklist shrinks by the total size of the readbacks it holds.
/// - measure: the sum, over pending entries, of the size of the readback of the
///   entry's continuation opened at a fresh level.
/// - boundedness: a peel replaces one entry by the disjoint sub-terms of its
///   own readback, dropping at least one `Bind` node, and every other step
///   removes an entry without adding one.
/// - input recursion: none.
pub(crate) fn canonical_spine(
    nbe: &mut Normalizer,
    spine: &[Elim],
) -> Result<Vec<Elim>, SemError>
{
    let mut pending: Vec<Elim> = spine.iter().rev().copied().collect();
    let mut canonical = Vec::with_capacity(spine.len());
    while let Some(entry) = pending.pop() {
        let Elim::Sequence(cont) = entry
        else {
            canonical.push(entry);
            continue;
        };
        if bool::from(normalized_continuation_is_identity(nbe, cont)?) {
            continue;
        }
        match reassociated(nbe, cont)? {
            | Some(parts) => pending.extend(parts.into_iter().rev()),
            | None => canonical.push(entry),
        }
    }
    Ok(canonical)
}

/// Whether a sequence continuation normalizes to a return of its own binder.
///
/// Enters the closure at one fresh rigid level and asks whether the result is
/// exactly a return of that level — the same shape the construction-site check
/// in [`crate::eval`] tests syntactically, asked of the normal form instead.
///
/// # Why the normal form and not the stored body
///
/// Triviality is a property of the continuation's normal form. The
/// construction-site check reads the stored body, which is the fast path and
/// catches a continuation written as a literal return — but a continuation
/// written as a call that *reduces* to one is equally the identity eliminator,
/// and the stored body cannot show that.
///
/// # The mode is fixed here, and that is the soundness point
///
/// The probe runs at [`ForceMode::Unfold`] **regardless of the mode the
/// surrounding comparison is running at**. Whether a continuation is the
/// identity eliminator is a fact about the term, so the canonical shape of a
/// spine must not depend on how much unfolding the ambient policy happened to
/// permit. Letting the ambient mode decide would mean a pair related at one
/// mode and refused at another — policy deciding which pairs are related rather
/// than how far it unfolds, which is the one thing the strategy fence forbids.
///
/// The residual variation is on the safe side: a probe that exhausts its fuel
/// reports *not* the identity eliminator, so the entry is **kept**. That can
/// cost a refusal and can never produce an acceptance another budget would
/// refute.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn normalized_continuation_is_identity(
    nbe: &mut Normalizer,
    cont: ClosureId,
) -> Result<TrivialContinuation, SemError>
{
    let level = nbe.fresh_level();
    let fresh = value(
        nbe,
        SemValueNode::Rigid(Rigid::Level(level), ValueUnfold::Rigid),
    )?;
    let body = enter_with(nbe, cont, &[fresh], ForceMode::Unfold)?;
    let SemCompNode::Return(produced) = *nbe.arena().comp(body)?.node()
    else {
        return Ok(TrivialContinuation::from(false));
    };
    Ok(TrivialContinuation::from(matches!(
        *nbe.arena().value(produced)?.node(),
        SemValueNode::Rigid(Rigid::Level(returned), _) if returned == level
    )))
}

/// Splits a sequence continuation whose normal form is itself a bind into the
/// spine entries the flattened form carries.
///
/// Returns `None` when nothing may be lifted, which is the ordinary case.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn reassociated(
    nbe: &mut Normalizer,
    cont: ClosureId,
) -> Result<Option<Vec<Elim>>, SemError>
{
    if nbe.arena().closure(cont)?.binders().len() != 1 {
        return Ok(None);
    }
    let level = nbe.fresh_level();
    let fresh = value(
        nbe,
        SemValueNode::Rigid(Rigid::Level(level), ValueUnfold::Rigid),
    )?;
    let whnf = enter_with(nbe, cont, &[fresh], ForceMode::Unfold)?;
    // The cheap precondition, checked before any readback: only a neutral whose
    // spine already ends in a sequence can carry eliminators that belong to the
    // spine beneath it.
    let stuck = match *nbe.arena().comp(whnf)?.node() {
        | SemCompNode::Neutral(stuck) => stuck,
        | _ => return Ok(None),
    };
    if !matches!(
        nbe.arena().neutral(stuck)?.spine().last(),
        Some(Elim::Sequence(_))
    ) {
        return Ok(None);
    }
    // The readback is what makes the occurs check exact: the binder is rendered
    // as a generated level name, which the surface grammar cannot produce and no
    // binder in the readback can shadow, because readback names every binder it
    // opens with a strictly later level.
    let body = quote_comp(nbe, whnf, QuoteMode::Canonical)?;
    let binder = level_name(level);
    let Some(chain) = peel_bind_chain(nbe.syntax(), body, &binder)
    else {
        return Ok(None);
    };
    let mut parts = Vec::with_capacity(chain.tails.len() + 1);
    let head = seal(nbe, binder, chain.head)?;
    parts.push(Elim::Sequence(head));
    for (tail_binder, tail) in chain.tails {
        let lifted = seal(nbe, tail_binder, tail)?;
        parts.push(Elim::Sequence(lifted));
    }
    Ok(Some(parts))
}

/// Closes a readback body over the levels it mentions, abstracting `binder`.
///
/// A readback renders every rigid level as a generated variable name, so a
/// closure built over the empty environment would evaluate those names to free
/// rigids rather than back to the levels they came from. Binding each one
/// restores the level it names; a name an inner binder shadows is bound
/// harmlessly, because the inner binding wins.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn seal(
    nbe: &mut Normalizer,
    binder: String,
    body: CompNodeId,
) -> Result<ClosureId, SemError>
{
    let mentioned = free_names(nbe.syntax(), body).unwrap_or_default();
    let mut env = SemArena::EMPTY_ENV;
    for name in mentioned {
        let Some(level) = parse_level_name(gandr_core_term::boundary::NameRef::from(name.as_str()))
        else {
            continue;
        };
        let rigid = value(
            nbe,
            SemValueNode::Rigid(Rigid::Level(level), ValueUnfold::Rigid),
        )?;
        env = nbe.arena_mut().bind(env, name, rigid)?;
    }
    nbe.arena_mut()
        .mint_closure(Closure::new(env, alloc::vec![binder], body))
}

/// One binder scope on the occurrence walk, as a shared cons list so extending
/// it costs a pointer rather than a copy of every enclosing binder.
enum Scope
{
    /// No binders in scope.
    Empty,
    /// One binder over an enclosing scope.
    Bound(String, Rc<Scope>),
}

impl Scope
{
    /// Whether `name` is bound in this scope.
    fn binds(
        self: &Rc<Self>,
        name: &str,
    ) -> bool
    {
        let mut scope = self;
        loop {
            match **scope {
                | Self::Empty => return false,
                | Self::Bound(ref bound, ref rest) => {
                    if bound == name {
                        return true;
                    }
                    scope = rest;
                },
            }
        }
    }

    /// This scope extended by one binder.
    fn with(
        self: &Rc<Self>,
        binder: &str,
    ) -> Rc<Self>
    {
        Rc::new(Self::Bound(binder.to_string(), Rc::clone(self)))
    }
}

/// One pending node on the occurrence walk, with the binders in scope over it.
enum Occurrence
{
    /// A computation node.
    Comp(CompNodeId, Rc<Scope>),
    /// A value node.
    Value(ValueNodeId, Rc<Scope>),
}

/// Every variable name free in `root`, or `None` when the walk met a node whose
/// variable occurrences it cannot account for.
///
/// `None` is the conservative answer and callers must treat it as *every name
/// may occur*: a type child, a reified stack, and an ascription all name places
/// this walk does not look, exactly as the solver's own occurrence scan does
/// not look into them. Reporting the uncertainty rather than an empty set is
/// what keeps a lifted continuation from escaping a binder the walk could not
/// see.
///
/// # Termination
/// - reason: the walk drains an explicit stack over a finite acyclic store.
/// - measure: pending nodes on the stack.
/// - boundedness: each node pushes only its own children.
/// - input recursion: none.
fn free_names(
    store: &FlatArena,
    root: CompNodeId,
) -> Option<BTreeSet<String>>
{
    let mut found = BTreeSet::new();
    let mut work = alloc::vec![Occurrence::Comp(root, Rc::new(Scope::Empty))];
    while let Some(node) = work.pop() {
        match node {
            | Occurrence::Comp(id, scope) => visit_comp(store, id, &scope, &mut work)?,
            | Occurrence::Value(id, scope) => {
                visit_value(store, id, &scope, &mut work, &mut found)?;
            },
        }
    }
    Some(found)
}

/// Visits one computation node, queueing its term children.
fn visit_comp(
    store: &FlatArena,
    id: CompNodeId,
    scope: &Rc<Scope>,
    work: &mut Vec<Occurrence>,
) -> Option<()>
{
    let node = store.comps.get(id)?;
    match *node {
        | CompNode::Hole(_) => {},
        | CompNode::Abs(ref binder, None, body) => {
            work.push(Occurrence::Comp(body, scope.with(binder)));
        },
        | CompNode::Fix(ref binder, body) | CompNode::Shift(ref binder, body) => {
            work.push(Occurrence::Comp(body, scope.with(binder)));
        },
        | CompNode::App(head, argument) => {
            work.push(Occurrence::Comp(head, Rc::clone(scope)));
            work.push(Occurrence::Value(argument, Rc::clone(scope)));
        },
        | CompNode::Ret(carried) | CompNode::Dup(carried) | CompNode::Drop(carried) => {
            work.push(Occurrence::Value(carried, Rc::clone(scope)));
        },
        | CompNode::Force(thunked) => work.push(Occurrence::Value(thunked, Rc::clone(scope))),
        | CompNode::Bind(bound, ref binder, cont) => {
            work.push(Occurrence::Comp(bound, Rc::clone(scope)));
            work.push(Occurrence::Comp(cont, scope.with(binder)));
        },
        | CompNode::Case(scrutinee, (ref left, on_left), (ref right, on_right)) => {
            work.push(Occurrence::Value(scrutinee, Rc::clone(scope)));
            work.push(Occurrence::Comp(on_left, scope.with(left)));
            work.push(Occurrence::Comp(on_right, scope.with(right)));
        },
        | CompNode::ListCase {
            scrut,
            nil,
            ref head,
            ref tail,
            cons,
        } => {
            work.push(Occurrence::Value(scrut, Rc::clone(scope)));
            work.push(Occurrence::Comp(nil, Rc::clone(scope)));
            work.push(Occurrence::Comp(cons, scope.with(head).with(tail)));
        },
        | CompNode::DataCase { scrut, ref arms } => {
            work.push(Occurrence::Value(scrut, Rc::clone(scope)));
            for &(ref binder, body) in arms {
                work.push(Occurrence::Comp(body, scope.with(binder)));
            }
        },
        | CompNode::RecordProj { record, .. } => {
            work.push(Occurrence::Value(record, Rc::clone(scope)));
        },
        | CompNode::With(fst, snd) => {
            work.push(Occurrence::Comp(fst, Rc::clone(scope)));
            work.push(Occurrence::Comp(snd, Rc::clone(scope)));
        },
        | CompNode::Prj(_, body) | CompNode::Reset(body) => {
            work.push(Occurrence::Comp(body, Rc::clone(scope)));
        },
        | CompNode::Resume(carried, body) => {
            work.push(Occurrence::Value(carried, Rc::clone(scope)));
            work.push(Occurrence::Comp(body, Rc::clone(scope)));
        },
        | CompNode::Native { ref args, .. } => {
            for &argument in args {
                work.push(Occurrence::Value(argument, Rc::clone(scope)));
            }
        },
        | CompNode::Split {
            scrut,
            ref fst_name,
            ref snd_name,
            motive: None,
            body,
        } => {
            work.push(Occurrence::Value(scrut, Rc::clone(scope)));
            work.push(Occurrence::Comp(body, scope.with(fst_name).with(snd_name)));
        },
        // Everything left carries a type child, a signature, or a reified
        // stack — places this walk does not look, so it cannot report where a
        // variable occurs inside them.
        | _ => return None,
    }
    Some(())
}

/// Visits one value node, queueing its term children.
fn visit_value(
    store: &FlatArena,
    id: ValueNodeId,
    scope: &Rc<Scope>,
    work: &mut Vec<Occurrence>,
    found: &mut BTreeSet<String>,
) -> Option<()>
{
    let node = store.values.get(id)?;
    match *node {
        | ValueNode::Var(ref name) => {
            if !scope.binds(name) {
                found.insert(name.clone());
            }
        },
        | ValueNode::Unit
        | ValueNode::Int(_)
        | ValueNode::Str(_)
        | ValueNode::Num(_)
        | ValueNode::Hole(_) => {},
        | ValueNode::Pair(fst, snd) => {
            work.push(Occurrence::Value(fst, Rc::clone(scope)));
            work.push(Occurrence::Value(snd, Rc::clone(scope)));
        },
        | ValueNode::Inj(_, payload)
        | ValueNode::Here(payload)
        | ValueNode::Ctor { payload, .. } => {
            work.push(Occurrence::Value(payload, Rc::clone(scope)));
        },
        | ValueNode::List(ref elements) => {
            for &element in elements {
                work.push(Occurrence::Value(element, Rc::clone(scope)));
            }
        },
        | ValueNode::Record(ref fields) => {
            for &field in fields.values() {
                work.push(Occurrence::Value(field, Rc::clone(scope)));
            }
        },
        | ValueNode::Thunk(_, body) | ValueNode::Run(body) => {
            work.push(Occurrence::Comp(body, Rc::clone(scope)));
        },
        // An ascription's type, a package's witnesses and a reified stack are
        // places this walk does not look.
        | _ => return None,
    }
    Some(())
}
