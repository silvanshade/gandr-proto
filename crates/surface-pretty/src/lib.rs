//! Core-state presentation documents.
//!
//! The proposal's `gandr-pretty` role (named per the current naming
//! authority): checked types and values of core CBPV laid out as documents in
//! the shared layout engine ([`gandr_surface_layout`]), so a page width — a
//! narrow terminal pane, the default batch page, an editor gutter — decides
//! how a form breaks, while every face reads one vocabulary.
//!
//! # The binding invariant: parity with the flat spellings
//!
//! The engine's flat renderers pin the presentation vocabulary today:
//! `Π(x : A). B`, `(A → B)`, `F A !ε`, `List A`, `Stk(B, C)`, `[1, 2, 3]`,
//! `#{label = value}`. Every document this crate builds is constructed so
//! that its **flattened image is byte-for-byte that flat spelling**: each
//! potential break point is a choice whose inline branch contributes the
//! exact byte the flat spelling carries there (a space after a comma, a
//! space before a codomain, nothing before a closing bracket). At a generous
//! width the resolver selects every inline branch — fewer lines always wins
//! when nothing overflows — so [`present_type`] and [`present_value`] return
//! the flat rendering without special-casing, and under width pressure the
//! same documents break, indenting continuations two columns.
//!
//! When the proposal's Stage 3 carriers land, this crate's input side becomes
//! the arena traversal §4 specifies; the output side — these documents, these
//! break points, this vocabulary — carries over unchanged.
//!
//! # Totality posture
//!
//! Nothing here panics. Both walkers drain explicit task stacks rather than
//! recursing, value rendering is depth-bounded at [`DEPTH_LIMIT`] with
//! `<deep>` beyond it, mirroring the flat renderer, and all construction and
//! rendering failures surface as [`PresentationError`].

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use gandr_core_term::syntax::NumLit;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;
use gandr_surface_layout::arena::DocArena;
use gandr_surface_layout::arena::DocId;
use gandr_surface_layout::arena::TextSource;
use gandr_surface_layout::build::DocBuilder;
use gandr_surface_layout::error::BuildError;
use gandr_surface_layout::error::RenderError;
use gandr_surface_layout::limits::BuildLimits;
use gandr_surface_layout::limits::BuildMeter;
use gandr_surface_layout::limits::RenderLimits;
use gandr_surface_layout::limits::RenderMeter;
use gandr_surface_layout::measure::LayoutOptions;
use gandr_surface_layout::measure::PhysicalLineEnding;
use gandr_surface_layout::render;
use gandr_surface_layout::units::ComputationWidth;
use gandr_surface_layout::units::MaxBuildSteps;
use gandr_surface_layout::units::MaxDocNodes;
use gandr_surface_layout::units::MaxFrontierEntries;
use gandr_surface_layout::units::MaxLayoutSteps;
use gandr_surface_layout::units::MaxLivePlanNodes;
use gandr_surface_layout::units::MaxMemoStates;
use gandr_surface_layout::units::MaxOutputBytes;
use gandr_surface_layout::units::MaxPlanNodesCreated;
use gandr_surface_layout::units::MaxResolverStack;
use gandr_surface_layout::units::MaxResolverWorkEntries;
use gandr_surface_layout::units::MaxTextBytes;
use gandr_surface_layout::units::MaxVerbatimLines;
use gandr_surface_layout::units::MaxVmStack;
use gandr_surface_layout::units::MaxVmSteps;
use gandr_surface_layout::units::NestAmount;
use gandr_surface_layout::units::PageWidth;

/// The maximum depth [`value_document`] descends before rendering `<deep>`,
/// mirroring the flat renderer's bound until the two paths share one home.
pub const DEPTH_LIMIT: usize = 32;

/// The indentation a broken continuation takes.
const CONTINUATION_INDENT: u32 = 2;

/// Generous ceilings for one presentation build; presentation is metered but
/// never budget-limited in ordinary use.
#[must_use]
fn build_limits() -> BuildLimits
{
    BuildLimits {
        max_doc_nodes: MaxDocNodes::from(1_000_000u32),
        max_text_bytes: MaxTextBytes::from(0x0400_0000_usize),
        max_verbatim_lines: MaxVerbatimLines::from(1_000_000u32),
        max_build_steps: MaxBuildSteps::from(20_000_000u64),
    }
}

/// Generous ceilings for one presentation render.
#[must_use]
fn render_limits() -> RenderLimits
{
    RenderLimits {
        max_live_plan_nodes: MaxLivePlanNodes::from(8_000_000u64),
        max_output_bytes: MaxOutputBytes::from(0x0400_0000u64),
        max_layout_steps: MaxLayoutSteps::from(100_000_000u64),
        max_resolver_work_entries: MaxResolverWorkEntries::from(100_000_000u64),
        max_resolver_stack: MaxResolverStack::from(1_000_000u64),
        max_memo_states: MaxMemoStates::from(100_000_000u64),
        max_frontier_entries: MaxFrontierEntries::from(100_000_000u64),
        max_plan_nodes_created: MaxPlanNodesCreated::from(100_000_000u64),
        max_vm_steps: MaxVmSteps::from(100_000_000u64),
        max_vm_stack: MaxVmStack::from(1_000_000u64),
    }
}

/// A failure on the way from a checked type or value to rendered text.
#[derive(Debug, thiserror::Error)]
pub enum PresentationError
{
    /// Document construction failed against its build ceilings.
    #[error(transparent)]
    Build(#[from] BuildError),
    /// Resolution or rendering failed against its budgets.
    #[error(transparent)]
    Render(#[from] RenderError),
}

/// Lays out `ty` at `width` and returns the selected text.
///
/// At a width no construct breaks, the result is byte-for-byte the flat
/// spelling. Breaks indent continuations by [`CONTINUATION_INDENT`] columns.
///
/// # Errors
/// Returns [`PresentationError`] when construction or rendering hits a
/// ceiling or an arithmetic bound.
#[inline]
pub fn present_type(
    ty: &Ty,
    width: PageWidth,
) -> Result<String, PresentationError>
{
    let mut meter = BuildMeter::try_new(build_limits())?;
    let mut builder = DocBuilder::try_new(&mut meter)?;
    let root = type_document(&mut builder, ty)?;
    finish_and_render(&builder.finish()?, root, width)
}

/// Lays out `value` at `width` and returns the selected text.
///
/// Depth-bounded at [`DEPTH_LIMIT`]; beyond it renders `<deep>`, as the flat
/// renderer does. Annotations are transparent.
///
/// # Errors
/// Returns [`PresentationError`] when construction or rendering hits a
/// ceiling or an arithmetic bound.
#[inline]
pub fn present_value(
    value: &Value,
    width: PageWidth,
) -> Result<String, PresentationError>
{
    let mut meter = BuildMeter::try_new(build_limits())?;
    let mut builder = DocBuilder::try_new(&mut meter)?;
    let root = value_document(&mut builder, value)?;
    finish_and_render(&builder.finish()?, root, width)
}

/// Renders a finished arena at `width` with line-feed endings.
fn finish_and_render(
    arena: &DocArena,
    root: DocId,
    width: PageWidth,
) -> Result<String, PresentationError>
{
    // The computation width doubles the page so the resolver has room to
    // prove which candidate layouts fit before choosing among them.
    let computation = u32::from(width).saturating_mul(2);
    let options = LayoutOptions::try_new(
        width,
        ComputationWidth::from(computation),
        PhysicalLineEnding::Lf,
    )?;
    let mut render_meter = RenderMeter::try_new(render_limits())?;
    let rendered = render::render(arena, root, &options, &mut render_meter)?;
    Ok(rendered.text.as_ref().to_owned())
}

/// One checked text leaf.
#[expect(
    unknown_lints,
    reason = "primitive_signature is supplied by the local dylint library"
)]
#[expect(
    clippy::allow_attributes,
    reason = "the stable compiler and local dylint library disagree about primitive_signature"
)]
#[allow(
    unfulfilled_lint_expectations,
    reason = "the unknown-lint expectation is fulfilled only under the stable compiler"
)]
#[expect(
    primitive_signature,
    reason = "the helper is exactly the bridge from fixed notation fragments into the layout engine's checked TextSource"
)]
fn leaf(
    builder: &mut DocBuilder<'_>,
    text: &str,
) -> Result<DocId, BuildError>
{
    builder.text(TextSource::from(text))
}

/// Concatenates two already-built documents.
fn concat2(
    builder: &mut DocBuilder<'_>,
    left: DocId,
    right: DocId,
) -> Result<DocId, BuildError>
{
    builder.concat(left, right)
}

/// Appends a separator to `head`: a plain space inline, or an indented hard
/// line when narrow.
///
/// The inline branch is the exact byte the flat spelling carries at this
/// position, so parity holds structurally; the broken branch starts the tail
/// indented [`CONTINUATION_INDENT`] columns.
fn space_or_break(
    builder: &mut DocBuilder<'_>,
    head: DocId,
) -> Result<DocId, BuildError>
{
    let space = leaf(builder, " ")?;
    let brk_line = builder.hard_line();
    let indented =
        builder.nest(NestAmount::from(CONTINUATION_INDENT), brk_line)?;
    let brk = builder.choice(space, indented)?;
    concat2(builder, head, brk)
}

/// Appends a zero-width-or-hard-line break to `head`: nothing inline, a bare
/// new line before a closing bracket when narrow.
fn none_or_break(
    builder: &mut DocBuilder<'_>,
    head: DocId,
) -> Result<DocId, BuildError>
{
    let none = leaf(builder, "")?;
    let brk = builder.hard_line();
    let choice = builder.choice(none, brk)?;
    concat2(builder, head, choice)
}

/// Builds `opener(item␣,␣item)` closing with `closer`.
///
/// Each separator after a comma is a space-or-break and the pre-close break
/// is a none-or-break, so the flattened image is exactly
/// `opener(item, item)closer` and narrow pages break after commas and before
/// the closer.
#[expect(
    unknown_lints,
    reason = "primitive_signature is supplied by the local dylint library"
)]
#[expect(
    clippy::allow_attributes,
    reason = "the stable compiler and local dylint library disagree about primitive_signature"
)]
#[allow(
    unfulfilled_lint_expectations,
    reason = "the unknown-lint expectation is fulfilled only under the stable compiler"
)]
#[expect(
    primitive_signature,
    reason = "bracket spellings are fixed notation fragments of the presentation vocabulary, not semantic state"
)]
fn assemble_bracketed(
    builder: &mut DocBuilder<'_>,
    opener: &str,
    closer: &str,
    items: &[DocId],
) -> Result<DocId, BuildError>
{
    let opener_node = leaf(builder, opener)?;
    let Some(first) = items.first().copied() else {
        let closer_node = leaf(builder, closer)?;
        return concat2(builder, opener_node, closer_node);
    };
    let started = concat2(builder, opener_node, first)?;
    let mut joined: Vec<DocId> =
        Vec::with_capacity(items.len().saturating_mul(3).saturating_add(1));
    joined.push(started);
    for item in items.iter().skip(1) {
        let comma = leaf(builder, ",")?;
        let separated = space_or_break(builder, comma)?;
        joined.push(separated);
        joined.push(*item);
    }
    let inner = builder.concat_all(joined)?;
    let preclose = none_or_break(builder, inner)?;
    let closer_node = leaf(builder, closer)?;
    concat2(builder, preclose, closer_node)
}

/// Builds the document for a first-order value.
///
/// The notation mirrors the flat renderer exactly: `()`, integers, quoted
/// strings, pairs, injections, lists, records, opaque thunks and constructors,
/// transparent annotations, and `here(witness)` witnesses.
///
/// # Errors
/// Returns [`BuildError`] from the underlying builder.
#[inline]
pub fn value_document(
    builder: &mut DocBuilder<'_>,
    value: &Value,
) -> Result<DocId, BuildError>
{
    build_value(builder, value, RenderDepth::ROOT)
}

/// One node of a checked type, either half of the `Ty` sum.
#[derive(Clone, Copy)]
enum TypeNode<'type_node>
{
    /// A value-type subtree.
    Value(&'type_node ValueType),
    /// A computation-type subtree.
    Comp(&'type_node CompType),
}

impl<'ty> TypeNode<'ty>
{
    /// Views the appropriate half of a whole `Ty`.
    fn of_ty(ty: &'ty Ty) -> Self
    {
        match *ty {
            | Ty::Value(ref value_type) => Self::Value(value_type),
            | Ty::Comp(ref comp_type) => Self::Comp(comp_type),
        }
    }
}

/// Builds the document for a type: value types and computation types share
/// one notation, exactly as the flat renderer spells them.
///
/// # Errors
/// Returns [`BuildError`] from the underlying builder.
#[inline]
pub fn type_document(
    builder: &mut DocBuilder<'_>,
    ty: &Ty,
) -> Result<DocId, BuildError>
{
    build_type(builder, TypeNode::of_ty(ty))
}

/// Builds the document for a value type.
///
/// # Errors
/// Returns [`BuildError`] from the underlying builder.
#[inline]
pub fn value_type_document(
    builder: &mut DocBuilder<'_>,
    value_type: &ValueType,
) -> Result<DocId, BuildError>
{
    build_type(builder, TypeNode::Value(value_type))
}

/// Builds the document for a computation type.
///
/// # Errors
/// Returns [`BuildError`] from the underlying builder.
#[inline]
pub fn comp_type_document(
    builder: &mut DocBuilder<'_>,
    comp_type: &CompType,
) -> Result<DocId, BuildError>
{
    build_type(builder, TypeNode::Comp(comp_type))
}

/// One pending step of the explicit type-task stack behind [`build_type`].
enum TypeTask<'type_task>
{
    /// Produce the document for one type node.
    Build
    {
        /// The type node whose document this step produces.
        node: TypeNode<'type_task>,
    },
    /// Produce one fixed text leaf.
    Text
    {
        /// The exact bytes of the leaf.
        text: String,
    },
    /// Rewrite the top finished document with a breakable continuation.
    SpaceOrBreak,
    /// Combine finished documents into one finished document.
    Assemble
    {
        /// How many finished documents combine, and how.
        operation: TypeAssembleOperation,
    },
}

/// How one assemble step combines finished type documents below it.
enum TypeAssembleOperation
{
    /// Concatenate every finished document in order.
    Join
    {
        /// How many finished documents join.
        count: usize,
    },
    /// `(lhs symbol rhs)` around two operands, breaking before the second.
    Infix
    {
        /// The infix symbol without spaces.
        symbol: String,
    },
    /// A bracketed sequence around `count` finished items.
    Bracket
    {
        /// The text opening the bracketed form.
        opener: String,
        /// The text closing the bracketed form.
        closer: String,
        /// How many finished documents the bracket encloses.
        count: usize,
    },
}

/// Frames an infix assembly around two operand nodes.
fn infix_frame<'frame>(
    symbol: &str,
    lhs: TypeNode<'frame>,
    rhs: TypeNode<'frame>,
) -> Vec<TypeTask<'frame>>
{
    vec![
        TypeTask::Assemble {
            operation: TypeAssembleOperation::Infix {
                symbol: String::from(symbol),
            },
        },
        TypeTask::Build { node: lhs },
        TypeTask::Build { node: rhs },
    ]
}

/// Frames `opener(nodes…)closer`.
fn bracketed_frame<'frame>(
    opener: String,
    closer: &str,
    nodes: Vec<TypeNode<'frame>>,
) -> Vec<TypeTask<'frame>>
{
    let count = nodes.len();
    let mut tasks = vec![TypeTask::Assemble {
        operation: TypeAssembleOperation::Bracket {
            opener,
            closer: String::from(closer),
            count,
        },
    }];
    tasks.extend(nodes.into_iter().map(|node| TypeTask::Build { node }));
    tasks
}

/// Frames `(arg → res)`.
fn arrow_frame<'frame>(
    arg: &'frame ValueType,
    res: &'frame CompType,
) -> Vec<TypeTask<'frame>>
{
    vec![
        TypeTask::Assemble {
            operation: TypeAssembleOperation::Join { count: 6 },
        },
        TypeTask::Text {
            text: String::from("("),
        },
        TypeTask::Build {
            node: TypeNode::Value(arg),
        },
        TypeTask::Text {
            text: String::from(" →"),
        },
        TypeTask::SpaceOrBreak,
        TypeTask::Build {
            node: TypeNode::Comp(res),
        },
        TypeTask::Text {
            text: String::from(")"),
        },
    ]
}

/// Frames `Π(binder : arg). res`.
fn dependent_frame<'frame>(
    binder: &str,
    arg: &'frame ValueType,
    res: &'frame CompType,
) -> Vec<TypeTask<'frame>>
{
    vec![
        TypeTask::Assemble {
            operation: TypeAssembleOperation::Join { count: 5 },
        },
        TypeTask::Text {
            text: format!("Π({binder} : "),
        },
        TypeTask::Build {
            node: TypeNode::Value(arg),
        },
        TypeTask::Text {
            text: String::from(")."),
        },
        TypeTask::SpaceOrBreak,
        TypeTask::Build {
            node: TypeNode::Comp(res),
        },
    ]
}

/// Builds the document for a type with an explicit task stack.
///
/// # Termination
/// - reason: the driver drains an explicit task stack; a build step pushes
///   only steps for its own children plus their assemble frames, never
///   itself, and the stack empties once the root document is assembled.
/// - measure: pending tasks on the stack; every pushed child is a strict
///   subterm of the node that spawned it.
/// - boundedness: source types are finite Rust values.
/// - input recursion: none; caller-supplied types are walked by the explicit
///   task stack.
fn build_type(
    builder: &mut DocBuilder<'_>,
    root: TypeNode<'_>,
) -> Result<DocId, BuildError>
{
    /// Schedules a frame's tasks given in run order: the leading assemble
    /// step sinks to the bottom of the stack, and the trailing build steps
    /// stack so the first-listed runs first.
    fn schedule<'task>(
        stack: &mut Vec<TypeTask<'task>>,
        mut tasks: Vec<TypeTask<'task>>,
    )
    {
        if tasks.is_empty() {
            return;
        }
        let rest = tasks.split_off(1);
        for task in tasks {
            stack.push(task);
        }
        stack.extend(rest.into_iter().rev());
    }

    /// Pops exactly one finished document; the walk's own bookkeeping keeps
    /// the stack balanced, so exhaustion names an internal fault.
    fn pop_one(done: &mut Vec<DocId>) -> Result<DocId, BuildError>
    {
        done.pop().ok_or(BuildError::UnknownDoc)
    }

    /// Pops the `count` finished documents one assemble frame combines.
    fn pop_documents(done: &mut Vec<DocId>, count: usize) -> Vec<DocId>
    {
        let split = done.len().saturating_sub(count);
        done.split_off(split)
    }

    let mut stack: Vec<TypeTask<'_>> = vec![TypeTask::Build { node: root }];
    let mut done: Vec<DocId> = Vec::new();
    while let Some(step) = stack.pop() {
        match step {
            | TypeTask::Text { text } => {
                done.push(leaf(builder, text.as_str())?);
            },
            | TypeTask::SpaceOrBreak => {
                let head = pop_one(&mut done)?;
                done.push(space_or_break(builder, head)?);
            },
            | TypeTask::Build { node } => match node {
                | TypeNode::Value(value_type) => match value_type {
                    | ValueType::Atom(name) => {
                        done.push(leaf(builder, name.as_str())?);
                    },
                    | ValueType::Unit => done.push(leaf(builder, "Unit")?),
                    | ValueType::Prod(fst, snd) => {
                        let frame =
                            infix_frame("×", TypeNode::Value(fst.as_ref()), TypeNode::Value(snd.as_ref()));
                        schedule(&mut stack, frame);
                    },
                    | ValueType::Sum(lhs, rhs) => {
                        let frame =
                            infix_frame("+", TypeNode::Value(lhs.as_ref()), TypeNode::Value(rhs.as_ref()));
                        schedule(&mut stack, frame);
                    },
                    | ValueType::List(element) => {
                        let frame = vec![
                            TypeTask::Assemble {
                                operation: TypeAssembleOperation::Join { count: 2 },
                            },
                            TypeTask::Text {
                                text: String::from("List "),
                            },
                            TypeTask::Build {
                                node: TypeNode::Value(element.as_ref()),
                            },
                        ];
                        schedule(&mut stack, frame);
                    },
                    | ValueType::Thunk(_, body) => {
                        let frame = vec![
                            TypeTask::Assemble {
                                operation: TypeAssembleOperation::Join { count: 2 },
                            },
                            TypeTask::Text {
                                text: String::from("U "),
                            },
                            TypeTask::Build {
                                node: TypeNode::Comp(body.as_ref()),
                            },
                        ];
                        schedule(&mut stack, frame);
                    },
                    | ValueType::Stk(consumes, delivers) => {
                        let frame = bracketed_frame(
                            String::from("Stk("),
                            ")",
                            vec![
                                TypeNode::Comp(consumes.as_ref()),
                                TypeNode::Comp(delivers.as_ref()),
                            ],
                        );
                        schedule(&mut stack, frame);
                    },
                    | ValueType::Data { id, args } => {
                        let nodes =
                            args.iter()
                                .map(|arg| TypeNode::Value(arg.as_ref()))
                                .collect::<Vec<_>>();
                        let frame = bracketed_frame(
                            format!("{}(", id.name().as_ref()),
                            ")",
                            nodes,
                        );
                        schedule(&mut stack, frame);
                    },
                    | _ => done.push(leaf(builder, "?")?),
                },
                | TypeNode::Comp(comp_type) => match comp_type {
                    | CompType::F(payload, row) => {
                        if bool::from(row.is_empty()) {
                            let frame = vec![
                                TypeTask::Assemble {
                                    operation: TypeAssembleOperation::Join {
                                        count: 2,
                                    },
                                },
                                TypeTask::Text {
                                    text: String::from("F "),
                                },
                                TypeTask::Build {
                                    node: TypeNode::Value(payload.as_ref()),
                                },
                            ];
                            schedule(&mut stack, frame);
                        }
                        else {
                            let frame = vec![
                                TypeTask::Assemble {
                                    operation: TypeAssembleOperation::Join {
                                        count: 3,
                                    },
                                },
                                TypeTask::Text {
                                    text: String::from(" !ε"),
                                },
                                TypeTask::Text {
                                    text: String::from("F "),
                                },
                                TypeTask::Build {
                                    node: TypeNode::Value(payload.as_ref()),
                                },
                            ];
                            schedule(&mut stack, frame);
                        }
                    },
                    | CompType::Arrow {
                        binder: Some(binder),
                        arg,
                        res,
                    } => {
                        let frame =
                            dependent_frame(binder.as_ref(), arg.as_ref(), res.as_ref());
                        schedule(&mut stack, frame);
                    },
                    | CompType::Arrow {
                        binder: None,
                        arg,
                        res,
                    } => {
                        let frame = arrow_frame(arg.as_ref(), res.as_ref());
                        schedule(&mut stack, frame);
                    },
                    | CompType::With(fst, snd) => {
                        let frame = infix_frame(
                            "&",
                            TypeNode::Comp(fst.as_ref()),
                            TypeNode::Comp(snd.as_ref()),
                        );
                        schedule(&mut stack, frame);
                    },
                    | _ => done.push(leaf(builder, "?")?),
                },
            },
            | TypeTask::Assemble { operation } => match operation {
                | TypeAssembleOperation::Join { count } => {
                    let parts = pop_documents(&mut done, count);
                    done.push(builder.concat_all(parts)?);
                },
                | TypeAssembleOperation::Infix { symbol } => {
                    let rhs = pop_one(&mut done)?;
                    let lhs = pop_one(&mut done)?;
                    let close = leaf(builder, ")")?;
                    let open = leaf(builder, "(")?;
                    let symbol_text = format!(" {symbol}");
                    let symbol_node = leaf(builder, symbol_text.as_str())?;
                    let opened = concat2(builder, open, lhs)?;
                    let spaced = concat2(builder, opened, symbol_node)?;
                    let continued = space_or_break(builder, spaced)?;
                    let body = concat2(builder, continued, rhs)?;
                    done.push(concat2(builder, body, close)?);
                },
                | TypeAssembleOperation::Bracket {
                    opener,
                    closer,
                    count,
                } => {
                    let items = pop_documents(&mut done, count);
                    done.push(assemble_bracketed(
                        builder,
                        opener.as_str(),
                        closer.as_str(),
                        &items,
                    )?);
                },
            },
        }
    }
    pop_one(&mut done)
}

/// Current depth in the bounded value renderer.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RenderDepth(usize);

impl RenderDepth
{
    /// Root depth for a freshly rendered value.
    const ROOT: Self = Self(0);

    /// Descends one level without overflowing the host representation.
    const fn descend(self) -> Self
    {
        Self(self.0.saturating_add(1))
    }

    /// Whether further descent is refused in favor of `<deep>`.
    const fn exhausted(self) -> bool
    {
        self.0 >= DEPTH_LIMIT
    }
}

/// One pending step of the explicit value-task stack behind [`build_value`].
enum WalkTask<'value>
{
    /// Produce the document for one value node.
    Build
    {
        /// The value whose document this step produces.
        value: &'value Value,
        /// The depth fuel remaining for this node.
        depth: RenderDepth,
    },
    /// Combine finished documents into one finished document.
    Assemble
    {
        /// How many finished documents combine, and how.
        operation: AssembleOperation,
    },
}

/// How one assemble step combines the finished documents below it.
enum AssembleOperation
{
    /// One payload between a fixed opening and closing text.
    Wrap
    {
        /// The text opened immediately before the payload.
        prefix: String,
        /// The text closed immediately after the payload.
        suffix: String,
    },
    /// `label = field` for one record field.
    Field
    {
        /// The record label followed by ` = `.
        label: String,
    },
    /// A bracketed sequence around `count` finished items.
    Bracket
    {
        /// The text opening the bracketed form.
        opener: String,
        /// The text closing the bracketed form.
        closer: String,
        /// How many finished documents the bracket encloses.
        count: usize,
    },
}

/// Builds the document for a first-order value with an explicit task stack.
///
/// The notation mirrors the flat renderer exactly: `()`, integers, quoted
/// strings, pairs, injections, lists, records, opaque thunks and constructors,
/// transparent annotations, and `here(witness)` witnesses.
///
/// # Termination
/// - reason: the driver drains an explicit task stack; a build step pushes
///   only steps for its own children plus their assemble frames, never
///   itself, and the stack empties once the root document is assembled.
/// - measure: pending tasks on the stack; every pushed child carries depth
///   fuel strictly smaller than its parent's.
/// - boundedness: source values are finite Rust values, and fuel starts at
///   [`DEPTH_LIMIT`], past which a `<deep>` leaf renders without descent.
/// - input recursion: none; caller-supplied values are walked by the
///   explicit task stack.
fn build_value(
    builder: &mut DocBuilder<'_>,
    value: &Value,
    depth: RenderDepth,
) -> Result<DocId, BuildError>
{
    /// Pops exactly one finished document; the walk's own bookkeeping keeps
    /// the stack balanced, so exhaustion names an internal fault.
    fn pop_one(done: &mut Vec<DocId>) -> Result<DocId, BuildError>
    {
        done.pop().ok_or(BuildError::UnknownDoc)
    }

    /// Pops the `count` finished documents one assemble frame combines.
    fn pop_documents(done: &mut Vec<DocId>, count: usize) -> Vec<DocId>
    {
        let split = done.len().saturating_sub(count);
        done.split_off(split)
    }

    let mut stack = vec![WalkTask::Build { value, depth }];
    let mut done: Vec<DocId> = Vec::new();
    while let Some(step) = stack.pop() {
        match step {
            | WalkTask::Build { value, depth } => {
                if depth.exhausted() {
                    done.push(leaf(builder, "<deep>")?);
                    continue;
                }
                let below = depth.descend();
                match *value {
                    | Value::Var(ref name) => {
                        let text = format!("<var {name}>");
                        done.push(leaf(builder, text.as_str())?);
                    },
                    | Value::Unit => done.push(leaf(builder, "()")?),
                    | Value::Int(int) => {
                        let text = int.to_string();
                        done.push(leaf(builder, text.as_str())?);
                    },
                    | Value::Str(ref text) => {
                        let quoted = format!("\"{text}\"");
                        done.push(leaf(builder, quoted.as_str())?);
                    },
                    | Value::Num(num) => {
                        let text = render_num(num);
                        done.push(leaf(builder, text.as_str())?);
                    },
                    | Value::Pair(ref fst, ref snd) => {
                        stack.push(WalkTask::Assemble {
                            operation: AssembleOperation::Bracket {
                                opener: String::from("("),
                                closer: String::from(")"),
                                count: 2,
                            },
                        });
                        stack.push(WalkTask::Build {
                            value: snd.as_ref(),
                            depth: below,
                        });
                        stack.push(WalkTask::Build {
                            value: fst.as_ref(),
                            depth: below,
                        });
                    },
                    | Value::Inj(side, ref payload) => {
                        let prefix = match side {
                            | Side::Fst => "Inl(",
                            | Side::Snd => "Inr(",
                        };
                        stack.push(WalkTask::Assemble {
                            operation: AssembleOperation::Wrap {
                                prefix: String::from(prefix),
                                suffix: String::from(")"),
                            },
                        });
                        stack.push(WalkTask::Build {
                            value: payload.as_ref(),
                            depth: below,
                        });
                    },
                    | Value::List(ref items) => {
                        let count = items.len();
                        stack.push(WalkTask::Assemble {
                            operation: AssembleOperation::Bracket {
                                opener: String::from("["),
                                closer: String::from("]"),
                                count,
                            },
                        });
                        for item in items.iter().rev() {
                            stack.push(WalkTask::Build {
                                value: item.as_ref(),
                                depth: below,
                            });
                        }
                    },
                    | Value::Record(ref fields) => {
                        let count = fields.len();
                        stack.push(WalkTask::Assemble {
                            operation: AssembleOperation::Bracket {
                                opener: String::from("#{"),
                                closer: String::from("}"),
                                count,
                            },
                        });
                        for (label, field) in fields.iter().rev() {
                            stack.push(WalkTask::Assemble {
                                operation: AssembleOperation::Field {
                                    label: format!("{label} = "),
                                },
                            });
                            stack.push(WalkTask::Build {
                                value: field.as_ref(),
                                depth: below,
                            });
                        }
                    },
                    | Value::Thunk(..) => done.push(leaf(builder, "<thunk>")?),
                    | Value::Annot(ref payload, _) => {
                        stack.push(WalkTask::Build {
                            value: payload.as_ref(),
                            depth: below,
                        });
                    },
                    | Value::Here(ref witness) => {
                        stack.push(WalkTask::Assemble {
                            operation: AssembleOperation::Wrap {
                                prefix: String::from("here("),
                                suffix: String::from(")"),
                            },
                        });
                        stack.push(WalkTask::Build {
                            value: witness.as_ref(),
                            depth: below,
                        });
                    },
                    | _ => done.push(leaf(builder, "<opaque>")?),
                }
            },
            | WalkTask::Assemble { operation } => match operation {
                | AssembleOperation::Wrap { prefix, suffix } => {
                    let payload = pop_one(&mut done)?;
                    let prefix_node = leaf(builder, prefix.as_str())?;
                    let suffix_node = leaf(builder, suffix.as_str())?;
                    let opened = concat2(builder, prefix_node, payload)?;
                    done.push(concat2(builder, opened, suffix_node)?);
                },
                | AssembleOperation::Field { label } => {
                    let field = pop_one(&mut done)?;
                    let label_node = leaf(builder, label.as_str())?;
                    done.push(concat2(builder, label_node, field)?);
                },
                | AssembleOperation::Bracket { opener, closer, count } => {
                    let items = pop_documents(&mut done, count);
                    done.push(assemble_bracketed(
                        builder,
                        opener.as_str(),
                        closer.as_str(),
                        &items,
                    )?);
                },
            },
        }
    }
    pop_one(&mut done)
}

/// Renders a typed numeric literal (`5u32`, `1.5f64`, …), mirroring the flat
/// renderer's spellings.
#[must_use]
fn render_num(num: NumLit) -> String
{
    match num {
        | NumLit::U32(n) => format!("{n}u32"),
        | NumLit::U64(n) => format!("{n}u64"),
        | NumLit::I32(n) => format!("{n}i32"),
        | NumLit::I64(n) => format!("{n}i64"),
        | NumLit::F32(bits) => format!("{}f32", f32::from_bits(bits)),
        | NumLit::F64(bits) => format!("{}f64", f64::from_bits(bits)),
    }
}
