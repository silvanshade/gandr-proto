//! Pattern compilation: the seam between the arms a programmer writes and the
//! tag-indexed arms the core eliminator carries.
//!
//! # Why the arm reader is a compiler rather than a shape check
//!
//! The core's [`Comp::DataCase`] is already *compiled*: one arm per declared
//! constructor, placed by tag, with source order gone. Surface arms are
//! ordered, may overlap, may nest, and may carry a typed hole standing where a
//! test is not written yet. Turning the second into the first is a compilation
//! step, and this module is where it happens — so the arm reader no longer
//! decides what it can handle by inspecting a CST node kind.
//!
//! Three properties the compiled form must keep, each of which a shape check
//! could not have stated:
//!
//! * **Precedence.** The first arm that can match a scrutinee wins, and the
//!   compiler resolves that per constructor tag rather than per arm.
//! * **Indeterminacy is a verdict, not an absence.** A pattern hole makes a
//!   branch neither satisfied nor refuted, so every tag it shadows becomes a
//!   stuck [`Comp::Hole`] carrying [`HoleNote::IndeterminatePattern`]. The
//!   unfilled hole never guesses a branch and never widens into a catch-all:
//!   the arms it shadows stop being reachable, which is exactly the difference
//!   between an indeterminate arm and a dropped one.
//! * **Non-exhaustiveness stays distinguishable.** A tag no arm reaches is a
//!   [`HoleNote::MissingCaseArm`] hole, as it always was. Both notes reach the
//!   same runtime outcome, so the note is what tells a reader which of the two
//!   happened.
//!
//! # Its own pattern view, and why it is not the analysis's
//!
//! [`crate::live_match`] translates the same grammar into
//! [`PatternShape`](crate::live_match::PatternShape), and that view is
//! deliberately **node-free**: it is a snapshot a session retains after the
//! parse is gone, so it can carry no CST identity. Emission needs the opposite
//! — every synthesized node wants the originating node's identity, hash, and
//! byte range for the [`crate::origin`] side table. So this module reads the
//! pattern grammar again into [`PatternPlan`], which carries its nodes.
//!
//! The two readers answer different questions over one grammar and must move
//! together: a pattern form added to the grammar is added to both, or the
//! analysis will describe an arm this compiler declines (or the reverse).
//!
//! # What it emits, and what it declines by name
//!
//! Compiled: constructor patterns at any nesting depth, wildcard and binder
//! sub-patterns, as-binders, pattern holes anywhere, and or-patterns all of
//! whose alternatives are indeterminate.
//!
//! Three shapes the tag walk cannot place go to [`super::matrix`] instead: a
//! top-level catch-all, an or-pattern with distinguishable alternatives, and
//! two arms sharing one constructor head. Each needs one arm body reached from
//! more than one branch, which that module supplies as a bound thunk — the
//! join point — rather than as a core former. The primitives here are what it
//! emits with: this module owns reading the pattern grammar and the shape of
//! one compiled arm, and the matrix compiler owns the order the tests run in.
//!
//! Still declined by name rather than dropped by shape: the literal, tuple,
//! record, and list forms, whose head domain this eliminator does not switch
//! on. That is a missing *test* rather than a missing join, so no amount of
//! sharing reaches it. A declined arm behaves exactly as an unreadable arm did
//! before this module existed: strict mode fails with
//! [`LowerError::Unsupported`], total mode skips it.
//!
//! [`Comp::DataCase`]: gandr_core_term::syntax::Comp::DataCase
//! [`Comp::Hole`]: gandr_core_term::syntax::Comp::Hole
//! [`HoleNote::IndeterminatePattern`]: crate::origin::HoleNote::IndeterminatePattern
//! [`HoleNote::MissingCaseArm`]: crate::origin::HoleNote::MissingCaseArm

use alloc::borrow::ToOwned as _;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::boundary::ConstructorTag;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::HoleId;
use gandr_core_term::syntax::Value;

use super::COut;
use super::LowerError;
use super::LowerResult;
use super::Lowerer;
use super::entry;
use super::node_kinds;
use crate::boundary::ConstructorCount;
use crate::boundary::ConstructorName;
use crate::boundary::CoreVariableName;
use crate::boundary::SourceRange;
use crate::boundary::SubPatternCount;
use crate::boundary::SyntaxKind;
use crate::boundary::TypeName;
use crate::origin::ElabKind;
use crate::origin::HoleNote;
use crate::origin::OriginEntry;
use crate::origin::OriginNode;
use crate::synnode::SynNode;

/// One pattern as the compiler reads it: the landed pattern grammar, with the
/// CST node retained at every position that can originate a synthesized core
/// node.
///
/// # Contract
/// - provides: a total reading of any pattern node — a form outside the
///   compiled set arrives as [`Self::Declined`] naming its kind, never as a
///   guess and never as a silent omission.
#[derive(Clone)]
pub(super) enum PatternPlan<'tree>
{
    /// A pattern-position typed hole `?` / `?name`: an unfinished test, which
    /// no scrutinee satisfies and no scrutinee refutes.
    Hole
    {
        /// The `?` node, whose byte range localizes the goal.
        node: SynNode<'tree>,
        /// The hole's name when written `?name`.
        name: Option<String>,
    },
    /// The wildcard `_`, and the rest marker outside a list: binds nothing and
    /// tests nothing.
    Discard,
    /// A variable binder, which matches anything and names it.
    Bind
    {
        /// The bound name.
        name: String,
        /// The binder node, for the shadowing report.
        node: SynNode<'tree>,
    },
    /// A constructor pattern `C` or `C(p̄)`.
    Ctor
    {
        /// The constructor's name.
        name: String,
        /// The constructor pattern node.
        node: SynNode<'tree>,
        /// The argument patterns, in source order.
        arguments: Vec<Self>,
    },
    /// An as-pattern `p as x`.
    As
    {
        /// The tested pattern.
        pattern: Box<Self>,
        /// The name the whole match is bound to.
        binder: String,
        /// The binder node, for the shadowing report.
        node: SynNode<'tree>,
    },
    /// An or-pattern `p | q | …`, flattened.
    Or
    {
        /// The alternatives, in source order.
        alternatives: Vec<Self>,
        /// The or-pattern node.
        node: SynNode<'tree>,
    },
    /// A form outside the compiled set, named rather than dropped.
    Declined
    {
        /// The declined node's kind.
        kind: SyntaxKind,
        /// The declined node's byte range.
        byte_range: SourceRange,
    },
}

/// One step of the iterative pattern reading.
enum ReadStep<'tree>
{
    /// Read one node.
    Visit(SynNode<'tree>),
    /// Assemble a compound plan from the sub-plans on top of the output stack.
    Finish(ReadFrame<'tree>),
}

/// A compound plan waiting on its sub-plans.
enum ReadFrame<'tree>
{
    /// A constructor pattern with `usize` arguments.
    Ctor
    {
        /// The constructor's name.
        name: String,
        /// The constructor pattern node.
        node: SynNode<'tree>,
        /// The argument count.
        arity: usize,
    },
    /// An as-pattern over one sub-plan.
    As
    {
        /// The binder name.
        binder: String,
        /// The binder node.
        node: SynNode<'tree>,
    },
    /// An or-pattern over `usize` alternatives.
    Or
    {
        /// The or-pattern node.
        node: SynNode<'tree>,
        /// The alternative count.
        arity: usize,
    },
}

/// Reads one pattern node into the compiler's view.
///
/// # Contract
/// - requires: `node` sits in a pattern slot — a case arm's pattern, a binding
///   statement's binder, or a sub-pattern of one.
/// - ensures: total on any parse; a form the compiler does not emit becomes
///   [`PatternPlan::Declined`] carrying that form's kind and range.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — a form that fails to cross this reading is invisible to
///   the compiler afterwards and degrades into a declined arm rather than
///   failing, so the witnesses drive each compiled form through to its
///   observable outcome rather than asserting the reading itself. A reading
///   that dropped a form would turn its witness's expected value into a
///   `Blame::Hole` or a lowering decline.
/// - witness: `pattern_holes::tests::a_nested_constructor_pattern_binds_and_runs`
/// - witness: `pattern_holes::tests::an_as_binder_over_a_hole_does_not_settle_it`
/// - witness: `pattern_holes::tests::an_or_pattern_of_holes_is_indeterminate`
/// - witness: `pattern_holes::tests::an_uncompiled_arm_is_declined_and_a_hole_is_not`
///
/// # Intension
/// The walk carries an explicit work stack. Patterns come from user source, so
/// nesting depth is an input-controlled resource, and a reading that recursed
/// would make a deeply nested pattern a way to end the process rather than a
/// way to get an answer.
pub(super) fn read_pattern(node: SynNode<'_>) -> PatternPlan<'_>
{
    let mut work: Vec<ReadStep<'_>> = alloc::vec![ReadStep::Visit(node)];
    let mut built: Vec<PatternPlan<'_>> = Vec::new();
    while let Some(step) = work.pop() {
        match step {
            | ReadStep::Visit(node) => visit(node, &mut work, &mut built),
            | ReadStep::Finish(frame) => finish(frame, &mut built),
        }
    }
    built.pop().unwrap_or_else(|| declined(node))
}

/// Reads one node: emits a leaf plan, or queues a compound plan's assembly
/// followed by its children in source order.
fn visit<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<ReadStep<'tree>>,
    built: &mut Vec<PatternPlan<'tree>>,
)
{
    if bool::from(node.is_error()) {
        built.push(declined(node));
        return;
    }
    match node.kind() {
        | node_kinds::HOLE => built.push(PatternPlan::Hole {
            node,
            name: node
                .child_by_field_name(node_kinds::FIELD_HOLE_NAME)
                .map(|name| name.text().as_ref().to_owned()),
        }),
        // The wildcard, and a rest pattern outside a list: both stand for
        // whatever they are applied to and test nothing.
        | node_kinds::WILDCARD | node_kinds::REST_PATTERN => built.push(PatternPlan::Discard),
        | node_kinds::IDENTIFIER => built.push(PatternPlan::Bind {
            name: node.text().as_ref().to_owned(),
            node,
        }),
        // A bare nullary constructor reaching a pattern slot as a token.
        | node_kinds::CONSTRUCTOR => built.push(PatternPlan::Ctor {
            name: node.text().as_ref().to_owned(),
            node,
            arguments: Vec::new(),
        }),
        | node_kinds::CONSTRUCTOR_PATTERN => visit_constructor(node, work, built),
        // A parenthesized pattern is its interior; the parens only group.
        | node_kinds::PARENTHESIZED_EXPRESSION => match node.named_children().into_iter().next() {
            | Some(inner) => work.push(ReadStep::Visit(inner)),
            | None => built.push(declined(node)),
        },
        | node_kinds::OR_PATTERN => visit_or(node, work, built),
        | node_kinds::AS_PATTERN => visit_as(node, work, built),
        | _ => built.push(declined(node)),
    }
}

/// The declined plan for one node.
fn declined(node: SynNode<'_>) -> PatternPlan<'_>
{
    PatternPlan::Declined {
        kind: node.kind(),
        byte_range: node.byte_range(),
    }
}

/// Queues a constructor pattern `C` / `C(p̄)`.
fn visit_constructor<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<ReadStep<'tree>>,
    built: &mut Vec<PatternPlan<'tree>>,
)
{
    let Some(name) = node
        .child_by_field_name(node_kinds::FIELD_CONSTRUCTOR)
        .map(|constructor| constructor.text().as_ref().to_owned())
        .filter(|name| !name.is_empty())
    else {
        built.push(declined(node));
        return;
    };
    let arguments: Vec<SynNode<'tree>> = node.children_by_field_name(node_kinds::FIELD_ARGUMENT);
    queue(
        work,
        ReadFrame::Ctor {
            name,
            node,
            arity: arguments.len(),
        },
        arguments,
    );
}

/// Queues an or-pattern from its operands.
fn visit_or<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<ReadStep<'tree>>,
    built: &mut Vec<PatternPlan<'tree>>,
)
{
    let alternatives = node.named_children();
    if alternatives.len() < 2 {
        built.push(declined(node));
        return;
    }
    queue(
        work,
        ReadFrame::Or {
            node,
            arity: alternatives.len(),
        },
        alternatives,
    );
}

/// Queues an as-pattern from its tested pattern and its binder.
fn visit_as<'tree>(
    node: SynNode<'tree>,
    work: &mut Vec<ReadStep<'tree>>,
    built: &mut Vec<PatternPlan<'tree>>,
)
{
    let mut children = node.named_children().into_iter();
    let (Some(pattern), Some(binder)) = (children.next(), children.next())
    else {
        built.push(declined(node));
        return;
    };
    queue(
        work,
        ReadFrame::As {
            binder: binder.text().as_ref().to_owned(),
            node: binder,
        },
        alloc::vec![pattern],
    );
}

/// Queues a compound plan's assembly followed by its children, so the children
/// are visited in source order and their plans are on the stack when the frame
/// is assembled.
fn queue<'tree>(
    work: &mut Vec<ReadStep<'tree>>,
    frame: ReadFrame<'tree>,
    children: Vec<SynNode<'tree>>,
)
{
    work.push(ReadStep::Finish(frame));
    for child in children.into_iter().rev() {
        work.push(ReadStep::Visit(child));
    }
}

/// Assembles one compound plan from the sub-plans on top of `built`.
fn finish<'tree>(
    frame: ReadFrame<'tree>,
    built: &mut Vec<PatternPlan<'tree>>,
)
{
    match frame {
        | ReadFrame::Ctor { name, node, arity } => {
            let arguments = take(built, arity.into());
            built.push(PatternPlan::Ctor {
                name,
                node,
                arguments,
            });
        },
        | ReadFrame::As { binder, node } => {
            let pattern = take(built, SubPatternCount::from(1_usize))
                .into_iter()
                .next()
                .unwrap_or_else(|| declined(node));
            built.push(PatternPlan::As {
                pattern: Box::new(pattern),
                binder,
                node,
            });
        },
        | ReadFrame::Or { node, arity } => {
            let alternatives = take(built, arity.into());
            built.push(PatternPlan::Or { alternatives, node });
        },
    }
}

/// Takes the last `count` plans off `built`, restored to source order.
///
/// A short stack is padded with declines rather than truncated, so a frame
/// whose children did not all arrive still assembles at the right arity.
fn take<'tree>(
    built: &mut Vec<PatternPlan<'tree>>,
    count: SubPatternCount,
) -> Vec<PatternPlan<'tree>>
{
    let count = usize::from(count);
    let mut taken: Vec<PatternPlan<'tree>> = Vec::with_capacity(count);
    for _ in 0 .. count {
        taken.push(built.pop().unwrap_or(PatternPlan::Declined {
            kind: node_kinds::WILDCARD,
            byte_range: SourceRange(0 .. 0),
        }));
    }
    taken.reverse();
    taken
}

impl<'tree> PatternPlan<'tree>
{
    /// The hole whose indeterminacy this pattern cannot get past, when the
    /// pattern is undecidable **for every scrutinee**.
    ///
    /// A top-level hole is the base case; an as-binder is transparent, and an
    /// or-pattern qualifies only when every alternative does, because one
    /// decidable alternative is a test the scrutinee can settle.
    ///
    /// # Contract
    /// - ensures: `Some` only when no scrutinee whatever satisfies or refutes
    ///   this pattern.
    /// - panics: none.
    ///
    /// # Intension
    /// Iterative: an or-pattern's alternatives are walked from an explicit
    /// stack, and the reported hole is the first in source order.
    pub(super) fn unconditional_hole(&self) -> Option<(SynNode<'tree>, Option<String>)>
    {
        let mut first: Option<(SynNode<'tree>, Option<String>)> = None;
        let mut pending: Vec<&Self> = alloc::vec![self];
        while let Some(plan) = pending.pop() {
            match *plan {
                | Self::Hole { node, ref name } => {
                    if first.is_none() {
                        first = Some((node, name.clone()));
                    }
                },
                | Self::As { ref pattern, .. } => pending.push(pattern),
                | Self::Or {
                    ref alternatives, ..
                } => pending.extend(alternatives.iter().rev()),
                | Self::Discard | Self::Bind { .. } | Self::Ctor { .. } | Self::Declined { .. } => {
                    return None;
                },
            }
        }
        first
    }
}

/// A pattern hole that stops a match, with everything a stuck slot needs.
///
/// # Contract
/// - ensures: one value per *written* hole — the identity is minted once and
///   shared by every slot the hole stops, so a single unfinished test reports a
///   single goal.
#[derive(Clone)]
pub(super) struct StuckPattern
{
    /// The core hole identity every stuck slot carries.
    hole: HoleId,
    /// The origin entry, carrying [`HoleNote::IndeterminatePattern`].
    note: OriginEntry,
}

/// One nesting step of a matched arm, in the order the tests run.
///
/// The list is flat and folded outward-in from the arm body, so a layer earlier
/// in the list encloses every later one. That ordering is what makes the tests
/// conjunctive: an earlier test refuting means a later one never runs.
pub(super) enum Layer<'tree>
{
    /// Switch on a nested constructor: eliminate `value` and continue under the
    /// named tag, with every other tag a missing-arm hole.
    Switch
    {
        /// The core variable holding the value being eliminated.
        value: String,
        /// The datatype the constructor belongs to.
        data: String,
        /// The constructor's tag.
        tag: ConstructorTag,
        /// The names bound to the constructor's fields, in declaration order.
        components: Vec<String>,
        /// The constructor pattern node, for the synthesized node's origin.
        node: SynNode<'tree>,
    },
    /// Bind `name` to `value` for the arm body — an as-binder, or a user binder
    /// beneath a nested test.
    Alias
    {
        /// The core variable holding the value being named.
        value: String,
        /// The name the arm body sees.
        name: String,
        /// The binder node, for the synthesized node's origin.
        node: SynNode<'tree>,
    },
}

/// What the compiler concluded about one arm, before any tag is considered.
pub(super) enum ArmVerdict<'tree>
{
    /// A constructor-headed arm: it fills exactly the named tag, and nothing
    /// else.
    Head
    {
        /// The datatype every arm of this `case` must share.
        data: String,
        /// The tag this arm fills.
        tag: ConstructorTag,
        /// The constructor's argument patterns, in declaration order.
        arguments: Vec<PatternPlan<'tree>>,
        /// The as-binders wrapping the head, outermost first.
        aliases: Vec<(String, SynNode<'tree>)>,
        /// The constructor pattern node.
        node: SynNode<'tree>,
    },
    /// The arm's verdict is undecidable for every scrutinee, so every tag it
    /// reaches is stuck on this hole.
    Indeterminate(
        /// The hole every tag this arm reaches is stuck on.
        StuckPattern,
    ),
    /// The arm is outside the compiled set and is treated exactly as an
    /// unreadable arm: strict mode fails, total mode skips it.
    Declined
    {
        /// The declined form's kind.
        kind: SyntaxKind,
        /// The declined form's byte range.
        byte_range: SourceRange,
    },
}

impl ArmVerdict<'_>
{
    /// The [`LowerError`] a declined arm raises in strict mode.
    pub(super) fn declined_error(
        kind: SyntaxKind,
        byte_range: SourceRange,
    ) -> LowerError
    {
        LowerError::Unsupported { kind, byte_range }
    }
}

impl Lowerer<'_>
{
    /// Classifies one arm's pattern into the verdict the tag loop reads.
    ///
    /// # Contract
    /// - ensures: a constructor-headed pattern whose head resolves and whose
    ///   argument count equals the constructor's declared field count is
    ///   [`ArmVerdict::Head`]; an unconditionally indeterminate pattern mints
    ///   one hole identity and is [`ArmVerdict::Indeterminate`]; everything
    ///   else is [`ArmVerdict::Declined`] naming the form.
    /// - ensures: every user binder the verdict admits is reported to
    ///   recognition, so a binder shadowing an outermost-scope root is
    ///   diagnosed exactly as it was before this seam existed.
    /// - fails: propagates [`LowerError::ShadowedBuiltin`] from the binder
    ///   report under a refusing policy.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the three verdicts are separated by what the pattern
    ///   settles: a resolved head settles which tag the arm fills, a hole
    ///   settles nothing, and an uncompiled form settles nothing this
    ///   eliminator can act on. A classifier collapsing indeterminate into
    ///   declined loses the stuck arm and reproduces the dropped-arm behavior
    ///   this seam exists to end.
    /// - witness: `pattern_holes::tests::a_hole_shadows_the_arms_written_after_it`
    /// - witness: `pattern_holes::tests::an_unfilled_hole_never_runs_its_own_arm`
    /// - witness: `pattern_holes::tests::an_uncompiled_arm_is_declined_and_a_hole_is_not`
    pub(super) fn classify_arm<'tree>(
        &mut self,
        plan: PatternPlan<'tree>,
    ) -> LowerResult<ArmVerdict<'tree>>
    {
        if let Some((node, name)) = plan.unconditional_hole() {
            return Ok(ArmVerdict::Indeterminate(self.mint_stuck(node, name)));
        }
        // Peel the as-binders: they name the whole match and change no test.
        let mut aliases: Vec<(String, SynNode<'tree>)> = Vec::new();
        let mut current = plan;
        while let PatternPlan::As {
            pattern,
            binder,
            node,
        } = current
        {
            self.note_binder(binder.as_str().into(), node)?;
            aliases.push((binder, node));
            current = *pattern;
        }
        let PatternPlan::Ctor {
            name,
            node,
            arguments,
        } = current
        else {
            let (kind, byte_range) = match current {
                | PatternPlan::Declined { kind, byte_range } => (kind, byte_range),
                | PatternPlan::Discard => (node_kinds::WILDCARD, SourceRange(0 .. 0)),
                | PatternPlan::Bind { node, .. }
                | PatternPlan::Or { node, .. }
                | PatternPlan::Hole { node, .. } => (node.kind(), node.byte_range()),
                | PatternPlan::As { node, .. } | PatternPlan::Ctor { node, .. } => {
                    (node.kind(), node.byte_range())
                },
            };
            return Ok(ArmVerdict::Declined { kind, byte_range });
        };
        let Some(entry) = self.constructors.get(&name)
        else {
            return Ok(ArmVerdict::Declined {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        };
        let data = entry.0.clone();
        let tag = ConstructorTag::from(entry.1);
        Ok(ArmVerdict::Head {
            data,
            tag,
            arguments,
            aliases,
            node,
        })
    }
}

/// One step of the iterative nested-pattern walk.
struct NestStep<'tree>
{
    /// The core variable holding the value this plan is matched against.
    value: String,
    /// The plan to match.
    plan: PatternPlan<'tree>,
}

/// The nesting an arm's argument patterns compile to: the ordered tests, and
/// the hole (if any) that stops the match once every test has passed.
pub(super) struct ArmNesting<'tree>
{
    /// The tests, outermost first.
    layers: Vec<Layer<'tree>>,
    /// The hole the match is stuck on after every test above it has passed.
    ///
    /// **It is applied last rather than where it was written**, and the order
    /// is the whole correctness argument: a hole beside a test that refutes
    /// must not report indeterminacy, because the arm is refuted whatever the
    /// hole comes to be. Running every decidable test first and only then
    /// getting stuck is what keeps a determinate program determinate.
    stuck: Option<StuckPattern>,
}

impl Lowerer<'_>
{
    /// Names the constructor's fields and compiles whatever nesting its
    /// argument patterns carry.
    ///
    /// # Contract
    /// - requires: `arguments` has the constructor's declared field count.
    /// - ensures: one component name per field, in declaration order — the
    ///   user's own binder where the argument is a plain binder, so an arm the
    ///   pre-compilation reader already accepted keeps the exact core it had.
    /// - ensures: every user binder is reported to recognition.
    /// - fails: [`LowerError::Unsupported`] naming the first argument form
    ///   outside the compiled set.
    /// - panics: none.
    ///
    /// # Intension
    /// The walk carries an explicit work stack and appends layers in source
    /// pre-order, so an earlier layer encloses every later one. Pattern nesting
    /// depth is an input-controlled resource.
    pub(super) fn nest_arguments<'tree>(
        &mut self,
        arguments: Vec<PatternPlan<'tree>>,
    ) -> LowerResult<(Vec<String>, ArmNesting<'tree>)>
    {
        let (components, work) = self.name_arguments(arguments)?;
        let nesting = self.nest_layers(work)?;
        Ok((components, nesting))
    }

    /// Names one constructor's fields, and queues whatever of them still has a
    /// pattern to test.
    ///
    /// # Contract
    /// - ensures: one component name per argument, in declaration order, and
    ///   one queued step per argument that is neither a discard nor a plain
    ///   binder.
    /// - ensures: the queued steps are returned in **pop** order, so a caller
    ///   pushing them onto a stack visits the arguments in source order.
    /// - fails: propagates the binder report under a refusing policy.
    /// - panics: none.
    fn name_arguments<'tree>(
        &mut self,
        arguments: Vec<PatternPlan<'tree>>,
    ) -> LowerResult<(Vec<String>, Vec<NestStep<'tree>>)>
    {
        let mut components: Vec<String> = Vec::with_capacity(arguments.len());
        let mut work: Vec<NestStep<'tree>> = Vec::new();
        for argument in arguments {
            match argument {
                | PatternPlan::Discard => {
                    components.push(node_kinds::DISCARD_BINDER.to_owned());
                },
                // A plain binder names the field directly, which is what the
                // constructor-shaped reader did before this seam existed.
                | PatternPlan::Bind { name, node } => {
                    self.note_binder(name.as_str().into(), node)?;
                    components.push(name);
                },
                | plan => {
                    let value = self.fresh_name();
                    components.push(value.clone());
                    work.push(NestStep { value, plan });
                },
            }
        }
        // The stack pops last-pushed first; reversing puts argument zero on top.
        work.reverse();
        Ok((components, work))
    }

    /// Drains the nesting work stack into ordered layers.
    ///
    /// # Contract
    /// - ensures: layers come out in source pre-order, so an earlier layer
    ///   encloses every later one.
    /// - fails: [`LowerError::Unsupported`] naming the first sub-pattern form
    ///   outside the compiled set.
    /// - panics: none.
    ///
    /// # Intension
    /// One stack drains the whole pattern tree: a nested constructor's own
    /// arguments are pushed back onto **this** stack rather than reached by a
    /// nested call, so pattern depth costs heap and never host stack.
    fn nest_layers<'tree>(
        &mut self,
        mut work: Vec<NestStep<'tree>>,
    ) -> LowerResult<ArmNesting<'tree>>
    {
        let mut layers: Vec<Layer<'tree>> = Vec::new();
        let mut stuck: Option<StuckPattern> = None;
        while let Some(NestStep { value, plan }) = work.pop() {
            match plan {
                | PatternPlan::Discard => {},
                | PatternPlan::Hole { node, name } => {
                    if stuck.is_none() {
                        stuck = Some(self.mint_stuck(node, name));
                    }
                },
                | PatternPlan::Bind { name, node } => {
                    self.note_binder(name.as_str().into(), node)?;
                    layers.push(Layer::Alias { value, name, node });
                },
                | PatternPlan::As {
                    pattern,
                    binder,
                    node,
                } => {
                    self.note_binder(binder.as_str().into(), node)?;
                    layers.push(Layer::Alias {
                        value: value.clone(),
                        name: binder,
                        node,
                    });
                    work.push(NestStep {
                        value,
                        plan: *pattern,
                    });
                },
                | PatternPlan::Ctor {
                    name,
                    node,
                    arguments,
                } => {
                    let Some(resolved) = self.constructors.get(&name)
                    else {
                        return Err(ArmVerdict::declined_error(node.kind(), node.byte_range()));
                    };
                    let data = resolved.0.clone();
                    let tag = ConstructorTag::from(resolved.1);
                    let arity = usize::from(self.field_arity(data.as_str().into(), tag));
                    if arguments.len() != arity {
                        return Err(ArmVerdict::declined_error(node.kind(), node.byte_range()));
                    }
                    let (components, sub) = self.name_arguments(arguments)?;
                    layers.push(Layer::Switch {
                        value,
                        data,
                        tag,
                        components,
                        node,
                    });
                    // The sub-patterns rejoin this same stack, so the walk
                    // stays one loop however deep the pattern nests.
                    work.extend(sub);
                },
                | PatternPlan::Or { node, .. } => {
                    return Err(ArmVerdict::declined_error(node.kind(), node.byte_range()));
                },
                | PatternPlan::Declined { kind, byte_range } => {
                    return Err(ArmVerdict::declined_error(kind, byte_range));
                },
            }
        }
        Ok(ArmNesting { layers, stuck })
    }
}

impl Lowerer<'_>
{
    /// Binds a constructor's payload for one eliminator arm.
    ///
    /// # Contract
    /// - ensures: a nullary constructor binds a discard over the unit payload,
    ///   a one-field constructor binds the field directly, and a many-field
    ///   constructor binds a fresh payload variable positionally destructured
    ///   into `components` — the arity discipline the declared-data design
    ///   fixes, unchanged by pattern compilation.
    /// - fails: propagates a readback failure from the wrapped body.
    /// - panics: none.
    pub(super) fn bind_payload(
        &mut self,
        node: SynNode<'_>,
        components: &[String],
        body: COut,
    ) -> LowerResult<(String, COut)>
    {
        match (components.first(), components.len()) {
            | (_, 0) => Ok((node_kinds::DISCARD_BINDER.to_owned(), body)),
            | (Some(sole), 1) => Ok((sole.clone(), body)),
            | _ => self.destructure_payload(node, components, body),
        }
    }

    /// Folds one arm's nesting over its body, innermost layer first.
    ///
    /// # Contract
    /// - ensures: the result runs every layer's test in the layers' own order,
    ///   reaching `body` exactly when all of them pass; a tag no layer names is
    ///   a [`HoleNote::MissingCaseArm`] hole, because a nested test that fails
    ///   has no later arm to fall through to — the compiler declines every arm
    ///   set in which one would.
    /// - ensures: a nesting carrying a stuck hole reaches that hole rather than
    ///   `body`, after every test above it has passed.
    /// - fails: propagates readback and hole-construction failures.
    /// - panics: none.
    ///
    /// # Intension
    /// The fold walks the flat layer list in reverse and allocates no host
    /// stack frame per layer, so pattern nesting depth costs heap rather than
    /// stack.
    pub(super) fn fold_nesting(
        &mut self,
        nesting: ArmNesting<'_>,
        body: COut,
    ) -> LowerResult<COut>
    {
        let ArmNesting { layers, stuck } = nesting;
        let mut acc = match stuck {
            | Some(ref stuck) => Self::stuck_slot(stuck)?,
            | None => body,
        };
        for layer in layers.into_iter().rev() {
            acc = match layer {
                | Layer::Alias { value, name, node } => {
                    Self::alias_value(node, value.as_str().into(), name, acc)?
                },
                | Layer::Switch {
                    value,
                    data,
                    tag,
                    components,
                    node,
                } => {
                    let (payload, wrapped) = self.bind_payload(node, &components, acc)?;
                    self.switch_on(
                        node,
                        value.as_str().into(),
                        data.as_str().into(),
                        tag,
                        payload,
                        wrapped,
                    )?
                },
            };
        }
        Ok(acc)
    }

    /// Names `value` for the arm body: `p as x` and a binder beneath a nested
    /// test both reach the body this way.
    ///
    /// # Contract
    /// - ensures: `body` runs with `name` bound to the value `value` holds, and
    ///   `value` keeps exactly one occurrence — the binding is a `Bind` over a
    ///   `Ret` rather than a substitution, so no binder can capture and no
    ///   sub-term is duplicated.
    /// - fails: propagates readback failure from `body`.
    /// - panics: none.
    pub(super) fn alias_value(
        node: SynNode<'_>,
        value: CoreVariableName<'_>,
        name: String,
        body: COut,
    ) -> LowerResult<COut>
    {
        let alias = entry(node, Some(ElabKind::PatternAlias));
        let comp = Comp::Bind(
            Rc::new(Comp::Ret(Rc::new(Value::var(value.as_ref())))),
            name,
            Rc::new({
                let readback_comp = body.readback_comp()?;
                core::convert::identity(readback_comp)
            }),
        );
        let bound = OriginNode::new(alias.clone(), alloc::vec![OriginNode::leaf(alias.clone())]);
        COut::from_legacy_comp(
            &comp,
            OriginNode::new(alias, alloc::vec![bound, body.origin]),
        )
    }

    /// Builds a nested `DataCase` on `value`: the named tag runs `body`, and
    /// every other tag is a missing-arm hole.
    ///
    /// # Contract
    /// - ensures: the arms vector has the datatype's constructor count, with
    ///   `body` at `tag` and a hole at every other tag.
    /// - fails: propagates readback and hole-construction failures.
    /// - panics: none.
    fn switch_on(
        &mut self,
        node: SynNode<'_>,
        value: CoreVariableName<'_>,
        data: TypeName<'_>,
        tag: ConstructorTag,
        payload: String,
        body: COut,
    ) -> LowerResult<COut>
    {
        let count = usize::from(self.constructor_count(data));
        let scrut = Value::var(value.as_ref());
        let mut arms: Vec<(String, Rc<Comp>)> = Vec::with_capacity(count);
        let mut origins: Vec<OriginNode> =
            alloc::vec![OriginNode::leaf(entry(node, Some(ElabKind::PatternNest)))];
        let mut placed = Some((payload, body));
        for slot in 0 .. count {
            let (binder, arm_body) = if slot == usize::from(tag) {
                match placed.take() {
                    | Some(placed) => placed,
                    | None => self.missing_arm(node)?,
                }
            }
            else {
                self.missing_arm(node)?
            };
            arms.push((
                binder,
                Rc::new({
                    let readback_comp = arm_body.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            ));
            origins.push(arm_body.origin);
        }
        COut::from_legacy_comp(
            &Comp::DataCase(Rc::new(scrut), arms),
            OriginNode::new(entry(node, Some(ElabKind::PatternNest)), origins),
        )
    }

    /// The discard binder and hole one unreached constructor tag takes.
    ///
    /// # Contract
    /// - ensures: the hole carries [`HoleNote::MissingCaseArm`], so a tag no
    ///   arm reaches stays distinguishable from a tag a hole made stuck.
    /// - fails: propagates hole construction failure.
    /// - panics: none.
    pub(super) fn missing_arm(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<(String, COut)>
    {
        let error = LowerError::MissingCaseArm {
            constructor: node_kinds::DISCARD_BINDER,
            byte_range: node.byte_range(),
        };
        let hole = self.comp_hole(node, &error)?;
        Ok((node_kinds::DISCARD_BINDER.to_owned(), hole))
    }

    /// The declared constructor count of `data`, or zero when the datatype is
    /// not in scope.
    pub(super) fn constructor_count(
        &self,
        data: TypeName<'_>,
    ) -> ConstructorCount
    {
        self.data
            .get(data.as_ref())
            .map_or(0, |decl| decl.ctors.len())
            .into()
    }
}

impl Lowerer<'_>
{
    /// The stuck hole one arm forces on every slot it reaches, when the arm's
    /// pattern is undecidable for **every** scrutinee.
    ///
    /// This is the eliminator-independent half of pattern compilation: an arm
    /// that settles nothing settles nothing whatever the scrutinee's
    /// constructor family is, so the sum, list, and declared-data eliminators
    /// all read it the same way and the answer needs no type information.
    ///
    /// # Contract
    /// - ensures: `Some` exactly when the arm's pattern is a hole under any
    ///   number of as-binders, or an or-pattern every alternative of which is;
    ///   `None` for a pattern any scrutinee can satisfy or refute.
    /// - ensures: one hole identity per arm, so a single unfinished test
    ///   reports one goal however many slots it stops.
    /// - fails: propagates the arm's malformed-node error.
    /// - panics: none.
    pub(super) fn arm_indeterminacy(
        &mut self,
        arm_node: SynNode<'_>,
    ) -> LowerResult<Option<StuckPattern>>
    {
        let pattern = super::required_field(arm_node, node_kinds::FIELD_PATTERN)?;
        let Some((node, name)) = read_pattern(pattern).unconditional_hole()
        else {
            return Ok(None);
        };
        Ok(Some(self.mint_stuck(node, name)))
    }

    /// Mints the one identity a written pattern hole carries wherever it stops
    /// a match.
    ///
    /// # Contract
    /// - ensures: the identity comes from the lowerer's own hole allocator, so
    ///   a pattern hole addresses the same way an expression hole does and the
    ///   goal stream needs no second addressing scheme.
    /// - panics: none.
    pub(super) fn mint_stuck(
        &mut self,
        node: SynNode<'_>,
        name: Option<String>,
    ) -> StuckPattern
    {
        StuckPattern {
            hole: self.fresh_hole().into(),
            note: super::user_hole_entry(node, HoleNote::IndeterminatePattern { name }),
        }
    }

    /// The computation one stuck slot runs: a `Comp::Hole` carrying the
    /// pattern hole's own identity and origin.
    ///
    /// # Contract
    /// - ensures: the term is `Comp::Hole(hole)` and the origin carries
    ///   [`HoleNote::IndeterminatePattern`], so a reader can tell an
    ///   indeterminate slot from a missing one.
    /// - fails: propagates construction failure.
    /// - panics: none.
    pub(super) fn stuck_slot(stuck: &StuckPattern) -> LowerResult<COut>
    {
        COut::from_legacy_comp(
            &Comp::Hole(stuck.hole),
            OriginNode::leaf(stuck.note.clone()),
        )
    }
}

impl Lowerer<'_>
{
    /// The body one unfilled binary-eliminator slot takes.
    ///
    /// The sum and list eliminators have a fixed pair of slots rather than a
    /// declared constructor family, so their unfilled slots are settled here
    /// instead of in the tag walk. The two reasons a slot can be unfilled stay
    /// apart: an arm that settles nothing leaves every later slot **stuck** on
    /// its hole, and a slot no arm named at all is **missing**.
    ///
    /// # Contract
    /// - ensures: with `stuck`, the hole carries
    ///   [`HoleNote::IndeterminatePattern`] and is produced in strict mode as
    ///   well as total mode, because a hole the user wrote is a legitimate term
    ///   rather than a recovery artifact.
    /// - ensures: without `stuck`, the behaviour is the missing-arm behaviour
    ///   unchanged — [`LowerError::MissingCaseArm`] in strict mode, a hole
    ///   carrying that note in total mode.
    /// - fails: [`LowerError::MissingCaseArm`] in strict mode when the slot is
    ///   genuinely missing.
    /// - panics: none.
    pub(super) fn unfilled_slot(
        &mut self,
        node: SynNode<'_>,
        constructor: ConstructorName<'static>,
        stuck: Option<&StuckPattern>,
    ) -> LowerResult<COut>
    {
        if let Some(stuck) = stuck {
            return Self::stuck_slot(stuck);
        }
        let error = LowerError::MissingCaseArm {
            constructor: constructor.0,
            byte_range: node.byte_range(),
        };
        if !bool::from(self.total()) {
            return Err(error);
        }
        self.comp_hole(node, &error)
    }
}
