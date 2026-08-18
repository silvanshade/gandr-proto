//! The total **semantic marking** layer (ADR-17 "marks not aborts"; Zhao et
//! al., *Total Type Error Localization and Recovery with Holes*, POPL 2024).
//!
//! This is a *third* realization of the type system, additive to the
//! [`crate::judgements::checker`] / `gandr_core_machine` pair and **never a
//! modification of either** — the conformance lockstep (ADR-9) stays untouched.
//! Where the checker is fail-fast (the first
//! [`gandr_core_term::error::TypeError`] short-circuits the whole derivation),
//! the marker is **total over type errors**: it decorates *every* node, and at
//! each of the five abort sites it records a localized [`Mark`] and recovers,
//! so typing continues. (Totality is over *type* errors only — the traversal is
//! direct-style, as the checker, so a term that exceeds the host call stack
//! still aborts the process; see [`mark_value`].) Recovery generalizes the
//! existing hole discipline — a node whose subsumption, shape, grade, scope, or
//! rule lookup fails recovers with the matched-`Unknown` type the checker
//! already uses for [`gandr_core_term::syntax::Value::Hole`] (`subtype.rs`,
//! `stack.rs`). Every recovery fallback is `Unknown` / `EffectRow::EMPTY` —
//! consistent/bottom in `subtype.rs`, hence *absorbing*: a recovered child can
//! never make a parent's subsumption fail, which is exactly what keeps recovery
//! from cascading spurious marks (the no-false-positive half of the oracle). A
//! refactor must keep that fallback invariant or the oracle's accept direction
//! breaks.
//!
//! # The oracle against the checker (the soundness anchor)
//!
//! The marker is not held honest by the lockstep but by an **oracle** against
//! the recursive checker, pinned by the property tests below: for every
//! `(term, dir)`,
//!
//! * the checker *accepting* (`Ok(ty)`) is equivalent to the marking carrying
//!   **no error mark** (empty-hole marks excepted) **and** synthesizing the
//!   same root type `ty`;
//! * the checker *rejecting* (`Err`) forces **at least one error mark**;
//! * the marking is **total** — every `origin`-addressable node has a
//!   [`NodeFacts`], on every input, with no panic.
//!
//! The oracle is *tight*, not approximate: recovery is type-stable (in checking
//! mode a failed node recovers with the *expected* type — exactly what the
//! inlined Sub rule [`crate::discipline::subtype::finish_value`] returns on
//! success), so a well-typed program takes only success paths and the marker's
//! per-node types coincide with the checker's there.
//!
//! # Decoration is a side-table keyed by stable node identity
//!
//! A [`Marking`] is a `BTreeMap<MarkNodeId, NodeFacts>`: the dual analyzed /
//! synthesized type, the node's marks, and a dirty bit, keyed by a stable
//! arena node identity. A separate compatibility path snapshot mirrors the
//! pipeline's `origin::resolve` child-index convention for source-span
//! rendering only. Decoration is deliberately **not** an inline field on the
//! `Rc`-shared `Value` / `Comp` nodes: that would corrupt their derived
//! `PartialEq` (load-bearing for conformance and the trace equality) and leak
//! typing artifacts into the machine trace.
//!
//! # The carrier and the mark taxonomy
//!
//! Marks reconcile the syntactic empty-hole mark and the five semantic failure
//! kinds into **one** discipline, multiplied with the spec's grade-budget /
//! effect-row / thunkability kinds. The Pantograph typed error-boundary
//! `{t}_{T1/T2}` (Prinz et al., POPL 2025, harvest-only) is realized as the
//! node's dual-type [`NodeFacts`] plus a [`Boundary`] on the mismatch marks —
//! it never truncates the (reusable) decoration.
//!
//! # Scope
//!
//! This module is span-free (compatibility paths carry child indices, not byte
//! ranges), so it lives beside the checker it is oracle-bound to. Wiring marks
//! to source ranges (the reserved `diag::Report.marks` slot, compatibility path
//! ➜ stable origin id ➜ span) and setting the dirty bit from the edit / order-
//! maintenance layer is the deferred pipeline-side consumer; [`NodeFacts`]
//! carries `dirty` (the representation is complete) but the marker leaves it at
//! its `false` default. A reified stack [`gandr_core_term::syntax::Value::Stk`]
//! is typed with full `Γ; answer` fidelity, and its interior frames are
//! decorated under explicit compatibility paths as *bonus* entries (they are
//! not `origin::resolve`-addressable — the Stk-descent resync is deferred).

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec::IntoIter;
use alloc::vec::Vec;

use gandr_core_term::boundary::ContextLength;
use gandr_core_term::boundary::ErrorStatus;
use gandr_core_term::boundary::I64Literal;
use gandr_core_term::boundary::MarkingEmptyStatus;
use gandr_core_term::boundary::PathIndex;
use gandr_core_term::boundary::PathSegments;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::effect::EffectRow;
use gandr_core_term::effect::EffectSig;
use gandr_core_term::effect::combine_bind_row;
use gandr_core_term::effect::handle_natural_type;
use gandr_core_term::effect::resolve_handler_coverage;
use gandr_core_term::effect::resume_stack_type;
use gandr_core_term::error::TypeError;
use gandr_core_term::error::text;
use gandr_core_term::grade::Grade;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::CompNode;
use gandr_core_term::syntax::CompNodeId;
use gandr_core_term::syntax::FlatArena;
use gandr_core_term::syntax::HoleId;
use gandr_core_term::syntax::OpClause;
use gandr_core_term::syntax::OpClauseNode;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::SplitMotive;
use gandr_core_term::syntax::SplitMotiveNode;
use gandr_core_term::syntax::Stack;
use gandr_core_term::syntax::StackNode;
use gandr_core_term::syntax::StackNodeId;
use gandr_core_term::syntax::Value;
use gandr_core_term::syntax::ValueNode;
use gandr_core_term::syntax::ValueNodeId;
use gandr_core_term::syntax::ValueTypeNodeId;
use gandr_core_term::syntax::WalkBase;
use gandr_core_term::syntax::WalkBaseNode;
use gandr_core_term::syntax::WalkMotive;
use gandr_core_term::syntax::WalkMotiveNode;
use gandr_core_term::types::CompType;
use gandr_core_term::types::SealId;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

use crate::discipline::subtype::comp_subtype;
use crate::discipline::subtype::int_literal_fits;
use crate::discipline::subtype::pick;
use crate::discipline::subtype::value_subtype;
use crate::judgements::checker::base_diagonal_type;
use crate::judgements::checker::motive_result_type;
use crate::judgements::checker::split_expectations;
use crate::judgements::checker::split_unknown_expectations;
use crate::judgements::control::Dir;
use crate::judgements::control::unrc;
use crate::judgements::stack::arrow_components;
use crate::judgements::stack::returner_components;
use crate::judgements::stack::stk_components;
use crate::judgements::stack::with_component;

/// A stable node identity for marking facts.
///
/// The variants reuse the ADR-50 typed arena ids so value, computation, and
/// reified-stack nodes cannot be mixed accidentally. `BTreeMap` ordering over
/// this key preserves deterministic snapshots without depending on structural
/// path shape.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MarkNodeId
{
    /// A value node in the canonical value arena.
    Value(ValueNodeId),
    /// A computation node in the canonical computation arena.
    Comp(CompNodeId),
    /// A reified stack node in the canonical stack arena.
    Stack(StackNodeId),
}

/// The Pantograph typed error-boundary `{t}_{T1/T2}` (POPL 2025), realized
/// over CBPV types: the node's synthesized type `actual` (`T1`) paired with the
/// analyzed type `expected` (`T2`) it failed to meet.
///
/// Grade-budget (`U_r` vs `U_r'`) and effect-row (`F^ε` vs `F^ε'`) mismatch
/// information rides inside the [`Ty`] pair (the offending grade / row is a
/// sub-component of `expected` / `actual`); the dedicated [`Mark::GradeBudget`]
/// / [`Mark::Thunkability`] / [`Mark::EffectRowMismatch`] kinds name the
/// specialized cases.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Boundary
{
    /// The analyzed (checked-against) type `T2`.
    pub expected: Ty,
    /// The synthesized type `T1` the node actually produced.
    pub actual: Ty,
}

impl Boundary
{
    /// Builds a boundary from the expected and actual types.
    #[inline]
    #[must_use]
    pub fn new(
        expected: Ty,
        actual: Ty,
    ) -> Self
    {
        Self { expected, actual }
    }
}

/// One semantic mark on a node — the localized image of a [`TypeError`] abort,
/// reconciled with the syntactic empty-hole mark into a single discipline.
///
/// Every kind except [`Mark::EmptyHole`] and [`Mark::PatternHole`] is an
/// **error** mark ([`Mark::is_error`]); the two hole marks are
/// *complete-but-incomplete* nodes (the program is well-typed, the hole is
/// simply unfilled).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Mark
{
    /// An empty hole `?u` — the Hazel empty-hole mark, reconciling the
    /// syntactic hole into the one discipline. **Not** an error: the checker
    /// accepts holes (they infer `Unknown` and check against anything).
    EmptyHole(
        /// The hole's identifier.
        HoleId,
    ),
    /// An empty hole `?u` standing in **pattern** position — the position mark
    /// this discipline was missing, and the only thing that distinguishes an
    /// unfinished pattern from an unfinished expression once both are marks.
    ///
    /// **Not** an error, for the same reason [`Mark::EmptyHole`] is not: an
    /// unfilled pattern leaves the program well-typed and merely incomplete,
    /// so checking proceeds and the branches beside it still land.
    ///
    /// The kinds are kept apart because what a consumer may conclude from each
    /// is different. An expression hole stands for a *value* and infers
    /// `Unknown`; a pattern hole stands for a *test*, and the only sound
    /// reading of an unfinished test is that it might succeed and might fail.
    /// That indeterminacy is what a live match analysis propagates — a branch
    /// carrying one is neither satisfied nor refuted — and collapsing the two
    /// kinds would hand it an expression's reading of a pattern's hole.
    ///
    /// This module's own marker never emits it: the core calculus compiles
    /// patterns away before it is reached (`Comp::DataCase` carries tag-indexed
    /// arms, not patterns), so the mark is minted where patterns still exist,
    /// by the surface's pattern analysis. It lives here because the mark
    /// taxonomy is one taxonomy, and a second one on the surface side would be
    /// a fork of the discipline rather than an extension of it.
    PatternHole(
        /// The hole's identifier.
        HoleId,
    ),
    /// Subsumption failed: the synthesized type is not a consistent subtype of
    /// the analyzed type (the typed error-boundary `{t}_{actual ⇐ expected}`).
    TypeMismatch(
        /// The expected / actual boundary.
        Boundary,
    ),
    /// An effect-row mismatch: both sides are returners `F^ε A` with consistent
    /// payloads, but the synthesized row is not a subset of the expected row
    /// (`ε ⊄ ε′`) — the specialized boundary for effects.
    EffectRowMismatch(
        /// The expected / actual boundary (both are `F^…` returners).
        Boundary,
    ),
    /// An elimination's principal premise synthesized a type of the wrong shape
    /// (a non-arrow applied, a non-thunk forced, a non-returner bound, …).
    ShapeMismatch
    {
        /// A description of the expected type constructor (e.g. "an arrow
        /// type").
        expected: &'static str,
        /// The type the principal premise actually synthesized.
        actual: Ty,
    },
    /// A variable used with no hypothesis in scope.
    FreeVariable
    {
        /// The variable's name.
        name: String,
    },
    /// A grade-budget requirement `required ⊑ available` failed — the thunk's
    /// usage budget is too small for what is demanded (`thunk_r` checked
    /// against `U_s` with `s ⋢ r`, or `dup`'s conservation `r + s ⊑ g`).
    GradeBudget
    {
        /// The demanded grade `required`.
        required: Grade,
        /// The thunk's available grade.
        available: Grade,
    },
    /// A thunk cannot be forced at all: `force v` requires `1 ⊑ r` on `v ⇑ U_r
    /// B`, and that failed (the specialized grade-budget case `required = 1`).
    Thunkability
    {
        /// The thunk's available grade (`r`, which fails `1 ⊑ r`).
        available: Grade,
    },
    /// A well-formed node with no typing rule in this direction (an
    /// unannotated binder in inference mode, a bare injection, a check-only
    /// form away from its expectation, a `shift` with no enclosing `reset`, an
    /// undeclared effect operation, mismatched handler coverage, …).
    Stuck
    {
        /// A hint for making progress (the checker's `StuckExpr` hint).
        hint: &'static str,
    },
}

impl Mark
{
    /// Whether this mark is an **error** (every kind but the two hole marks).
    ///
    /// # Contract
    /// - ensures: returns `false` for [`Mark::EmptyHole`] and
    ///   [`Mark::PatternHole`] (complete-but-incomplete nodes the checker
    ///   accepts), `true` for every other kind.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the hole kinds and the error kinds are distinguished
    ///   by this predicate alone, so a pattern hole classified as an error
    ///   would make an unfinished pattern block its item — the failure the live
    ///   analysis exists to avoid.
    /// - witness: `mark::tests::pattern_holes_are_incomplete_not_erroneous`
    #[inline]
    #[must_use]
    pub fn is_error(&self) -> ErrorStatus
    {
        (!matches!(*self, Self::EmptyHole(_) | Self::PatternHole(_))).into()
    }
}

/// The per-node decoration: the dual analyzed / synthesized type, the node's
/// marks, and the incremental dirty bit (Porter's layout).
///
/// `analyzed` is the checking-direction expectation (`None` in inference
/// mode); `synthesized` is the type the node produced. `marks` are the node's
/// own marks (children's marks live on the children). `dirty` is the
/// representation slot for the incremental layer — the marker leaves it
/// `false` (its producer is the edit / order-maintenance layer, deferred).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeFacts
{
    /// The analyzed (checked-against) type, when the node was typed in checking
    /// mode; `None` in inference mode.
    pub analyzed: Option<Ty>,
    /// The synthesized type the node produced.
    pub synthesized: Option<Ty>,
    /// The node's own marks (children's marks are on the children).
    pub marks: Vec<Mark>,
    /// The incremental dirty bit (left `false` by the marker; set by the edit
    /// layer).
    pub dirty: bool,
}

impl NodeFacts
{
    /// Whether this node carries any **error** mark (empty holes excepted).
    ///
    /// # Contract
    /// - ensures: returns `true` iff at least one of `marks` is an error mark
    ///   ([`Mark::is_error`]).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn has_error(&self) -> ErrorStatus
    {
        self.marks
            .iter()
            .any(|mark| bool::from(mark.is_error()))
            .into()
    }
}

/// A complete marking of a term: every node's [`NodeFacts`], keyed by
/// stable [`MarkNodeId`], plus the root's synthesized type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Marking
{
    /// Per-node facts, keyed by stable arena identity (deterministic `BTreeMap`
    /// iteration for golden stability).
    facts: BTreeMap<MarkNodeId, NodeFacts>,
    /// Explicit compatibility lookup from structural child-index snapshots to
    /// stable node ids for path-oriented diagnostics/rendering.
    compatibility_paths: BTreeMap<Vec<u32>, MarkNodeId>,
    /// The root term's synthesized type (the value the checker would return on
    /// success).
    root: Ty,
}

impl Marking
{
    /// The facts at a stable node id, if any.
    ///
    /// # Contract
    /// - ensures: returns the [`NodeFacts`] keyed by `id`, or `None` when no
    ///   node is recorded there.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        id: MarkNodeId,
    ) -> Option<&NodeFacts>
    {
        self.facts.get(&id)
    }

    /// The facts at a compatibility structural path snapshot, if any.
    ///
    /// # Contract
    /// - ensures: resolves `path` through the explicit compatibility table and
    ///   returns the stable-id facts, or `None` when no node is recorded there.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn get_compat_path<'source, P>(
        &self,
        path: P,
    ) -> Option<&NodeFacts>
    where
        P: Into<PathSegments<'source>>,
    {
        let path = path.into();
        let id = self.compatibility_paths.get(path.as_ref())?;
        self.facts.get(id)
    }

    /// Iterates every `(stable_id, facts)` pair in stable-id order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&MarkNodeId, &NodeFacts)>
    {
        self.facts.iter()
    }

    /// Iterates explicit compatibility `(path_snapshot, stable_id)` pairs.
    #[inline]
    pub fn compatibility_paths(&self) -> impl Iterator<Item = (&[u32], &MarkNodeId)>
    {
        self.compatibility_paths
            .iter()
            .map(|(path, id)| (path.as_slice(), id))
    }

    /// The root term's synthesized type.
    ///
    /// # Contract
    /// - ensures: returns the type the root node synthesized — equal to the
    ///   checker's `Ok` result on a well-typed input.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn root_type(&self) -> &Ty
    {
        &self.root
    }

    /// The number of decorated nodes.
    #[inline]
    #[must_use]
    pub fn len(&self) -> ContextLength
    {
        self.facts.len().into()
    }

    /// Whether the marking is empty (it never is — the root is always
    /// decorated; provided for the `len`/`is_empty` lint pair).
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> MarkingEmptyStatus
    {
        self.facts.is_empty().into()
    }

    /// Every `(stable_id, mark)` pair, in stable-id order then mark order.
    #[inline]
    pub fn marks(&self) -> impl Iterator<Item = (&MarkNodeId, &Mark)>
    {
        self.facts
            .iter()
            .flat_map(|(id, facts)| facts.marks.iter().map(move |mark| (id, mark)))
    }

    /// Every error `(stable_id, mark)` pair, in stable-id order then mark
    /// order.
    #[inline]
    pub fn errors(&self) -> impl Iterator<Item = (&MarkNodeId, &Mark)>
    {
        self.marks()
            .filter(|&(_, mark)| bool::from(mark.is_error()))
    }

    /// Whether any node carries an **error** mark (empty holes excepted) — the
    /// oracle's "the checker rejected" witness.
    ///
    /// # Contract
    /// - ensures: returns `true` iff some node has an error mark
    ///   ([`Mark::is_error`]).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn has_errors(&self) -> ErrorStatus
    {
        self.facts
            .values()
            .any(|facts| bool::from(facts.has_error()))
            .into()
    }
}

/// Marks a value under a direction: the total counterpart of
/// [`crate::judgements::checker::run_value`].
///
/// # Contract
/// - ensures: returns a [`Marking`] decorating every `origin::resolve`-
///   addressable node of `value` (plus bonus interior nodes of any reified
///   stack); `root_type()` is `Ty::Value(_)`; on a checker-accepted input the
///   marking has no error mark and `root_type()` equals the checker's inferred
///   / checked type.
/// - provides: the total semantic marking (the agent stream's mark substrate).
/// - panics: none. "Total" means *type* totality — every node is decorated and
///   every `TypeError` abort site recovers to a mark. Like
///   [`crate::judgements::checker`] the traversal is direct-style (it recurses
///   on the host call stack), so a term whose nesting exceeds the thread's
///   stack aborts the process (stack overflow); there is no machine-backed
///   marker for adversarial-depth input.
#[inline]
#[must_use]
pub fn mark_value(
    ctx: Ctx,
    value: Value,
    dir: Dir<ValueType>,
) -> Marking
{
    let mut marker = Marker::new(ctx);
    let root_id = marker.intern_value(&value);
    let root = marker.value(value, root_id, dir);
    Marking {
        facts: marker.facts,
        compatibility_paths: marker.compatibility_paths,
        root: Ty::Value(root),
    }
}

/// Marks a computation under a direction: the total counterpart of
/// [`crate::judgements::checker::run_comp`].
///
/// # Contract
/// - ensures: returns a [`Marking`] decorating every `origin::resolve`-
///   addressable node of `comp` (plus bonus reified-stack interior nodes);
///   `root_type()` is `Ty::Comp(_)`; on a checker-accepted input the marking
///   has no error mark and `root_type()` equals the checker's inferred /
///   checked type.
/// - provides: the total semantic marking.
/// - panics: none — "total" is *type* totality (every node decorated, every
///   abort site recovered); the direct-style recursion still aborts the process
///   on a term that exceeds the host stack, as [`mark_value`].
#[inline]
#[must_use]
pub fn mark_comp(
    ctx: Ctx,
    comp: Comp,
    dir: Dir<CompType>,
) -> Marking
{
    let mut marker = Marker::new(ctx);
    let root_id = marker.intern_comp(&comp);
    let root = marker.comp(comp, root_id, dir);
    Marking {
        facts: marker.facts,
        compatibility_paths: marker.compatibility_paths,
        root: Ty::Comp(root),
    }
}

/// The marking traversal's state: the typing context, the ambient `reset`
/// answer (as [`crate::judgements::checker`]), the stable arena carrier, the
/// accumulating facts table, and the current compatibility path snapshot.
struct Marker
{
    /// The two-zone typing context `Γ; Σ`.
    ctx: Ctx,
    /// The ambient answer type the nearest enclosing `reset` established
    /// (`None` outside any `reset`); read by `shift`, set by `reset`, exactly
    /// as the checker's register.
    answer: Option<CompType>,
    /// Canonical arena nodes used to mint stable marking identities.
    arena: FlatArena,
    /// Shared legacy value nodes already interned into `arena`.
    value_ids: BTreeMap<*const Value, ValueNodeId>,
    /// Shared legacy computation nodes already interned into `arena`.
    comp_ids: BTreeMap<*const Comp, CompNodeId>,
    /// Shared legacy stack nodes already interned into `arena`.
    stack_ids: BTreeMap<*const Stack, StackNodeId>,
    /// The accumulating per-node facts, keyed by stable node identity.
    facts: BTreeMap<MarkNodeId, NodeFacts>,
    /// Explicit compatibility lookup from structural child-index snapshots to
    /// stable node identities.
    compatibility_paths: BTreeMap<Vec<u32>, MarkNodeId>,
    /// The compatibility path of the node currently being typed (extended on
    /// descent, popped on return).
    path: Vec<u32>,
}

/// A value node whose facts are pending until its rule has synthesized a type.
struct PendingValue
{
    /// Stable node id, absent when arena allocation failed.
    id: Option<ValueNodeId>,
    /// Checking-direction expectation, absent in inference mode.
    analyzed: Option<Ty>,
    /// Marks localized to this value node.
    marks: Vec<Mark>,
}

/// A computation node whose facts are pending until its rule has synthesized a
/// type.
struct PendingComp
{
    /// Stable node id, absent when arena allocation failed.
    id: Option<CompNodeId>,
    /// Checking-direction expectation, absent in inference mode.
    analyzed: Option<Ty>,
    /// Marks localized to this computation node.
    marks: Vec<Mark>,
}

/// One unit of work in the iterative marking machine.
enum MarkWork
{
    /// Type a value node at the current compatibility path.
    Value
    {
        /// Source value.
        value: Value,
        /// Stable arena id for `value`.
        id: Option<ValueNodeId>,
        /// Typing direction.
        dir: Dir<ValueType>,
    },
    /// Type a computation node at the current compatibility path.
    Comp
    {
        /// Source computation.
        comp: Comp,
        /// Stable arena id for `comp`.
        id: Option<CompNodeId>,
        /// Typing direction.
        dir: Dir<CompType>,
    },
    /// Continue typing a reified stack owned by a `Value::Stk` node.
    StackValue(StackValueWork),
}

/// The in-flight typing state of a reified stack owned by a `Value::Stk`
/// node (the payload of [`MarkWork::StackValue`]): the machine walks the
/// stack's frames, threading the consumed type through each step.
struct StackValueWork
{
    /// Remaining stack suffix.
    stack: Stack,
    /// Type consumed by the next frame.
    input: CompType,
    /// Type consumed by the original stack value.
    root_input: CompType,
    /// Compatibility child index for the next addressable interior term.
    frame: u32,
    /// Pending facts for the owning `Value::Stk` node.
    pending: PendingValue,
    /// Original typing direction of the stack value.
    dir: Dir<ValueType>,
}

/// A completed marking sub-computation.
enum MarkResult
{
    /// A value type.
    Value(ValueType),
    /// A computation type.
    Comp(CompType),
}

/// The transient state of one [`Marker::drive_marking`] run: the continuation
/// frames plus the pending-work / completed-result slots the machine
/// alternates between (exactly one slot is `Some` at every loop head).
struct MarkRun
{
    /// Continuation frames awaiting child results.
    frames: Vec<MarkFrame>,
    /// The next work item to start, present when a rule scheduled a child.
    work: Option<MarkWork>,
    /// The most recently completed sub-computation, present when a rule
    /// finished without scheduling a child.
    result: Option<MarkResult>,
}

/// The in-flight checking state of a `handle` computation's operation-clause
/// loop: the clauses still to check plus the signature and result types they
/// are checked against (the payload the [`MarkFrame::CompHandleAfterRet`] and
/// [`MarkFrame::CompHandleAfterOp`] frames thread through the loop).
struct HandleOps
{
    /// The effect signature the handler covers.
    sig: EffectSig,
    /// Remaining operation clauses to check.
    ops: IntoIter<OpClause>,
    /// The answer type each clause body checks against.
    answer: CompType,
    /// The handler's natural type (from [`handle_natural_type`]), returned
    /// once every clause checks.
    natural: CompType,
    /// Compatibility child index of the next clause.
    next_index: u32,
}

/// Continuation frames for the iterative marking machine.
enum MarkFrame
{
    /// Finish a dependent pair after the first child was checked.
    ValuePairSigmaAfterFst
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Second component.
        snd: Rc<Value>,
        /// Direction for the second component.
        snd_dir: Dir<ValueType>,
        /// Synthesized pair type.
        result: ValueType,
    },
    /// Finish a dependent pair after the second child was checked.
    ValuePairSigmaAfterSnd
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Synthesized pair type.
        result: ValueType,
    },
    /// Continue an ordinary pair after the first child.
    ValuePairAfterFst
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Second component.
        snd: Rc<Value>,
        /// Direction for the second component.
        snd_dir: Dir<ValueType>,
        /// Original pair direction.
        dir: Dir<ValueType>,
    },
    /// Finish an ordinary pair after the second child.
    ValuePairAfterSnd
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Original pair direction.
        dir: Dir<ValueType>,
        /// First component type.
        fst_ty: ValueType,
    },
    /// Return a fixed value type after a single value child.
    ValueReturnAfterChild
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Synthesized parent type.
        result: ValueType,
    },
    /// Finish a constructed value type through the marking Sub rule after one
    /// child.
    ValueFinishAfterChild
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Constructed parent type.
        constructed: ValueType,
        /// Original direction.
        dir: Dir<ValueType>,
    },
    /// Continue a list literal after one element.
    ValueElements
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Remaining elements.
        elements: IntoIter<Rc<Value>>,
        /// Child index of the next element.
        next_index: u32,
        /// Direction shared by every element.
        dir: Dir<ValueType>,
        /// Synthesized list type.
        result: ValueType,
    },
    /// Continue a record literal after one field.
    ValueRecordAfterField
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Remaining fields in canonical order.
        fields: alloc::collections::btree_map::IntoIter<String, Rc<Value>>,
        /// Child index of the next field.
        next_index: u32,
        /// Original record direction.
        dir: Dir<ValueType>,
        /// Field types accumulated so far.
        typed: BTreeMap<String, Rc<ValueType>>,
        /// Label of the field whose child just returned.
        label: String,
    },
    /// Finish an inferred or shape-checked thunk after its body.
    ValueThunkAfterBody
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Source grade.
        grade: Grade,
        /// Original direction.
        dir: Dir<ValueType>,
    },
    /// Finish a thunk checked against an expected thunk type.
    ValueThunkExpectedAfterBody
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Expected grade.
        expected_grade: Grade,
        /// Expected body type.
        expected_body: Rc<CompType>,
    },
    /// Finish a value annotation after its inner value.
    ValueAnnotAfterInner
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Original direction.
        dir: Dir<ValueType>,
    },
    /// Finish a `here` value after its witness.
    ValueHereAfterWitness
    {
        /// Pending parent facts.
        pending: PendingValue,
        /// Witness value.
        witness: Rc<Value>,
        /// Original direction.
        dir: Dir<ValueType>,
    },
    /// Finish an unannotated abstraction checked against an arrow.
    CompAbsCheckArrowAfterBody
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Arrow domain.
        arg: Rc<ValueType>,
        /// Arrow codomain expectation.
        res: Rc<CompType>,
    },
    /// Finish an unannotated abstraction checked against unknown.
    CompAbsCheckUnknownAfterBody
    {
        /// Pending parent facts.
        pending: PendingComp,
    },
    /// Finish an annotated abstraction after its body.
    CompAbsAnnotatedAfterBody
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Annotation type.
        annot_ty: Rc<ValueType>,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish a stuck abstraction after its body has been decorated.
    CompAbsStuckAfterBody
    {
        /// Pending parent facts.
        pending: PendingComp,
    },
    /// Continue an application after the head.
    CompAppAfterHead
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Argument value.
        arg: Rc<Value>,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish an application after the argument.
    CompAppAfterArg
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Arrow codomain.
        res: CompType,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish a returner after its payload.
    CompRetAfterPayload
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Continue a bind after the bound computation.
    CompBindAfterBound
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Continuation binder.
        name: String,
        /// Continuation body.
        cont: Rc<Comp>,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish a bind after the continuation.
    CompBindAfterCont
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Effects of the bound computation.
        bound_row: EffectRow,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish a force after its thunk.
    CompForceAfterThunk
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Continue a case after the scrutinee.
    CompCaseAfterScrut
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// First arm.
        arm_fst: (String, Rc<Comp>),
        /// Second arm.
        arm_snd: (String, Rc<Comp>),
        /// Expected answer.
        expected: CompType,
    },
    /// Continue a case after the first arm.
    CompCaseAfterFst
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Second arm binder.
        snd_name: String,
        /// Second arm body.
        snd_body: Rc<Comp>,
        /// Second arm binder type.
        snd_ty: ValueType,
        /// Expected answer.
        expected: CompType,
    },
    /// Finish a case after the second arm.
    CompCaseAfterSnd
    {
        /// Pending parent facts.
        pending: PendingComp,
    },
    /// Continue a data case after its scrutinee.
    CompDataCaseAfterScrut
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Remaining arms.
        arms: IntoIter<(String, Rc<Comp>)>,
        /// Expected answer.
        expected: CompType,
        /// Last arm result, or the expectation for an empty arm list.
        result: CompType,
        /// Child index of the next arm.
        next_index: u32,
    },
    /// Continue a data case after one arm.
    CompDataCaseAfterArm
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Remaining arms.
        arms: IntoIter<(String, Rc<Comp>)>,
        /// Expected answer.
        expected: CompType,
        /// Child index of the next arm.
        next_index: u32,
    },
    /// Continue a list case after its scrutinee.
    CompListCaseAfterScrut
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Nil arm.
        nil: Rc<Comp>,
        /// Cons arm tuple.
        cons_arm: (String, String, Rc<Comp>),
        /// Expected answer.
        expected: CompType,
    },
    /// Continue a list case after the nil arm.
    CompListCaseAfterNil
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Cons head binder.
        head: String,
        /// Cons tail binder.
        tail: String,
        /// Cons body.
        cons: Rc<Comp>,
        /// Head type.
        head_ty: ValueType,
        /// Tail type.
        tail_ty: ValueType,
        /// Expected answer.
        expected: CompType,
    },
    /// Finish a list case after the cons arm.
    CompListCaseAfterCons
    {
        /// Pending parent facts.
        pending: PendingComp,
    },
    /// Continue a split after its scrutinee.
    CompSplitAfterScrut
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Split binders.
        binders: (String, String),
        /// Optional motive.
        motive: Option<SplitMotive>,
        /// Body.
        body: Rc<Comp>,
        /// Original direction.
        dir: Dir<CompType>,
        /// Scrutinee value clone for motive substitution.
        scrut_value: Value,
    },
    /// Finish a split after its body.
    CompSplitAfterBody
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Original direction.
        dir: Dir<CompType>,
        /// Result type.
        result: CompType,
    },
    /// Continue an unpack after its scrutinee.
    CompUnpackAfterScrut
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// The module variable.
        binder: String,
        /// The type the module variable binds at.
        bound: ValueType,
        /// The body.
        body: Rc<Comp>,
        /// The direction the body is decorated in.
        body_dir: Dir<CompType>,
        /// Original direction.
        dir: Dir<CompType>,
        /// Result type.
        result: CompType,
    },
    /// Finish an unpack after its body.
    CompUnpackAfterBody
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Original direction.
        dir: Dir<CompType>,
        /// Result type.
        result: CompType,
    },
    /// Continue a `with` after its first component.
    CompWithAfterFst
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Second component.
        snd: Rc<Comp>,
        /// Second direction.
        snd_dir: Dir<CompType>,
        /// Whether the node was stuck before child typing.
        stuck: bool,
    },
    /// Finish a `with` after its second component.
    CompWithAfterSnd
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// First component type.
        fst_ty: CompType,
        /// Whether the node was stuck before child typing.
        stuck: bool,
    },
    /// Finish a projection after the target.
    CompPrjAfterTarget
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Projection side.
        side: Side,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish a record projection after the record.
    CompRecordProjAfterRecord
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Projected label.
        label: String,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish `dup` after the thunk.
    CompDupAfterThunk
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Requested first split grade.
        r: Grade,
        /// Requested second split grade.
        s: Grade,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish `drop` after the thunk.
    CompDropAfterThunk
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish `perform` after the payload.
    CompPerformAfterArg
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Constructed type.
        constructed: CompType,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Continue a handler after the scrutinee.
    CompHandleAfterScrutinee
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Handler signature.
        sig: EffectSig,
        /// Return clause.
        ret: (String, Rc<Comp>),
        /// Operation clauses.
        ops: IntoIter<OpClause>,
        /// Answer expected from every clause.
        answer: CompType,
    },
    /// Continue a handler after its return clause.
    CompHandleAfterRet
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Handler signature.
        sig: EffectSig,
        /// Operation clauses.
        ops: IntoIter<OpClause>,
        /// Answer expected from every clause.
        answer: CompType,
        /// Natural type after residual effects are removed.
        natural: CompType,
        /// Child index of the next operation clause.
        next_index: u32,
    },
    /// Continue a handler after one operation clause.
    CompHandleAfterOp
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Handler signature.
        sig: EffectSig,
        /// Operation clauses.
        ops: IntoIter<OpClause>,
        /// Answer expected from every clause.
        answer: CompType,
        /// Natural type after residual effects are removed.
        natural: CompType,
        /// Child index of the next operation clause.
        next_index: u32,
    },
    /// Continue a resume after the stack value.
    CompResumeAfterStack
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Computation being resumed.
        comp: Rc<Comp>,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish a resume after the resumed computation.
    CompResumeAfterComp
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Delivered type.
        delivered: CompType,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Finish a reset after its body.
    CompResetAfterBody
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Saved ambient answer.
        saved: Option<CompType>,
    },
    /// Finish a shift after its body.
    CompShiftAfterBody
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Captured type delivered by the shift.
        captured: CompType,
    },
    /// Continue a walk after its scrutinee.
    CompWalkAfterScrut
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Motive.
        motive: WalkMotive,
        /// Base branch.
        base: WalkBase,
        /// Original direction.
        dir: Dir<CompType>,
        /// Scrutinee value clone for motive substitution.
        scrut_value: Value,
    },
    /// Finish a walk after its base branch.
    CompWalkAfterBase
    {
        /// Pending parent facts.
        pending: PendingComp,
        /// Result type.
        result: CompType,
        /// Original direction.
        dir: Dir<CompType>,
    },
    /// Continue a stack argument frame after the value child.
    StackArgAfterValue
    {
        /// Rest of the stack.
        rest: Rc<Stack>,
        /// Type after consuming the argument frame.
        result_ty: CompType,
        /// Type consumed by the original stack value.
        root_input: CompType,
        /// Next frame index.
        next_frame: u32,
        /// Pending owner facts.
        pending: PendingValue,
        /// Original stack-value direction.
        dir: Dir<ValueType>,
    },
    /// Continue a stack bind frame after the continuation.
    StackBindAfterCont
    {
        /// Rest of the stack.
        rest: Rc<Stack>,
        /// Effects consumed by the input returner.
        consumed_row: EffectRow,
        /// Type consumed by the original stack value.
        root_input: CompType,
        /// Next frame index.
        next_frame: u32,
        /// Pending owner facts.
        pending: PendingValue,
        /// Original stack-value direction.
        dir: Dir<ValueType>,
    },
}

/// Work item for the iterative arena interner.
enum InternWork
{
    /// Intern a value node, optionally caching the resulting id for an Rc
    /// pointer.
    Value
    {
        /// Value to intern.
        value: Value,
        /// Rc identity to cache.
        cache_key: Option<*const Value>,
    },
    /// Intern a computation node, optionally caching the resulting id for an Rc
    /// pointer.
    Comp
    {
        /// Computation to intern.
        comp: Comp,
        /// Rc identity to cache.
        cache_key: Option<*const Comp>,
    },
    /// Intern a stack node, optionally caching the resulting id for an Rc
    /// pointer.
    Stack
    {
        /// Stack to intern.
        stack: Stack,
        /// Rc identity to cache.
        cache_key: Option<*const Stack>,
    },
}

/// Completed interning result.
#[derive(Clone, Copy)]
enum InternResult
{
    /// Value id, or `None` if arena allocation failed.
    Value(Option<ValueNodeId>),
    /// Computation id, or `None` if arena allocation failed.
    Comp(Option<CompNodeId>),
    /// Stack id, or `None` if arena allocation failed.
    Stack(Option<StackNodeId>),
}

/// The transient state of one [`Marker::drive_interning`] run: the
/// continuation frames plus the pending-work / completed-result slots (the
/// same one-slot alternation discipline as [`MarkRun`]).
struct InternRun
{
    /// Continuation frames awaiting child ids.
    frames: Vec<InternFrame>,
    /// The next node to intern, present when a rule scheduled a child.
    work: Option<InternWork>,
    /// The most recently interned node, present when a rule finished without
    /// scheduling a child.
    result: Option<InternResult>,
}

/// The in-flight interning state of a `handle` node: the pieces that, once
/// every operation clause is interned, assemble into a [`CompNode::Handle`].
struct InternHandle
{
    /// The effect signature the handler covers.
    sig: EffectSig,
    /// The interned scrutinee computation.
    scrutinee: CompNodeId,
    /// The `(name, body)` return clause, with the body interned.
    ret: (String, CompNodeId),
    /// Remaining source operation clauses.
    ops: IntoIter<OpClause>,
    /// Accumulated interned operation clauses.
    flat_ops: Vec<OpClauseNode>,
}

/// Continuations for the iterative arena interner.
enum InternFrame
{
    /// Continue a value pair after the first child.
    ValuePairAfterFst
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
        /// Second child.
        snd: Rc<Value>,
    },
    /// Finish a value pair.
    ValuePairAfterSnd
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
        /// First child id.
        fst: ValueNodeId,
    },
    /// Finish a unary value node after a value child.
    ValueUnaryAfterChild
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
        /// Unary value constructor.
        kind: ValueUnaryIntern,
    },
    /// Continue a value list after one element.
    ValueListAfterElement
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
        /// Remaining elements.
        elements: IntoIter<Rc<Value>>,
        /// Accumulated child ids.
        ids: Vec<ValueNodeId>,
    },
    /// Continue a value record after one field.
    ValueRecordAfterField
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
        /// Remaining fields.
        fields: alloc::collections::btree_map::IntoIter<String, Rc<Value>>,
        /// Accumulated fields.
        ids: BTreeMap<String, ValueNodeId>,
        /// Label for the child that just finished.
        label: String,
    },
    /// Finish a thunk after its computation body.
    ValueThunkAfterBody
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
        /// Thunk grade.
        grade: Grade,
    },
    /// Continue a value annotation after its inner value.
    ValueAnnotAfterInner
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
        /// Annotation type.
        ty: Rc<ValueType>,
    },
    /// Finish a stack value after its stack node.
    ValueStkAfterStack
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
    },
    /// Finish a constructor after its payload.
    ValueCtorAfterPayload
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
        /// Data id.
        id: gandr_core_term::types::DataId,
        /// Constructor tag.
        tag: usize,
    },
    /// Finish a packed module after its payload.
    ValuePackAfterPayload
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Value>,
        /// The already-interned witness type ids.
        witnesses: Vec<ValueTypeNodeId>,
    },
    /// Finish a unary computation after a computation child.
    CompUnaryCompAfterChild
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Unary computation constructor.
        kind: CompUnaryCompIntern,
    },
    /// Finish a unary computation after a value child.
    CompUnaryValueAfterChild
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Unary computation constructor.
        kind: CompUnaryValueIntern,
    },
    /// Continue an application after the head.
    CompAppAfterHead
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Argument.
        arg: Rc<Value>,
    },
    /// Finish an application.
    CompAppAfterArg
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Head id.
        head: CompNodeId,
    },
    /// Continue a bind after the bound computation.
    CompBindAfterBound
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Binder name.
        name: String,
        /// Continuation.
        cont: Rc<Comp>,
    },
    /// Finish a bind.
    CompBindAfterCont
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Bound computation id.
        bound: CompNodeId,
        /// Binder name.
        name: String,
    },
    /// Continue a case after the scrutinee.
    CompCaseAfterScrut
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// First arm.
        arm_fst: (String, Rc<Comp>),
        /// Second arm.
        arm_snd: (String, Rc<Comp>),
    },
    /// Continue a case after the first arm.
    CompCaseAfterFst
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Scrutinee id.
        scrut: ValueNodeId,
        /// First arm binder.
        fst_name: String,
        /// Second arm.
        arm_snd: (String, Rc<Comp>),
    },
    /// Finish a case.
    CompCaseAfterSnd
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Scrutinee id.
        scrut: ValueNodeId,
        /// First arm.
        arm_fst: (String, CompNodeId),
        /// Second arm binder.
        snd_name: String,
    },
    /// Continue native argument interning after one argument.
    CompNativeAfterArg
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Native primitive.
        prim: gandr_core_term::prim::NativePrim,
        /// Remaining arguments.
        args: IntoIter<Rc<Value>>,
        /// Accumulated argument ids.
        ids: Vec<ValueNodeId>,
    },
    /// Continue a list case after the scrutinee.
    CompListCaseAfterScrut
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Nil arm.
        nil: Rc<Comp>,
        /// Head binder.
        head: String,
        /// Tail binder.
        tail: String,
        /// Cons arm.
        cons: Rc<Comp>,
    },
    /// Continue a list case after nil.
    CompListCaseAfterNil
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Scrutinee id.
        scrut: ValueNodeId,
        /// Head binder.
        head: String,
        /// Tail binder.
        tail: String,
        /// Cons arm.
        cons: Rc<Comp>,
    },
    /// Finish a list case.
    CompListCaseAfterCons
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Scrutinee id.
        scrut: ValueNodeId,
        /// Nil arm id.
        nil: CompNodeId,
        /// Head binder.
        head: String,
        /// Tail binder.
        tail: String,
    },
    /// Continue a split after the scrutinee.
    CompSplitAfterScrut
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// First binder.
        fst_name: String,
        /// Second binder.
        snd_name: String,
        /// Optional motive.
        motive: Option<SplitMotive>,
        /// Body.
        body: Rc<Comp>,
    },
    /// Finish a split.
    CompSplitAfterBody
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Scrutinee id.
        scrut: ValueNodeId,
        /// First binder.
        fst_name: String,
        /// Second binder.
        snd_name: String,
        /// Motive node.
        motive: Option<SplitMotiveNode>,
    },
    /// Continue a data case after the scrutinee.
    CompDataCaseAfterScrut
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Arms.
        arms: IntoIter<(String, Rc<Comp>)>,
        /// Accumulated arm ids.
        arm_ids: Vec<(String, CompNodeId)>,
    },
    /// Continue a data case after one arm.
    CompDataCaseAfterArm
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Scrutinee id.
        scrut: ValueNodeId,
        /// Arms.
        arms: IntoIter<(String, Rc<Comp>)>,
        /// Accumulated arm ids.
        arm_ids: Vec<(String, CompNodeId)>,
        /// Binder for the child that just finished.
        binder: String,
    },
    /// Continue a `with` after the first component.
    CompWithAfterFst
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Second component.
        snd: Rc<Comp>,
    },
    /// Finish a `with`.
    CompWithAfterSnd
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// First component id.
        fst: CompNodeId,
    },
    /// Continue a handle after the scrutinee.
    CompHandleAfterScrutinee
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Signature.
        sig: EffectSig,
        /// Return clause.
        ret: (String, Rc<Comp>),
        /// Operation clauses.
        ops: IntoIter<OpClause>,
    },
    /// Continue a handle after the return clause.
    CompHandleAfterRet
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Signature.
        sig: EffectSig,
        /// Scrutinee id.
        scrutinee: CompNodeId,
        /// Return binder.
        ret_name: String,
        /// Operation clauses.
        ops: IntoIter<OpClause>,
        /// Accumulated operation clause nodes.
        flat_ops: Vec<OpClauseNode>,
    },
    /// Continue a handle after an operation clause.
    CompHandleAfterOp
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Signature.
        sig: EffectSig,
        /// Scrutinee id.
        scrutinee: CompNodeId,
        /// Return clause.
        ret: (String, CompNodeId),
        /// Operation clauses.
        ops: IntoIter<OpClause>,
        /// Accumulated operation clause nodes.
        flat_ops: Vec<OpClauseNode>,
        /// Clause whose body just finished.
        clause: OpClause,
    },
    /// Continue a resume after the stack value.
    CompResumeAfterStack
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Computation.
        comp: Rc<Comp>,
    },
    /// Finish a resume.
    CompResumeAfterComp
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Stack value id.
        stack: ValueNodeId,
    },
    /// Continue a walk after the scrutinee.
    CompWalkAfterScrut
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Motive.
        motive: WalkMotive,
        /// Base.
        base: WalkBase,
    },
    /// Finish a walk.
    CompWalkAfterBase
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Scrutinee id.
        scrut: ValueNodeId,
        /// Motive node.
        motive: WalkMotiveNode,
        /// Base binder.
        base_x: String,
    },
    /// Continue an unpack after the scrutinee.
    CompUnpackAfterScrut
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// The already-interned ascribed signature id.
        signature: ValueTypeNodeId,
        /// The atoms the elimination binds its components to.
        atoms: Vec<SealId>,
        /// The module variable.
        binder: String,
        /// The body, still to be interned.
        body: Rc<Comp>,
    },
    /// Finish an unpack after the body.
    CompUnpackAfterBody
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Comp>,
        /// Scrutinee id.
        scrut: ValueNodeId,
        /// The already-interned ascribed signature id.
        signature: ValueTypeNodeId,
        /// The atoms the elimination binds its components to.
        atoms: Vec<SealId>,
        /// The module variable.
        binder: String,
    },
    /// Continue a stack argument after the value child.
    StackArgAfterValue
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Stack>,
        /// Rest stack.
        rest: Rc<Stack>,
    },
    /// Finish a stack argument.
    StackArgAfterRest
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Stack>,
        /// Value id.
        value: ValueNodeId,
    },
    /// Continue a stack bind after the continuation.
    StackBindAfterCont
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Stack>,
        /// Binder name.
        name: String,
        /// Rest stack.
        rest: Rc<Stack>,
    },
    /// Finish a stack bind.
    StackBindAfterRest
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Stack>,
        /// Binder name.
        name: String,
        /// Continuation id.
        cont: CompNodeId,
    },
    /// Finish a stack projection.
    StackPrjAfterRest
    {
        /// Optional Rc cache key for the parent.
        cache_key: Option<*const Stack>,
        /// Projection side.
        side: Side,
    },
}

/// Unary value constructors used by the iterative interner.
enum ValueUnaryIntern
{
    /// Injection.
    Inj(Side),
    /// `here` introduction.
    Here,
}

/// Unary computation constructors over computation children used by the
/// interner.
enum CompUnaryCompIntern
{
    /// Abstraction.
    Abs(String, Option<ValueTypeNodeId>),
    /// Projection.
    Prj(Side),
    /// Reset.
    Reset,
    /// Shift.
    Shift(String),
}

/// Unary computation constructors over value children used by the interner.
enum CompUnaryValueIntern
{
    /// Returner.
    Ret,
    /// Force.
    Force,
    /// Duplication.
    Dup,
    /// Drop.
    Drop,
    /// Perform.
    Perform(EffectSig, String),
    /// Record projection.
    RecordProj(String),
}

impl PendingValue
{
    /// Records the synthesized value type and returns it.
    fn finish(
        self,
        marker: &mut Marker,
        synth: ValueType,
    ) -> ValueType
    {
        if let Some(id) = self.id {
            marker.record(MarkNodeId::Value(id), NodeFacts {
                analyzed: self.analyzed,
                synthesized: Some(Ty::Value(synth.clone())),
                marks: self.marks,
                dirty: false,
            });
        }
        synth
    }
}

impl PendingComp
{
    /// Records the synthesized computation type and returns it.
    fn finish(
        self,
        marker: &mut Marker,
        synth: CompType,
    ) -> CompType
    {
        if let Some(id) = self.id {
            marker.record(MarkNodeId::Comp(id), NodeFacts {
                analyzed: self.analyzed,
                synthesized: Some(Ty::Comp(synth.clone())),
                marks: self.marks,
                dirty: false,
            });
        }
        synth
    }
}

impl Marker
{
    /// Creates an empty marker state.
    fn new(ctx: Ctx) -> Self
    {
        Self {
            ctx,
            answer: None,
            arena: FlatArena::new(),
            value_ids: BTreeMap::new(),
            comp_ids: BTreeMap::new(),
            stack_ids: BTreeMap::new(),
            facts: BTreeMap::new(),
            compatibility_paths: BTreeMap::new(),
            path: Vec::new(),
        }
    }

    /// Records facts for `id` and the current compatibility path.
    fn record(
        &mut self,
        id: MarkNodeId,
        facts: NodeFacts,
    )
    {
        self.compatibility_paths.insert(self.path.clone(), id);
        self.facts.insert(id, facts);
    }

    /// Types a value at the current compatibility path with an explicit heap
    /// stack. # Termination
    /// - reason: the marking machine drains explicit finite work and
    ///   continuation stacks.
    /// - measure: pending work items plus continuation frames.
    /// - boundedness: each scheduled child comes from a finite source syntax or
    ///   stack node.
    /// - input recursion: none.
    fn value(
        &mut self,
        value: Value,
        id: Option<ValueNodeId>,
        dir: Dir<ValueType>,
    ) -> ValueType
    {
        expect_value(self.drive_marking(MarkWork::Value { value, id, dir }))
    }

    /// Types a computation at the current compatibility path with an explicit
    /// heap stack. # Termination
    /// - reason: the marking machine drains explicit finite work and
    ///   continuation stacks.
    /// - measure: pending work items plus continuation frames.
    /// - boundedness: each scheduled child comes from a finite source syntax or
    ///   stack node.
    /// - input recursion: none.
    fn comp(
        &mut self,
        comp: Comp,
        id: Option<CompNodeId>,
        dir: Dir<CompType>,
    ) -> CompType
    {
        expect_comp(self.drive_marking(MarkWork::Comp { comp, id, dir }))
    }

    /// Interns a shared value node, preserving `Rc` identity across paths.
    /// # Termination
    /// - reason: the interner drains explicit finite work and continuation
    ///   stacks.
    /// - measure: pending arena nodes plus continuation frames.
    /// - boundedness: every scheduled child is a child of a finite source node.
    /// - input recursion: none.
    fn intern_value_rc(
        &mut self,
        value: &Rc<Value>,
    ) -> Option<ValueNodeId>
    {
        let key = Rc::as_ptr(value);
        if let Some(id) = self.value_ids.get(&key).copied() {
            return Some(id);
        }
        expect_intern_value(self.drive_interning(InternWork::Value {
            value: value.as_ref().clone(),
            cache_key: Some(key),
        }))
    }

    /// Interns a value node into the stable flat arena.
    /// # Termination
    /// - reason: the interner drains explicit finite work and continuation
    ///   stacks.
    /// - measure: pending arena nodes plus continuation frames.
    /// - boundedness: every scheduled child is a child of a finite source node.
    /// - input recursion: none.
    fn intern_value(
        &mut self,
        value: &Value,
    ) -> Option<ValueNodeId>
    {
        expect_intern_value(self.drive_interning(InternWork::Value {
            value: value.clone(),
            cache_key: None,
        }))
    }

    /// Interns a shared computation node, preserving `Rc` identity across
    /// paths. # Termination
    /// - reason: the interner drains explicit finite work and continuation
    ///   stacks.
    /// - measure: pending arena nodes plus continuation frames.
    /// - boundedness: every scheduled child is a child of a finite source node.
    /// - input recursion: none.
    fn intern_comp_rc(
        &mut self,
        comp: &Rc<Comp>,
    ) -> Option<CompNodeId>
    {
        let key = Rc::as_ptr(comp);
        if let Some(id) = self.comp_ids.get(&key).copied() {
            return Some(id);
        }
        expect_intern_comp(self.drive_interning(InternWork::Comp {
            comp: comp.as_ref().clone(),
            cache_key: Some(key),
        }))
    }

    /// Interns a computation node into the stable flat arena.
    /// # Termination
    /// - reason: the interner drains explicit finite work and continuation
    ///   stacks.
    /// - measure: pending arena nodes plus continuation frames.
    /// - boundedness: every scheduled child is a child of a finite source node.
    /// - input recursion: none.
    fn intern_comp(
        &mut self,
        comp: &Comp,
    ) -> Option<CompNodeId>
    {
        expect_intern_comp(self.drive_interning(InternWork::Comp {
            comp: comp.clone(),
            cache_key: None,
        }))
    }

    /// Interns a value type using the canonical flat arena bridge.
    fn intern_value_type(
        &mut self,
        ty: &ValueType,
    ) -> Option<ValueTypeNodeId>
    {
        self.arena.alloc_value_type(ty).ok()
    }

    /// Drives marking work to completion without host recursion.
    ///
    /// # Contract
    /// - ensures: returns the root work item's result; every scheduled child is
    ///   typed and recorded before its frame resumes.
    /// - panics: none in release builds. The machine alternates the two
    ///   [`MarkRun`] slots, so a completed result is always present when no
    ///   work is pending; a `debug_assert!` guards that invariant in test /
    ///   debug builds, and the `Unknown` fallback (the module's absorbing
    ///   recovery type) keeps the driver total — it is never reached (the
    ///   marking oracle proptests would fail first on any desync).
    fn drive_marking(
        &mut self,
        root: MarkWork,
    ) -> MarkResult
    {
        let mut run = MarkRun {
            frames: Vec::new(),
            work: Some(root),
            result: None,
        };
        loop {
            if let Some(item) = run.work.take() {
                self.start_mark_work(item, &mut run);
                continue;
            }
            debug_assert!(
                run.result.is_some(),
                "marking machine stalled without work or result"
            );
            let Some(done) = run.result.take()
            else {
                return MarkResult::Value(ValueType::Unknown);
            };
            let Some(frame) = run.frames.pop()
            else {
                return done;
            };
            self.resume_mark_frame(frame, done, &mut run);
        }
    }

    /// Starts one marking work item.
    fn start_mark_work(
        &mut self,
        item: MarkWork,
        run: &mut MarkRun,
    )
    {
        match item {
            | MarkWork::Value { value, id, dir } => {
                self.start_value(value, id, dir, run);
            },
            | MarkWork::Comp { comp, id, dir } => {
                self.start_comp(comp, id, dir, run);
            },
            | MarkWork::StackValue(item) => self.start_stack_value(item, run),
        }
    }

    /// Starts a value node in the marking machine.
    fn start_value(
        &mut self,
        value: Value,
        id: Option<ValueNodeId>,
        dir: Dir<ValueType>,
        run: &mut MarkRun,
    )
    {
        let mut pending = PendingValue {
            id,
            analyzed: analyzed_value(&dir),
            marks: Vec::new(),
        };
        match value {
            | Value::Var(name) => {
                let ty = self.rule_var(name, dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | Value::Unit => {
                let ty = finish_value(ValueType::Unit, dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | Value::Int(literal) => {
                let ty = finish_int_literal_marked(literal, dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | Value::Str(_) => {
                let ty = finish_value(ValueType::string(), dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | Value::Num(literal) => {
                let ty = finish_value(literal.value_type(), dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | Value::Hole(hole) => {
                pending.marks.push(Mark::EmptyHole(hole));
                let ty = finish_value(ValueType::Unknown, dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | Value::Pair(fst, snd) => {
                if let Dir::Check(ValueType::Sigma {
                    fst: head,
                    binder,
                    snd: tail,
                }) = dir
                {
                    let tail_ty = gandr_core_term::identity::subst_valuetype(
                        &tail,
                        gandr_core_term::boundary::NameRef::from(binder.as_str()),
                        &fst,
                    );
                    let result_ty = ValueType::Sigma {
                        fst: Rc::clone(&head),
                        binder,
                        snd: tail,
                    };
                    self.schedule_child_value(
                        0,
                        fst,
                        Dir::Check(head.as_ref().clone()),
                        MarkFrame::ValuePairSigmaAfterFst {
                            pending,
                            snd,
                            snd_dir: Dir::Check(tail_ty),
                            result: result_ty,
                        },
                        run,
                    );
                }
                else {
                    let (fst_dir, snd_dir) = dir.pair_components();
                    self.schedule_child_value(
                        0,
                        fst,
                        fst_dir,
                        MarkFrame::ValuePairAfterFst {
                            pending,
                            snd,
                            snd_dir,
                            dir,
                        },
                        run,
                    );
                }
            },
            | Value::Inj(side, payload) => match dir {
                | Dir::Check(ValueType::Sum(lhs, rhs)) => {
                    let payload_ty = pick(side, &lhs, &rhs);
                    self.schedule_child_value(
                        0,
                        payload,
                        Dir::Check(payload_ty),
                        MarkFrame::ValueReturnAfterChild {
                            pending,
                            result: ValueType::Sum(lhs, rhs),
                        },
                        run,
                    );
                },
                | Dir::Check(ValueType::Unknown) => {
                    self.schedule_child_value(
                        0,
                        payload,
                        Dir::Check(ValueType::Unknown),
                        MarkFrame::ValueReturnAfterChild {
                            pending,
                            result: ValueType::Unknown,
                        },
                        run,
                    );
                },
                | Dir::Infer | Dir::Check(_) => {
                    pending.marks.push(Mark::Stuck {
                        hint: text::ANNOTATE_INJECTION,
                    });
                    self.schedule_child_value(
                        0,
                        payload,
                        Dir::Infer,
                        MarkFrame::ValueReturnAfterChild {
                            pending,
                            result: ValueType::Unknown,
                        },
                        run,
                    );
                },
            },
            | Value::List(elements) => match dir {
                | Dir::Check(ValueType::List(elem)) => {
                    self.schedule_value_elements(
                        pending,
                        elements.into_iter(),
                        0,
                        Dir::Check(elem.as_ref().clone()),
                        ValueType::List(elem),
                        run,
                    );
                },
                | Dir::Check(ValueType::Unknown) => {
                    self.schedule_value_elements(
                        pending,
                        elements.into_iter(),
                        0,
                        Dir::Check(ValueType::Unknown),
                        ValueType::Unknown,
                        run,
                    );
                },
                | Dir::Infer | Dir::Check(_) => {
                    pending.marks.push(Mark::Stuck {
                        hint: text::ANNOTATE_LIST,
                    });
                    self.schedule_value_elements(
                        pending,
                        elements.into_iter(),
                        0,
                        Dir::Infer,
                        ValueType::Unknown,
                        run,
                    );
                },
            },
            | Value::Record(fields) => {
                self.schedule_value_record(
                    pending,
                    fields.into_iter(),
                    0,
                    dir,
                    BTreeMap::new(),
                    run,
                );
            },
            | Value::Thunk(grade, body) => match dir {
                | Dir::Check(ValueType::Unknown) => {
                    self.schedule_child_comp(
                        0,
                        body,
                        Dir::Check(CompType::Unknown),
                        MarkFrame::ValueThunkAfterBody {
                            pending,
                            grade,
                            dir: Dir::Check(ValueType::Unknown),
                        },
                        run,
                    );
                },
                | Dir::Check(ValueType::Thunk(expected_grade, expected_body)) => {
                    if !bool::from(expected_grade.leq(grade)) {
                        pending.marks.push(Mark::GradeBudget {
                            required: expected_grade,
                            available: grade,
                        });
                    }
                    self.schedule_child_comp(
                        0,
                        body,
                        Dir::Check(expected_body.as_ref().clone()),
                        MarkFrame::ValueThunkExpectedAfterBody {
                            pending,
                            expected_grade,
                            expected_body,
                        },
                        run,
                    );
                },
                | other => {
                    self.schedule_child_comp(
                        0,
                        body,
                        Dir::Infer,
                        MarkFrame::ValueThunkAfterBody {
                            pending,
                            grade,
                            dir: other,
                        },
                        run,
                    );
                },
            },
            | Value::Annot(inner, ty) => {
                self.schedule_child_value(
                    0,
                    inner,
                    Dir::Check(unrc(ty)),
                    MarkFrame::ValueAnnotAfterInner { pending, dir },
                    run,
                );
            },
            | Value::Stk(stack) => {
                let consumed: CompType = match dir {
                    | Dir::Check(ValueType::Stk(ref b, _)) => b.as_ref().clone(),
                    | Dir::Check(ValueType::Unknown) => CompType::Unknown,
                    | Dir::Infer | Dir::Check(_) => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::STK_NEEDS_STK_TYPE,
                        });
                        CompType::Unknown
                    },
                };
                run.work = Some(MarkWork::StackValue(StackValueWork {
                    stack: unrc(stack),
                    input: consumed.clone(),
                    root_input: consumed,
                    frame: 0,
                    pending,
                    dir,
                }));
            },
            | Value::Here(witness) => {
                self.schedule_child_value(
                    0,
                    Rc::clone(&witness),
                    Dir::Infer,
                    MarkFrame::ValueHereAfterWitness {
                        pending,
                        witness,
                        dir,
                    },
                    run,
                );
            },
            | Value::Ctor {
                id: data_id,
                payload,
                ..
            } => match dir {
                | Dir::Check(ValueType::Data {
                    id: expected_id,
                    args,
                }) => {
                    self.schedule_child_value(
                        0,
                        payload,
                        Dir::Infer,
                        MarkFrame::ValueFinishAfterChild {
                            pending,
                            constructed: ValueType::Data {
                                id: data_id,
                                args: Vec::new(),
                            },
                            dir: Dir::Check(ValueType::Data {
                                id: expected_id,
                                args,
                            }),
                        },
                        run,
                    );
                },
                | Dir::Check(ValueType::Unknown) => {
                    self.schedule_child_value(
                        0,
                        payload,
                        Dir::Infer,
                        MarkFrame::ValueReturnAfterChild {
                            pending,
                            result: ValueType::Unknown,
                        },
                        run,
                    );
                },
                | Dir::Infer | Dir::Check(_) => {
                    pending.marks.push(Mark::Stuck {
                        hint: text::ANNOTATE_CTOR,
                    });
                    self.schedule_child_value(
                        0,
                        payload,
                        Dir::Infer,
                        MarkFrame::ValueReturnAfterChild {
                            pending,
                            result: ValueType::Unknown,
                        },
                        run,
                    );
                },
            },
            // Rule Pack⇓ marked: every refusal the checker raises becomes a
            // mark here and the node recovers at `Unknown`, so the payload is
            // still decorated. The one term child is the payload — the
            // witnesses are types and carry no term facts.
            | Value::Pack { witnesses, payload } => match dir {
                | Dir::Check(ValueType::Package {
                    grade,
                    abstracts,
                    payload: signature_payload,
                }) => {
                    let expected = crate::judgements::package::pack_payload_expectation(
                        grade,
                        &abstracts,
                        signature_payload.as_ref(),
                        &witnesses,
                    );
                    match expected {
                        | Ok(expected) => {
                            self.schedule_child_value(
                                0,
                                payload,
                                Dir::Check(expected),
                                MarkFrame::ValueReturnAfterChild {
                                    pending,
                                    result: ValueType::Package {
                                        grade,
                                        abstracts,
                                        payload: signature_payload,
                                    },
                                },
                                run,
                            );
                        },
                        | Err(refusal) => {
                            pending
                                .marks
                                .push(crate::judgements::package::refusal_mark(refusal));
                            self.schedule_child_value(
                                0,
                                payload,
                                Dir::Check(ValueType::Unknown),
                                MarkFrame::ValueReturnAfterChild {
                                    pending,
                                    result: ValueType::Unknown,
                                },
                                run,
                            );
                        },
                    }
                },
                | Dir::Check(ValueType::Unknown) => {
                    self.schedule_child_value(
                        0,
                        payload,
                        Dir::Check(ValueType::Unknown),
                        MarkFrame::ValueReturnAfterChild {
                            pending,
                            result: ValueType::Unknown,
                        },
                        run,
                    );
                },
                | Dir::Infer | Dir::Check(_) => {
                    pending.marks.push(Mark::Stuck {
                        hint: text::ANNOTATE_PACK,
                    });
                    self.schedule_child_value(
                        0,
                        payload,
                        Dir::Check(ValueType::Unknown),
                        MarkFrame::ValueReturnAfterChild {
                            pending,
                            result: ValueType::Unknown,
                        },
                        run,
                    );
                },
            },
        }
    }

    /// Starts a computation node in the marking machine.
    fn start_comp(
        &mut self,
        comp: Comp,
        id: Option<CompNodeId>,
        dir: Dir<CompType>,
        run: &mut MarkRun,
    )
    {
        let mut pending = PendingComp {
            id,
            analyzed: analyzed_comp(&dir),
            marks: Vec::new(),
        };
        match comp {
            | Comp::Abs(name, annot, body) => match (annot, dir) {
                | (None, Dir::Check(CompType::Arrow(arg, res))) => {
                    self.ctx.bind(name, arg.as_ref().clone());
                    self.schedule_child_comp(
                        0,
                        body,
                        Dir::Check(res.as_ref().clone()),
                        MarkFrame::CompAbsCheckArrowAfterBody { pending, arg, res },
                        run,
                    );
                },
                | (None, Dir::Check(CompType::Unknown)) => {
                    self.ctx.bind(name, ValueType::Unknown);
                    self.schedule_child_comp(
                        0,
                        body,
                        Dir::Check(CompType::Unknown),
                        MarkFrame::CompAbsCheckUnknownAfterBody { pending },
                        run,
                    );
                },
                | (Some(annot_ty), any_dir) => {
                    self.ctx.bind(name, annot_ty.as_ref().clone());
                    self.schedule_child_comp(
                        0,
                        body,
                        Dir::Infer,
                        MarkFrame::CompAbsAnnotatedAfterBody {
                            pending,
                            annot_ty,
                            dir: any_dir,
                        },
                        run,
                    );
                },
                | (None, Dir::Infer) => {
                    pending.marks.push(Mark::Stuck {
                        hint: text::ANNOTATE_BINDER,
                    });
                    self.ctx.bind(name, ValueType::Unknown);
                    self.schedule_child_comp(
                        0,
                        body,
                        Dir::Infer,
                        MarkFrame::CompAbsStuckAfterBody { pending },
                        run,
                    );
                },
                | (None, Dir::Check(_)) => {
                    pending.marks.push(Mark::Stuck {
                        hint: text::ABS_NEEDS_ARROW,
                    });
                    self.ctx.bind(name, ValueType::Unknown);
                    self.schedule_child_comp(
                        0,
                        body,
                        Dir::Infer,
                        MarkFrame::CompAbsStuckAfterBody { pending },
                        run,
                    );
                },
            },
            | Comp::App(head, arg) => {
                self.schedule_child_comp(
                    0,
                    head,
                    Dir::Infer,
                    MarkFrame::CompAppAfterHead { pending, arg, dir },
                    run,
                );
            },
            | Comp::Ret(payload) => {
                let payload_dir = dir.ret_payload();
                self.schedule_child_value(
                    0,
                    payload,
                    payload_dir,
                    MarkFrame::CompRetAfterPayload { pending, dir },
                    run,
                );
            },
            | Comp::Bind(bound, name, cont) => {
                self.schedule_child_comp(
                    0,
                    bound,
                    Dir::Infer,
                    MarkFrame::CompBindAfterBound {
                        pending,
                        name,
                        cont,
                        dir,
                    },
                    run,
                );
            },
            | Comp::Force(thunked) => {
                self.schedule_child_value(
                    0,
                    thunked,
                    Dir::Infer,
                    MarkFrame::CompForceAfterThunk { pending, dir },
                    run,
                );
            },
            | Comp::Case(scrut, arm_fst, arm_snd) => {
                let expected = match dir {
                    | Dir::Check(expected) => expected,
                    | Dir::Infer => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::CASE_NEEDS_CHECK,
                        });
                        CompType::Unknown
                    },
                };
                self.schedule_child_value(
                    0,
                    scrut,
                    Dir::Infer,
                    MarkFrame::CompCaseAfterScrut {
                        pending,
                        arm_fst,
                        arm_snd,
                        expected,
                    },
                    run,
                );
            },
            | Comp::DataCase(scrut, arms) => {
                let expected = match dir {
                    | Dir::Check(expected) => expected,
                    | Dir::Infer => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::DATA_CASE_NEEDS_CHECK,
                        });
                        CompType::Unknown
                    },
                };
                self.schedule_child_value(
                    0,
                    scrut,
                    Dir::Infer,
                    MarkFrame::CompDataCaseAfterScrut {
                        pending,
                        arms: arms.into_iter(),
                        expected: expected.clone(),
                        result: expected,
                        next_index: 1,
                    },
                    run,
                );
            },
            | Comp::ListCase {
                scrut,
                nil,
                head,
                tail,
                cons,
            } => {
                let expected = match dir {
                    | Dir::Check(expected) => expected,
                    | Dir::Infer => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::LIST_CASE_NEEDS_CHECK,
                        });
                        CompType::Unknown
                    },
                };
                self.schedule_child_value(
                    0,
                    scrut,
                    Dir::Infer,
                    MarkFrame::CompListCaseAfterScrut {
                        pending,
                        nil,
                        cons_arm: (head, tail, cons),
                        expected,
                    },
                    run,
                );
            },
            | Comp::Split {
                scrut,
                fst_name,
                snd_name,
                motive,
                body,
            } => {
                let scrut_value = scrut.as_ref().clone();
                self.schedule_child_value(
                    0,
                    scrut,
                    Dir::Infer,
                    MarkFrame::CompSplitAfterScrut {
                        pending,
                        binders: (fst_name, snd_name),
                        motive: motive.map(|boxed| *boxed),
                        body,
                        dir,
                        scrut_value,
                    },
                    run,
                );
            },
            // Rule Unpack⇓ marked: the ascription decides everything before the
            // scrutinee is visited, exactly as in the checker, so each refusal
            // becomes a mark here and the node recovers by binding the module
            // variable at `Unknown`. Term children are the scrutinee (0) and
            // the body (1); the signature is a type child.
            | Comp::Unpack {
                scrut,
                signature,
                atoms,
                binder,
                body,
            } => {
                let bound = match *signature {
                    | ValueType::Package {
                        grade,
                        ref abstracts,
                        payload: ref signature_payload,
                    } => {
                        if bool::from(Grade::ONE.leq(grade)) {
                            let bound = crate::judgements::package::unpack_binding(
                                grade,
                                abstracts,
                                signature_payload.as_ref(),
                                &atoms,
                            );
                            match bound {
                                | Ok(bound) => bound,
                                | Err(refusal) => {
                                    pending
                                        .marks
                                        .push(crate::judgements::package::refusal_mark(refusal));
                                    ValueType::Unknown
                                },
                            }
                        }
                        else {
                            pending.marks.push(Mark::Thunkability { available: grade });
                            ValueType::Unknown
                        }
                    },
                    | ValueType::Unknown => ValueType::Unknown,
                    | ref other => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::SHAPE_PACKAGE,
                            actual: Ty::Value(other.clone()),
                        });
                        ValueType::Unknown
                    },
                };
                let (body_dir, result) = match dir {
                    | Dir::Check(ref expected) => (Dir::Check(expected.clone()), expected.clone()),
                    | Dir::Infer => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::UNPACK_NEEDS_CHECK,
                        });
                        (Dir::Check(CompType::Unknown), CompType::Unknown)
                    },
                };
                let ascribed = signature.as_ref().clone();
                self.schedule_child_value(
                    0,
                    scrut,
                    Dir::Check(ascribed),
                    MarkFrame::CompUnpackAfterScrut {
                        pending,
                        binder,
                        bound,
                        body,
                        body_dir,
                        dir,
                        result,
                    },
                    run,
                );
            },
            | Comp::With(fst, snd) => {
                let (fst_dir, snd_dir, stuck) = match dir {
                    | Dir::Check(CompType::With(lhs, rhs)) => (
                        Dir::Check(lhs.as_ref().clone()),
                        Dir::Check(rhs.as_ref().clone()),
                        false,
                    ),
                    | Dir::Check(CompType::Unknown) => (
                        Dir::Check(CompType::Unknown),
                        Dir::Check(CompType::Unknown),
                        false,
                    ),
                    | Dir::Infer | Dir::Check(_) => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::WITH_NEEDS_WITH,
                        });
                        (Dir::Infer, Dir::Infer, true)
                    },
                };
                self.schedule_child_comp(
                    0,
                    fst,
                    fst_dir,
                    MarkFrame::CompWithAfterFst {
                        pending,
                        snd,
                        snd_dir,
                        stuck,
                    },
                    run,
                );
            },
            | Comp::Prj(side, target) => {
                self.schedule_child_comp(
                    0,
                    target,
                    Dir::Infer,
                    MarkFrame::CompPrjAfterTarget { pending, side, dir },
                    run,
                );
            },
            | Comp::RecordProj { record, label } => {
                self.schedule_child_value(
                    0,
                    record,
                    Dir::Infer,
                    MarkFrame::CompRecordProjAfterRecord {
                        pending,
                        label,
                        dir,
                    },
                    run,
                );
            },
            | Comp::Dup(thunked) => {
                let split = match dir {
                    | Dir::Check(CompType::F(ref payload, _)) => match payload.as_ref() {
                        | &ValueType::Prod(ref lhs, ref rhs) => {
                            match (lhs.as_ref(), rhs.as_ref()) {
                                | (&ValueType::Thunk(r, _), &ValueType::Thunk(s, _)) => {
                                    Some((r, s))
                                },
                                | _ => None,
                            }
                        },
                        | _ => None,
                    },
                    | _ => None,
                };
                let Some((r, s)) = split
                else {
                    pending.marks.push(Mark::Stuck {
                        hint: text::DUP_NEEDS_RETURNER_PRODUCT,
                    });
                    self.schedule_child_value(
                        0,
                        thunked,
                        Dir::Infer,
                        MarkFrame::CompPerformAfterArg {
                            pending,
                            constructed: CompType::Unknown,
                            dir,
                        },
                        run,
                    );
                    return;
                };
                self.schedule_child_value(
                    0,
                    thunked,
                    Dir::Infer,
                    MarkFrame::CompDupAfterThunk { pending, r, s, dir },
                    run,
                );
            },
            | Comp::Drop(thunked) => {
                self.schedule_child_value(
                    0,
                    thunked,
                    Dir::Infer,
                    MarkFrame::CompDropAfterThunk { pending, dir },
                    run,
                );
            },
            | Comp::Perform(sig, op, arg) => match sig
                .op(gandr_core_term::boundary::OperationName::from(op.as_str()))
                .cloned()
            {
                | Some(op_def) => {
                    let row = EffectRow::singleton(*sig);
                    self.schedule_child_value(
                        0,
                        arg,
                        Dir::Check(op_def.payload().clone()),
                        MarkFrame::CompPerformAfterArg {
                            pending,
                            constructed: CompType::returner_eff(op_def.reply().clone(), row),
                            dir,
                        },
                        run,
                    );
                },
                | None => {
                    pending.marks.push(Mark::Stuck {
                        hint: text::PERFORM_UNKNOWN_OP,
                    });
                    self.schedule_child_value(
                        0,
                        arg,
                        Dir::Infer,
                        MarkFrame::CompPerformAfterArg {
                            pending,
                            constructed: CompType::Unknown,
                            dir,
                        },
                        run,
                    );
                },
            },
            | Comp::Handle {
                sig,
                scrutinee,
                ret,
                ops,
            } => {
                let answer: CompType = match dir {
                    | Dir::Check(CompType::F(payload, eps)) => CompType::F(payload, eps),
                    | Dir::Check(CompType::Unknown) => CompType::Unknown,
                    | Dir::Infer => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::HANDLE_NEEDS_CHECK,
                        });
                        CompType::Unknown
                    },
                    | Dir::Check(_) => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::HANDLE_NEEDS_RETURNER,
                        });
                        CompType::Unknown
                    },
                };
                if resolve_handler_coverage(&sig, &ops).is_none() {
                    pending.marks.push(Mark::Stuck {
                        hint: text::HANDLER_CLAUSES_MISMATCH,
                    });
                }
                self.schedule_child_comp(
                    0,
                    scrutinee,
                    Dir::Infer,
                    MarkFrame::CompHandleAfterScrutinee {
                        pending,
                        sig: *sig,
                        ret,
                        ops: ops.into_iter(),
                        answer,
                    },
                    run,
                );
            },
            | Comp::Resume(stack, comp) => {
                self.schedule_child_value(
                    0,
                    stack,
                    Dir::Infer,
                    MarkFrame::CompResumeAfterStack { pending, comp, dir },
                    run,
                );
            },
            | Comp::Reset(body) => {
                let answer = match dir {
                    | Dir::Check(answer) => answer,
                    | Dir::Infer => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::RESET_NEEDS_CHECK,
                        });
                        CompType::Unknown
                    },
                };
                let saved = self.answer.replace(answer.clone());
                self.schedule_child_comp(
                    0,
                    body,
                    Dir::Check(answer),
                    MarkFrame::CompResetAfterBody { pending, saved },
                    run,
                );
            },
            | Comp::Shift(k, body) => {
                let Dir::Check(captured) = dir
                else {
                    pending.marks.push(Mark::Stuck {
                        hint: text::SHIFT_NEEDS_CHECK,
                    });
                    self.ctx
                        .bind(k, ValueType::stk(CompType::Unknown, CompType::Unknown));
                    self.schedule_child_comp(
                        0,
                        body,
                        Dir::Infer,
                        MarkFrame::CompShiftAfterBody {
                            pending,
                            captured: CompType::Unknown,
                        },
                        run,
                    );
                    return;
                };
                let answer = match self.answer.clone() {
                    | Some(answer) => answer,
                    | None => {
                        pending.marks.push(Mark::Stuck {
                            hint: text::SHIFT_NEEDS_RESET,
                        });
                        CompType::Unknown
                    },
                };
                self.ctx
                    .bind(k, ValueType::stk(captured.clone(), answer.clone()));
                self.schedule_child_comp(
                    0,
                    body,
                    Dir::Check(answer),
                    MarkFrame::CompShiftAfterBody { pending, captured },
                    run,
                );
            },
            | Comp::Hole(hole) => {
                pending.marks.push(Mark::EmptyHole(hole));
                let ty = finish_comp(CompType::Unknown, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | Comp::Native { prim, args } => {
                let ty = finish_comp(prim.residual_type(args.len()), dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | Comp::Walk {
                scrut,
                motive,
                base,
            } => {
                let scrut_value = scrut.as_ref().clone();
                self.schedule_child_value(
                    0,
                    scrut,
                    Dir::Infer,
                    MarkFrame::CompWalkAfterScrut {
                        pending,
                        motive: *motive,
                        base,
                        dir,
                        scrut_value,
                    },
                    run,
                );
            },
        }
    }

    /// Starts or continues stack typing for a `Value::Stk` node.
    fn start_stack_value(
        &mut self,
        item: StackValueWork,
        run: &mut MarkRun,
    )
    {
        let StackValueWork {
            stack,
            input,
            root_input,
            frame,
            mut pending,
            dir,
        } = item;
        match stack {
            | Stack::Empty => {
                let ty = finish_value(ValueType::stk(root_input, input), dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | Stack::Arg(value, rest) => {
                let (arg_ty, result_ty) = recover_pair(arrow_components(input), &mut pending.marks);
                self.schedule_child_value(
                    frame,
                    value,
                    Dir::Check(arg_ty),
                    MarkFrame::StackArgAfterValue {
                        rest,
                        result_ty,
                        root_input,
                        next_frame: frame.saturating_add(1),
                        pending,
                        dir,
                    },
                    run,
                );
            },
            | Stack::Bind(name, cont, rest) => {
                let (payload, consumed_row) =
                    recover_pair(returner_components(input), &mut pending.marks);
                self.ctx.bind(name, payload);
                self.schedule_child_comp(
                    frame,
                    cont,
                    Dir::Infer,
                    MarkFrame::StackBindAfterCont {
                        rest,
                        consumed_row,
                        root_input,
                        next_frame: frame.saturating_add(1),
                        pending,
                        dir,
                    },
                    run,
                );
            },
            | Stack::Prj(side, rest) => {
                let projected = recover(
                    with_component(input, side),
                    CompType::Unknown,
                    &mut pending.marks,
                );
                run.work = Some(MarkWork::StackValue(StackValueWork {
                    stack: unrc(rest),
                    input: projected,
                    root_input,
                    frame,
                    pending,
                    dir,
                }));
            },
        }
    }

    /// Schedules a value child at `index`.
    fn schedule_child_value<I>(
        &mut self,
        index: I,
        value: Rc<Value>,
        dir: Dir<ValueType>,
        frame: MarkFrame,
        run: &mut MarkRun,
    ) where
        I: Into<PathIndex>,
    {
        let id = self.intern_value_rc(&value);
        self.path.push(u32::from(index.into()));
        run.frames.push(frame);
        run.work = Some(MarkWork::Value {
            value: unrc(value),
            id,
            dir,
        });
    }

    /// Schedules a computation child at `index`.
    fn schedule_child_comp<I>(
        &mut self,
        index: I,
        comp: Rc<Comp>,
        dir: Dir<CompType>,
        frame: MarkFrame,
        run: &mut MarkRun,
    ) where
        I: Into<PathIndex>,
    {
        let id = self.intern_comp_rc(&comp);
        self.path.push(u32::from(index.into()));
        run.frames.push(frame);
        run.work = Some(MarkWork::Comp {
            comp: unrc(comp),
            id,
            dir,
        });
    }

    /// Schedules the next value-list element or finishes the list.
    fn schedule_value_elements<I>(
        &mut self,
        pending: PendingValue,
        mut elements: IntoIter<Rc<Value>>,
        next_index: I,
        dir: Dir<ValueType>,
        result_ty: ValueType,
        run: &mut MarkRun,
    ) where
        I: Into<PathIndex>,
    {
        let next_index = u32::from(next_index.into());
        if let Some(element) = elements.next() {
            self.schedule_child_value(
                next_index,
                element,
                dir.clone(),
                MarkFrame::ValueElements {
                    pending,
                    elements,
                    next_index: next_index.saturating_add(1),
                    dir,
                    result: result_ty,
                },
                run,
            );
        }
        else {
            run.result = Some(MarkResult::Value(pending.finish(self, result_ty)));
        }
    }

    /// Schedules the next record field or finishes the record.
    fn schedule_value_record<I>(
        &mut self,
        mut pending: PendingValue,
        mut fields: alloc::collections::btree_map::IntoIter<String, Rc<Value>>,
        next_index: I,
        dir: Dir<ValueType>,
        typed: BTreeMap<String, Rc<ValueType>>,
        run: &mut MarkRun,
    ) where
        I: Into<PathIndex>,
    {
        let next_index = u32::from(next_index.into());
        if let Some((label, value)) = fields.next() {
            let field_dir =
                dir.record_field_dir(gandr_core_term::boundary::FieldName::from(label.as_str()));
            self.schedule_child_value(
                next_index,
                value,
                field_dir,
                MarkFrame::ValueRecordAfterField {
                    pending,
                    fields,
                    next_index: next_index.saturating_add(1),
                    dir,
                    typed,
                    label,
                },
                run,
            );
        }
        else {
            let ty = finish_value(ValueType::Record(typed), dir, &mut pending.marks);
            run.result = Some(MarkResult::Value(pending.finish(self, ty)));
        }
    }

    /// Schedules the next data-case arm or finishes the case.
    fn schedule_data_case_arm<I>(
        &mut self,
        pending: PendingComp,
        mut arms: IntoIter<(String, Rc<Comp>)>,
        expected: CompType,
        result_ty: CompType,
        next_index: I,
        run: &mut MarkRun,
    ) where
        I: Into<PathIndex>,
    {
        let next_index = u32::from(next_index.into());
        if let Some((binder, body)) = arms.next() {
            self.ctx.bind(binder, ValueType::Unknown);
            self.schedule_child_comp(
                next_index,
                body,
                Dir::Check(expected.clone()),
                MarkFrame::CompDataCaseAfterArm {
                    pending,
                    arms,
                    expected,
                    next_index: next_index.saturating_add(1),
                },
                run,
            );
        }
        else {
            run.result = Some(MarkResult::Comp(pending.finish(self, result_ty)));
        }
    }

    /// Schedules the next handler operation or finishes the handler.
    fn schedule_handle_op(
        &mut self,
        mut pending: PendingComp,
        handler: HandleOps,
        run: &mut MarkRun,
    )
    {
        let HandleOps {
            sig,
            mut ops,
            answer,
            natural,
            next_index,
        } = handler;
        if let Some(clause) = ops.next() {
            let (payload_ty, resume_ty) = match sig.op(
                gandr_core_term::boundary::OperationName::from(clause.op.as_str()),
            ) {
                | Some(op_def) => (
                    op_def.payload().clone(),
                    resume_stack_type(&answer, op_def.reply().clone()),
                ),
                | None => (
                    ValueType::Unknown,
                    ValueType::stk(CompType::Unknown, CompType::Unknown),
                ),
            };
            self.ctx.bind(clause.payload.clone(), payload_ty);
            self.ctx.bind(clause.resume.clone(), resume_ty);
            self.schedule_child_comp(
                next_index,
                Rc::clone(&clause.body),
                Dir::Check(answer.clone()),
                MarkFrame::CompHandleAfterOp {
                    pending,
                    sig,
                    ops,
                    answer,
                    natural,
                    next_index: next_index.saturating_add(1),
                },
                run,
            );
        }
        else {
            let ty = finish_comp(natural, Dir::Check(answer), &mut pending.marks);
            run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
        }
    }

    /// Resumes one marking continuation frame.
    fn resume_mark_frame(
        &mut self,
        frame: MarkFrame,
        done: MarkResult,
        run: &mut MarkRun,
    )
    {
        match frame {
            | MarkFrame::ValuePairSigmaAfterFst {
                pending,
                snd,
                snd_dir,
                result: result_ty,
            } => {
                let _fst_ty = expect_value(done);
                let _popped = self.path.pop();
                self.schedule_child_value(
                    1,
                    snd,
                    snd_dir,
                    MarkFrame::ValuePairSigmaAfterSnd {
                        pending,
                        result: result_ty,
                    },
                    run,
                );
            },
            | MarkFrame::ValuePairSigmaAfterSnd {
                pending,
                result: result_ty,
            }
            | MarkFrame::ValueReturnAfterChild {
                pending,
                result: result_ty,
            } => {
                let _child_ty = expect_value(done);
                let _popped = self.path.pop();
                run.result = Some(MarkResult::Value(pending.finish(self, result_ty)));
            },
            | MarkFrame::ValuePairAfterFst {
                pending,
                snd,
                snd_dir,
                dir,
            } => {
                let fst_ty = expect_value(done);
                let _popped = self.path.pop();
                self.schedule_child_value(
                    1,
                    snd,
                    snd_dir,
                    MarkFrame::ValuePairAfterSnd {
                        pending,
                        dir,
                        fst_ty,
                    },
                    run,
                );
            },
            | MarkFrame::ValuePairAfterSnd {
                mut pending,
                dir,
                fst_ty,
            } => {
                let snd_ty = expect_value(done);
                let _popped = self.path.pop();
                let ty = finish_value(
                    ValueType::Prod(Rc::new(fst_ty), Rc::new(snd_ty)),
                    dir,
                    &mut pending.marks,
                );
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | MarkFrame::ValueFinishAfterChild {
                mut pending,
                constructed,
                dir,
            } => {
                let _child = expect_value(done);
                let _popped = self.path.pop();
                let ty = finish_value(constructed, dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | MarkFrame::ValueElements {
                pending,
                elements,
                next_index,
                dir,
                result: result_ty,
            } => {
                let _element = expect_value(done);
                let _popped = self.path.pop();
                self.schedule_value_elements(pending, elements, next_index, dir, result_ty, run);
            },
            | MarkFrame::ValueRecordAfterField {
                pending,
                fields,
                next_index,
                dir,
                mut typed,
                label,
            } => {
                let field_ty = expect_value(done);
                let _popped = self.path.pop();
                typed.insert(label, Rc::new(field_ty));
                self.schedule_value_record(pending, fields, next_index, dir, typed, run);
            },
            | MarkFrame::ValueThunkAfterBody {
                mut pending,
                grade,
                dir,
            } => {
                let body_ty = expect_comp(done);
                let _popped = self.path.pop();
                let ty = finish_value(
                    ValueType::Thunk(grade, Rc::new(body_ty)),
                    dir,
                    &mut pending.marks,
                );
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | MarkFrame::ValueThunkExpectedAfterBody {
                mut pending,
                expected_grade,
                expected_body,
            } => {
                let body_ty = expect_comp(done);
                let _popped = self.path.pop();
                let expected = ValueType::Thunk(expected_grade, expected_body);
                let ty = finish_value(
                    ValueType::Thunk(expected_grade, Rc::new(body_ty)),
                    Dir::Check(expected),
                    &mut pending.marks,
                );
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | MarkFrame::ValueAnnotAfterInner { mut pending, dir } => {
                let checked = expect_value(done);
                let _popped = self.path.pop();
                let ty = finish_value(checked, dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | MarkFrame::ValueHereAfterWitness {
                mut pending,
                witness,
                dir,
            } => {
                let witness_ty = expect_value(done);
                let _popped = self.path.pop();
                let witness_value = unrc(witness);
                let natural = ValueType::path(witness_ty, witness_value.clone(), witness_value);
                let ty = finish_value(natural, dir, &mut pending.marks);
                run.result = Some(MarkResult::Value(pending.finish(self, ty)));
            },
            | MarkFrame::CompAbsCheckArrowAfterBody {
                mut pending,
                arg,
                res,
            } => {
                let res_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                let ty = finish_comp(
                    CompType::Arrow(Rc::clone(&arg), Rc::new(res_ty)),
                    Dir::Check(CompType::Arrow(arg, res)),
                    &mut pending.marks,
                );
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompAbsCheckUnknownAfterBody { mut pending } => {
                let res_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                let ty = finish_comp(
                    CompType::Arrow(Rc::new(ValueType::Unknown), Rc::new(res_ty)),
                    Dir::Check(CompType::Unknown),
                    &mut pending.marks,
                );
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompAbsAnnotatedAfterBody {
                mut pending,
                annot_ty,
                dir,
            } => {
                let res_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                let ty = finish_comp(
                    CompType::Arrow(annot_ty, Rc::new(res_ty)),
                    dir,
                    &mut pending.marks,
                );
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompAbsStuckAfterBody { pending } => {
                let _body_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                run.result = Some(MarkResult::Comp(pending.finish(self, CompType::Unknown)));
            },
            | MarkFrame::CompAppAfterHead {
                mut pending,
                arg,
                dir,
            } => {
                let head_ty = expect_comp(done);
                let _popped = self.path.pop();
                let (param, res) = recover_pair(arrow_components(head_ty), &mut pending.marks);
                self.schedule_child_value(
                    1,
                    arg,
                    Dir::Check(param),
                    MarkFrame::CompAppAfterArg { pending, res, dir },
                    run,
                );
            },
            | MarkFrame::CompAppAfterArg {
                mut pending,
                res,
                dir,
            } => {
                let _arg_ty = expect_value(done);
                let _popped = self.path.pop();
                let ty = finish_comp(res, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompRetAfterPayload { mut pending, dir } => {
                let payload_ty = expect_value(done);
                let _popped = self.path.pop();
                let ty = finish_comp(CompType::returner(payload_ty), dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompBindAfterBound {
                mut pending,
                name,
                cont,
                dir,
            } => {
                let bound_ty = expect_comp(done);
                let _popped = self.path.pop();
                let (payload, bound_row): (ValueType, EffectRow) = match bound_ty {
                    | CompType::F(payload, row) => (unrc(payload), row),
                    | CompType::Unknown => (ValueType::Unknown, EffectRow::EMPTY),
                    | other => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::SHAPE_RETURNER,
                            actual: Ty::Comp(other),
                        });
                        (ValueType::Unknown, EffectRow::EMPTY)
                    },
                };
                self.ctx.bind(name, payload);
                self.schedule_child_comp(
                    1,
                    cont,
                    dir.clone(),
                    MarkFrame::CompBindAfterCont {
                        pending,
                        bound_row,
                        dir,
                    },
                    run,
                );
            },
            | MarkFrame::CompBindAfterCont {
                mut pending,
                bound_row,
                dir,
            } => {
                let cont_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                let sequenced = recover(
                    combine_bind_row(&bound_row, cont_ty),
                    CompType::Unknown,
                    &mut pending.marks,
                );
                let ty = finish_comp(sequenced, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompForceAfterThunk { mut pending, dir } => {
                let thunk_ty = expect_value(done);
                let _popped = self.path.pop();
                let ty = match thunk_ty {
                    | ValueType::Thunk(grade, body) => {
                        if !bool::from(Grade::ONE.leq(grade)) {
                            pending.marks.push(Mark::Thunkability { available: grade });
                        }
                        finish_comp(unrc(body), dir, &mut pending.marks)
                    },
                    | ValueType::Unknown => finish_comp(CompType::Unknown, dir, &mut pending.marks),
                    | other => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::SHAPE_THUNK,
                            actual: Ty::Value(other),
                        });
                        finish_comp(CompType::Unknown, dir, &mut pending.marks)
                    },
                };
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompCaseAfterScrut {
                mut pending,
                arm_fst,
                arm_snd,
                expected,
            } => {
                let scrut_ty = expect_value(done);
                let _popped = self.path.pop();
                let (fst_ty, snd_ty): (ValueType, ValueType) = match scrut_ty {
                    | ValueType::Sum(lhs, rhs) => (unrc(lhs), unrc(rhs)),
                    | ValueType::Unknown => (ValueType::Unknown, ValueType::Unknown),
                    | scrut_id @ ValueType::Path { .. } => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::CASE_ON_PATH_WITHOUT_K,
                            actual: Ty::Value(scrut_id),
                        });
                        (ValueType::Unknown, ValueType::Unknown)
                    },
                    | other => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::SHAPE_SUM,
                            actual: Ty::Value(other),
                        });
                        (ValueType::Unknown, ValueType::Unknown)
                    },
                };
                let (fst_name, fst_body) = arm_fst;
                let (snd_name, snd_body) = arm_snd;
                self.ctx.bind(fst_name, fst_ty);
                self.schedule_child_comp(
                    1,
                    fst_body,
                    Dir::Check(expected.clone()),
                    MarkFrame::CompCaseAfterFst {
                        pending,
                        snd_name,
                        snd_body,
                        snd_ty,
                        expected,
                    },
                    run,
                );
            },
            | MarkFrame::CompCaseAfterFst {
                pending,
                snd_name,
                snd_body,
                snd_ty,
                expected,
            } => {
                let _fst_result = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                self.ctx.bind(snd_name, snd_ty);
                self.schedule_child_comp(
                    2,
                    snd_body,
                    Dir::Check(expected),
                    MarkFrame::CompCaseAfterSnd { pending },
                    run,
                );
            },
            | MarkFrame::CompCaseAfterSnd { pending } => {
                let snd_result = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                run.result = Some(MarkResult::Comp(pending.finish(self, snd_result)));
            },
            | MarkFrame::CompDataCaseAfterScrut {
                mut pending,
                arms,
                expected,
                result: result_ty,
                next_index,
            } => {
                let scrut_ty = expect_value(done);
                let _popped = self.path.pop();
                match scrut_ty {
                    | ValueType::Data { .. } | ValueType::Unknown => {},
                    | other => pending.marks.push(Mark::ShapeMismatch {
                        expected: text::SHAPE_DATA,
                        actual: Ty::Value(other),
                    }),
                }
                self.schedule_data_case_arm(pending, arms, expected, result_ty, next_index, run);
            },
            | MarkFrame::CompDataCaseAfterArm {
                pending,
                arms,
                expected,
                next_index,
            } => {
                let arm_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                self.schedule_data_case_arm(pending, arms, expected, arm_ty, next_index, run);
            },
            | MarkFrame::CompListCaseAfterScrut {
                mut pending,
                nil,
                cons_arm,
                expected,
            } => {
                let scrut_ty = expect_value(done);
                let _popped = self.path.pop();
                let (head_ty, tail_ty): (ValueType, ValueType) = match scrut_ty {
                    | ValueType::List(elem) => (elem.as_ref().clone(), ValueType::List(elem)),
                    | ValueType::Unknown => (ValueType::Unknown, ValueType::Unknown),
                    | other => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::SHAPE_LIST,
                            actual: Ty::Value(other),
                        });
                        (ValueType::Unknown, ValueType::Unknown)
                    },
                };
                let (head, tail, cons) = cons_arm;
                self.schedule_child_comp(
                    1,
                    nil,
                    Dir::Check(expected.clone()),
                    MarkFrame::CompListCaseAfterNil {
                        pending,
                        head,
                        tail,
                        cons,
                        head_ty,
                        tail_ty,
                        expected,
                    },
                    run,
                );
            },
            | MarkFrame::CompListCaseAfterNil {
                pending,
                head,
                tail,
                cons,
                head_ty,
                tail_ty,
                expected,
            } => {
                let _nil_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.bind(head, head_ty);
                self.ctx.bind(tail, tail_ty);
                self.schedule_child_comp(
                    2,
                    cons,
                    Dir::Check(expected),
                    MarkFrame::CompListCaseAfterCons { pending },
                    run,
                );
            },
            | MarkFrame::CompListCaseAfterCons { pending } => {
                let cons_result = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                self.ctx.unbind();
                run.result = Some(MarkResult::Comp(pending.finish(self, cons_result)));
            },
            | MarkFrame::CompSplitAfterScrut {
                mut pending,
                binders,
                motive,
                body,
                dir,
                scrut_value,
            } => {
                let scrut_ty = expect_value(done);
                let _popped = self.path.pop();
                let (fst_name, snd_name) = binders;
                let motive_ref = motive.as_ref();
                let (fst_ty, snd_ty, body_expected, split_result): (
                    ValueType,
                    ValueType,
                    Dir<CompType>,
                    CompType,
                ) = match scrut_ty {
                    | ValueType::Prod(lhs, rhs) => {
                        let (body_expected, result_ty) = split_expectations(
                            motive_ref,
                            &dir,
                            &fst_name,
                            &snd_name,
                            &scrut_value,
                        );
                        (unrc(lhs), unrc(rhs), body_expected, result_ty)
                    },
                    | ValueType::Sigma {
                        fst: head,
                        binder,
                        snd: tail,
                    } => {
                        let tail_ty = gandr_core_term::identity::subst_valuetype(
                            &tail,
                            gandr_core_term::boundary::NameRef::from(binder.as_str()),
                            &Value::var(&fst_name),
                        );
                        let (body_expected, result_ty) = split_expectations(
                            motive_ref,
                            &dir,
                            &fst_name,
                            &snd_name,
                            &scrut_value,
                        );
                        (unrc(head), tail_ty, body_expected, result_ty)
                    },
                    | ValueType::Unknown => {
                        let (body_expected, result_ty) =
                            split_unknown_expectations(motive_ref, &dir);
                        (
                            ValueType::Unknown,
                            ValueType::Unknown,
                            body_expected,
                            result_ty,
                        )
                    },
                    | other => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::SHAPE_PROD,
                            actual: Ty::Value(other),
                        });
                        let (body_expected, result_ty) = split_expectations(
                            motive_ref,
                            &dir,
                            &fst_name,
                            &snd_name,
                            &scrut_value,
                        );
                        (
                            ValueType::Unknown,
                            ValueType::Unknown,
                            body_expected,
                            result_ty,
                        )
                    },
                };
                if motive.is_none() && matches!(dir, Dir::Infer) {
                    pending.marks.push(Mark::Stuck {
                        hint: text::SPLIT_NEEDS_MOTIVE,
                    });
                }
                self.ctx.bind(fst_name, fst_ty);
                self.ctx.bind(snd_name, snd_ty);
                self.schedule_child_comp(
                    1,
                    body,
                    body_expected,
                    MarkFrame::CompSplitAfterBody {
                        pending,
                        dir,
                        result: split_result,
                    },
                    run,
                );
            },
            | MarkFrame::CompSplitAfterBody {
                mut pending,
                dir,
                result: split_result,
            } => {
                let _body_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                self.ctx.unbind();
                let ty = finish_comp(split_result, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompUnpackAfterScrut {
                pending,
                binder,
                bound,
                body,
                body_dir,
                dir,
                result,
            } => {
                let _scrut_ty = expect_value(done);
                let _popped = self.path.pop();
                self.ctx.bind(binder, bound);
                self.schedule_child_comp(
                    1,
                    body,
                    body_dir,
                    MarkFrame::CompUnpackAfterBody {
                        pending,
                        dir,
                        result,
                    },
                    run,
                );
            },
            | MarkFrame::CompUnpackAfterBody {
                mut pending,
                dir,
                result,
            } => {
                let _body_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                let ty = finish_comp(result, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompWithAfterFst {
                pending,
                snd,
                snd_dir,
                stuck,
            } => {
                let fst_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.schedule_child_comp(
                    1,
                    snd,
                    snd_dir,
                    MarkFrame::CompWithAfterSnd {
                        pending,
                        fst_ty,
                        stuck,
                    },
                    run,
                );
            },
            | MarkFrame::CompWithAfterSnd {
                pending,
                fst_ty,
                stuck,
            } => {
                let snd_ty = expect_comp(done);
                let _popped = self.path.pop();
                let ty = if stuck {
                    CompType::Unknown
                }
                else {
                    CompType::With(Rc::new(fst_ty), Rc::new(snd_ty))
                };
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompPrjAfterTarget {
                mut pending,
                side,
                dir,
            } => {
                let target_ty = expect_comp(done);
                let _popped = self.path.pop();
                let projected = recover(
                    with_component(target_ty, side),
                    CompType::Unknown,
                    &mut pending.marks,
                );
                let ty = finish_comp(projected, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompRecordProjAfterRecord {
                mut pending,
                label,
                dir,
            } => {
                let record_ty = expect_value(done);
                let _popped = self.path.pop();
                let projected = match record_ty {
                    | ValueType::Record(fields) => match fields.get(label.as_str()) {
                        | Some(field_ty) => CompType::returner(field_ty.as_ref().clone()),
                        | None => {
                            pending.marks.push(Mark::Stuck {
                                hint: text::RECORD_NO_FIELD,
                            });
                            CompType::Unknown
                        },
                    },
                    | ValueType::Unknown => CompType::Unknown,
                    | other => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::SHAPE_RECORD,
                            actual: Ty::Value(other),
                        });
                        CompType::Unknown
                    },
                };
                let ty = finish_comp(projected, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompDupAfterThunk {
                mut pending,
                r,
                s,
                dir,
            } => {
                let thunk_ty = expect_value(done);
                let _popped = self.path.pop();
                let body = match thunk_ty {
                    | ValueType::Thunk(grade, body) => {
                        let total = r.plus(s);
                        if !bool::from(total.leq(grade)) {
                            pending.marks.push(Mark::GradeBudget {
                                required: total,
                                available: grade,
                            });
                        }
                        unrc(body)
                    },
                    | ValueType::Unknown => CompType::Unknown,
                    | other => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::SHAPE_THUNK,
                            actual: Ty::Value(other),
                        });
                        CompType::Unknown
                    },
                };
                let natural = CompType::returner(ValueType::Prod(
                    Rc::new(ValueType::Thunk(r, Rc::new(body.clone()))),
                    Rc::new(ValueType::Thunk(s, Rc::new(body))),
                ));
                let ty = finish_comp(natural, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompDropAfterThunk { mut pending, dir } => {
                let thunk_ty = expect_value(done);
                let _popped = self.path.pop();
                match thunk_ty {
                    | ValueType::Thunk(..) | ValueType::Unknown => {},
                    | other => pending.marks.push(Mark::ShapeMismatch {
                        expected: text::SHAPE_THUNK,
                        actual: Ty::Value(other),
                    }),
                }
                let ty = finish_comp(CompType::returner(ValueType::Unit), dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompPerformAfterArg {
                mut pending,
                constructed,
                dir,
            } => {
                let _arg_ty = expect_value(done);
                let _popped = self.path.pop();
                let ty = finish_comp(constructed, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompHandleAfterScrutinee {
                mut pending,
                sig,
                ret,
                ops,
                answer,
            } => {
                let t_ty = expect_comp(done);
                let _popped = self.path.pop();
                let (eps_t, payload_a): (EffectRow, ValueType) = match t_ty {
                    | CompType::F(payload, row) => (row, unrc(payload)),
                    | CompType::Unknown => (EffectRow::EMPTY, ValueType::Unknown),
                    | other => {
                        pending.marks.push(Mark::ShapeMismatch {
                            expected: text::SHAPE_RETURNER,
                            actual: Ty::Comp(other),
                        });
                        (EffectRow::EMPTY, ValueType::Unknown)
                    },
                };
                let residual = eps_t.without(sig.name());
                let natural = handle_natural_type(&answer, residual);
                let (ret_var, ret_body) = ret;
                self.ctx.bind(ret_var, payload_a);
                self.schedule_child_comp(
                    1,
                    ret_body,
                    Dir::Check(answer.clone()),
                    MarkFrame::CompHandleAfterRet {
                        pending,
                        sig,
                        ops,
                        answer,
                        natural,
                        next_index: 2,
                    },
                    run,
                );
            },
            | MarkFrame::CompHandleAfterRet {
                pending,
                sig,
                ops,
                answer,
                natural,
                next_index,
            } => {
                let _ret_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                self.schedule_handle_op(
                    pending,
                    HandleOps {
                        sig,
                        ops,
                        answer,
                        natural,
                        next_index,
                    },
                    run,
                );
            },
            | MarkFrame::CompHandleAfterOp {
                pending,
                sig,
                ops,
                answer,
                natural,
                next_index,
            } => {
                let _op_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                self.ctx.unbind();
                self.schedule_handle_op(
                    pending,
                    HandleOps {
                        sig,
                        ops,
                        answer,
                        natural,
                        next_index,
                    },
                    run,
                );
            },
            | MarkFrame::CompResumeAfterStack {
                mut pending,
                comp,
                dir,
            } => {
                let stack_ty = expect_value(done);
                let _popped = self.path.pop();
                let (consumed, delivered) =
                    recover_pair(stk_components(stack_ty), &mut pending.marks);
                self.schedule_child_comp(
                    1,
                    comp,
                    Dir::Check(consumed),
                    MarkFrame::CompResumeAfterComp {
                        pending,
                        delivered,
                        dir,
                    },
                    run,
                );
            },
            | MarkFrame::CompResumeAfterComp {
                mut pending,
                delivered,
                dir,
            } => {
                let _comp_ty = expect_comp(done);
                let _popped = self.path.pop();
                let ty = finish_comp(delivered, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::CompResetAfterBody { pending, saved } => {
                let body_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.answer = saved;
                run.result = Some(MarkResult::Comp(pending.finish(self, body_ty)));
            },
            | MarkFrame::CompShiftAfterBody { pending, captured } => {
                let _body_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                run.result = Some(MarkResult::Comp(pending.finish(self, captured)));
            },
            | MarkFrame::CompWalkAfterScrut {
                mut pending,
                motive,
                base,
                dir,
                scrut_value,
            } => {
                let scrut_ty = expect_value(done);
                let _popped = self.path.pop();
                let (carrier, base_expected, result_ty): (ValueType, CompType, CompType) =
                    match scrut_ty {
                        | ValueType::Path { ty, lhs, rhs } => {
                            let carrier = unrc(ty);
                            let diagonal = base_diagonal_type(&motive, &base.x);
                            let result_ty = motive_result_type(
                                &motive,
                                lhs.as_ref(),
                                rhs.as_ref(),
                                &scrut_value,
                            );
                            (carrier, diagonal, result_ty)
                        },
                        | ValueType::Unknown => {
                            (ValueType::Unknown, CompType::Unknown, CompType::Unknown)
                        },
                        | other => {
                            pending.marks.push(Mark::ShapeMismatch {
                                expected: text::SHAPE_PATH,
                                actual: Ty::Value(other),
                            });
                            (ValueType::Unknown, CompType::Unknown, CompType::Unknown)
                        },
                    };
                self.ctx.bind(base.x.clone(), carrier);
                self.schedule_child_comp(
                    1,
                    base.body,
                    Dir::Check(base_expected),
                    MarkFrame::CompWalkAfterBase {
                        pending,
                        result: result_ty,
                        dir,
                    },
                    run,
                );
            },
            | MarkFrame::CompWalkAfterBase {
                mut pending,
                result: result_ty,
                dir,
            } => {
                let _base_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                let ty = finish_comp(result_ty, dir, &mut pending.marks);
                run.result = Some(MarkResult::Comp(pending.finish(self, ty)));
            },
            | MarkFrame::StackArgAfterValue {
                rest,
                result_ty,
                root_input,
                next_frame,
                pending,
                dir,
            } => {
                let _value_ty = expect_value(done);
                let _popped = self.path.pop();
                run.work = Some(MarkWork::StackValue(StackValueWork {
                    stack: unrc(rest),
                    input: result_ty,
                    root_input,
                    frame: next_frame,
                    pending,
                    dir,
                }));
            },
            | MarkFrame::StackBindAfterCont {
                mut pending,
                rest,
                consumed_row,
                root_input,
                next_frame,
                dir,
            } => {
                let cont_ty = expect_comp(done);
                let _popped = self.path.pop();
                self.ctx.unbind();
                let sequenced = recover(
                    combine_bind_row(&consumed_row, cont_ty),
                    CompType::Unknown,
                    &mut pending.marks,
                );
                run.work = Some(MarkWork::StackValue(StackValueWork {
                    stack: unrc(rest),
                    input: sequenced,
                    root_input,
                    frame: next_frame,
                    pending,
                    dir,
                }));
            },
        }
    }

    /// Drives arena interning work to completion without host recursion.
    ///
    /// # Contract
    /// - ensures: returns the root work item's result; every scheduled child is
    ///   interned before its frame resumes.
    /// - panics: none in release builds. The interner alternates the two
    ///   [`InternRun`] slots, so a completed result is always present when no
    ///   work is pending; a `debug_assert!` guards that invariant in test /
    ///   debug builds, and the absent-id fallback (the allocation-failure
    ///   shape) keeps the driver total — it is never reached.
    fn drive_interning(
        &mut self,
        root: InternWork,
    ) -> InternResult
    {
        let mut run = InternRun {
            frames: Vec::new(),
            work: Some(root),
            result: None,
        };
        loop {
            if let Some(item) = run.work.take() {
                self.start_intern_work(item, &mut run);
                continue;
            }
            debug_assert!(
                run.result.is_some(),
                "interner stalled without work or result"
            );
            let Some(done) = run.result.take()
            else {
                return InternResult::Value(None);
            };
            let Some(frame) = run.frames.pop()
            else {
                return done;
            };
            self.resume_intern_frame(frame, done, &mut run);
        }
    }

    /// Starts one interning work item.
    fn start_intern_work(
        &mut self,
        item: InternWork,
        run: &mut InternRun,
    )
    {
        match item {
            | InternWork::Value { value, cache_key } => {
                self.start_intern_value(value, cache_key, run);
            },
            | InternWork::Comp { comp, cache_key } => {
                self.start_intern_comp(comp, cache_key, run);
            },
            | InternWork::Stack { stack, cache_key } => {
                self.start_intern_stack(stack, cache_key, run);
            },
        }
    }

    /// Starts value interning.
    fn start_intern_value(
        &mut self,
        value: Value,
        cache_key: Option<*const Value>,
        run: &mut InternRun,
    )
    {
        match value {
            | Value::Var(name) => {
                run.result = Some(self.finish_intern_value(ValueNode::Var(name), cache_key));
            },
            | Value::Unit => {
                run.result = Some(self.finish_intern_value(ValueNode::Unit, cache_key));
            },
            | Value::Int(literal) => {
                run.result = Some(self.finish_intern_value(ValueNode::Int(literal), cache_key));
            },
            | Value::Str(literal) => {
                run.result = Some(self.finish_intern_value(ValueNode::Str(literal), cache_key));
            },
            | Value::Num(literal) => {
                run.result = Some(self.finish_intern_value(ValueNode::Num(literal), cache_key));
            },
            | Value::Pair(fst, snd) => self.schedule_intern_value_rc(
                fst,
                InternFrame::ValuePairAfterFst { cache_key, snd },
                run,
            ),
            | Value::Inj(side, payload) => self.schedule_intern_value_rc(
                payload,
                InternFrame::ValueUnaryAfterChild {
                    cache_key,
                    kind: ValueUnaryIntern::Inj(side),
                },
                run,
            ),
            | Value::List(elements) => {
                self.schedule_intern_value_list(cache_key, elements.into_iter(), Vec::new(), run);
            },
            | Value::Record(fields) => self.schedule_intern_value_record(
                cache_key,
                fields.into_iter(),
                BTreeMap::new(),
                run,
            ),
            | Value::Thunk(grade, body) => self.schedule_intern_comp_rc(
                body,
                InternFrame::ValueThunkAfterBody { cache_key, grade },
                run,
            ),
            | Value::Annot(inner, ty) => self.schedule_intern_value_rc(
                inner,
                InternFrame::ValueAnnotAfterInner { cache_key, ty },
                run,
            ),
            | Value::Hole(hole) => {
                run.result = Some(self.finish_intern_value(ValueNode::Hole(hole), cache_key));
            },
            | Value::Stk(stack) => self.schedule_intern_stack_rc(
                stack,
                InternFrame::ValueStkAfterStack { cache_key },
                run,
            ),
            | Value::Here(witness) => self.schedule_intern_value_rc(
                witness,
                InternFrame::ValueUnaryAfterChild {
                    cache_key,
                    kind: ValueUnaryIntern::Here,
                },
                run,
            ),
            | Value::Ctor { id, tag, payload } => self.schedule_intern_value_rc(
                payload,
                InternFrame::ValueCtorAfterPayload { cache_key, id, tag },
                run,
            ),
            | Value::Pack { witnesses, payload } => {
                let mut interned = Vec::with_capacity(witnesses.len());
                for witness in &witnesses {
                    match self.arena.alloc_value_type(witness.as_ref()).ok() {
                        | Some(id) => interned.push(id),
                        | None => {
                            run.result = Some(InternResult::Value(None));
                            return;
                        },
                    }
                }
                self.schedule_intern_value_rc(
                    payload,
                    InternFrame::ValuePackAfterPayload {
                        cache_key,
                        witnesses: interned,
                    },
                    run,
                );
            },
        }
    }

    /// Starts computation interning.
    fn start_intern_comp(
        &mut self,
        comp: Comp,
        cache_key: Option<*const Comp>,
        run: &mut InternRun,
    )
    {
        match comp {
            | Comp::Abs(name, annot, body) => {
                let annot = match annot.as_ref() {
                    | Some(ty) => match self.intern_value_type(ty) {
                        | Some(ty) => Some(ty),
                        | None => {
                            run.result = Some(InternResult::Comp(None));
                            return;
                        },
                    },
                    | None => None,
                };
                self.schedule_intern_comp_rc(
                    body,
                    InternFrame::CompUnaryCompAfterChild {
                        cache_key,
                        kind: CompUnaryCompIntern::Abs(name, annot),
                    },
                    run,
                );
            },
            | Comp::App(head, arg) => self.schedule_intern_comp_rc(
                head,
                InternFrame::CompAppAfterHead { cache_key, arg },
                run,
            ),
            | Comp::Ret(payload) => self.schedule_intern_value_rc(
                payload,
                InternFrame::CompUnaryValueAfterChild {
                    cache_key,
                    kind: CompUnaryValueIntern::Ret,
                },
                run,
            ),
            | Comp::Bind(bound, name, cont) => self.schedule_intern_comp_rc(
                bound,
                InternFrame::CompBindAfterBound {
                    cache_key,
                    name,
                    cont,
                },
                run,
            ),
            | Comp::Force(thunked) => self.schedule_intern_value_rc(
                thunked,
                InternFrame::CompUnaryValueAfterChild {
                    cache_key,
                    kind: CompUnaryValueIntern::Force,
                },
                run,
            ),
            | Comp::Case(scrut, arm_fst, arm_snd) => self.schedule_intern_value_rc(
                scrut,
                InternFrame::CompCaseAfterScrut {
                    cache_key,
                    arm_fst,
                    arm_snd,
                },
                run,
            ),
            | Comp::ListCase {
                scrut,
                nil,
                head,
                tail,
                cons,
            } => self.schedule_intern_value_rc(
                scrut,
                InternFrame::CompListCaseAfterScrut {
                    cache_key,
                    nil,
                    head,
                    tail,
                    cons,
                },
                run,
            ),
            | Comp::Split {
                scrut,
                fst_name,
                snd_name,
                motive,
                body,
            } => self.schedule_intern_value_rc(
                scrut,
                InternFrame::CompSplitAfterScrut {
                    cache_key,
                    fst_name,
                    snd_name,
                    motive: motive.map(|boxed| *boxed),
                    body,
                },
                run,
            ),
            | Comp::DataCase(scrut, arms) => self.schedule_intern_value_rc(
                scrut,
                InternFrame::CompDataCaseAfterScrut {
                    cache_key,
                    arms: arms.into_iter(),
                    arm_ids: Vec::new(),
                },
                run,
            ),
            | Comp::RecordProj { record, label } => self.schedule_intern_value_rc(
                record,
                InternFrame::CompUnaryValueAfterChild {
                    cache_key,
                    kind: CompUnaryValueIntern::RecordProj(label),
                },
                run,
            ),
            | Comp::With(fst, snd) => self.schedule_intern_comp_rc(
                fst,
                InternFrame::CompWithAfterFst { cache_key, snd },
                run,
            ),
            | Comp::Prj(side, target) => self.schedule_intern_comp_rc(
                target,
                InternFrame::CompUnaryCompAfterChild {
                    cache_key,
                    kind: CompUnaryCompIntern::Prj(side),
                },
                run,
            ),
            | Comp::Dup(thunked) => self.schedule_intern_value_rc(
                thunked,
                InternFrame::CompUnaryValueAfterChild {
                    cache_key,
                    kind: CompUnaryValueIntern::Dup,
                },
                run,
            ),
            | Comp::Drop(thunked) => self.schedule_intern_value_rc(
                thunked,
                InternFrame::CompUnaryValueAfterChild {
                    cache_key,
                    kind: CompUnaryValueIntern::Drop,
                },
                run,
            ),
            | Comp::Perform(sig, op, arg) => self.schedule_intern_value_rc(
                arg,
                InternFrame::CompUnaryValueAfterChild {
                    cache_key,
                    kind: CompUnaryValueIntern::Perform(*sig, op),
                },
                run,
            ),
            | Comp::Handle {
                sig,
                scrutinee,
                ret,
                ops,
            } => self.schedule_intern_comp_rc(
                scrutinee,
                InternFrame::CompHandleAfterScrutinee {
                    cache_key,
                    sig: *sig,
                    ret,
                    ops: ops.into_iter(),
                },
                run,
            ),
            | Comp::Resume(stack, comp) => self.schedule_intern_value_rc(
                stack,
                InternFrame::CompResumeAfterStack { cache_key, comp },
                run,
            ),
            | Comp::Reset(body) => self.schedule_intern_comp_rc(
                body,
                InternFrame::CompUnaryCompAfterChild {
                    cache_key,
                    kind: CompUnaryCompIntern::Reset,
                },
                run,
            ),
            | Comp::Shift(k, body) => self.schedule_intern_comp_rc(
                body,
                InternFrame::CompUnaryCompAfterChild {
                    cache_key,
                    kind: CompUnaryCompIntern::Shift(k),
                },
                run,
            ),
            | Comp::Hole(hole) => {
                run.result = Some(self.finish_intern_comp(CompNode::Hole(hole), cache_key));
            },
            | Comp::Native { prim, args } => {
                self.schedule_intern_native_args(
                    cache_key,
                    prim,
                    args.into_iter(),
                    Vec::new(),
                    run,
                );
            },
            | Comp::Walk {
                scrut,
                motive,
                base,
            } => self.schedule_intern_value_rc(
                scrut,
                InternFrame::CompWalkAfterScrut {
                    cache_key,
                    motive: *motive,
                    base,
                },
                run,
            ),
            | Comp::Unpack {
                scrut,
                signature,
                atoms,
                binder,
                body,
            } => match self.arena.alloc_value_type(signature.as_ref()).ok() {
                | Some(signature) => self.schedule_intern_value_rc(
                    scrut,
                    InternFrame::CompUnpackAfterScrut {
                        cache_key,
                        signature,
                        atoms,
                        binder,
                        body,
                    },
                    run,
                ),
                | None => run.result = Some(InternResult::Comp(None)),
            },
        }
    }

    /// Starts stack interning.
    fn start_intern_stack(
        &mut self,
        stack: Stack,
        cache_key: Option<*const Stack>,
        run: &mut InternRun,
    )
    {
        match stack {
            | Stack::Empty => {
                run.result = Some(self.finish_intern_stack(StackNode::Empty, cache_key));
            },
            | Stack::Arg(value, rest) => self.schedule_intern_value_rc(
                value,
                InternFrame::StackArgAfterValue { cache_key, rest },
                run,
            ),
            | Stack::Bind(name, cont, rest) => self.schedule_intern_comp_rc(
                cont,
                InternFrame::StackBindAfterCont {
                    cache_key,
                    name,
                    rest,
                },
                run,
            ),
            | Stack::Prj(side, rest) => self.schedule_intern_stack_rc(
                rest,
                InternFrame::StackPrjAfterRest { cache_key, side },
                run,
            ),
        }
    }

    /// Schedules interning for a shared value child.
    fn schedule_intern_value_rc(
        &self,
        value: Rc<Value>,
        frame: InternFrame,
        run: &mut InternRun,
    )
    {
        let key = Rc::as_ptr(&value);
        run.frames.push(frame);
        if let Some(id) = self.value_ids.get(&key).copied() {
            run.result = Some(InternResult::Value(Some(id)));
        }
        else {
            run.work = Some(InternWork::Value {
                value: unrc(value),
                cache_key: Some(key),
            });
        }
    }

    /// Schedules interning for a shared computation child.
    fn schedule_intern_comp_rc(
        &self,
        comp: Rc<Comp>,
        frame: InternFrame,
        run: &mut InternRun,
    )
    {
        let key = Rc::as_ptr(&comp);
        run.frames.push(frame);
        if let Some(id) = self.comp_ids.get(&key).copied() {
            run.result = Some(InternResult::Comp(Some(id)));
        }
        else {
            run.work = Some(InternWork::Comp {
                comp: unrc(comp),
                cache_key: Some(key),
            });
        }
    }

    /// Schedules interning for a shared stack child.
    fn schedule_intern_stack_rc(
        &self,
        stack: Rc<Stack>,
        frame: InternFrame,
        run: &mut InternRun,
    )
    {
        let key = Rc::as_ptr(&stack);
        run.frames.push(frame);
        if let Some(id) = self.stack_ids.get(&key).copied() {
            run.result = Some(InternResult::Stack(Some(id)));
        }
        else {
            run.work = Some(InternWork::Stack {
                stack: unrc(stack),
                cache_key: Some(key),
            });
        }
    }

    /// Schedules the next value-list element for interning.
    fn schedule_intern_value_list(
        &mut self,
        cache_key: Option<*const Value>,
        mut elements: IntoIter<Rc<Value>>,
        ids: Vec<ValueNodeId>,
        run: &mut InternRun,
    )
    {
        if let Some(element) = elements.next() {
            self.schedule_intern_value_rc(
                element,
                InternFrame::ValueListAfterElement {
                    cache_key,
                    elements,
                    ids,
                },
                run,
            );
        }
        else {
            run.result = Some(self.finish_intern_value(ValueNode::List(ids), cache_key));
        }
    }

    /// Schedules the next value-record field for interning.
    fn schedule_intern_value_record(
        &mut self,
        cache_key: Option<*const Value>,
        mut fields: alloc::collections::btree_map::IntoIter<String, Rc<Value>>,
        ids: BTreeMap<String, ValueNodeId>,
        run: &mut InternRun,
    )
    {
        if let Some((label, value)) = fields.next() {
            self.schedule_intern_value_rc(
                value,
                InternFrame::ValueRecordAfterField {
                    cache_key,
                    fields,
                    ids,
                    label,
                },
                run,
            );
        }
        else {
            run.result = Some(self.finish_intern_value(ValueNode::Record(ids), cache_key));
        }
    }

    /// Schedules the next native argument for interning.
    fn schedule_intern_native_args(
        &mut self,
        cache_key: Option<*const Comp>,
        prim: gandr_core_term::prim::NativePrim,
        mut args: IntoIter<Rc<Value>>,
        ids: Vec<ValueNodeId>,
        run: &mut InternRun,
    )
    {
        if let Some(arg) = args.next() {
            self.schedule_intern_value_rc(
                arg,
                InternFrame::CompNativeAfterArg {
                    cache_key,
                    prim,
                    args,
                    ids,
                },
                run,
            );
        }
        else {
            run.result =
                Some(self.finish_intern_comp(CompNode::Native { prim, args: ids }, cache_key));
        }
    }

    /// Finishes value-node interning.
    fn finish_intern_value(
        &mut self,
        node: ValueNode,
        cache_key: Option<*const Value>,
    ) -> InternResult
    {
        let id = self.arena.values.alloc(node);
        if let (Some(key), Some(id)) = (cache_key, id) {
            self.value_ids.insert(key, id);
        }
        InternResult::Value(id)
    }

    /// Finishes computation-node interning.
    fn finish_intern_comp(
        &mut self,
        node: CompNode,
        cache_key: Option<*const Comp>,
    ) -> InternResult
    {
        let id = self.arena.comps.alloc(node);
        if let (Some(key), Some(id)) = (cache_key, id) {
            self.comp_ids.insert(key, id);
        }
        InternResult::Comp(id)
    }

    /// Finishes stack-node interning.
    fn finish_intern_stack(
        &mut self,
        node: StackNode,
        cache_key: Option<*const Stack>,
    ) -> InternResult
    {
        let id = self.arena.stacks.alloc(node);
        if let (Some(key), Some(id)) = (cache_key, id) {
            self.stack_ids.insert(key, id);
        }
        InternResult::Stack(id)
    }

    /// Resumes one interning continuation frame.
    fn resume_intern_frame(
        &mut self,
        frame: InternFrame,
        done: InternResult,
        run: &mut InternRun,
    )
    {
        match frame {
            | InternFrame::ValuePairAfterFst { cache_key, snd } => {
                match expect_intern_value(done) {
                    | Some(fst) => self.schedule_intern_value_rc(
                        snd,
                        InternFrame::ValuePairAfterSnd { cache_key, fst },
                        run,
                    ),
                    | None => run.result = Some(InternResult::Value(None)),
                }
            },
            | InternFrame::ValuePairAfterSnd { cache_key, fst } => {
                match expect_intern_value(done) {
                    | Some(snd) => {
                        run.result =
                            Some(self.finish_intern_value(ValueNode::Pair(fst, snd), cache_key));
                    },
                    | None => run.result = Some(InternResult::Value(None)),
                }
            },
            | InternFrame::ValueUnaryAfterChild { cache_key, kind } => {
                match expect_intern_value(done) {
                    | Some(child) => {
                        let node = match kind {
                            | ValueUnaryIntern::Inj(side) => ValueNode::Inj(side, child),
                            | ValueUnaryIntern::Here => ValueNode::Here(child),
                        };
                        run.result = Some(self.finish_intern_value(node, cache_key));
                    },
                    | None => run.result = Some(InternResult::Value(None)),
                }
            },
            | InternFrame::ValueListAfterElement {
                cache_key,
                elements,
                mut ids,
            } => match expect_intern_value(done) {
                | Some(id) => {
                    ids.push(id);
                    self.schedule_intern_value_list(cache_key, elements, ids, run);
                },
                | None => run.result = Some(InternResult::Value(None)),
            },
            | InternFrame::ValueRecordAfterField {
                cache_key,
                fields,
                mut ids,
                label,
            } => match expect_intern_value(done) {
                | Some(id) => {
                    ids.insert(label, id);
                    self.schedule_intern_value_record(cache_key, fields, ids, run);
                },
                | None => run.result = Some(InternResult::Value(None)),
            },
            | InternFrame::ValueThunkAfterBody { cache_key, grade } => {
                match expect_intern_comp(done) {
                    | Some(body) => {
                        run.result = Some(
                            self.finish_intern_value(ValueNode::Thunk(grade, body), cache_key),
                        );
                    },
                    | None => run.result = Some(InternResult::Value(None)),
                }
            },
            | InternFrame::ValueAnnotAfterInner { cache_key, ty } => {
                match expect_intern_value(done) {
                    | Some(inner) => match self.intern_value_type(&ty) {
                        | Some(ty) => {
                            run.result = Some(
                                self.finish_intern_value(ValueNode::Annot(inner, ty), cache_key),
                            );
                        },
                        | None => run.result = Some(InternResult::Value(None)),
                    },
                    | None => run.result = Some(InternResult::Value(None)),
                }
            },
            | InternFrame::ValueStkAfterStack { cache_key } => match expect_intern_stack(done) {
                | Some(stack) => {
                    run.result = Some(self.finish_intern_value(ValueNode::Stk(stack), cache_key));
                },
                | None => run.result = Some(InternResult::Value(None)),
            },
            | InternFrame::ValueCtorAfterPayload { cache_key, id, tag } => {
                match expect_intern_value(done) {
                    | Some(payload) => {
                        run.result =
                            Some(self.finish_intern_value(
                                ValueNode::Ctor { id, tag, payload },
                                cache_key,
                            ));
                    },
                    | None => run.result = Some(InternResult::Value(None)),
                }
            },
            | InternFrame::ValuePackAfterPayload {
                cache_key,
                witnesses,
            } => match expect_intern_value(done) {
                | Some(payload) => {
                    run.result = Some(
                        self.finish_intern_value(ValueNode::Pack { witnesses, payload }, cache_key),
                    );
                },
                | None => run.result = Some(InternResult::Value(None)),
            },
            | InternFrame::CompUnaryCompAfterChild { cache_key, kind } => {
                match expect_intern_comp(done) {
                    | Some(child) => {
                        let node = match kind {
                            | CompUnaryCompIntern::Abs(name, annot) => {
                                CompNode::Abs(name, annot, child)
                            },
                            | CompUnaryCompIntern::Prj(side) => CompNode::Prj(side, child),
                            | CompUnaryCompIntern::Reset => CompNode::Reset(child),
                            | CompUnaryCompIntern::Shift(k) => CompNode::Shift(k, child),
                        };
                        run.result = Some(self.finish_intern_comp(node, cache_key));
                    },
                    | None => run.result = Some(InternResult::Comp(None)),
                }
            },
            | InternFrame::CompUnaryValueAfterChild { cache_key, kind } => {
                match expect_intern_value(done) {
                    | Some(child) => {
                        let node = match kind {
                            | CompUnaryValueIntern::Ret => CompNode::Ret(child),
                            | CompUnaryValueIntern::Force => CompNode::Force(child),
                            | CompUnaryValueIntern::Dup => CompNode::Dup(child),
                            | CompUnaryValueIntern::Drop => CompNode::Drop(child),
                            | CompUnaryValueIntern::Perform(sig, op) => {
                                CompNode::Perform(Box::new(sig), op, child)
                            },
                            | CompUnaryValueIntern::RecordProj(label) => CompNode::RecordProj {
                                record: child,
                                label,
                            },
                        };
                        run.result = Some(self.finish_intern_comp(node, cache_key));
                    },
                    | None => run.result = Some(InternResult::Comp(None)),
                }
            },
            | InternFrame::CompAppAfterHead { cache_key, arg } => match expect_intern_comp(done) {
                | Some(head) => self.schedule_intern_value_rc(
                    arg,
                    InternFrame::CompAppAfterArg { cache_key, head },
                    run,
                ),
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompAppAfterArg { cache_key, head } => match expect_intern_value(done) {
                | Some(arg) => {
                    run.result = Some(self.finish_intern_comp(CompNode::App(head, arg), cache_key));
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompBindAfterBound {
                cache_key,
                name,
                cont,
            } => match expect_intern_comp(done) {
                | Some(bound) => self.schedule_intern_comp_rc(
                    cont,
                    InternFrame::CompBindAfterCont {
                        cache_key,
                        bound,
                        name,
                    },
                    run,
                ),
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompBindAfterCont {
                cache_key,
                bound,
                name,
            } => match expect_intern_comp(done) {
                | Some(cont) => {
                    run.result =
                        Some(self.finish_intern_comp(CompNode::Bind(bound, name, cont), cache_key));
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompCaseAfterScrut {
                cache_key,
                arm_fst,
                arm_snd,
            } => match expect_intern_value(done) {
                | Some(scrut) => {
                    let (fst_name, fst_body) = arm_fst;
                    self.schedule_intern_comp_rc(
                        fst_body,
                        InternFrame::CompCaseAfterFst {
                            cache_key,
                            scrut,
                            fst_name,
                            arm_snd,
                        },
                        run,
                    );
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompCaseAfterFst {
                cache_key,
                scrut,
                fst_name,
                arm_snd,
            } => match expect_intern_comp(done) {
                | Some(fst_body) => {
                    let (snd_name, snd_body) = arm_snd;
                    self.schedule_intern_comp_rc(
                        snd_body,
                        InternFrame::CompCaseAfterSnd {
                            cache_key,
                            scrut,
                            arm_fst: (fst_name, fst_body),
                            snd_name,
                        },
                        run,
                    );
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompCaseAfterSnd {
                cache_key,
                scrut,
                arm_fst,
                snd_name,
            } => match expect_intern_comp(done) {
                | Some(snd_body) => {
                    run.result = Some(self.finish_intern_comp(
                        CompNode::Case(scrut, arm_fst, (snd_name, snd_body)),
                        cache_key,
                    ));
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompNativeAfterArg {
                cache_key,
                prim,
                args,
                mut ids,
            } => match expect_intern_value(done) {
                | Some(id) => {
                    ids.push(id);
                    self.schedule_intern_native_args(cache_key, prim, args, ids, run);
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompListCaseAfterScrut {
                cache_key,
                nil,
                head,
                tail,
                cons,
            } => match expect_intern_value(done) {
                | Some(scrut) => self.schedule_intern_comp_rc(
                    nil,
                    InternFrame::CompListCaseAfterNil {
                        cache_key,
                        scrut,
                        head,
                        tail,
                        cons,
                    },
                    run,
                ),
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompListCaseAfterNil {
                cache_key,
                scrut,
                head,
                tail,
                cons,
                ..
            } => match expect_intern_comp(done) {
                | Some(nil) => self.schedule_intern_comp_rc(
                    cons,
                    InternFrame::CompListCaseAfterCons {
                        cache_key,
                        scrut,
                        nil,
                        head,
                        tail,
                    },
                    run,
                ),
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompListCaseAfterCons {
                cache_key,
                scrut,
                nil,
                head,
                tail,
            } => match expect_intern_comp(done) {
                | Some(cons) => {
                    run.result = Some(self.finish_intern_comp(
                        CompNode::ListCase {
                            scrut,
                            nil,
                            head,
                            tail,
                            cons,
                        },
                        cache_key,
                    ));
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompSplitAfterScrut {
                cache_key,
                fst_name,
                snd_name,
                motive,
                body,
            } => match expect_intern_value(done) {
                | Some(scrut) => {
                    let motive = match motive {
                        | Some(motive) => match self.arena.alloc_comp_type(&motive.body).ok() {
                            | Some(body_id) => Some(SplitMotiveNode {
                                binder: motive.binder,
                                body: body_id,
                            }),
                            | None => {
                                run.result = Some(InternResult::Comp(None));
                                return;
                            },
                        },
                        | None => None,
                    };
                    self.schedule_intern_comp_rc(
                        body,
                        InternFrame::CompSplitAfterBody {
                            cache_key,
                            scrut,
                            fst_name,
                            snd_name,
                            motive,
                        },
                        run,
                    );
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompSplitAfterBody {
                cache_key,
                scrut,
                fst_name,
                snd_name,
                motive,
            } => match expect_intern_comp(done) {
                | Some(body) => {
                    run.result = Some(self.finish_intern_comp(
                        CompNode::Split {
                            scrut,
                            fst_name,
                            snd_name,
                            motive,
                            body,
                        },
                        cache_key,
                    ));
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompDataCaseAfterScrut {
                cache_key,
                arms,
                arm_ids,
            } => match expect_intern_value(done) {
                | Some(scrut) => {
                    self.schedule_intern_data_arm(cache_key, scrut, arms, arm_ids, run);
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompDataCaseAfterArm {
                cache_key,
                scrut,
                arms,
                mut arm_ids,
                binder,
            } => match expect_intern_comp(done) {
                | Some(body) => {
                    arm_ids.push((binder, body));
                    self.schedule_intern_data_arm(cache_key, scrut, arms, arm_ids, run);
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompWithAfterFst { cache_key, snd } => match expect_intern_comp(done) {
                | Some(fst) => self.schedule_intern_comp_rc(
                    snd,
                    InternFrame::CompWithAfterSnd { cache_key, fst },
                    run,
                ),
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompWithAfterSnd { cache_key, fst } => match expect_intern_comp(done) {
                | Some(snd) => {
                    run.result = Some(self.finish_intern_comp(CompNode::With(fst, snd), cache_key));
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompHandleAfterScrutinee {
                cache_key,
                sig,
                ret,
                ops,
            } => match expect_intern_comp(done) {
                | Some(scrutinee) => {
                    let (ret_name, ret_body) = ret;
                    self.schedule_intern_comp_rc(
                        ret_body,
                        InternFrame::CompHandleAfterRet {
                            cache_key,
                            sig,
                            scrutinee,
                            ret_name,
                            ops,
                            flat_ops: Vec::new(),
                        },
                        run,
                    );
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompHandleAfterRet {
                cache_key,
                sig,
                scrutinee,
                ret_name,
                ops,
                flat_ops,
                ..
            } => match expect_intern_comp(done) {
                | Some(ret_body) => self.schedule_intern_handle_op(
                    cache_key,
                    InternHandle {
                        sig,
                        scrutinee,
                        ret: (ret_name, ret_body),
                        ops,
                        flat_ops,
                    },
                    run,
                ),
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompHandleAfterOp {
                cache_key,
                sig,
                scrutinee,
                ret,
                ops,
                mut flat_ops,
                clause,
            } => match expect_intern_comp(done) {
                | Some(body) => {
                    flat_ops.push(OpClauseNode {
                        op: clause.op,
                        payload: clause.payload,
                        resume: clause.resume,
                        body,
                    });
                    self.schedule_intern_handle_op(
                        cache_key,
                        InternHandle {
                            sig,
                            scrutinee,
                            ret,
                            ops,
                            flat_ops,
                        },
                        run,
                    );
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompResumeAfterStack { cache_key, comp } => {
                match expect_intern_value(done) {
                    | Some(stack) => self.schedule_intern_comp_rc(
                        comp,
                        InternFrame::CompResumeAfterComp { cache_key, stack },
                        run,
                    ),
                    | None => run.result = Some(InternResult::Comp(None)),
                }
            },
            | InternFrame::CompResumeAfterComp { cache_key, stack } => {
                match expect_intern_comp(done) {
                    | Some(comp) => {
                        run.result =
                            Some(self.finish_intern_comp(CompNode::Resume(stack, comp), cache_key));
                    },
                    | None => run.result = Some(InternResult::Comp(None)),
                }
            },
            | InternFrame::CompWalkAfterScrut {
                cache_key,
                motive,
                base,
            } => match expect_intern_value(done) {
                | Some(scrut) => match self.arena.alloc_comp_type(&motive.body).ok() {
                    | Some(body) => {
                        let motive = WalkMotiveNode {
                            x: motive.x,
                            y: motive.y,
                            q: motive.q,
                            body,
                        };
                        self.schedule_intern_comp_rc(
                            base.body,
                            InternFrame::CompWalkAfterBase {
                                cache_key,
                                scrut,
                                motive,
                                base_x: base.x,
                            },
                            run,
                        );
                    },
                    | None => run.result = Some(InternResult::Comp(None)),
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompWalkAfterBase {
                cache_key,
                scrut,
                motive,
                base_x,
            } => match expect_intern_comp(done) {
                | Some(body) => {
                    run.result = Some(self.finish_intern_comp(
                        CompNode::Walk {
                            scrut,
                            motive,
                            base: WalkBaseNode { x: base_x, body },
                        },
                        cache_key,
                    ));
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompUnpackAfterScrut {
                cache_key,
                signature,
                atoms,
                binder,
                body,
            } => match expect_intern_value(done) {
                | Some(scrut) => self.schedule_intern_comp_rc(
                    body,
                    InternFrame::CompUnpackAfterBody {
                        cache_key,
                        scrut,
                        signature,
                        atoms,
                        binder,
                    },
                    run,
                ),
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::CompUnpackAfterBody {
                cache_key,
                scrut,
                signature,
                atoms,
                binder,
            } => match expect_intern_comp(done) {
                | Some(body) => {
                    run.result = Some(self.finish_intern_comp(
                        CompNode::Unpack {
                            scrut,
                            signature,
                            atoms,
                            binder,
                            body,
                        },
                        cache_key,
                    ));
                },
                | None => run.result = Some(InternResult::Comp(None)),
            },
            | InternFrame::StackArgAfterValue { cache_key, rest } => {
                match expect_intern_value(done) {
                    | Some(value) => self.schedule_intern_stack_rc(
                        rest,
                        InternFrame::StackArgAfterRest { cache_key, value },
                        run,
                    ),
                    | None => run.result = Some(InternResult::Stack(None)),
                }
            },
            | InternFrame::StackArgAfterRest { cache_key, value } => {
                match expect_intern_stack(done) {
                    | Some(rest) => {
                        run.result =
                            Some(self.finish_intern_stack(StackNode::Arg(value, rest), cache_key));
                    },
                    | None => run.result = Some(InternResult::Stack(None)),
                }
            },
            | InternFrame::StackBindAfterCont {
                cache_key,
                name,
                rest,
            } => match expect_intern_comp(done) {
                | Some(cont) => self.schedule_intern_stack_rc(
                    rest,
                    InternFrame::StackBindAfterRest {
                        cache_key,
                        name,
                        cont,
                    },
                    run,
                ),
                | None => run.result = Some(InternResult::Stack(None)),
            },
            | InternFrame::StackBindAfterRest {
                cache_key,
                name,
                cont,
            } => match expect_intern_stack(done) {
                | Some(rest) => {
                    run.result = Some(
                        self.finish_intern_stack(StackNode::Bind(name, cont, rest), cache_key),
                    );
                },
                | None => run.result = Some(InternResult::Stack(None)),
            },
            | InternFrame::StackPrjAfterRest { cache_key, side } => match expect_intern_stack(done)
            {
                | Some(rest) => {
                    run.result =
                        Some(self.finish_intern_stack(StackNode::Prj(side, rest), cache_key));
                },
                | None => run.result = Some(InternResult::Stack(None)),
            },
        }
    }

    /// Schedules the next data-case arm for interning.
    fn schedule_intern_data_arm(
        &mut self,
        cache_key: Option<*const Comp>,
        scrut: ValueNodeId,
        mut arms: IntoIter<(String, Rc<Comp>)>,
        arm_ids: Vec<(String, CompNodeId)>,
        run: &mut InternRun,
    )
    {
        if let Some((binder, body)) = arms.next() {
            self.schedule_intern_comp_rc(
                body,
                InternFrame::CompDataCaseAfterArm {
                    cache_key,
                    scrut,
                    arms,
                    arm_ids,
                    binder,
                },
                run,
            );
        }
        else {
            run.result = Some(self.finish_intern_comp(
                CompNode::DataCase {
                    scrut,
                    arms: arm_ids,
                },
                cache_key,
            ));
        }
    }

    /// Schedules the next handler operation clause for interning.
    fn schedule_intern_handle_op(
        &mut self,
        cache_key: Option<*const Comp>,
        handle: InternHandle,
        run: &mut InternRun,
    )
    {
        let InternHandle {
            sig,
            scrutinee,
            ret,
            mut ops,
            flat_ops,
        } = handle;
        if let Some(clause) = ops.next() {
            self.schedule_intern_comp_rc(
                Rc::clone(&clause.body),
                InternFrame::CompHandleAfterOp {
                    cache_key,
                    sig,
                    scrutinee,
                    ret,
                    ops,
                    flat_ops,
                    clause,
                },
                run,
            );
        }
        else {
            run.result = Some(self.finish_intern_comp(
                CompNode::Handle {
                    sig: Box::new(sig),
                    scrutinee,
                    ret,
                    ops: flat_ops,
                },
                cache_key,
            ));
        }
    }

    /// Rule Var: look the hypothesis up; a free variable marks and recovers
    /// with `Unknown`.
    fn rule_var(
        &self,
        name: String,
        dir: Dir<ValueType>,
        marks: &mut Vec<Mark>,
    ) -> ValueType
    {
        match self
            .ctx
            .lookup(gandr_core_term::boundary::NameRef::from(name.as_str()))
        {
            | Some(found) => finish_value(found.clone(), dir, marks),
            | None => {
                marks.push(Mark::FreeVariable { name });
                finish_value(ValueType::Unknown, dir, marks)
            },
        }
    }
}

/// Extracts a value marking result.
///
/// # Contract
/// - requires: `result` comes from a value-rooted marking run.
/// - ensures: returns the synthesized value type.
/// - panics: none in release builds. The machine's sort discipline keeps a
///   value-rooted run on the [`MarkResult::Value`] sort; a `debug_assert!`
///   guards that invariant in test / debug builds, and a mismatch falls back to
///   the module's absorbing [`ValueType::Unknown`] recovery type (never reached
///   — the marking oracle proptests would fail first).
fn expect_value(result: MarkResult) -> ValueType
{
    debug_assert!(
        matches!(result, MarkResult::Value(_)),
        "value marking returned a computation"
    );
    match result {
        | MarkResult::Value(ty) => ty,
        | MarkResult::Comp(_) => ValueType::Unknown,
    }
}

/// Extracts a computation marking result.
///
/// # Contract
/// - requires: `result` comes from a computation-rooted marking run.
/// - ensures: returns the synthesized computation type.
/// - panics: none in release builds (a `debug_assert!` guards the machine's
///   sort discipline in test / debug builds; a mismatch falls back to
///   [`CompType::Unknown`], the absorbing recovery type).
fn expect_comp(result: MarkResult) -> CompType
{
    debug_assert!(
        matches!(result, MarkResult::Comp(_)),
        "computation marking returned a value"
    );
    match result {
        | MarkResult::Comp(ty) => ty,
        | MarkResult::Value(_) => CompType::Unknown,
    }
}

/// Extracts a value interning result.
///
/// # Contract
/// - requires: `result` comes from a value-rooted interning run.
/// - ensures: returns the interned value id, or `None` when arena allocation
///   failed.
/// - panics: none in release builds (a `debug_assert!` guards the interner's
///   sort discipline in test / debug builds; a mismatch falls back to the
///   `None` allocation-failure shape).
fn expect_intern_value(result: InternResult) -> Option<ValueNodeId>
{
    debug_assert!(
        matches!(result, InternResult::Value(_)),
        "value interning returned a non-value"
    );
    match result {
        | InternResult::Value(id) => id,
        | InternResult::Comp(_) | InternResult::Stack(_) => None,
    }
}

/// Extracts a computation interning result.
///
/// # Contract
/// - requires: `result` comes from a computation-rooted interning run.
/// - ensures: returns the interned computation id, or `None` when arena
///   allocation failed.
/// - panics: none in release builds (a `debug_assert!` guards the interner's
///   sort discipline in test / debug builds; a mismatch falls back to the
///   `None` allocation-failure shape).
fn expect_intern_comp(result: InternResult) -> Option<CompNodeId>
{
    debug_assert!(
        matches!(result, InternResult::Comp(_)),
        "computation interning returned a non-computation"
    );
    match result {
        | InternResult::Comp(id) => id,
        | InternResult::Value(_) | InternResult::Stack(_) => None,
    }
}

/// Extracts a stack interning result.
///
/// # Contract
/// - requires: `result` comes from a stack-rooted interning run.
/// - ensures: returns the interned stack id, or `None` when arena allocation
///   failed.
/// - panics: none in release builds (a `debug_assert!` guards the interner's
///   sort discipline in test / debug builds; a mismatch falls back to the
///   `None` allocation-failure shape).
fn expect_intern_stack(result: InternResult) -> Option<StackNodeId>
{
    debug_assert!(
        matches!(result, InternResult::Stack(_)),
        "stack interning returned a non-stack"
    );
    match result {
        | InternResult::Stack(id) => id,
        | InternResult::Value(_) | InternResult::Comp(_) => None,
    }
}

/// The analyzed type of a value direction (the `Check` payload, `None` in
/// inference mode).
#[inline]
fn analyzed_value(dir: &Dir<ValueType>) -> Option<Ty>
{
    match *dir {
        | Dir::Check(ref ty) => Some(Ty::Value(ty.clone())),
        | Dir::Infer => None,
    }
}

/// The analyzed type of a computation direction.
#[inline]
fn analyzed_comp(dir: &Dir<CompType>) -> Option<Ty>
{
    match *dir {
        | Dir::Check(ref ty) => Some(Ty::Comp(ty.clone())),
        | Dir::Infer => None,
    }
}

/// Completes the integer-literal rule under a direction in the marking pass
/// (ADR-39 D4) — the marking twin of
/// [`crate::discipline::subtype::finish_int_literal`].
///
/// Inference yields `Integer`; checking accepts any integer numeric atom the
/// literal is representable in ([`int_literal_fits`], the Rust `{integer}`
/// rule) with no mark, and otherwise falls back to the marking [`finish_value`]
/// for `Integer` (which records a mismatch mark unless `Integer` subsumes the
/// expected type). The accept/reject decision matches the checker and machine.
#[inline]
fn finish_int_literal_marked<L>(
    n: L,
    dir: Dir<ValueType>,
    marks: &mut Vec<Mark>,
) -> ValueType
where
    L: Into<I64Literal>,
{
    let n = i64::from(n.into());
    match dir {
        | Dir::Infer => ValueType::integer(),
        | Dir::Check(expected) => {
            if bool::from(int_literal_fits(
                gandr_core_term::boundary::IntegerLiteral::from(n),
                &expected,
            )) {
                expected
            }
            else {
                finish_value(ValueType::integer(), Dir::Check(expected), marks)
            }
        },
    }
}

/// Completes a value rule under a direction (the marking Sub rule): infer
/// returns the constructed type; check recovers with the *expected* type,
/// recording a classified mismatch mark when consistent subtyping fails.
#[inline]
fn finish_value(
    constructed: ValueType,
    dir: Dir<ValueType>,
    marks: &mut Vec<Mark>,
) -> ValueType
{
    match dir {
        | Dir::Infer => constructed,
        | Dir::Check(expected) => {
            if !bool::from(value_subtype(&constructed, &expected)) {
                marks.push(classify_value_mismatch(constructed, expected.clone()));
            }
            expected
        },
    }
}

/// Completes a computation rule under a direction (the marking Sub rule).
#[inline]
fn finish_comp(
    constructed: CompType,
    dir: Dir<CompType>,
    marks: &mut Vec<Mark>,
) -> CompType
{
    match dir {
        | Dir::Infer => constructed,
        | Dir::Check(expected) => {
            if !bool::from(comp_subtype(&constructed, &expected)) {
                marks.push(classify_comp_mismatch(constructed, expected.clone()));
            }
            expected
        },
    }
}

/// Classifies a value subsumption failure into a mark: a graded-thunk leg whose
/// only failing component is the grade becomes a [`Mark::GradeBudget`];
/// anything else is a generic [`Mark::TypeMismatch`] boundary.
#[inline]
fn classify_value_mismatch(
    actual: ValueType,
    expected: ValueType,
) -> Mark
{
    // Detect a grade-only failure (the bodies are consistent, only the budget
    // fails) without consuming the types — they are still needed for the
    // fallback boundary. Grades are `Copy`, so the verdict carries them out and
    // the borrow ends before the move below.
    let budget = match (&actual, &expected) {
        | (
            &ValueType::Thunk(actual_grade, ref actual_body),
            &ValueType::Thunk(expected_grade, ref expected_body),
        ) if bool::from(comp_subtype(actual_body, expected_body))
            && !bool::from(expected_grade.leq(actual_grade)) =>
        {
            Some((expected_grade, actual_grade))
        },
        | _ => None,
    };
    match budget {
        | Some((required, available)) => Mark::GradeBudget {
            required,
            available,
        },
        | None => Mark::TypeMismatch(Boundary::new(Ty::Value(expected), Ty::Value(actual))),
    }
}

/// Classifies a computation subsumption failure: two returners whose only
/// failing leg is the effect row become a [`Mark::EffectRowMismatch`]; anything
/// else is a generic [`Mark::TypeMismatch`] boundary.
#[inline]
fn classify_comp_mismatch(
    actual: CompType,
    expected: CompType,
) -> Mark
{
    // A returner-vs-returner failure whose only failing leg is the effect row
    // (consistent payloads) is the specialized effect-row boundary. The borrow
    // ends with the verdict, before the types move into the mark.
    let row_only = match (&actual, &expected) {
        | (
            &CompType::F(ref actual_of, ref actual_row),
            &CompType::F(ref expected_of, ref expected_row),
        ) if bool::from(value_subtype(actual_of, expected_of))
            && !bool::from(actual_row.is_subset(expected_row)) =>
        {
            true
        },
        | _ => false,
    };
    if row_only {
        Mark::EffectRowMismatch(Boundary::new(Ty::Comp(expected), Ty::Comp(actual)))
    }
    else {
        Mark::TypeMismatch(Boundary::new(Ty::Comp(expected), Ty::Comp(actual)))
    }
}

/// Recovers from a fallible typing helper: on `Ok` returns the value, on `Err`
/// pushes the converted mark and returns the fallback.
#[inline]
fn recover<T>(
    result: Result<T, TypeError>,
    fallback: T,
    marks: &mut Vec<Mark>,
) -> T
{
    match result {
        | Ok(value) => value,
        | Err(error) => {
            marks.push(mark_of_error(error));
            fallback
        },
    }
}

/// Recovers from a fallible destructure helper that returns a pair, recovering
/// with `(Unknown, Unknown)` on failure.
#[inline]
fn recover_pair<L, R>(
    result: Result<(L, R), TypeError>,
    marks: &mut Vec<Mark>,
) -> (L, R)
where
    L: Recoverable,
    R: Recoverable,
{
    match result {
        | Ok(pair) => pair,
        | Err(error) => {
            marks.push(mark_of_error(error));
            (L::unknown(), R::unknown())
        },
    }
}

/// A type with an `Unknown` recovery inhabitant, for [`recover_pair`].
trait Recoverable
{
    /// The `Unknown` recovery value of this type.
    fn unknown() -> Self;
}

impl Recoverable for ValueType
{
    #[inline]
    fn unknown() -> Self
    {
        Self::Unknown
    }
}

impl Recoverable for CompType
{
    #[inline]
    fn unknown() -> Self
    {
        Self::Unknown
    }
}

impl Recoverable for EffectRow
{
    #[inline]
    fn unknown() -> Self
    {
        Self::EMPTY
    }
}

/// Converts a checker [`TypeError`] into the corresponding [`Mark`].
///
/// Used for the fallible destructure / row helpers reused from the checker; the
/// subsumption and grade abort sites are handled inline (with the
/// grade-budget / effect-row / thunkability refinements) and do not route here.
#[inline]
fn mark_of_error(error: TypeError) -> Mark
{
    match error {
        | TypeError::TypeMismatch { expected, actual } => {
            Mark::TypeMismatch(Boundary::new(expected, actual))
        },
        | TypeError::ShapeMismatch { expected, actual } => Mark::ShapeMismatch { expected, actual },
        | TypeError::StuckExpr { hint, .. } => Mark::Stuck { hint },
        | TypeError::UnboundVariable { name } => Mark::FreeVariable { name },
        | TypeError::GradeError { lower, upper } => Mark::GradeBudget {
            required: lower,
            available: upper,
        },
    }
}
