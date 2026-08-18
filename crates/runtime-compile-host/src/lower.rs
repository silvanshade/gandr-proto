//! Lowering a checked core computation into a program image.
//!
//! The core's positive fragment and the image's node set are the same six
//! transitions with two producer leaves, so the lowering is a change of
//! representation rather than a translation: names become de Bruijn distances,
//! the tree becomes a flat arena in dependency order, and everything outside
//! the fragment is refused by name.

use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;

use crate::image::BinderIndex;
use crate::image::CtorTag;
use crate::image::Image;
use crate::image::ImageError;
use crate::image::Literal;
use crate::image::Node;
use crate::image::NodeIndex;
use crate::image::NodeKind;

/// What can go wrong while lowering a core computation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LowerError
{
    /// The computation uses a form the compiled slice does not lower.
    ///
    /// The message names the form rather than the position, because the
    /// slice's boundary is a statement about the language and not about one
    /// term: a caller learns which rung it is waiting on.
    #[error("the computation uses {form}, which the compiled slice does not lower")]
    OutsideSlice
    {
        /// The core form that was met.
        form: FormName,
    },
    /// A variable names no binder in scope.
    #[error("the variable {name} names no binder in scope")]
    UnboundVariable
    {
        /// The variable's name.
        name: VariableName,
    },
    /// The image outgrew what the host's decoder accepts.
    #[error("the computation does not fit a program image: {source}")]
    ImageRefused
    {
        /// Why the arena refused the node.
        #[from]
        source: ImageError,
    },
}

/// The name of a core form the slice does not lower.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct FormName(&'static str);

impl core::fmt::Display for FormName
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(self.0)
    }
}

/// A variable's name, as the core spells it.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct VariableName(String);

impl core::fmt::Display for VariableName
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(&self.0)
    }
}

/// What the walk assembles once its operands are on the results stack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Assemble
{
    /// A binder frame over its bound producer and its body.
    Bind,
    /// A dispatch over its scrutinee and two arms.
    Case,
    /// A duplication over its source.
    Dup,
    /// A discard over its source.
    Drop,
    /// A pair constructor over its two fields.
    Pair,
    /// An injection over its payload.
    Inj(CtorTag),
    /// The terminal cut over the produced value.
    Cut,
}

/// One step of the lowering walk.
///
/// The walk is an explicit stack rather than recursion: a core term's depth is
/// caller-supplied, so a recursive lowering would be a stack overflow waiting
/// for a deep term.
enum Step<'term>
{
    /// Emit a producer node for this computation.
    Computation(&'term Comp),
    /// Emit a producer node for this value.
    Producer(&'term Value),
    /// Build a node from the operands already emitted.
    Build(Assemble),
    /// Bring a binder into scope for the steps that follow.
    OpenBinder(&'term str),
    /// Take the innermost binder back out of scope.
    CloseBinder,
}

/// The walk's mutable state.
struct Walk<'term>
{
    /// The image being assembled.
    image: Image,
    /// The steps still to run, innermost last.
    steps: Vec<Step<'term>>,
    /// The indices of the nodes emitted so far, in emission order.
    emitted: Vec<NodeIndex>,
    /// The binders in scope, innermost last.
    scope: Vec<&'term str>,
}

/// Lowers a core computation into a program image.
///
/// # Contract
/// - requires: `comp` is a core computation; nothing about its type is assumed
///   here, because the checking happens one layer up.
/// - ensures: the returned image's nodes are in dependency order with the
///   terminal cut last, which is the shape the host's decoder requires.
/// - provides: the one route from the Rust core into the compiled slice's
///   input.
/// - fails: [`LowerError::OutsideSlice`] for a form the slice excludes,
///   [`LowerError::UnboundVariable`] for a free variable, and
///   [`LowerError::ImageRefused`] when the arena's bound is reached.
/// - panics: none; the walk is an explicit stack, so term depth costs heap
///   rather than the host stack.
///
/// # Errors
/// The variants above, each at the first term that triggers it.
///
/// # Adequacy
/// - hypothesis: L2 with an L3 residue — the shape of every accepted form is
///   carried by the agreement differential against the L machine, and the
///   refusals are the pointwise residue, each triggered by a term naming
///   exactly one excluded form.
/// - witness: `bridge::the_bridge_agrees_with_the_l_machine_on_every_named_program`
/// - witness: `lowering::every_excluded_core_form_is_refused_by_name`
/// - witness: `lowering::a_free_variable_is_refused_rather_than_lowered`
#[inline]
pub fn lower_computation(comp: &Comp) -> Result<Image, LowerError>
{
    let mut walk = Walk {
        image: Image::new(),
        steps: alloc::vec![Step::Build(Assemble::Cut), Step::Computation(comp)],
        emitted: Vec::new(),
        scope: Vec::new(),
    };

    while let Some(step) = walk.steps.pop() {
        match step {
            | Step::OpenBinder(name) => walk.scope.push(name),
            | Step::CloseBinder => {
                let _ = walk.scope.pop();
            },
            | Step::Computation(comp) => {
                let () = push_computation(&mut walk, comp)?;
            },
            | Step::Producer(value) => {
                let () = push_producer(&mut walk, value)?;
            },
            | Step::Build(assemble) => {
                let () = build(&mut walk, assemble)?;
            },
        }
    }

    Ok(walk.image)
}

/// Schedules a computation's operands, or emits its leaf directly.
fn push_computation<'term>(
    walk: &mut Walk<'term>,
    comp: &'term Comp,
) -> Result<(), LowerError>
{
    match *comp {
        | Comp::Ret(ref value) => {
            walk.steps.push(Step::Producer(value));
            Ok(())
        },
        | Comp::Bind(ref bound, ref name, ref body) => {
            walk.steps.push(Step::Build(Assemble::Bind));
            walk.steps.push(Step::CloseBinder);
            walk.steps.push(Step::Computation(body));
            walk.steps.push(Step::OpenBinder(name));
            walk.steps.push(Step::Computation(bound));
            Ok(())
        },
        | Comp::Case(ref scrutinee, (ref left_name, ref left), (ref right_name, ref right)) => {
            walk.steps.push(Step::Build(Assemble::Case));
            walk.steps.push(Step::CloseBinder);
            walk.steps.push(Step::Computation(right));
            walk.steps.push(Step::OpenBinder(right_name));
            walk.steps.push(Step::CloseBinder);
            walk.steps.push(Step::Computation(left));
            walk.steps.push(Step::OpenBinder(left_name));
            walk.steps.push(Step::Producer(scrutinee));
            Ok(())
        },
        | Comp::Dup(ref source) => {
            walk.steps.push(Step::Build(Assemble::Dup));
            walk.steps.push(Step::Producer(source));
            Ok(())
        },
        | Comp::Drop(ref source) => {
            walk.steps.push(Step::Build(Assemble::Drop));
            walk.steps.push(Step::Producer(source));
            Ok(())
        },
        | _ => Err(LowerError::OutsideSlice {
            form: computation_form(comp),
        }),
    }
}

/// Schedules a value's operands, or emits its leaf directly.
fn push_producer<'term>(
    walk: &mut Walk<'term>,
    value: &'term Value,
) -> Result<(), LowerError>
{
    match *value {
        | Value::Int(literal) => {
            let index = walk.image.push(Node {
                kind: NodeKind::Lit,
                tag: CtorTag::Unit,
                binder: BinderIndex::default(),
                literal: Literal::from(literal),
                operands: Vec::new(),
            })?;
            walk.emitted.push(index);
            Ok(())
        },
        | Value::Unit => {
            let index = walk.image.push(Node {
                kind: NodeKind::Ctor,
                tag: CtorTag::Unit,
                binder: BinderIndex::default(),
                literal: Literal::default(),
                operands: Vec::new(),
            })?;
            walk.emitted.push(index);
            Ok(())
        },
        | Value::Var(ref name) => {
            let distance = walk
                .scope
                .iter()
                .rev()
                .position(|bound| *bound == name.as_str());
            let Some(distance) = distance
            else {
                return Err(LowerError::UnboundVariable {
                    name: VariableName(name.clone()),
                });
            };
            let binder =
                BinderIndex::try_from(distance).map_err(|_narrowing| ImageError::TooManyNodes)?;
            let index = walk.image.push(Node {
                kind: NodeKind::Var,
                tag: CtorTag::Unit,
                binder,
                literal: Literal::default(),
                operands: Vec::new(),
            })?;
            walk.emitted.push(index);
            Ok(())
        },
        | Value::Pair(ref first, ref second) => {
            walk.steps.push(Step::Build(Assemble::Pair));
            walk.steps.push(Step::Producer(second));
            walk.steps.push(Step::Producer(first));
            Ok(())
        },
        | Value::Annot(ref inner, _) => {
            // An annotation is a typing artifact with no runtime content: the
            // focusing translation drops it on the way to the L machine, and
            // the image has nowhere to put it. Lowering it transparently is
            // what lets the bridge accept exactly what the checker accepts,
            // since an injection needs its sum type to check at all.
            walk.steps.push(Step::Producer(inner));
            Ok(())
        },
        | Value::Inj(side, ref payload) => {
            let tag = if side == Side::Fst {
                CtorTag::Inl
            }
            else {
                CtorTag::Inr
            };
            walk.steps.push(Step::Build(Assemble::Inj(tag)));
            walk.steps.push(Step::Producer(payload));
            Ok(())
        },
        | _ => Err(LowerError::OutsideSlice {
            form: producer_form(value),
        }),
    }
}

/// Builds one node from the operands the walk has already emitted.
fn build(
    walk: &mut Walk<'_>,
    assemble: Assemble,
) -> Result<(), LowerError>
{
    let (kind, tag, arity) = match assemble {
        | Assemble::Bind => (NodeKind::Bind, CtorTag::Unit, 2),
        | Assemble::Case => (NodeKind::Case, CtorTag::Unit, 3),
        | Assemble::Dup => (NodeKind::Dup, CtorTag::Unit, 1),
        | Assemble::Drop => (NodeKind::Drop, CtorTag::Unit, 1),
        | Assemble::Pair => (NodeKind::Ctor, CtorTag::Pair, 2),
        | Assemble::Inj(tag) => (NodeKind::Ctor, tag, 1),
        | Assemble::Cut => (NodeKind::Cut, CtorTag::Unit, 1),
    };

    let available = walk.emitted.len();
    let Some(first) = available.checked_sub(arity)
    else {
        // Unreachable on a well-formed schedule: every `Build` is pushed
        // together with exactly the operand steps it consumes. Reporting it as
        // a refusal rather than asserting keeps the lowering total.
        return Err(LowerError::OutsideSlice {
            form: FormName("a term whose operands were not all emitted"),
        });
    };
    let operands: Vec<NodeIndex> = walk.emitted.split_off(first);

    let index = walk.image.push(Node {
        kind,
        tag,
        binder: BinderIndex::default(),
        literal: Literal::default(),
        operands,
    })?;
    walk.emitted.push(index);
    Ok(())
}

/// The name of a computation form the slice does not lower.
fn computation_form(comp: &Comp) -> FormName
{
    FormName(match *comp {
        | Comp::Abs(..) => "an abstraction",
        | Comp::App(..) => "an application",
        | Comp::Force(..) => "a forced thunk",
        | Comp::DataCase(..) => "a declared-data dispatch",
        | Comp::ListCase { .. } => "a list dispatch",
        | _ => "a computation form outside the positive core",
    })
}

/// The name of a value form the slice does not lower.
fn producer_form(value: &Value) -> FormName
{
    FormName(match *value {
        | Value::Str(..) => "a string literal",
        | Value::Num(..) => "a typed numeric literal",
        | Value::List(..) => "a list literal",
        | Value::Record(..) => "a record",
        | Value::Thunk(..) => "a thunk",
        | Value::Hole(..) => "a hole",
        | _ => "a value form outside the positive core",
    })
}
