//! The **support of one node check**: everything the checker's answer for a
//! single node depends on, and nothing else.
//!
//! # Why this is the whole soundness argument
//!
//! A memo is sound exactly when equal supports force equal answers. So the
//! support has to be the *complete* input to one goal expansion, and each
//! component below is here because dropping it would let two different
//! questions collide:
//!
//! * **the node identity** — an arena id. The same id in the same arena names
//!   the same node, which is the argument the kernel's own id-equality
//!   conversion fast path already rests on. Two structurally equal nodes at
//!   different ids simply miss, which costs reuse and never correctness.
//! * **the direction, with the expected type** — checking a value against `A`
//!   and against `B` are different questions, and synthesis is a third.
//! * **the binder slice** — variables are de Bruijn indices resolved against
//!   the machine's context, so the same node under two different contexts means
//!   two different things. Only the slice the node can actually reach is
//!   included: the last [`LooseDepth`] entries, which is the precision that
//!   lets one shared subterm collapse across binder positions that agree where
//!   it looks.
//!
//! Level parameters and the admitted-declaration log are *not* in the key
//! because they are fixed for the whole of one declaration's check, and a memo
//! never outlives that — see the lifetime note on
//! [`check_declaration`](crate::check::check_declaration).
//!
//! # The binder slice is computed, not guessed
//!
//! [`LooseDepths`] computes one more than each node's largest free de Bruijn
//! index, bottom-up over the graph, iteratively, caching per node — so the
//! computation is itself sharing-aware and costs one pass per *distinct* node
//! rather than per occurrence. Where it cannot answer (a dangling id, an
//! index at the representable ceiling) it returns the maximum, which widens
//! the slice to the whole context: **the failure direction is always more
//! context in the key, never less**, so a defect here costs collapse and
//! cannot manufacture a hit.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::arena::CompTypeId;
use crate::arena::ComputationId;
use crate::arena::TermArena;
use crate::arena::ValueId;
use crate::arena::ValueTypeId;
use crate::term::Computation;
use crate::term::Value;

/// One more than the largest free de Bruijn index a node mentions; zero for a
/// closed node.
///
/// This is the number of enclosing binders the node can reach, so it is
/// exactly the length of the context slice its meaning depends on.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LooseDepth(u32);

impl From<u32> for LooseDepth
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl LooseDepth
{
    /// The saturated maximum — the conservative answer, widening the key's
    /// slice to the whole context.
    const WIDEST: Self = Self(u32::MAX);

    /// The larger of two reaches.
    #[inline]
    const fn join(
        self,
        other: Self,
    ) -> Self
    {
        if self.0 >= other.0 { self } else { other }
    }

    /// The reach seen from outside one binder: a body reaching `n` binders
    /// reaches `n - 1` of its parent's, and a closed body stays closed.
    #[inline]
    const fn under_binder(self) -> Self
    {
        Self(self.0.saturating_sub(1))
    }

    /// Where the reached slice of `context` starts — the offset of the first
    /// slot this reach can see, saturating to the whole context when the reach
    /// is at or past its length.
    #[inline]
    fn start_in(
        self,
        context: &[ValueTypeId],
    ) -> ContextOffset
    {
        let wanted = usize::try_from(self.0).unwrap_or(usize::MAX);
        ContextOffset(context.len().saturating_sub(wanted))
    }
}

/// An offset into the machine's typing context.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ContextOffset(usize);

/// Which question one goal expansion asks, apart from the binder context.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SupportGoal
{
    /// Synthesize a value's type.
    SynthValue(
        /// The value node.
        ValueId,
    ),
    /// Check a value against an expected value type.
    CheckValue(
        /// The value node.
        ValueId,
        /// The expected type.
        ValueTypeId,
    ),
    /// Synthesize a computation's type.
    SynthComp(
        /// The computation node.
        ComputationId,
    ),
    /// Check a computation against an expected computation type.
    CheckComp(
        /// The computation node.
        ComputationId,
        /// The expected type.
        CompTypeId,
    ),
    /// Form a value type and read off its universe level.
    ValueTypeLevel(
        /// The value-type node.
        ValueTypeId,
    ),
    /// Form a computation type and read off its universe level.
    CompTypeLevel(
        /// The computation-type node.
        CompTypeId,
    ),
}

/// What one goal expansion produced, as the memo stores it.
///
/// The four variants are the two machines' answers: the checker's produced
/// register, and the type-formation walk's level. They share one memo because
/// they share one lifetime — a declaration's check — and separating them would
/// buy nothing but a second type parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeOutcome
{
    /// A check succeeded; no type is carried.
    Checked,
    /// A value type was synthesized.
    ValueType(
        /// The synthesized type.
        ValueTypeId,
    ),
    /// A computation type was synthesized.
    CompType(
        /// The synthesized type.
        CompTypeId,
    ),
    /// A type formed at this level.
    Formed(
        /// The universe level.
        gandr_kernel_strata::Level,
    ),
}

/// The complete support of one checker goal expansion.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeSupport
{
    /// The question, with its direction and expected type.
    goal: SupportGoal,
    /// The innermost context slots the node can reach, outermost first — the
    /// binder slice, not the whole context.
    binders: Vec<ValueTypeId>,
}

impl NodeSupport
{
    /// Build the support of `goal` under `context`, taking only the binder
    /// slice the node reaches.
    ///
    /// # Contract
    /// - requires: `context` is the machine's typing context at the expansion;
    ///   `reach` is the node's own [`LooseDepth`].
    /// - ensures: two supports are equal exactly when the goals agree and the
    ///   reached binder slices agree. A wider `reach` yields a longer slice, so
    ///   an over-estimate can only split a support and never merge two.
    /// - provides: the memo key.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    pub fn new(
        goal: SupportGoal,
        context: &[ValueTypeId],
        reach: LooseDepth,
    ) -> Self
    {
        let ContextOffset(start) = reach.start_in(context);
        let binders = match context.get(start ..) {
            | Some(slice) => slice.to_vec(),
            | None => context.to_vec(),
        };
        Self { goal, binders }
    }

    /// Build the support of a goal that reads no binder at all.
    ///
    /// # Contract
    /// - requires: `goal` names a computation whose answer is independent of
    ///   the machine's context. At S1 that is exactly type formation: no type
    ///   former embeds a value term, so a type in the context is closed and its
    ///   level is a function of the type node alone.
    /// - ensures: a support with an empty binder slice.
    /// - provides: the type-formation walk's memo key.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    pub fn closed(goal: SupportGoal) -> Self
    {
        Self {
            goal,
            binders: Vec::new(),
        }
    }

    /// Which of the two machines this support belongs to.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: [`SupportPlane::Type`] for the two type-formation goals and
    ///   [`SupportPlane::Term`] for the four checker goals.
    /// - provides: the per-plane split a measurement needs. One memo serves
    ///   both machines, and an aggregate entry count cannot distinguish a live
    ///   memo from one whose type half silently never fires.
    /// - fails: never.
    /// - panics: none.
    #[cfg(test)]
    #[inline]
    pub const fn plane(&self) -> SupportPlane
    {
        match self.goal {
            | SupportGoal::ValueTypeLevel(_) | SupportGoal::CompTypeLevel(_) => SupportPlane::Type,
            | SupportGoal::SynthValue(_)
            | SupportGoal::CheckValue(..)
            | SupportGoal::SynthComp(_)
            | SupportGoal::CheckComp(..) => SupportPlane::Term,
        }
    }
}

/// Which machine a support belongs to.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SupportPlane
{
    /// The checker's goal loop, over terms.
    Term,
    /// The type-formation walk, over types.
    Type,
}

/// The per-node binder reach, computed once per distinct node and cached.
#[derive(Clone, Debug, Default)]
pub struct LooseDepths
{
    /// Reaches already computed for value nodes.
    values: BTreeMap<ValueId, LooseDepth>,
    /// Reaches already computed for computation nodes.
    computations: BTreeMap<ComputationId, LooseDepth>,
}

/// One step of the iterative bottom-up reach walk.
#[derive(Clone, Copy)]
enum ReachTask
{
    /// Ensure this value node's children are computed, then finish it.
    OpenValue(ValueId),
    /// Combine this value node's children's reaches into its own.
    CloseValue(ValueId),
    /// Ensure this computation node's children are computed, then finish it.
    OpenComp(ComputationId),
    /// Combine this computation node's children's reaches into its own.
    CloseComp(ComputationId),
}

impl LooseDepths
{
    /// An empty cache.
    #[inline]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// The binder reach of a value node.
    ///
    /// # Contract
    /// - requires: nothing — a dangling id is answered conservatively rather
    ///   than refused, since this feeds a key and not a verdict.
    /// - ensures: one more than the node's largest free de Bruijn index, or
    ///   [`LooseDepth::WIDEST`] where the graph could not be read. The walk is
    ///   iterative over an explicit task stack, so it is total on any depth,
    ///   and each distinct node is combined once however many times it occurs.
    /// - provides: the binder-slice length of a value goal's support.
    /// - fails: never.
    /// - panics: none.
    pub fn value_reach(
        &mut self,
        arena: &TermArena,
        root: ValueId,
    ) -> LooseDepth
    {
        self.run(arena, ReachTask::OpenValue(root));
        self.values
            .get(&root)
            .copied()
            .unwrap_or(LooseDepth::WIDEST)
    }

    /// The binder reach of a computation node; see [`Self::value_reach`].
    pub fn comp_reach(
        &mut self,
        arena: &TermArena,
        root: ComputationId,
    ) -> LooseDepth
    {
        self.run(arena, ReachTask::OpenComp(root));
        self.computations
            .get(&root)
            .copied()
            .unwrap_or(LooseDepth::WIDEST)
    }

    /// Drive the reach walk to completion from one root task.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: every node reachable from the root has a cached reach.
    /// - provides: the shared engine of the two reach faces.
    /// - fails: never — an unreadable node caches [`LooseDepth::WIDEST`].
    /// - panics: none.
    ///
    /// # Termination
    /// - reason: an explicit task stack, not recursion.
    /// - measure: the number of nodes without a cached reach, which strictly
    ///   falls at every `Close` step and never rises, since an `Open` on a
    ///   cached node pushes nothing.
    /// - boundedness: the arena is finite and children have strictly smaller
    ///   ids, so the reachable set is finite and acyclic.
    /// - input recursion: none.
    fn run(
        &mut self,
        arena: &TermArena,
        root: ReachTask,
    )
    {
        let mut tasks: Vec<ReachTask> = Vec::new();
        tasks.push(root);
        while let Some(task) = tasks.pop() {
            match task {
                | ReachTask::OpenValue(id) => {
                    if self.values.contains_key(&id) {
                        continue;
                    }
                    let Some(node) = arena.value(id)
                    else {
                        let _prior = self.values.insert(id, LooseDepth::WIDEST);
                        continue;
                    };
                    tasks.push(ReachTask::CloseValue(id));
                    match *node {
                        | Value::Variable(_)
                        | Value::Constant(_)
                        | Value::Unit
                        | Value::Literal(_) => {},
                        | Value::Pair(first, second) => {
                            tasks.push(ReachTask::OpenValue(first));
                            tasks.push(ReachTask::OpenValue(second));
                        },
                        | Value::Injection(_, body) | Value::Lift { body, .. } => {
                            tasks.push(ReachTask::OpenValue(body));
                        },
                        | Value::Thunk(body) => tasks.push(ReachTask::OpenComp(body)),
                    }
                },
                | ReachTask::CloseValue(id) => {
                    let reach = self.combine_value(arena, id);
                    let _prior = self.values.insert(id, reach);
                },
                | ReachTask::OpenComp(id) => {
                    if self.computations.contains_key(&id) {
                        continue;
                    }
                    let Some(node) = arena.computation(id)
                    else {
                        let _prior = self.computations.insert(id, LooseDepth::WIDEST);
                        continue;
                    };
                    tasks.push(ReachTask::CloseComp(id));
                    match *node {
                        | Computation::Lambda(body) => tasks.push(ReachTask::OpenComp(body)),
                        | Computation::Application(head, argument) => {
                            tasks.push(ReachTask::OpenComp(head));
                            tasks.push(ReachTask::OpenValue(argument));
                        },
                        | Computation::Return(value) | Computation::Force(value) => {
                            tasks.push(ReachTask::OpenValue(value));
                        },
                        | Computation::Bind(bound, body) => {
                            tasks.push(ReachTask::OpenComp(bound));
                            tasks.push(ReachTask::OpenComp(body));
                        },
                        | Computation::Case {
                            scrutinee,
                            on_left,
                            on_right,
                        } => {
                            tasks.push(ReachTask::OpenValue(scrutinee));
                            tasks.push(ReachTask::OpenComp(on_left));
                            tasks.push(ReachTask::OpenComp(on_right));
                        },
                    }
                },
                | ReachTask::CloseComp(id) => {
                    let reach = self.combine_comp(arena, id);
                    let _prior = self.computations.insert(id, reach);
                },
            }
        }
    }

    /// A cached value reach, conservatively widest when absent.
    #[inline]
    fn cached_value(
        &self,
        id: ValueId,
    ) -> LooseDepth
    {
        self.values.get(&id).copied().unwrap_or(LooseDepth::WIDEST)
    }

    /// A cached computation reach, conservatively widest when absent.
    #[inline]
    fn cached_comp(
        &self,
        id: ComputationId,
    ) -> LooseDepth
    {
        self.computations
            .get(&id)
            .copied()
            .unwrap_or(LooseDepth::WIDEST)
    }

    /// Combine one value node's children's reaches into its own.
    ///
    /// A variable at index `i` reaches `i + 1` binders — index zero names the
    /// innermost slot. No value former binds, so every other case is the join
    /// of its children.
    fn combine_value(
        &self,
        arena: &TermArena,
        id: ValueId,
    ) -> LooseDepth
    {
        let Some(node) = arena.value(id)
        else {
            return LooseDepth::WIDEST;
        };
        match *node {
            | Value::Variable(index) => LooseDepth::from(u32::from(index).saturating_add(1)),
            | Value::Constant(_) | Value::Unit | Value::Literal(_) => LooseDepth::default(),
            | Value::Pair(first, second) => {
                self.cached_value(first).join(self.cached_value(second))
            },
            | Value::Injection(_, body) | Value::Lift { body, .. } => self.cached_value(body),
            | Value::Thunk(body) => self.cached_comp(body),
        }
    }

    /// Combine one computation node's children's reaches into its own.
    ///
    /// The three binding formers are the whole content: a lambda binds its
    /// argument for the body, a bind binds its payload for the body only (not
    /// for the bound computation), and a case binds each summand for its own
    /// branch only (not for the scrutinee). Each bound position is therefore
    /// read one binder further out than its child computed.
    fn combine_comp(
        &self,
        arena: &TermArena,
        id: ComputationId,
    ) -> LooseDepth
    {
        let Some(node) = arena.computation(id)
        else {
            return LooseDepth::WIDEST;
        };
        match *node {
            | Computation::Lambda(body) => self.cached_comp(body).under_binder(),
            | Computation::Application(head, argument) => {
                self.cached_comp(head).join(self.cached_value(argument))
            },
            | Computation::Return(value) | Computation::Force(value) => self.cached_value(value),
            | Computation::Bind(bound, body) => self
                .cached_comp(bound)
                .join(self.cached_comp(body).under_binder()),
            | Computation::Case {
                scrutinee,
                on_left,
                on_right,
            } => self
                .cached_value(scrutinee)
                .join(self.cached_comp(on_left).under_binder())
                .join(self.cached_comp(on_right).under_binder()),
        }
    }
}
