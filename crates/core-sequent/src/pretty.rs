//! The command-IL pretty-printer (the sequent-machines design's §9 inspection
//! surface).
//!
//! Renders a command, producer, or consumer in the §2.1 concrete notation
//! (`⟨p |ε c⟩`, `μα.s`, `μ̃x.s`, `K(p̄; c̄)`, `cocase { … }`, `case { … }`, `★`,
//! …) so a focused term is dumpable and debuggable and, once L1 lands, corpus
//! goldens can pin the rendered seam. Rendering is depth-bounded (`…` beyond
//! the ceiling), the ADR-47 bounded-rendering posture the corpus harness also
//! uses, so it is total and its output size is bounded regardless of the term.

use alloc::format;
use alloc::string::String;
use alloc::string::ToString as _;
use alloc::vec::Vec;

use gandr_core_term::syntax::NumLit;
use gandr_core_term::syntax::Side;

use crate::boundary::RenderDepth;
use crate::boundary::RenderToken;
use crate::il::CommandArena;
use crate::il::CommandId;
use crate::il::CommandNode;
use crate::il::ConsumerId;
use crate::il::ConsumerNode;
use crate::il::CtorTag;
use crate::il::DtorTag;
use crate::il::Lit;
use crate::il::Polarity;
use crate::il::PrimOp;
use crate::il::ProducerId;
use crate::il::ProducerNode;

/// The rendering depth ceiling; beyond it a node renders as `…`.
const RENDER_DEPTH_LIMIT: u32 = 64;

/// The configured render-depth ceiling as a semantic wrapper.
fn render_depth_limit() -> RenderDepth
{
    RENDER_DEPTH_LIMIT.into()
}

/// Renders a command in the §2.1 notation.
///
/// # Contract
/// - ensures: total (never panics); depth-bounded output (`…` past the
///   ceiling); an unresolved child id renders as `<dangling>` rather than
///   aborting.
#[inline]
#[must_use]
pub fn render_command(
    arena: &CommandArena,
    command: CommandId,
) -> String
{
    Printer { arena }.command(command, RenderDepth::ROOT)
}

/// Renders a producer in the §2.1 notation.
///
/// # Contract
/// - ensures: total and depth-bounded, as [`render_command`].
#[inline]
#[must_use]
pub fn render_producer(
    arena: &CommandArena,
    producer: ProducerId,
) -> String
{
    Printer { arena }.producer(producer, RenderDepth::ROOT)
}

/// Renders a consumer in the §2.1 notation.
///
/// # Contract
/// - ensures: total and depth-bounded, as [`render_command`].
#[inline]
#[must_use]
pub fn render_consumer(
    arena: &CommandArena,
    consumer: ConsumerId,
) -> String
{
    Printer { arena }.consumer(consumer, RenderDepth::ROOT)
}

/// The renderer, borrowing the arena for the duration.
#[repr(transparent)]
struct Printer<'arena>
{
    /// The arena whose nodes are rendered.
    arena: &'arena CommandArena,
}

impl Printer<'_>
{
    /// Renders a command.
    fn command(
        &self,
        id: CommandId,
        depth: RenderDepth,
    ) -> String
    {
        self.render(RenderNode::Command { id, depth })
    }

    /// Renders a producer.
    fn producer(
        &self,
        id: ProducerId,
        depth: RenderDepth,
    ) -> String
    {
        self.render(RenderNode::Producer { id, depth })
    }

    /// Renders a consumer.
    fn consumer(
        &self,
        id: ConsumerId,
        depth: RenderDepth,
    ) -> String
    {
        self.render(RenderNode::Consumer { id, depth })
    }

    /// Renders command-IL nodes with an explicit stack of pending formatting
    /// frames, keeping cyclic or deep caller input off the Rust call stack.
    fn render(
        &self,
        initial: RenderNode,
    ) -> String
    {
        struct ArmRender
        {
            head: String,
            binders: String,
        }

        struct HandlerRender
        {
            sig: String,
            ret_binder: String,
            ops: Vec<(String, String, String)>,
        }

        enum Build
        {
            Cut
            {
                pol: Polarity,
            },
            Call
            {
                head: String,
            },
            Wrap
            {
                prefix: String,
                suffix: &'static str,
            },
            Arguments
            {
                producer_count: usize,
                consumer_count: usize,
            },
            Arms(Vec<ArmRender>),
            CoArms(Vec<ArmRender>),
            Handler(HandlerRender),
            Prompt,
        }

        enum Task
        {
            Node(RenderNode),
            Build(Build),
        }

        let push_arguments =
            |ps: &[ProducerId], cs: &[ConsumerId], depth: RenderDepth, work: &mut Vec<Task>| {
                work.push(Task::Build(Build::Arguments {
                    producer_count: ps.len(),
                    consumer_count: cs.len(),
                }));
                for &consumer in cs.iter().rev() {
                    work.push(Task::Node(RenderNode::Consumer {
                        id: consumer,
                        depth,
                    }));
                }
                for &producer in ps.iter().rev() {
                    work.push(Task::Node(RenderNode::Producer {
                        id: producer,
                        depth,
                    }));
                }
            };
        let render_arms = |metas: Vec<ArmRender>, rendered: &mut Vec<String>| -> String {
            let mut bodies = Vec::with_capacity(metas.len());
            for _ in 0 .. metas.len() {
                bodies.push(pop_rendered(rendered));
            }
            bodies.reverse();
            metas
                .into_iter()
                .zip(bodies)
                .map(|(meta, body)| format!("{}({}) ⇒ {}", meta.head, meta.binders, body))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut work = alloc::vec![Task::Node(initial)];
        let mut rendered: Vec<String> = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                | Task::Node(node) => match node {
                    | RenderNode::Command { id, depth } => {
                        if bool::from(depth.reached(render_depth_limit())) {
                            rendered.push(String::from("…"));
                            continue;
                        }
                        let below = depth.below();
                        let Some(command) = self.arena.command(id)
                        else {
                            rendered.push(String::from("<dangling>"));
                            continue;
                        };
                        match *command {
                            | CommandNode::Cut {
                                pol,
                                producer,
                                consumer,
                            } => {
                                work.push(Task::Build(Build::Cut { pol }));
                                work.push(Task::Node(RenderNode::Consumer {
                                    id: consumer,
                                    depth: below,
                                }));
                                work.push(Task::Node(RenderNode::Producer {
                                    id: producer,
                                    depth: below,
                                }));
                            },
                            | CommandNode::Prim { op, ref ps, ref cs } => {
                                work.push(Task::Build(Build::Call { head: prim_op(op) }));
                                push_arguments(ps, cs, below, &mut work);
                            },
                            | CommandNode::Jump {
                                ref def,
                                ref ps,
                                ref cs,
                            } => {
                                work.push(Task::Build(Build::Call { head: def.clone() }));
                                push_arguments(ps, cs, below, &mut work);
                            },
                        }
                    },
                    | RenderNode::Producer { id, depth } => {
                        if bool::from(depth.reached(render_depth_limit())) {
                            rendered.push(String::from("…"));
                            continue;
                        }
                        let below = depth.below();
                        let Some(producer) = self.arena.producer(id)
                        else {
                            rendered.push(String::from("<dangling>"));
                            continue;
                        };
                        match *producer {
                            | ProducerNode::Var(ref name) => rendered.push(name.clone()),
                            | ProducerNode::Lit(ref lit) => rendered.push(literal(lit)),
                            | ProducerNode::Mu(ref covar, body) => {
                                work.push(Task::Build(Build::Wrap {
                                    prefix: format!("μ{covar}. "),
                                    suffix: "",
                                }));
                                work.push(Task::Node(RenderNode::Command {
                                    id: body,
                                    depth: below,
                                }));
                            },
                            | ProducerNode::Ctor {
                                ref tag,
                                ref ps,
                                ref cs,
                            } => {
                                work.push(Task::Build(Build::Call {
                                    head: ctor_tag(tag),
                                }));
                                push_arguments(ps, cs, below, &mut work);
                            },
                            | ProducerNode::Cocase { ref arms } => {
                                let metas = arms
                                    .iter()
                                    .map(|arm| ArmRender {
                                        head: dtor_tag(&arm.dtor),
                                        binders: binders(&arm.binders, &arm.cobinders),
                                    })
                                    .collect::<Vec<_>>();
                                work.push(Task::Build(Build::CoArms(metas)));
                                for arm in arms.iter().rev() {
                                    work.push(Task::Node(RenderNode::Command {
                                        id: arm.body,
                                        depth: below,
                                    }));
                                }
                            },
                            | ProducerNode::Thunk {
                                grade,
                                ref cobinder,
                                body,
                            } => {
                                work.push(Task::Build(Build::Wrap {
                                    prefix: format!("thunk_{grade:?} {{ force({cobinder}) ⇒ "),
                                    suffix: " }",
                                }));
                                work.push(Task::Node(RenderNode::Command {
                                    id: body,
                                    depth: below,
                                }));
                            },
                            | ProducerNode::Boxed(consumer) => {
                                work.push(Task::Build(Build::Wrap {
                                    prefix: String::from("box("),
                                    suffix: ")",
                                }));
                                work.push(Task::Node(RenderNode::Consumer {
                                    id: consumer,
                                    depth: below,
                                }));
                            },
                            | ProducerNode::Shift { ref binder, body } => {
                                work.push(Task::Build(Build::Wrap {
                                    prefix: format!("shift {binder}. "),
                                    suffix: "",
                                }));
                                work.push(Task::Node(RenderNode::Command {
                                    id: body,
                                    depth: below,
                                }));
                            },
                        }
                    },
                    | RenderNode::Consumer { id, depth } => {
                        if bool::from(depth.reached(render_depth_limit())) {
                            rendered.push(String::from("…"));
                            continue;
                        }
                        let below = depth.below();
                        let Some(consumer) = self.arena.consumer(id)
                        else {
                            rendered.push(String::from("<dangling>"));
                            continue;
                        };
                        match *consumer {
                            | ConsumerNode::CoVar(ref name) => rendered.push(name.clone()),
                            | ConsumerNode::MuTilde(ref binder, body) => {
                                work.push(Task::Build(Build::Wrap {
                                    prefix: format!("μ̃{binder}. "),
                                    suffix: "",
                                }));
                                work.push(Task::Node(RenderNode::Command {
                                    id: body,
                                    depth: below,
                                }));
                            },
                            | ConsumerNode::Dtor {
                                ref tag,
                                ref ps,
                                ref cs,
                            } => {
                                work.push(Task::Build(Build::Call {
                                    head: dtor_tag(tag),
                                }));
                                push_arguments(ps, cs, below, &mut work);
                            },
                            | ConsumerNode::Case { ref arms } => {
                                let metas = arms
                                    .iter()
                                    .map(|arm| ArmRender {
                                        head: ctor_tag(&arm.ctor),
                                        binders: binders(&arm.binders, &arm.cobinders),
                                    })
                                    .collect::<Vec<_>>();
                                work.push(Task::Build(Build::Arms(metas)));
                                for arm in arms.iter().rev() {
                                    work.push(Task::Node(RenderNode::Command {
                                        id: arm.body,
                                        depth: below,
                                    }));
                                }
                            },
                            | ConsumerNode::Top => rendered.push(String::from("★")),
                            | ConsumerNode::Handler(ref handler) => {
                                let meta = HandlerRender {
                                    sig: handler.sig.clone(),
                                    ret_binder: handler.ret_binder.clone(),
                                    ops: handler
                                        .ops
                                        .iter()
                                        .map(|clause| {
                                            (
                                                clause.op.clone(),
                                                clause.payload.clone(),
                                                clause.resume.clone(),
                                            )
                                        })
                                        .collect(),
                                };
                                work.push(Task::Build(Build::Handler(meta)));
                                work.push(Task::Node(RenderNode::Consumer {
                                    id: handler.continuation,
                                    depth: below,
                                }));
                                for clause in handler.ops.iter().rev() {
                                    work.push(Task::Node(RenderNode::Command {
                                        id: clause.body,
                                        depth: below,
                                    }));
                                }
                                work.push(Task::Node(RenderNode::Command {
                                    id: handler.ret_body,
                                    depth: below,
                                }));
                            },
                            | ConsumerNode::Prompt(inner) => {
                                work.push(Task::Build(Build::Prompt));
                                work.push(Task::Node(RenderNode::Consumer {
                                    id: inner,
                                    depth: below,
                                }));
                            },
                        }
                    },
                },
                | Task::Build(build) => match build {
                    | Build::Cut { pol } => {
                        let consumer = pop_rendered(&mut rendered);
                        let producer = pop_rendered(&mut rendered);
                        rendered.push(format!("⟨{} |{} {}⟩", producer, polarity(pol), consumer));
                    },
                    | Build::Call { head } => {
                        let args = pop_rendered(&mut rendered);
                        rendered.push(format!("{head}({args})"));
                    },
                    | Build::Wrap { prefix, suffix } => {
                        let body = pop_rendered(&mut rendered);
                        rendered.push(format!("{prefix}{body}{suffix}"));
                    },
                    | Build::Arguments {
                        producer_count,
                        consumer_count,
                    } => {
                        let mut consumers = Vec::with_capacity(consumer_count);
                        for _ in 0 .. consumer_count {
                            consumers.push(pop_rendered(&mut rendered));
                        }
                        consumers.reverse();
                        let mut producers = Vec::with_capacity(producer_count);
                        for _ in 0 .. producer_count {
                            producers.push(pop_rendered(&mut rendered));
                        }
                        producers.reverse();
                        rendered.push(format!(
                            "{}; {}",
                            producers.join(", "),
                            consumers.join(", ")
                        ));
                    },
                    | Build::Arms(metas) => {
                        let arms = render_arms(metas, &mut rendered);
                        rendered.push(arms);
                    },
                    | Build::CoArms(metas) => {
                        let arms = render_arms(metas, &mut rendered);
                        rendered.push(format!("cocase {{ {arms} }}"));
                    },
                    | Build::Handler(meta) => {
                        let continuation = pop_rendered(&mut rendered);
                        let mut op_bodies = Vec::with_capacity(meta.ops.len());
                        for _ in 0 .. meta.ops.len() {
                            op_bodies.push(pop_rendered(&mut rendered));
                        }
                        op_bodies.reverse();
                        let ret_body = pop_rendered(&mut rendered);
                        let mut parts = Vec::with_capacity(meta.ops.len().saturating_add(1));
                        parts.push(format!("return({}) ⇒ {}", meta.ret_binder, ret_body));
                        for ((op, payload, resume), body) in meta.ops.into_iter().zip(op_bodies) {
                            parts.push(format!("{op}({payload}; {resume}) ⇒ {body}"));
                        }
                        rendered.push(format!(
                            "handler[{}] {{ {} }}({})",
                            meta.sig,
                            parts.join(", "),
                            continuation
                        ));
                    },
                    | Build::Prompt => {
                        let inner = pop_rendered(&mut rendered);
                        rendered.push(format!("prompt({inner})"));
                    },
                },
            }
        }
        let result = rendered.pop();
        debug_assert!(
            result.is_some(),
            "renderer produces one string for one root"
        );
        result.unwrap_or_default()
    }
}

/// Pops a rendered fragment off the render stack.
///
/// # Contract
/// - requires: the render task machine pushed a fragment for the current build
///   task.
/// - ensures: returns that fragment.
/// - panics: none in release builds (a `debug_assert!` guards the render-stack
///   discipline in test / debug builds; a desync falls back to the empty
///   fragment — never reached, the snapshot tests would fail first).
fn pop_rendered(rendered: &mut Vec<String>) -> String
{
    let fragment = rendered.pop();
    debug_assert!(fragment.is_some(), "render stack underflowed");
    fragment.unwrap_or_default()
}

/// One renderer work-stack node reference: an arena id plus the remaining
/// render depth at that position.
#[derive(Clone, Copy)]
enum RenderNode
{
    /// A command node to render.
    Command
    {
        /// The command id.
        id: CommandId,
        /// The remaining render depth.
        depth: RenderDepth,
    },
    /// A producer node to render.
    Producer
    {
        /// The producer id.
        id: ProducerId,
        /// The remaining render depth.
        depth: RenderDepth,
    },
    /// A consumer node to render.
    Consumer
    {
        /// The consumer id.
        id: ConsumerId,
        /// The remaining render depth.
        depth: RenderDepth,
    },
}

/// Renders a cut polarity marker.
fn polarity(pol: Polarity) -> RenderToken<'static>
{
    match pol {
        | Polarity::Positive => "+".into(),
        | Polarity::Negative => "−".into(),
    }
}

/// Renders a binder list `x̄; ᾱ`.
fn binders(
    vars: &[String],
    covars: &[String],
) -> String
{
    format!("{}; {}", vars.join(", "), covars.join(", "))
}

/// Renders a positive scalar leaf.
fn literal(lit: &Lit) -> String
{
    match *lit {
        | Lit::Unit => String::from("()"),
        | Lit::Int(value) => value.to_string(),
        | Lit::Str(ref value) => format!("\"{value}\""),
        | Lit::Num(num) => num_lit(num),
        | Lit::Hole(hole) => format!("?{hole}"),
    }
}

/// Renders a typed numeric literal without a `Debug` widening.
fn num_lit(num: NumLit) -> String
{
    match num {
        | NumLit::U32(value) => format!("{value}u32"),
        | NumLit::U64(value) => format!("{value}u64"),
        | NumLit::I32(value) => format!("{value}i32"),
        | NumLit::I64(value) => format!("{value}i64"),
        | NumLit::F32(bits) => format!("{}f32", f32::from_bits(bits)),
        | NumLit::F64(bits) => format!("{}f64", f64::from_bits(bits)),
    }
}

/// Renders a constructor tag head.
fn ctor_tag(tag: &CtorTag) -> String
{
    match *tag {
        | CtorTag::Pair => String::from("Pair"),
        | CtorTag::Inj(side) => side_inj(side).to_string(),
        | CtorTag::Nil => String::from("Nil"),
        | CtorTag::Cons => String::from("Cons"),
        | CtorTag::Record(ref labels) => format!("Record{{{}}}", labels.join(", ")),
        | CtorTag::Op { ref sig, ref op } => {
            format!("Op[{}.{}]", sig.name().as_ref(), op)
        },
        | CtorTag::Here => String::from("Here"),
        | CtorTag::Data(tag) => format!("Data[{tag}]"),
    }
}

/// Renders a side selector's injection spelling.
fn side_inj(side: Side) -> RenderToken<'static>
{
    match side {
        | Side::Fst => "Inl".into(),
        | Side::Snd => "Inr".into(),
    }
}

/// Renders a destructor tag head.
fn dtor_tag(tag: &DtorTag) -> String
{
    match *tag {
        | DtorTag::Ap => String::from("ap"),
        | DtorTag::Force => String::from("force"),
        | DtorTag::Prj(side) => format!("prj{}", side_prj(side)),
        | DtorTag::RecordProj(ref label) => format!(".{label}"),
        | DtorTag::Resume => String::from("resume"),
    }
}

/// Renders a side selector's projection index.
fn side_prj(side: Side) -> RenderToken<'static>
{
    match side {
        | Side::Fst => "1".into(),
        | Side::Snd => "2".into(),
    }
}

/// Renders a primitive-operation head.
fn prim_op(op: PrimOp) -> String
{
    match op {
        | PrimOp::Native(prim) => format!("native[{}]", native_name(prim)),
        | PrimOp::Dup => String::from("dup"),
        | PrimOp::Drop => String::from("drop"),
    }
}

/// A stable spelling of a native builtin (avoids a `Debug` widening on the
/// tag).
fn native_name(prim: gandr_core_term::prim::NativePrim) -> RenderToken<'static>
{
    use gandr_core_term::prim::NativePrim;
    match prim {
        | NativePrim::Id => "id",
        | NativePrim::Const => "const",
        | NativePrim::Add => "add",
        | NativePrim::Sub => "sub",
        | NativePrim::Mul => "mul",
        | NativePrim::Div => "div",
        | NativePrim::Mod => "mod",
        | NativePrim::Eq => "eq",
        | NativePrim::Ne => "ne",
        | NativePrim::Lt => "lt",
        | NativePrim::Le => "le",
        | NativePrim::Gt => "gt",
        | NativePrim::Ge => "ge",
        | NativePrim::And => "and",
        | NativePrim::Or => "or",
        | NativePrim::Not => "not",
        | NativePrim::Neg => "neg",
        | NativePrim::ListConcat => "list-concat",
        | NativePrim::Each => "each",
        | NativePrim::Where => "where",
        | NativePrim::Reduce => "reduce",
        | NativePrim::Any => "any",
        | NativePrim::All => "all",
        | NativePrim::Flatten => "flatten",
        | NativePrim::Uniq => "uniq",
        | NativePrim::Sort => "sort",
        | NativePrim::ListLength => "list-length",
        | NativePrim::ListAt => "list-at",
        | NativePrim::Get => "get",
        | NativePrim::Insert => "insert",
        | NativePrim::RecordUpdate => "record-update",
        | NativePrim::Set => "set",
        | NativePrim::UpdateAt => "update-at",
        | NativePrim::InsertAt => "insert-at",
        | NativePrim::RemoveAt => "remove-at",
        | NativePrim::Push => "push",
        | NativePrim::UpdateWhere => "update-where",
        | NativePrim::StringEscape => "string-escape",
        | NativePrim::StringContains => "string-contains",
        | NativePrim::StringStartsWith => "string-starts-with",
        | NativePrim::StringEndsWith => "string-ends-with",
        | NativePrim::StringEq => "string-eq",
        | NativePrim::StringSplit => "string-split",
        | NativePrim::StringAppend => "string-append",
        | NativePrim::StringLength => "string-length",
        | NativePrim::RegexExtract => "regex-extract",
        | NativePrim::PathJoin => "path-join",
        | NativePrim::PathBasename => "path-basename",
        | NativePrim::PathExtension => "path-extension",
    }
    .into()
}

#[cfg(test)]
mod tests
{
    use gandr_core_term::grade::Grade;
    use gandr_core_term::prim::NativePrim;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Value;

    use super::*;
    use crate::focus;

    /// `ret ()` focuses and renders as a terminal positive cut.
    #[test]
    fn renders_terminal_cut()
    {
        let comp = Comp::ret(Value::Unit);
        let focused = focus::focus_comp(&comp).expect("focuses");
        let rendered = render_command(focused.arena(), focused.root());
        assert_eq!("⟨() |+ ★⟩", rendered, "ret () is a positive cut against ★");
    }

    /// A thunk renders with its usage grade (carried end-to-end, rendered
    /// through the sealed grade `Debug`).
    #[test]
    fn renders_thunk_grade()
    {
        let comp = Comp::force(Value::thunk(Grade::ONE, Comp::ret(Value::Unit)));
        let focused = focus::focus_comp(&comp).expect("focuses");
        let rendered = render_command(focused.arena(), focused.root());
        assert!(
            rendered.contains("thunk_"),
            "a thunk renders its grade prefix, got: {rendered}"
        );
    }

    /// A lambda renders as a negative cut of an `ap` cocase.
    #[test]
    fn renders_lambda_cocase()
    {
        let comp = Comp::lam("x", Comp::ret(Value::var("x")));
        let focused = focus::focus_comp(&comp).expect("focuses");
        let rendered = render_command(focused.arena(), focused.root());
        assert!(
            rendered.starts_with("⟨cocase { ap(x;"),
            "a lambda is an ap-copattern cocase, got: {rendered}"
        );
        assert!(
            rendered.contains("|− ★⟩"),
            "and meets a negative cut: {rendered}"
        );
    }

    /// The record / list / native heads render in their notation.
    #[test]
    fn renders_structural_heads()
    {
        assert_eq!(
            "Record{a, b}",
            ctor_tag(&CtorTag::Record(alloc::boxed::Box::from([
                String::from("a"),
                String::from("b")
            ]))),
            "records show their labels"
        );
        assert_eq!("Cons", ctor_tag(&CtorTag::Cons), "cons head");
        assert_eq!("force", dtor_tag(&DtorTag::Force), "force head");
        assert_eq!(
            ".field",
            dtor_tag(&DtorTag::RecordProj(String::from("field"))),
            "record projection head"
        );
        assert_eq!(
            "native[add]",
            prim_op(PrimOp::Native(NativePrim::Add)),
            "native head"
        );
    }
}
