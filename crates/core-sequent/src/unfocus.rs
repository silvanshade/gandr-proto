//! The un-focusing readback `𝓕⁻¹` (`proposal-sequent-kernel.md` §7a) — the
//! inverse of the static focusing translation [`crate::focus`].
//!
//! `𝓕` erases the direct-style source syntax into the polarized command IL; a
//! machine value's suspended body therefore no longer carries a source
//! [`gandr_core_checker::syntax::Comp`]. This module reconstructs that source
//! syntax from the focused IL, so:
//!
//! - a higher-order native combinator (`each` / `where` / `reduce` / `any` /
//!   `all` / `update_where`) can un-focus its thunk-closure arguments to source
//!   [`Value`]s and dispatch through the shared `gandr_core_checker::prim`
//!   registry exactly as the CEK oracle does ([`crate::machine`]
//!   `dispatch_native_higher_order`);
//! - a returned thunk / function / lazy-pair / partial-native terminal reads
//!   back to an exact source term ([`unfocus_terminal`]), matching the CEK's
//!   `quote_value` / `read_terminal`, so the differential compares them
//!   structurally rather than at kind granularity;
//! - a higher-order host-effect payload reads back exactly over the public
//!   [`Value`] surface.
//!
//! # Correspondence to `𝓕`
//!
//! [`decode_command`] inverts the [`crate::focus`] rules by dispatching on the
//! IL node shape (self-describing — the constructor and destructor tags fix the
//! source construct), consulting the focusing provenance only for the one hole
//! the IL conflates (a `Comp::Hole` cut vs a returned `Value::Hole`; see
//! [`FocusOrigin::CompHole`]). A cut's producer fixes the head (a value, a
//! codata intro `λ` / lazy pair, a `shift`, a delimiter entry `reset` /
//! `handle`, or an operation `perform`); its consumer spine of destructor
//! frames (`force` / `ap` / `prj` / `.ℓ` / `resume`) reconstructs the
//! elimination context, terminating at the continuation (`ret`), a `μ̃` binder
//! (a `bind`), or a positive `case` (`case` / `split` / `listcase` / `walk` /
//! `datacase`).
//!
//! # What round-trips exactly, and what cannot
//!
//! `𝓕⁻¹ ∘ 𝓕` reproduces the source computation up to the data `𝓕` erases,
//! which the differential's normalizer drops on both sides
//! (`crate::differential`): **type annotations** (`𝓕` focuses through
//! [`Value::Annot`]), an **effect signature's operation list** (`𝓕` keeps only
//! the signature name), a **`Walk` motive** (runtime-erased, ADR-76), and a
//! **declared-data nominal id** (render-only, ADR-80). A **reified stack**
//! (`box(c)` / a captured continuation crossing into value position) stays
//! opaque: the runtime frame representation diverges from the CEK's α-renamed
//! side-table continuation, an un-reconcilable readback residual (§7a) — it
//! reads back as an opaque [`Value::Stk`] carrier so its outcome KIND still
//! agrees.

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::boundary::EffectSignatureName;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::OpClause;
use gandr_core_checker::syntax::Side;
use gandr_core_checker::syntax::Stack;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::syntax::WalkBase;
use gandr_core_checker::syntax::WalkMotive;
use gandr_core_checker::types::CompType;
use gandr_core_checker::types::DataId;
use gandr_core_checker::types::ValueType;

use crate::focus::FocusOrigin;
use crate::il::CoName;
use crate::il::CommandArena;
use crate::il::CommandId;
use crate::il::CommandNode;
use crate::il::ConsumerId;
use crate::il::ConsumerNode;
use crate::il::CtorTag;
use crate::il::DtorTag;
use crate::il::Lit;
use crate::il::PrimOp;
use crate::il::ProducerId;
use crate::il::ProducerNode;
use crate::machine::LEnv;
use crate::machine::LValue;

/// The focusing provenance table — the map the L machine consults to
/// distinguish the two holes `𝓕` conflates (a `Comp::Hole` cut vs a returned
/// `Value::Hole`).
type OriginTable = BTreeMap<CommandId, FocusOrigin>;

/// The continuation a command was focused against — the readback tail sentinel.
///
/// A thunk / arm / lazy-pair body is focused against a fresh covariable
/// consumer, so its tail is that covariable; a `shift` / `reset` / `handle`
/// body is focused against a fresh terminal `★`, so its tail is that `Top`.
#[derive(Clone, Debug)]
enum Tail
{
    /// The tail is the fresh covariable of the enclosing binder.
    ByCoVar(CoName),
    /// The tail is the fresh terminal `★` of a delimited body.
    ByTop,
}

/// One partially-decoded head — a source value being eliminated, or a source
/// computation being continued.
enum Piece
{
    /// A source value (a `ret`-able producer or an elimination scrutinee).
    Value(Value),
    /// A source computation (a codata intro, a delimiter, or an applied head).
    Comp(Comp),
}

impl Piece
{
    /// Views the piece as a source value, or `None` when it is a computation.
    fn into_value(self) -> Option<Value>
    {
        match self {
            | Self::Value(value) => Some(value),
            | Self::Comp(_) => None,
        }
    }

    /// Views the piece as a source computation, coercing a bare value through a
    /// `ret` (a returner is a computation).
    fn into_comp(self) -> Comp
    {
        match self {
            | Self::Comp(comp) => comp,
            | Self::Value(value) => Comp::ret(value),
        }
    }
}

/// Reads a runtime machine value [`LValue`] back to a source [`Value`] — the
/// value-level `𝓕⁻¹`, mirroring the retired CEK oracle's value readback.
///
/// A thunk closure's body is un-focused ([`decode_command`]) then **closed**
/// under the thunk's captured environment (each free environment variable
/// replaced by the readback of the value it denotes, respecting binder
/// shadowing), exactly as the CEK's readback closes a closure body.
///
/// # Contract
/// - ensures: `Some(v)` for a first-order value or a thunk closure whose body
///   decodes; `None` for a value carrier the readback cannot reconstruct (a
///   codata / partial-native / reified-stack in value position, or an operation
///   constructor), which the caller turns into a defined decline.
/// - panics: none.
/// # Termination
/// - reason: the runtime value, its captured environments, and the focused
///   arena are finite acyclic structures (`𝓕` builds children before parents),
///   each node visited once.
/// - measure: the value / arena node the recursion descends into.
/// - boundedness: runtime values and the arena are finite Rust structures.
/// - input recursion: the value spine and its thunk bodies flow into recursive
///   decoding; each strictly descends into a smaller finite structure.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "the readback descends a finite acyclic runtime value / arena; termination is proved above"
    )
)]
#[inline]
#[must_use]
pub fn unfocus_value(
    value: &Rc<LValue>,
    arena: &CommandArena,
    origins: &OriginTable,
) -> Option<Value>
{
    match **value {
        | LValue::Var(ref name) => Some(Value::var(name)),
        | LValue::Lit(ref lit) => Some(lit_value(lit)),
        | LValue::Ctor { ref tag, ref args } => unfocus_ctor(tag, args, arena, origins),
        | LValue::Thunk {
            grade,
            ref cobinder,
            body,
            ref env,
            ..
        } => {
            let decoded = decode_command(arena, origins, body, &Tail::ByCoVar(cobinder.clone()))?;
            let closed = close_comp(decoded, env, arena, origins)?;
            Some(Value::thunk(grade, closed))
        },
        // A reified stack (a captured continuation crossing into value
        // position — e.g. a handler resumption `k` bound in a captured
        // environment) reads back as an opaque carrier: the k-in-value residual
        // (§7a). Declining instead would fail the closing substitution on an
        // *unused* continuation binding — the CEK reads such a binding back as a
        // harmless neutral, so the readback must not choke on it.
        | LValue::Boxed(_) => Some(Value::stk(Stack::Empty)),
        // Codata and a partial native are not source values (they appear only
        // at a terminal, handled by `unfocus_terminal`).
        | LValue::Cocase { .. } | LValue::Native { .. } => None,
    }
}

/// Reads a **terminal** machine value back to an exact source computation — the
/// terminal `𝓕⁻¹`, mirroring the CEK's `read_terminal`.
///
/// A codata terminal reads back to its `λ` / lazy-pair intro (closed under the
/// captured environment); a partial native to `Comp::Native` with its exact
/// accumulated arguments; any other value to `ret v`. A reified-stack terminal
/// reads back as an opaque [`Value::Stk`] carrier (the k-in-value residual):
/// its outcome kind agrees, its structure stays coarse.
///
/// # Contract
/// - ensures: `Some(comp)` when the terminal reconstructs; `None` when its body
///   cannot be decoded (the caller keeps its prior placeholder readback so
///   totality never regresses).
/// - panics: none.
#[inline]
#[must_use]
pub fn unfocus_terminal(
    value: &Rc<LValue>,
    arena: &CommandArena,
    origins: &OriginTable,
) -> Option<Comp>
{
    match **value {
        | LValue::Cocase {
            ref arms, ref env, ..
        } => {
            let intro = decode_cocase(arena, origins, arms)?;
            close_comp(intro, env, arena, origins)
        },
        | LValue::Native { prim, ref args } => {
            let mut values = Vec::with_capacity(args.len());
            for arg in args {
                values.push(Rc::new(unfocus_value(arg, arena, origins)?));
            }
            Some(Comp::Native { prim, args: values })
        },
        // A reified stack crossing into value position stays opaque (the
        // k-in-value / reified-stack residual, §7a): an empty-stack carrier so
        // the outcome KIND (`Stk`) agrees without a divergent structural quote.
        | LValue::Boxed(_) => Some(Comp::ret(Value::stk(Stack::Empty))),
        | _ => Some(Comp::ret(unfocus_value(value, arena, origins)?)),
    }
}

/// Un-focuses a whole **focused program** back to a source computation — the
/// program-level `𝓕⁻¹`, decoding the root command against the terminal `★`.
///
/// This is the inverse of [`crate::focus::focus_comp`] up to the data `𝓕`
/// erases and the administrative redexes `𝓕` commutes, so `𝓕⁻¹ ∘ 𝓕` is a
/// **commuting normal form**: the differential drives it on both readbacks
/// ([`crate::differential`]) to converge the CEK's un-commuted source and the L
/// machine's commuted un-focusing onto the same term.
///
/// # Contract
/// - ensures: `Some(comp)` when the program inverts; `None` for a shape the
///   readback cannot reconstruct (a reified stack in value position).
/// - panics: none.
#[inline]
#[must_use]
pub fn unfocus_comp(focused: &crate::focus::Focused) -> Option<Comp>
{
    decode_command(
        focused.arena(),
        focused.origins(),
        focused.root(),
        &Tail::ByTop,
    )
}

/// Reconstructs a source [`Value`] from a runtime constructor.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends a finite constructor argument spine; termination as unfocus_value"
    )
)]
fn unfocus_ctor(
    tag: &CtorTag,
    args: &[Rc<LValue>],
    arena: &CommandArena,
    origins: &OriginTable,
) -> Option<Value>
{
    match *tag {
        | CtorTag::Pair => {
            let fst = unfocus_value(args.first()?, arena, origins)?;
            let snd = unfocus_value(args.get(1)?, arena, origins)?;
            Some(Value::pair(fst, snd))
        },
        | CtorTag::Inj(side) => {
            let payload = unfocus_value(args.first()?, arena, origins)?;
            Some(Value::Inj(side, Rc::new(payload)))
        },
        | CtorTag::Nil | CtorTag::Cons => {
            let mut elements = Vec::new();
            let mut cursor = args;
            let mut here_tag = tag.clone();
            loop {
                match here_tag {
                    | CtorTag::Nil => break,
                    | CtorTag::Cons => {
                        let head = unfocus_value(cursor.first()?, arena, origins)?;
                        elements.push(head);
                        let LValue::Ctor {
                            tag: ref next_tag,
                            args: ref next_args,
                        } = **cursor.get(1)?
                        else {
                            return None;
                        };
                        here_tag = next_tag.clone();
                        cursor = next_args;
                    },
                    | _ => return None,
                }
            }
            Some(Value::list(elements))
        },
        | CtorTag::Record(ref labels) => {
            let mut fields = Vec::with_capacity(labels.len());
            for (label, arg) in labels.iter().zip(args.iter()) {
                fields.push((label.clone(), unfocus_value(arg, arena, origins)?));
            }
            Some(Value::record(fields))
        },
        | CtorTag::Here => Some(Value::here(unfocus_value(args.first()?, arena, origins)?)),
        | CtorTag::Data(index) => {
            let payload = unfocus_value(args.first()?, arena, origins)?;
            Some(Value::ctor(placeholder_data_id(), index, payload))
        },
        // An operation constructor is a `perform`, never a data value.
        | CtorTag::Op { .. } => None,
    }
}

/// Un-focuses a command against its continuation `tail`, reconstructing the
/// source computation `𝓕⟦comp⟧ tail` came from — the computation-level `𝓕⁻¹`.
///
/// # Contract
/// - ensures: `Some(comp)` when the command inverts; `None` for a malformed or
///   not-yet-invertible shape (a defensive decline, never a panic).
/// - panics: none.
/// # Termination
/// - reason: the focused arena is a finite acyclic DAG (`𝓕` builds children
///   before parents), each command / producer / consumer visited once.
/// - measure: the arena node the recursion descends into.
/// - boundedness: the arena is a finite Rust structure.
/// - input recursion: sub-commands (arm / thunk / delimiter bodies) and value
///   producers flow into recursion; each descends into a strictly smaller arena
///   node.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends the finite acyclic focused arena; termination is proved above"
    )
)]
#[must_use]
fn decode_command(
    arena: &CommandArena,
    origins: &OriginTable,
    cmd: CommandId,
    tail: &Tail,
) -> Option<Comp>
{
    match *arena.command(cmd)? {
        | CommandNode::Cut {
            producer, consumer, ..
        } => decode_cut(arena, origins, cmd, producer, consumer, tail),
        | CommandNode::Prim { op, ref ps, ref cs } => {
            let head = match op {
                | PrimOp::Dup => Comp::dup(decode_value(arena, origins, *ps.first()?)?),
                | PrimOp::Drop => Comp::drop(decode_value(arena, origins, *ps.first()?)?),
                | PrimOp::Native(prim) => {
                    let mut args = Vec::with_capacity(ps.len());
                    for &p in ps {
                        args.push(Rc::new(decode_value(arena, origins, p)?));
                    }
                    Comp::Native { prim, args }
                },
            };
            apply_cont(arena, origins, Piece::Comp(head), *cs.first()?, tail)
        },
        // `𝓕` emits no top-level jump.
        | CommandNode::Jump { .. } => None,
    }
}

/// Decodes a cut `⟨producer | consumer⟩`: reconstruct the head and walk the
/// consumer spine.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends the finite acyclic focused arena; termination as decode_command"
    )
)]
fn decode_cut(
    arena: &CommandArena,
    origins: &OriginTable,
    cmd: CommandId,
    producer: ProducerId,
    consumer: ConsumerId,
    tail: &Tail,
) -> Option<Comp>
{
    let (head, walk_from): (Piece, ConsumerId) = match *arena.producer(producer)? {
        | ProducerNode::Cocase { ref arms } => {
            (Piece::Comp(decode_cocase(arena, origins, arms)?), consumer)
        },
        | ProducerNode::Shift { ref binder, body } => {
            let inner = decode_command(arena, origins, body, &Tail::ByTop)?;
            (Piece::Comp(Comp::shift(binder.as_str(), inner)), consumer)
        },
        | ProducerNode::Mu(_, body) => decode_delimiter(arena, origins, body, consumer)?,
        | ProducerNode::Ctor {
            tag: CtorTag::Op { ref sig, ref op },
            ref ps,
            ..
        } => {
            let payload = decode_value(arena, origins, *ps.first()?)?;
            let head = Comp::perform(
                effect_sig(EffectSignatureName::from(sig.as_str())),
                op.as_str(),
                payload,
            );
            (Piece::Comp(head), consumer)
        },
        | ProducerNode::Lit(Lit::Hole(hole))
            if matches!(origins.get(&cmd), Some(&FocusOrigin::CompHole)) =>
        {
            (Piece::Comp(Comp::hole(hole)), consumer)
        },
        // Any other producer is a value being eliminated / returned.
        | _ => (
            Piece::Value(decode_value(arena, origins, producer)?),
            consumer,
        ),
    };
    apply_cont(arena, origins, head, walk_from, tail)
}

/// Decodes a delimiter-entry cut — a `μ` producer against a `prompt` (`reset`)
/// or a `handler` (`handle`) consumer — returning the reconstructed head and
/// the delimiter's ambient continuation to walk from.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends the finite acyclic focused arena; termination as decode_command"
    )
)]
fn decode_delimiter(
    arena: &CommandArena,
    origins: &OriginTable,
    body: CommandId,
    consumer: ConsumerId,
) -> Option<(Piece, ConsumerId)>
{
    match *arena.consumer(consumer)? {
        | ConsumerNode::Prompt(inner) => {
            let delimited = decode_command(arena, origins, body, &Tail::ByTop)?;
            Some((Piece::Comp(Comp::reset(delimited)), inner))
        },
        | ConsumerNode::Handler(ref handler) => {
            let scrutinee = decode_command(arena, origins, body, &Tail::ByTop)?;
            let ret_body = decode_command(arena, origins, handler.ret_body, &Tail::ByTop)?;
            let mut ops = Vec::with_capacity(handler.ops.len());
            for clause in &handler.ops {
                let clause_body = decode_command(arena, origins, clause.body, &Tail::ByTop)?;
                ops.push(OpClause::new(
                    clause.op.as_str(),
                    clause.payload.as_str(),
                    clause.resume.as_str(),
                    clause_body,
                ));
            }
            let head = Comp::handle(
                effect_sig(EffectSignatureName::from(handler.sig.as_str())),
                scrutinee,
                handler.ret_binder.as_str(),
                ret_body,
                ops,
            );
            Some((Piece::Comp(head), handler.continuation))
        },
        // A `μ` against a non-delimiter consumer is not a shape `𝓕` emits.
        | _ => None,
    }
}

/// Walks a consumer spine, threading the head `current` through its elimination
/// frames until the tail continuation (`ret`), a `μ̃` binder (`bind`), or a
/// positive `case`.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "the spine loop is bounded by the finite consumer chain; the sub-decodes descend the finite arena (termination as decode_command)"
    )
)]
fn apply_cont(
    arena: &CommandArena,
    origins: &OriginTable,
    mut current: Piece,
    mut cont: ConsumerId,
    tail: &Tail,
) -> Option<Comp>
{
    loop {
        if matches!(is_tail(arena, cont, tail), TailPosition::Tail) {
            return Some(current.into_comp());
        }
        match *arena.consumer(cont)? {
            | ConsumerNode::MuTilde(ref binder, body) => {
                let bound = current.into_comp();
                let rest = decode_command(arena, origins, body, tail)?;
                return Some(Comp::bind(bound, binder.as_str(), rest));
            },
            | ConsumerNode::Case { ref arms } => {
                let scrut = current.into_value()?;
                return decode_case(arena, origins, scrut, arms, tail);
            },
            | ConsumerNode::Dtor {
                ref tag,
                ref ps,
                ref cs,
            } => {
                current = apply_frame(arena, origins, current, tag, ps)?;
                cont = *cs.first()?;
            },
            // A bare covariable that is not the tail, a stray `Top`, or a
            // delimiter mid-spine are not shapes `𝓕` emits in a closed body.
            | ConsumerNode::CoVar(_)
            | ConsumerNode::Top
            | ConsumerNode::Handler(_)
            | ConsumerNode::Prompt(_) => return None,
        }
    }
}

/// Applies one destructor frame to the current head.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends the finite acyclic focused arena; termination as decode_command"
    )
)]
fn apply_frame(
    arena: &CommandArena,
    origins: &OriginTable,
    current: Piece,
    tag: &DtorTag,
    ps: &[ProducerId],
) -> Option<Piece>
{
    match *tag {
        | DtorTag::Force => Some(Piece::Comp(Comp::force(current.into_value()?))),
        | DtorTag::RecordProj(ref label) => Some(Piece::Comp(Comp::record_proj(
            current.into_value()?,
            label.as_str(),
        ))),
        | DtorTag::Resume => {
            let stack = current.into_value()?;
            let reified = decode_reified(arena, origins, *ps.first()?)?;
            Some(Piece::Comp(Comp::resume(stack, reified)))
        },
        | DtorTag::Ap => {
            // A bare value meeting an `ap` frame is an applied returner
            // (`App(ret v, arg)` — an ill-typed application `𝓕` still emits);
            // wrap it through `ret` so the elimination re-focuses identically.
            let head = current.into_comp();
            let arg = decode_value(arena, origins, *ps.first()?)?;
            Some(Piece::Comp(Comp::app(head, arg)))
        },
        | DtorTag::Prj(side) => {
            let head = current.into_comp();
            Some(Piece::Comp(match side {
                | Side::Fst => Comp::prj1(head),
                | Side::Snd => Comp::prj2(head),
                | _ => return None,
            }))
        },
    }
}

/// Decodes a codata intro — an `ap` cocase (`λ`) or a `prj` cocase (lazy pair).
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends the finite acyclic focused arena; termination as decode_command"
    )
)]
fn decode_cocase(
    arena: &CommandArena,
    origins: &OriginTable,
    arms: &[crate::il::CoArm],
) -> Option<Comp>
{
    // A single `ap` arm is a `λ`; two `prj` arms are a lazy pair.
    if arms.len() == 1 {
        let arm = arms.first()?;
        if arm.dtor != DtorTag::Ap {
            return None;
        }
        let binder = arm.binders.first()?;
        let cobinder = arm.cobinders.first()?;
        let body = decode_command(arena, origins, arm.body, &Tail::ByCoVar(cobinder.clone()))?;
        return Some(Comp::lam(binder.as_str(), body));
    }
    if arms.len() == 2 {
        let fst = arms.first()?;
        let snd = arms.get(1)?;
        if fst.dtor != DtorTag::Prj(Side::Fst) || snd.dtor != DtorTag::Prj(Side::Snd) {
            return None;
        }
        let fst_cobinder = fst.cobinders.first()?;
        let snd_cobinder = snd.cobinders.first()?;
        let fst_body = decode_command(
            arena,
            origins,
            fst.body,
            &Tail::ByCoVar(fst_cobinder.clone()),
        )?;
        let snd_body = decode_command(
            arena,
            origins,
            snd.body,
            &Tail::ByCoVar(snd_cobinder.clone()),
        )?;
        return Some(Comp::with(fst_body, snd_body));
    }
    None
}

/// Decodes a positive `case` consumer against its scrutinee value — dispatched
/// by the arms' constructor family (`case` / `split` / `listcase` / `walk` /
/// `datacase`).
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends the finite acyclic focused arena; termination as decode_command"
    )
)]
fn decode_case(
    arena: &CommandArena,
    origins: &OriginTable,
    scrut: Value,
    arms: &[crate::il::Arm],
    tail: &Tail,
) -> Option<Comp>
{
    // An empty case is the absurd declared-data match.
    let Some(first) = arms.first()
    else {
        return Some(Comp::data_case(scrut, Vec::new()));
    };
    match first.ctor {
        | CtorTag::Inj(_) => {
            let left = arms.first()?;
            let right = arms.get(1)?;
            let left_body = decode_command(arena, origins, left.body, tail)?;
            let right_body = decode_command(arena, origins, right.body, tail)?;
            Some(Comp::case(
                scrut,
                left.binders.first()?.as_str(),
                left_body,
                right.binders.first()?.as_str(),
                right_body,
            ))
        },
        | CtorTag::Pair => {
            let body = decode_command(arena, origins, first.body, tail)?;
            Some(Comp::split(
                scrut,
                first.binders.first()?.as_str(),
                first.binders.get(1)?.as_str(),
                body,
            ))
        },
        | CtorTag::Nil | CtorTag::Cons => {
            let nil = arms.first()?;
            let cons = arms.get(1)?;
            let nil_body = decode_command(arena, origins, nil.body, tail)?;
            let cons_body = decode_command(arena, origins, cons.body, tail)?;
            Some(Comp::list_case(
                scrut,
                nil_body,
                cons.binders.first()?.as_str(),
                cons.binders.get(1)?.as_str(),
                cons_body,
            ))
        },
        | CtorTag::Here => {
            let body = decode_command(arena, origins, first.body, tail)?;
            Some(Comp::walk(
                scrut,
                placeholder_walk_motive(),
                WalkBase::new(first.binders.first()?.as_str(), body),
            ))
        },
        | CtorTag::Data(_) => {
            let mut decoded = Vec::with_capacity(arms.len());
            for arm in arms {
                let body = decode_command(arena, origins, arm.body, tail)?;
                decoded.push((String::from(arm.binders.first()?.as_str()), body));
            }
            Some(Comp::data_case(scrut, decoded))
        },
        | CtorTag::Record(_) | CtorTag::Op { .. } => None,
    }
}

/// Decodes a `resume`'s reified computation — a thunk-wrapped body against its
/// fresh covariable.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends the finite acyclic focused arena; termination as decode_command"
    )
)]
fn decode_reified(
    arena: &CommandArena,
    origins: &OriginTable,
    producer: ProducerId,
) -> Option<Comp>
{
    let ProducerNode::Thunk {
        ref cobinder, body, ..
    } = *arena.producer(producer)?
    else {
        return None;
    };
    decode_command(arena, origins, body, &Tail::ByCoVar(cobinder.clone()))
}

/// Decodes a value producer to a source [`Value`] (the syntactic `𝓥⁻¹`; free
/// variables stay [`Value::Var`], to be closed by the caller's environment).
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends the finite acyclic focused arena; termination as decode_command"
    )
)]
fn decode_value(
    arena: &CommandArena,
    origins: &OriginTable,
    producer: ProducerId,
) -> Option<Value>
{
    match *arena.producer(producer)? {
        | ProducerNode::Var(ref name) => Some(Value::var(name)),
        | ProducerNode::Lit(ref lit) => Some(lit_value(lit)),
        | ProducerNode::Thunk {
            grade,
            ref cobinder,
            body,
        } => {
            let inner = decode_command(arena, origins, body, &Tail::ByCoVar(cobinder.clone()))?;
            Some(Value::thunk(grade, inner))
        },
        | ProducerNode::Ctor {
            ref tag, ref ps, ..
        } => decode_ctor_value(arena, origins, tag, ps),
        // A codata / shift / delimiter producer is a computation, not a value.
        | ProducerNode::Cocase { .. }
        | ProducerNode::Shift { .. }
        | ProducerNode::Mu(..)
        // A boxed consumer (`stk K`) in value position is the reified-stack
        // residual — decline (the caller keeps it opaque).
        | ProducerNode::Boxed(_) => None,
    }
}

/// Reconstructs a source [`Value`] from a constructor producer.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends the finite acyclic focused arena; termination as decode_command"
    )
)]
fn decode_ctor_value(
    arena: &CommandArena,
    origins: &OriginTable,
    tag: &CtorTag,
    ps: &[ProducerId],
) -> Option<Value>
{
    match *tag {
        | CtorTag::Pair => {
            let fst = decode_value(arena, origins, *ps.first()?)?;
            let snd = decode_value(arena, origins, *ps.get(1)?)?;
            Some(Value::pair(fst, snd))
        },
        | CtorTag::Inj(side) => {
            let payload = decode_value(arena, origins, *ps.first()?)?;
            Some(Value::Inj(side, Rc::new(payload)))
        },
        | CtorTag::Nil => Some(Value::list(Vec::new())),
        | CtorTag::Cons => {
            let mut elements = Vec::new();
            let mut current_ps: &[ProducerId] = ps;
            loop {
                let head = decode_value(arena, origins, *current_ps.first()?)?;
                elements.push(head);
                let tail_producer = *current_ps.get(1)?;
                match *arena.producer(tail_producer)? {
                    | ProducerNode::Ctor {
                        tag: CtorTag::Nil, ..
                    } => break,
                    | ProducerNode::Ctor {
                        tag: CtorTag::Cons,
                        ps: ref next_ps,
                        ..
                    } => current_ps = next_ps,
                    | _ => return None,
                }
            }
            Some(Value::list(elements))
        },
        | CtorTag::Record(ref labels) => {
            let mut fields = Vec::with_capacity(labels.len());
            for (label, &p) in labels.iter().zip(ps.iter()) {
                fields.push((label.clone(), decode_value(arena, origins, p)?));
            }
            Some(Value::record(fields))
        },
        | CtorTag::Here => Some(Value::here(decode_value(arena, origins, *ps.first()?)?)),
        | CtorTag::Data(index) => {
            let payload = decode_value(arena, origins, *ps.first()?)?;
            Some(Value::ctor(placeholder_data_id(), index, payload))
        },
        // An operation constructor is a `perform`, never a value.
        | CtorTag::Op { .. } => None,
    }
}

/// Closes a decoded computation under a runtime environment: each free
/// environment variable is replaced by the readback of the value it denotes,
/// innermost binding first, respecting binder shadowing — mirroring the CEK's
/// `close_comp`.
///
/// # Termination
/// - reason: a finite environment folds a finite number of substitutions over a
///   finite computation.
/// - measure: the remaining environment bindings.
/// - boundedness: the environment is a finite Rust structure.
/// - input recursion: each binding's readback descends a smaller runtime value
///   (termination as `unfocus_value`).
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "folds substitutions over a finite environment; readback termination as unfocus_value"
    )
)]
#[must_use]
fn close_comp(
    comp: Comp,
    env: &LEnv,
    arena: &CommandArena,
    origins: &OriginTable,
) -> Option<Comp>
{
    let mut result = comp;
    for (name, bound) in env.bindings() {
        let replacement = unfocus_value(&bound, arena, origins)?;
        result = subst_comp(&result, SubstitutionName(name.as_str()), &replacement);
    }
    Some(result)
}

/// Classification of a consumer against the active readback tail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TailPosition
{
    /// The consumer is the active tail.
    Tail,
    /// The consumer is an interior continuation frame.
    Interior,
}

/// Whether a consumer is the readback tail (the enclosing binder's covariable,
/// or a delimited body's terminal `★`).
#[must_use]
fn is_tail(
    arena: &CommandArena,
    consumer: ConsumerId,
    tail: &Tail,
) -> TailPosition
{
    let Some(node) = arena.consumer(consumer)
    else {
        return TailPosition::Interior;
    };
    let matches_tail = match *tail {
        | Tail::ByCoVar(ref target) => {
            matches!(*node, ConsumerNode::CoVar(ref name) if name == target)
        },
        | Tail::ByTop => matches!(*node, ConsumerNode::Top),
    };
    if matches_tail {
        TailPosition::Tail
    }
    else {
        TailPosition::Interior
    }
}

/// Reads a positive scalar leaf back to a source [`Value`].
#[must_use]
fn lit_value(lit: &Lit) -> Value
{
    match *lit {
        | Lit::Unit => Value::Unit,
        | Lit::Int(value) => Value::int(value),
        | Lit::Str(ref value) => Value::string(value),
        | Lit::Num(num) => Value::Num(num),
        | Lit::Hole(hole) => Value::hole(hole),
    }
}

/// Builds a bare effect signature carrying only its name — the operation list
/// `𝓕` erased (the differential normalizer drops it on the oracle side too).
#[must_use]
fn effect_sig(name: EffectSignatureName<'_>) -> EffectSig
{
    EffectSig::new(name, Vec::new())
}

/// A render-only placeholder declared-data id — `𝓕` erases the nominal id
/// (ADR-80), so the readback carries a canonical stand-in.
#[must_use]
fn placeholder_data_id() -> DataId
{
    DataId::new(0_u64, "")
}

/// A runtime-erased placeholder Walk motive — `𝓕` drops the motive (ADR-76), so
/// the readback carries a canonical stand-in the normalizer also drops.
#[must_use]
fn placeholder_walk_motive() -> WalkMotive
{
    WalkMotive::new("x", "y", "q", CompType::returner(ValueType::Unknown))
}

/// Borrowed source variable selected for closing substitution.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct SubstitutionName<'name>(&'name str);

/// Capture-avoiding substitution of a **closed** value for the free occurrences
/// of `name` in a computation, respecting binder shadowing (the closing
/// substitution's engine; the replacement is closed in a well-typed program, so
/// no renaming is needed).
///
/// # Termination
/// - reason: descends a finite source computation, stopping at any binder that
///   rebinds `name`; each node is visited once.
/// - measure: the source computation node the recursion descends into.
/// - boundedness: source computations are finite `Rc` trees.
/// - input recursion: sub-computations and sub-values flow into recursion; each
///   descends into a strictly smaller node.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends a finite source computation, stopping at rebinding; termination is proved above"
    )
)]
#[must_use]
fn subst_comp(
    comp: &Comp,
    name: SubstitutionName<'_>,
    repl: &Value,
) -> Comp
{
    match *comp {
        | Comp::Abs(ref binder, ref ty, ref body) => {
            let inner = if binder == name.0 {
                body.as_ref().clone()
            }
            else {
                subst_comp(body, name, repl)
            };
            Comp::Abs(binder.clone(), ty.clone(), Rc::new(inner))
        },
        | Comp::App(ref head, ref arg) => Comp::App(
            Rc::new(subst_comp(head, name, repl)),
            Rc::new(subst_value(arg, name, repl)),
        ),
        | Comp::Ret(ref value) => Comp::Ret(Rc::new(subst_value(value, name, repl))),
        | Comp::Bind(ref bound, ref binder, ref cont) => {
            let bound = subst_comp(bound, name, repl);
            let cont = if binder == name.0 {
                cont.as_ref().clone()
            }
            else {
                subst_comp(cont, name, repl)
            };
            Comp::Bind(Rc::new(bound), binder.clone(), Rc::new(cont))
        },
        | Comp::Force(ref value) => Comp::Force(Rc::new(subst_value(value, name, repl))),
        | Comp::Case(ref scrut, (ref lb, ref lbody), (ref rb, ref rbody)) => {
            let scrut = subst_value(scrut, name, repl);
            let lbody = subst_under(SubstitutionName(lb.as_str()), lbody, name, repl);
            let rbody = subst_under(SubstitutionName(rb.as_str()), rbody, name, repl);
            Comp::Case(
                Rc::new(scrut),
                (lb.clone(), Rc::new(lbody)),
                (rb.clone(), Rc::new(rbody)),
            )
        },
        | Comp::DataCase(ref scrut, ref arms) => {
            let scrut = subst_value(scrut, name, repl);
            let arms = arms
                .iter()
                .map(|arm| {
                    (
                        arm.0.clone(),
                        Rc::new(subst_under(
                            SubstitutionName(arm.0.as_str()),
                            &arm.1,
                            name,
                            repl,
                        )),
                    )
                })
                .collect();
            Comp::DataCase(Rc::new(scrut), arms)
        },
        | Comp::ListCase {
            ref scrut,
            ref nil,
            ref head,
            ref tail,
            ref cons,
        } => {
            let scrut = subst_value(scrut, name, repl);
            let nil = subst_comp(nil, name, repl);
            // The cons arm binds both `head` and `tail`; either rebinding
            // shields the substitution below it.
            let cons = if head == name.0 || tail == name.0 {
                cons.as_ref().clone()
            }
            else {
                subst_comp(cons, name, repl)
            };
            Comp::ListCase {
                scrut: Rc::new(scrut),
                nil: Rc::new(nil),
                head: head.clone(),
                tail: tail.clone(),
                cons: Rc::new(cons),
            }
        },
        | Comp::Split {
            ref scrut,
            ref fst_name,
            ref snd_name,
            ref motive,
            ref body,
        } => {
            let scrut = subst_value(scrut, name, repl);
            let body = if fst_name == name.0 || snd_name == name.0 {
                body.as_ref().clone()
            }
            else {
                subst_comp(body, name, repl)
            };
            Comp::Split {
                scrut: Rc::new(scrut),
                fst_name: fst_name.clone(),
                snd_name: snd_name.clone(),
                motive: motive.clone(),
                body: Rc::new(body),
            }
        },
        | Comp::RecordProj {
            ref record,
            ref label,
        } => Comp::RecordProj {
            record: Rc::new(subst_value(record, name, repl)),
            label: label.clone(),
        },
        | Comp::With(ref fst, ref snd) => Comp::With(
            Rc::new(subst_comp(fst, name, repl)),
            Rc::new(subst_comp(snd, name, repl)),
        ),
        | Comp::Prj(side, ref inner) => Comp::Prj(side, Rc::new(subst_comp(inner, name, repl))),
        | Comp::Dup(ref value) => Comp::Dup(Rc::new(subst_value(value, name, repl))),
        | Comp::Drop(ref value) => Comp::Drop(Rc::new(subst_value(value, name, repl))),
        | Comp::Perform(ref sig, ref op, ref payload) => Comp::Perform(
            sig.clone(),
            op.clone(),
            Rc::new(subst_value(payload, name, repl)),
        ),
        | Comp::Handle {
            ref sig,
            ref scrutinee,
            ret: (ref ret_binder, ref ret_body),
            ref ops,
        } => {
            let scrutinee = subst_comp(scrutinee, name, repl);
            let ret_body = subst_under(SubstitutionName(ret_binder.as_str()), ret_body, name, repl);
            let ops = ops
                .iter()
                .map(|clause| {
                    // A clause binds the payload and the resumption; either
                    // rebinding shields the substitution in the clause body.
                    let body = if clause.payload == name.0 || clause.resume == name.0 {
                        clause.body.as_ref().clone()
                    }
                    else {
                        subst_comp(&clause.body, name, repl)
                    };
                    OpClause::new(
                        clause.op.as_str(),
                        clause.payload.as_str(),
                        clause.resume.as_str(),
                        body,
                    )
                })
                .collect();
            Comp::Handle {
                sig: sig.clone(),
                scrutinee: Rc::new(scrutinee),
                ret: (ret_binder.clone(), Rc::new(ret_body)),
                ops,
            }
        },
        | Comp::Resume(ref stack, ref comp) => Comp::Resume(
            Rc::new(subst_value(stack, name, repl)),
            Rc::new(subst_comp(comp, name, repl)),
        ),
        | Comp::Reset(ref inner) => Comp::Reset(Rc::new(subst_comp(inner, name, repl))),
        | Comp::Shift(ref binder, ref body) => {
            let body = subst_under(SubstitutionName(binder.as_str()), body, name, repl);
            Comp::Shift(binder.clone(), Rc::new(body))
        },
        | Comp::Hole(hole) => Comp::Hole(hole),
        | Comp::Native { prim, ref args } => Comp::Native {
            prim,
            args: args
                .iter()
                .map(|arg| Rc::new(subst_value(arg, name, repl)))
                .collect(),
        },
        | Comp::Walk {
            ref scrut,
            ref motive,
            ref base,
        } => {
            let scrut = subst_value(scrut, name, repl);
            let base_body = if base.x == name.0 {
                base.body.as_ref().clone()
            }
            else {
                subst_comp(&base.body, name, repl)
            };
            Comp::Walk {
                scrut: Rc::new(scrut),
                motive: motive.clone(),
                base: WalkBase::new(base.x.as_str(), base_body),
            }
        },
        // A future former the readback never constructs passes through.
        | _ => comp.clone(),
    }
}

/// Substitutes into a body guarded by a single binder (the substitution is
/// shielded when the binder rebinds `name`).
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "delegates to subst_comp on a strictly smaller node; termination as subst_comp"
    )
)]
#[must_use]
fn subst_under(
    binder: SubstitutionName<'_>,
    body: &Comp,
    name: SubstitutionName<'_>,
    repl: &Value,
) -> Comp
{
    if binder.0 == name.0 {
        body.clone()
    }
    else {
        subst_comp(body, name, repl)
    }
}

/// Capture-avoiding substitution of a closed value into a source value.
///
/// # Termination
/// - reason: descends a finite source value; each node is visited once.
/// - measure: the source value node the recursion descends into.
/// - boundedness: source values are finite `Rc` trees.
/// - input recursion: sub-values and thunk bodies flow into recursion; each
///   descends into a strictly smaller node.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends a finite source value, stopping at rebinding; termination is proved above"
    )
)]
#[must_use]
fn subst_value(
    value: &Value,
    name: SubstitutionName<'_>,
    repl: &Value,
) -> Value
{
    match *value {
        | Value::Var(ref var) => {
            if var == name.0 {
                repl.clone()
            }
            else {
                Value::Var(var.clone())
            }
        },
        | Value::Pair(ref fst, ref snd) => Value::Pair(
            Rc::new(subst_value(fst, name, repl)),
            Rc::new(subst_value(snd, name, repl)),
        ),
        | Value::Inj(side, ref payload) => {
            Value::Inj(side, Rc::new(subst_value(payload, name, repl)))
        },
        | Value::List(ref elements) => Value::List(
            elements
                .iter()
                .map(|element| Rc::new(subst_value(element, name, repl)))
                .collect(),
        ),
        | Value::Record(ref fields) => Value::Record(
            fields
                .iter()
                .map(|(label, field)| (label.clone(), Rc::new(subst_value(field, name, repl))))
                .collect(),
        ),
        | Value::Thunk(grade, ref body) => {
            Value::Thunk(grade, Rc::new(subst_comp(body, name, repl)))
        },
        | Value::Annot(ref inner, ref ty) => {
            Value::Annot(Rc::new(subst_value(inner, name, repl)), Rc::clone(ty))
        },
        | Value::Here(ref witness) => Value::Here(Rc::new(subst_value(witness, name, repl))),
        | Value::Ctor {
            ref id,
            tag,
            ref payload,
        } => Value::Ctor {
            id: id.clone(),
            tag,
            payload: Rc::new(subst_value(payload, name, repl)),
        },
        // Scalars, holes, reified stacks, and any future value former carry no
        // free term variable the closing substitution reaches.
        | _ => value.clone(),
    }
}
