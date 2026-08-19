//! Definitional equality: the six-step pipeline, decided over the glued
//! semantic domain.
//!
//! # The pipeline, and where each step lives
//!
//! | step | what it does                                     | where                           |
//! | ---- | ------------------------------------------------ | ------------------------------- |
//! | 1    | identity equality                                | the first line of every goal    |
//! | 2    | cached-word guards                               | [`Guard::settles_distinct`]     |
//! | 3    | iterative structural comparison                  | the goal worklist               |
//! | 4    | lazy unfolding with heights                      | [`unfold_value_side`]                 |
//! | 5    | smart unfolding gated on case progress           | [`made_progress`]               |
//! | 6    | three-state speculation                          | the choice-point frames         |
//!
//! Each step falls through to the next only on a non-answer.
//!
//! # Speculation is a choice point, not a nested call
//!
//! Two same-head glued neutrals are compared **spines first with no
//! commitments**: a choice point is pushed, the rigid comparison runs above it,
//! and a failure unwinds to the choice point and retries against the unfolded
//! faces. The rigid pass spends no unfolding at all, so the retry is the first
//! forcing rather than a second one — and it memoizes each forced face onto the
//! neutral that owns it, so a later comparison reaching the same neutral reads
//! that face instead of re-forcing it. **Nothing is evaluated twice.**
//!
//! The machine is one loop over one stack, so backtracking costs no host stack
//! and the depth of speculation is bounded by the goals themselves.
//!
//! # What the relation is
//!
//! It is symmetric, reflexive, and — exactly as the structural equality it
//! replaces — **not transitive once a hole participates**. A hole is consistent
//! with every value in both directions, which is the gradual discipline this
//! crate has always run and which subsumption depends on. An ascription is
//! transparent, because evaluation erases one. A motive on an eliminator is
//! likewise transparent: it names a type and contributes nothing to the value
//! computed.
//!
//! # Records, and the signature prohibition
//!
//! A record value is compared field by field over a canonically ordered map,
//! and a record **type** — which is what a module signature is — the same way.
//! That is structural equality on a canonical representation. It is **not** a
//! width rule and **not** a permutation rule, and neither exists anywhere in
//! this module: a conversion engine that grew signature comparison would
//! foreclose the telescope future from the inside, so the absence is a
//! commitment rather than an omission.
//!
//! [`Guard::settles_distinct`]: crate::sem::Guard::settles_distinct

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::boundary::BacktrackStatus;
use gandr_core_term::boundary::ClosureArity;
use gandr_core_term::boundary::NameRef;
use gandr_core_term::boundary::ProgressStatus;
use gandr_core_term::boundary::UnfoldPermission;
use gandr_core_term::boundary::ValueEquality;
use gandr_core_term::identity::occurs_free_comptype;
use gandr_core_term::identity::subst_comptype;
use gandr_core_term::identity::subst_valuetype;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::ValueType;

use crate::Normalizer;
use crate::eval::ForceMode;
use crate::eval::apply;
use crate::eval::enter_nullary;
use crate::eval::eval_value;
use crate::eval::force_head;
use crate::eval::force_value;
use crate::eval::project;
use crate::eval::rerun_spine;
use crate::eval::syntax_comp;
use crate::eval::value;
use crate::intern::canonical_stack_key;
use crate::intern::canonically_equal_value_types;
use crate::sem::ClosureId;
use crate::sem::CompUnfold;
use crate::sem::Elim;
use crate::sem::NeutralHead;
use crate::sem::Rigid;
use crate::sem::SemArena;
use crate::sem::SemCompId;
use crate::sem::SemCompNode;
use crate::sem::SemError;
use crate::sem::SemValueId;
use crate::sem::SemValueNode;
use crate::sem::ValueUnfold;

/// The speculation state a goal is compared under.
///
/// This is the design's rigid/flex/full triple. `Rigid` is the no-commitments
/// pass a choice point protects; `Flex` is the ordinary entry state, where the
/// height rule decides which side unfolds; `Full` is the retry after a
/// speculation failed, where unfolding is spent freely.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum ConvState
{
    /// No unfolding at all: a mismatch fails and lets a choice point retry.
    Rigid,
    /// The ordinary state: unfold as the height rule directs, and speculate
    /// before spending an unfolding on a same-head pair.
    #[default]
    Flex,
    /// The retry state: unfold whatever can unfold, progress gate included.
    Full,
}

impl ConvState
{
    /// Whether this state may spend an unfolding.
    #[inline]
    #[must_use]
    fn unfolds(self) -> UnfoldPermission
    {
        UnfoldPermission::from(!matches!(self, Self::Rigid))
    }

    /// The force mode this state drives a head with.
    #[inline]
    #[must_use]
    fn force(self) -> ForceMode
    {
        match self {
            | Self::Rigid => ForceMode::WeakHead,
            | Self::Flex => ForceMode::Speculative,
            | Self::Full => ForceMode::Unfold,
        }
    }
}

/// One frame on the conversion machine's stack.
enum Frame
{
    /// Two values still to compare.
    Value(SemValueId, SemValueId, ConvState),
    /// Two computations still to compare.
    Comp(SemCompId, SemCompId, ConvState),
    /// A speculation choice point: if everything above it fails, retry the
    /// recorded pair against their unfolded faces.
    Choice(SemCompId, SemCompId),
}

/// Decides definitional equality of two **source values**.
///
/// The values are evaluated in the empty environment, compared in the semantic
/// domain, and the run's semantic nodes are then truncated away — so nothing
/// the comparison built outlives the verdict.
///
/// # Contract
/// - ensures: returns whether `lhs` and `rhs` are definitionally equal under
///   this normalizer's definitional environment; the relation is reflexive and
///   symmetric, treats a hole as consistent with every value in both
///   directions, and is therefore not transitive once a hole participates.
/// - provides: the checker's conversion decision — the one relation every
///   caller of the retired structural equality now asks.
/// - fails: never; an arena error is absorbed into a **distinct** verdict,
///   which is the fail-closed direction and matches how the kernel's own
///   conversion answers a dangling id.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 for the relation as a whole, against readback as the
///   external oracle — two values convert exactly when their canonical
///   readbacks agree — plus L3 for the three properties the relation promises,
///   separated pointwise by a reflexive pair, a symmetric pair, and the
///   hole-consistency triple that witnesses non-transitivity.
/// - witness: `crate::tests::conversion_agrees_with_canonical_readback`
/// - witness: `crate::tests::conversion_is_reflexive_and_symmetric`
/// - witness: `crate::tests::a_hole_is_consistent_with_every_value`
#[must_use]
#[inline]
pub fn converts(
    nbe: &mut Normalizer,
    lhs: &Rc<Value>,
    rhs: &Rc<Value>,
) -> ValueEquality
{
    let opened = nbe.begin_run();
    let decision =
        converts_checked(nbe, lhs, rhs).unwrap_or_else(|_error| ValueEquality::from(false));
    nbe.finish_run(opened);
    decision
}

/// The fallible core of [`converts`].
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn converts_checked(
    nbe: &mut Normalizer,
    lhs: &Rc<Value>,
    rhs: &Rc<Value>,
) -> Result<ValueEquality, SemError>
{
    let lhs = nbe.lower_input(lhs)?;
    let rhs = nbe.lower_input(rhs)?;
    let lhs = eval_value(nbe, SemArena::EMPTY_ENV, lhs)?;
    let rhs = eval_value(nbe, SemArena::EMPTY_ENV, rhs)?;
    converts_values(nbe, lhs, rhs)
}

/// Decides definitional equality of two semantic values.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Termination
/// - reason: the machine is a loop over an explicit goal stack, not recursion.
/// - measure: pending goals, with unfoldings bounded by the normalizer's fuel.
/// - boundedness: every goal decomposes into strictly smaller goals or resolves
///   without pushing, and each choice point fires at most once.
/// - input recursion: none.
#[inline]
pub fn converts_values(
    nbe: &mut Normalizer,
    lhs: SemValueId,
    rhs: SemValueId,
) -> Result<ValueEquality, SemError>
{
    run(nbe, alloc::vec![Frame::Value(lhs, rhs, ConvState::Flex)])
}

/// Decides definitional equality of two semantic computations.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
#[inline]
pub fn converts_comps(
    nbe: &mut Normalizer,
    lhs: SemCompId,
    rhs: SemCompId,
) -> Result<ValueEquality, SemError>
{
    run(nbe, alloc::vec![Frame::Comp(lhs, rhs, ConvState::Flex)])
}

/// Drains the goal stack, backtracking to a choice point on failure.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Termination
/// - reason: the driver is a loop over an explicit goal stack.
/// - measure: pending goals plus unspent choice points.
/// - boundedness: unfolding is bounded by the normalizer's fuel, and a choice
///   point is discarded once fired.
/// - input recursion: none.
fn run(
    nbe: &mut Normalizer,
    mut goals: Vec<Frame>,
) -> Result<ValueEquality, SemError>
{
    while let Some(frame) = goals.pop() {
        let failed = match frame {
            | Frame::Choice(..) => false,
            | Frame::Value(lhs, rhs, state) => {
                !bool::from(value_goal(nbe, lhs, rhs, state, &mut goals)?)
            },
            | Frame::Comp(lhs, rhs, state) => {
                !bool::from(comp_goal(nbe, lhs, rhs, state, &mut goals)?)
            },
        };
        if failed && !bool::from(backtrack(nbe, &mut goals)?) {
            return Ok(ValueEquality::from(false));
        }
    }
    Ok(ValueEquality::from(true))
}

/// Unwinds to the nearest choice point and re-poses its pair against the
/// unfolded faces.
///
/// Returns whether a choice point was found and armed.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn backtrack(
    nbe: &mut Normalizer,
    goals: &mut Vec<Frame>,
) -> Result<BacktrackStatus, SemError>
{
    while let Some(frame) = goals.pop() {
        if let Frame::Choice(lhs, rhs) = frame {
            let unfolded_lhs = unfold_comp(nbe, lhs, ConvState::Full)?.unwrap_or(lhs);
            let unfolded_rhs = unfold_comp(nbe, rhs, ConvState::Full)?.unwrap_or(rhs);
            if unfolded_lhs == lhs && unfolded_rhs == rhs {
                continue;
            }
            goals.push(Frame::Comp(unfolded_lhs, unfolded_rhs, ConvState::Full));
            return Ok(BacktrackStatus::from(true));
        }
    }
    Ok(BacktrackStatus::from(false))
}

/// Resolves a suspended pure-computation embedding to the value it returns.
///
/// Returns `id` unchanged when the value is not an embedding, or when running
/// its computation does not reach a returner — stuck on a variable, or out of
/// budget. Budget exhaustion here is a **refusal carrying its evidence** rather
/// than an unsound acceptance: an unresolved embedding compares by congruence,
/// which can only report unequal what a longer run might have equated.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn resolve_embedding(
    nbe: &mut Normalizer,
    id: SemValueId,
    node: &SemValueNode,
) -> Result<SemValueId, SemError>
{
    let SemValueNode::Run(cell) = *node
    else {
        return Ok(id);
    };
    let whnf = enter_nullary(nbe, cell, ForceMode::Unfold)?;
    match *nbe.arena().comp(whnf)?.node() {
        | SemCompNode::Return(produced) => Ok(produced),
        | _ => Ok(id),
    }
}

/// Compares one value pair, pushing the sub-goals it decomposes into.
///
/// Returns whether the pair is still viable; `false` fails the goal.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn value_goal(
    nbe: &mut Normalizer,
    lhs: SemValueId,
    rhs: SemValueId,
    state: ConvState,
    goals: &mut Vec<Frame>,
) -> Result<ValueEquality, SemError>
{
    // Step 1: one id in one arena is one value.
    if lhs == rhs {
        return Ok(ValueEquality::from(true));
    }
    // Step 2: the cached guard word, read only in the distinct direction.
    let lhs_guard = nbe.arena().value(lhs)?.guard();
    let rhs_guard = nbe.arena().value(rhs)?.guard();
    if bool::from(lhs_guard.settles_distinct(rhs_guard)) {
        return Ok(ValueEquality::from(false));
    }
    let lhs_node = nbe.arena().value(lhs)?.node().clone();
    let rhs_node = nbe.arena().value(rhs)?.node().clone();
    // The gradual wildcard, in both directions, exactly as the structural
    // equality this replaces treated it.
    if matches!(lhs_node, SemValueNode::Rigid(Rigid::Hole(_), _))
        || matches!(rhs_node, SemValueNode::Rigid(Rigid::Hole(_), _))
    {
        return Ok(ValueEquality::from(true));
    }
    // **Resolve a suspended pure-computation embedding before comparing.**
    //
    // The embedding is suspended at construction rather than evaluated, so this
    // is where it computes — and this is the only consumer that needs it to,
    // because deciding that an endpoint written as an application equals the
    // endpoint written as its result is the whole reason the former exists.
    //
    // Resolution runs the computation and, when it reaches a returner, replaces
    // the embedding with the value it produced; a computation stuck on a
    // variable leaves the embedding alone and the pair falls through to the
    // congruence arm below. Either side may resolve, so the rule fires on both.
    let resolved_lhs = resolve_embedding(nbe, lhs, &lhs_node)?;
    let resolved_rhs = resolve_embedding(nbe, rhs, &rhs_node)?;
    if resolved_lhs != lhs || resolved_rhs != rhs {
        goals.push(Frame::Value(resolved_lhs, resolved_rhs, state));
        return Ok(ValueEquality::from(true));
    }
    // Step 3: structural comparison, head mismatch first.
    match (&lhs_node, &rhs_node) {
        | (&SemValueNode::Unit, &SemValueNode::Unit) => return Ok(ValueEquality::from(true)),
        | (&SemValueNode::Int(left), &SemValueNode::Int(right)) => {
            return Ok(ValueEquality::from(left == right));
        },
        | (&SemValueNode::Str(ref left), &SemValueNode::Str(ref right)) => {
            return Ok(ValueEquality::from(left == right));
        },
        | (&SemValueNode::Num(left), &SemValueNode::Num(right)) => {
            return Ok(ValueEquality::from(left == right));
        },
        | (&SemValueNode::Reified(left), &SemValueNode::Reified(right)) => {
            // A reified stack is opaque to conversion by construction, so the
            // only equality on offer is alpha-identity of its syntax, taken
            // through the canonical key rather than through its node id: two
            // equal stacks lowered twice are two ids and one key.
            let store = nbe.syntax();
            let decided = left == right
                || canonical_stack_key(store, left) == canonical_stack_key(store, right);
            return Ok(ValueEquality::from(decided));
        },
        | (&SemValueNode::Run(left), &SemValueNode::Run(right)) => {
            // Both sides resolved to something other than a returner — stuck on
            // a variable, or out of budget — so the only equality on offer is
            // congruence under the embedding, which is the ordinary
            // computation-conversion path one level down.
            closures_goal(nbe, left, right, ClosureArity::from(0_usize), state, goals)?;
            return Ok(ValueEquality::from(true));
        },
        | (&SemValueNode::Pair(left_fst, left_snd), &SemValueNode::Pair(right_fst, right_snd)) => {
            goals.push(Frame::Value(left_snd, right_snd, state));
            goals.push(Frame::Value(left_fst, right_fst, state));
            return Ok(ValueEquality::from(true));
        },
        | (
            &SemValueNode::Inj(left_side, left_payload),
            &SemValueNode::Inj(right_side, right_payload),
        ) => {
            if left_side != right_side {
                return Ok(ValueEquality::from(false));
            }
            goals.push(Frame::Value(left_payload, right_payload, state));
            return Ok(ValueEquality::from(true));
        },
        | (&SemValueNode::Here(left), &SemValueNode::Here(right)) => {
            goals.push(Frame::Value(left, right, state));
            return Ok(ValueEquality::from(true));
        },
        | (
            &SemValueNode::Ctor {
                id: ref left_id,
                tag: left_tag,
                payload: left_payload,
            },
            &SemValueNode::Ctor {
                id: ref right_id,
                tag: right_tag,
                payload: right_payload,
            },
        ) => {
            if left_id != right_id || left_tag != right_tag {
                return Ok(ValueEquality::from(false));
            }
            goals.push(Frame::Value(left_payload, right_payload, state));
            return Ok(ValueEquality::from(true));
        },
        | (
            &SemValueNode::Pack {
                witnesses: ref left_witnesses,
                payload: left_payload,
            },
            &SemValueNode::Pack {
                witnesses: ref right_witnesses,
                payload: right_payload,
            },
        ) => {
            // **The witnesses are compared, not erased.** They are types, and a
            // type inside a value is compared the way a reified stack is —
            // alpha-identity of its syntax through the canonical key — because
            // that is the equality this engine already offers on syntax it does
            // not evaluate. Erasing them would equate two packs at different
            // representations, which decides a parametricity fact judgmentally.
            if left_witnesses.len() != right_witnesses.len() {
                return Ok(ValueEquality::from(false));
            }
            let store = nbe.syntax();
            let aligned = left_witnesses
                .iter()
                .zip(right_witnesses.iter())
                .all(|(left, right)| {
                    bool::from(canonically_equal_value_types(store, *left, *right))
                });
            if !aligned {
                return Ok(ValueEquality::from(false));
            }
            goals.push(Frame::Value(left_payload, right_payload, state));
            return Ok(ValueEquality::from(true));
        },
        | (&SemValueNode::List(ref left), &SemValueNode::List(ref right)) => {
            if left.len() != right.len() {
                return Ok(ValueEquality::from(false));
            }
            for (left, right) in left.iter().zip(right.iter()).rev() {
                goals.push(Frame::Value(*left, *right, state));
            }
            return Ok(ValueEquality::from(true));
        },
        | (&SemValueNode::Record(ref left), &SemValueNode::Record(ref right)) => {
            // Field by field over a canonically ordered map. This is the whole
            // of record and signature conversion: no width rule, no permutation
            // rule, and no comparison of one signature against another.
            if left.len() != right.len() {
                return Ok(ValueEquality::from(false));
            }
            for ((left_label, left_field), (right_label, right_field)) in
                left.iter().zip(right.iter()).rev()
            {
                if left_label != right_label {
                    return Ok(ValueEquality::from(false));
                }
                goals.push(Frame::Value(*left_field, *right_field, state));
            }
            return Ok(ValueEquality::from(true));
        },
        | (
            &SemValueNode::Thunk(left_grade, left_cell),
            &SemValueNode::Thunk(right_grade, right_cell),
        ) => {
            if left_grade != right_grade {
                return Ok(ValueEquality::from(false));
            }
            let left = crate::eval::enter_nullary(nbe, left_cell, state.force())?;
            let right = crate::eval::enter_nullary(nbe, right_cell, state.force())?;
            goals.push(Frame::Comp(left, right, state));
            return Ok(ValueEquality::from(true));
        },
        | (
            &SemValueNode::Rigid(Rigid::Level(left), _),
            &SemValueNode::Rigid(Rigid::Level(right), _),
        ) => {
            return Ok(ValueEquality::from(left == right));
        },
        | (
            &SemValueNode::Rigid(Rigid::Free(ref left), _),
            &SemValueNode::Rigid(Rigid::Free(ref right), _),
        ) if left == right => {
            return Ok(ValueEquality::from(true));
        },
        | _ => {},
    }
    // Steps 4 and 5: the heads disagree, so unfold — the taller side first, and
    // the only side that can unfold when just one has a rule.
    if !bool::from(state.unfolds()) {
        return Ok(ValueEquality::from(false));
    }
    let unfolded = unfold_value_side(nbe, lhs, rhs, state)?;
    match unfolded {
        | Some((lhs, rhs)) => {
            goals.push(Frame::Value(lhs, rhs, state));
            Ok(ValueEquality::from(true))
        },
        | None => Ok(ValueEquality::from(false)),
    }
}

/// Unfolds whichever side of a value pair the height rule selects.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn unfold_value_side(
    nbe: &mut Normalizer,
    lhs: SemValueId,
    rhs: SemValueId,
    state: ConvState,
) -> Result<Option<(SemValueId, SemValueId)>, SemError>
{
    let lhs_height = value_height(nbe, lhs)?;
    let rhs_height = value_height(nbe, rhs)?;
    let mode = state.force();
    match (lhs_height, rhs_height) {
        | (None, None) => Ok(None),
        | (Some(_), None) => {
            let unfolded = force_value(nbe, lhs, mode)?;
            Ok((unfolded != lhs).then_some((unfolded, rhs)))
        },
        | (None, Some(_)) => {
            let unfolded = force_value(nbe, rhs, mode)?;
            Ok((unfolded != rhs).then_some((lhs, unfolded)))
        },
        | (Some(left), Some(right)) => {
            if u32::from(left) >= u32::from(right) {
                let unfolded = force_value(nbe, lhs, mode)?;
                Ok((unfolded != lhs).then_some((unfolded, rhs)))
            }
            else {
                let unfolded = force_value(nbe, rhs, mode)?;
                Ok((unfolded != rhs).then_some((lhs, unfolded)))
            }
        },
    }
}

/// The definitional height of a value's head, when it has an unfolding rule.
///
/// # Errors
///
/// Returns [`SemError`] when the id does not resolve.
fn value_height(
    nbe: &Normalizer,
    id: SemValueId,
) -> Result<Option<gandr_core_term::boundary::DefinitionHeightLevel>, SemError>
{
    let node = nbe.arena().value(id)?;
    Ok(match *node.node() {
        | SemValueNode::Rigid(_, face) => face.height(),
        | _ => None,
    })
}

/// Compares one computation pair, pushing the sub-goals it decomposes into.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn comp_goal(
    nbe: &mut Normalizer,
    lhs: SemCompId,
    rhs: SemCompId,
    state: ConvState,
    goals: &mut Vec<Frame>,
) -> Result<ValueEquality, SemError>
{
    if lhs == rhs {
        return Ok(ValueEquality::from(true));
    }
    let lhs_node = nbe.arena().comp(lhs)?.node().clone();
    let rhs_node = nbe.arena().comp(rhs)?.node().clone();
    match (&lhs_node, &rhs_node) {
        | (&SemCompNode::Return(left), &SemCompNode::Return(right)) => {
            goals.push(Frame::Value(left, right, state));
            Ok(ValueEquality::from(true))
        },
        // Eta for the negative formers: a function is compared with anything by
        // applying both to one fresh variable, and a lazy pair by projecting
        // both. This is the territory that has no differential to check against
        // and needs its own law coverage, which is exactly why it is written as
        // one rule rather than as a special case of the structural walk.
        | (&SemCompNode::Lambda(_), _) | (_, &SemCompNode::Lambda(_)) => {
            let level = nbe.fresh_level();
            let node = SemValueNode::Rigid(Rigid::Level(level), ValueUnfold::Rigid);
            let fresh = value(nbe, node)?;
            let left = apply(nbe, lhs, fresh, state.force())?;
            let right = apply(nbe, rhs, fresh, state.force())?;
            goals.push(Frame::Comp(left, right, state));
            Ok(ValueEquality::from(true))
        },
        | (&SemCompNode::LazyPair(..), _) | (_, &SemCompNode::LazyPair(..)) => {
            let left_fst = project(nbe, lhs, gandr_core_term::syntax::Side::Fst, state.force())?;
            let right_fst = project(nbe, rhs, gandr_core_term::syntax::Side::Fst, state.force())?;
            let left_snd = project(nbe, lhs, gandr_core_term::syntax::Side::Snd, state.force())?;
            let right_snd = project(nbe, rhs, gandr_core_term::syntax::Side::Snd, state.force())?;
            goals.push(Frame::Comp(left_snd, right_snd, state));
            goals.push(Frame::Comp(left_fst, right_fst, state));
            Ok(ValueEquality::from(true))
        },
        | (&SemCompNode::Neutral(left), &SemCompNode::Neutral(right)) => {
            neutral_goal(nbe, lhs, rhs, left, right, state, goals)
        },
        | _ => {
            // A returner against a neutral, or two canonical forms of different
            // polarity: only an unfolding can still reconcile them.
            if !bool::from(state.unfolds()) {
                return Ok(ValueEquality::from(false));
            }
            let left = unfold_comp(nbe, lhs, state)?;
            let right = unfold_comp(nbe, rhs, state)?;
            match (left, right) {
                | (None, None) => Ok(ValueEquality::from(false)),
                | (left, right) => {
                    goals.push(Frame::Comp(
                        left.unwrap_or(lhs),
                        right.unwrap_or(rhs),
                        state,
                    ));
                    Ok(ValueEquality::from(true))
                },
            }
        },
    }
}

/// Compares two neutral computations, speculating before spending an unfolding.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn neutral_goal(
    nbe: &mut Normalizer,
    lhs_comp: SemCompId,
    rhs_comp: SemCompId,
    lhs: crate::sem::NeutralId,
    rhs: crate::sem::NeutralId,
    state: ConvState,
    goals: &mut Vec<Frame>,
) -> Result<ValueEquality, SemError>
{
    let (lhs_head, lhs_spine, lhs_face) = {
        let neutral = nbe.arena().neutral(lhs)?;
        (
            neutral.head().clone(),
            neutral.spine().to_vec(),
            neutral.unfold(),
        )
    };
    let (rhs_head, rhs_spine, rhs_face) = {
        let neutral = nbe.arena().neutral(rhs)?;
        (
            neutral.head().clone(),
            neutral.spine().to_vec(),
            neutral.unfold(),
        )
    };
    // Step 6: with an unfolding available on either side, protect the rigid
    // comparison with a choice point so a failure retries against the forced
    // faces instead of giving up.
    let speculating = bool::from(state.unfolds())
        && (bool::from(lhs_face.unfoldable()) || bool::from(rhs_face.unfoldable()));
    if speculating {
        goals.push(Frame::Choice(lhs_comp, rhs_comp));
    }
    let inner = if speculating { ConvState::Rigid } else { state };
    if lhs_spine.len() != rhs_spine.len() {
        return Ok(ValueEquality::from(false));
    }
    for (left, right) in lhs_spine.iter().zip(rhs_spine.iter()).rev() {
        if !bool::from(spine_goal(nbe, *left, *right, inner, goals)?) {
            return Ok(ValueEquality::from(false));
        }
    }
    head_goal(nbe, &lhs_head, &rhs_head, inner, goals)
}

/// Compares one spine entry pair.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn spine_goal(
    nbe: &mut Normalizer,
    lhs: Elim,
    rhs: Elim,
    state: ConvState,
    goals: &mut Vec<Frame>,
) -> Result<ValueEquality, SemError>
{
    match (lhs, rhs) {
        | (Elim::Apply(left), Elim::Apply(right)) => {
            goals.push(Frame::Value(left, right, state));
            Ok(ValueEquality::from(true))
        },
        | (Elim::Project(left), Elim::Project(right)) => Ok(ValueEquality::from(left == right)),
        | (Elim::Sequence(left), Elim::Sequence(right)) => {
            closures_goal(nbe, left, right, ClosureArity::from(1_usize), state, goals)?;
            Ok(ValueEquality::from(true))
        },
        | _ => Ok(ValueEquality::from(false)),
    }
}

/// Opens two closures under one shared run of fresh levels and queues their
/// bodies for comparison.
///
/// Sharing the levels is what makes the comparison alpha-insensitive: both
/// bodies see the *same* rigid variables, so their binder names never enter the
/// answer.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn closures_goal(
    nbe: &mut Normalizer,
    lhs: ClosureId,
    rhs: ClosureId,
    arity: ClosureArity,
    state: ConvState,
    goals: &mut Vec<Frame>,
) -> Result<(), SemError>
{
    let arity = usize::from(arity);
    let mut fresh = Vec::with_capacity(arity);
    for _ in 0 .. arity {
        let level = nbe.fresh_level();
        let node = SemValueNode::Rigid(Rigid::Level(level), ValueUnfold::Rigid);
        fresh.push(value(nbe, node)?);
    }
    let left = crate::eval::enter_with(nbe, lhs, &fresh, state.force())?;
    let right = crate::eval::enter_with(nbe, rhs, &fresh, state.force())?;
    goals.push(Frame::Comp(left, right, state));
    Ok(())
}

/// Compares two neutral heads.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn head_goal(
    nbe: &mut Normalizer,
    lhs: &NeutralHead,
    rhs: &NeutralHead,
    state: ConvState,
    goals: &mut Vec<Frame>,
) -> Result<ValueEquality, SemError>
{
    match (lhs, rhs) {
        | (
            &NeutralHead::Case {
                scrutinee: left_scrutinee,
                on_left: left_on_left,
                on_right: left_on_right,
            },
            &NeutralHead::Case {
                scrutinee: right_scrutinee,
                on_left: right_on_left,
                on_right: right_on_right,
            },
        ) => {
            closures_goal(
                nbe,
                left_on_right,
                right_on_right,
                ClosureArity::from(1_usize),
                state,
                goals,
            )?;
            closures_goal(
                nbe,
                left_on_left,
                right_on_left,
                ClosureArity::from(1_usize),
                state,
                goals,
            )?;
            goals.push(Frame::Value(left_scrutinee, right_scrutinee, state));
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::DataCase {
                scrutinee: left_scrutinee,
                arms: ref left_arms,
            },
            &NeutralHead::DataCase {
                scrutinee: right_scrutinee,
                arms: ref right_arms,
            },
        ) => {
            if left_arms.len() != right_arms.len() {
                return Ok(ValueEquality::from(false));
            }
            for (left, right) in left_arms.iter().zip(right_arms.iter()).rev() {
                if left.0 != right.0 {
                    return Ok(ValueEquality::from(false));
                }
                closures_goal(
                    nbe,
                    left.1,
                    right.1,
                    ClosureArity::from(1_usize),
                    state,
                    goals,
                )?;
            }
            goals.push(Frame::Value(left_scrutinee, right_scrutinee, state));
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::ListCase {
                scrutinee: left_scrutinee,
                nil: left_nil,
                cons: left_cons,
            },
            &NeutralHead::ListCase {
                scrutinee: right_scrutinee,
                nil: right_nil,
                cons: right_cons,
            },
        ) => {
            closures_goal(
                nbe,
                left_cons,
                right_cons,
                ClosureArity::from(2_usize),
                state,
                goals,
            )?;
            closures_goal(
                nbe,
                left_nil,
                right_nil,
                ClosureArity::from(0_usize),
                state,
                goals,
            )?;
            goals.push(Frame::Value(left_scrutinee, right_scrutinee, state));
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::Split {
                scrutinee: left_scrutinee,
                body: left_body,
            },
            &NeutralHead::Split {
                scrutinee: right_scrutinee,
                body: right_body,
            },
        ) => {
            closures_goal(
                nbe,
                left_body,
                right_body,
                ClosureArity::from(2_usize),
                state,
                goals,
            )?;
            goals.push(Frame::Value(left_scrutinee, right_scrutinee, state));
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::Unpack {
                source: left_source,
                scrutinee: left_scrutinee,
                body: left_body,
            },
            &NeutralHead::Unpack {
                source: right_source,
                scrutinee: right_scrutinee,
                body: right_body,
            },
        ) => {
            // The ascribed signature and the minted atoms live on the source
            // nodes, so congruence reads them back rather than carrying copies
            // — the `Perform` and `Handle` discipline.
            if !bool::from(same_package_head(nbe, left_source, right_source)?) {
                return Ok(ValueEquality::from(false));
            }
            closures_goal(
                nbe,
                left_body,
                right_body,
                ClosureArity::from(1_usize),
                state,
                goals,
            )?;
            goals.push(Frame::Value(left_scrutinee, right_scrutinee, state));
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::Project {
                record: left_record,
                label: ref left_label,
            },
            &NeutralHead::Project {
                record: right_record,
                label: ref right_label,
            },
        ) => {
            if left_label != right_label {
                return Ok(ValueEquality::from(false));
            }
            goals.push(Frame::Value(left_record, right_record, state));
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::Walk {
                scrutinee: left_scrutinee,
                base: left_base,
            },
            &NeutralHead::Walk {
                scrutinee: right_scrutinee,
                base: right_base,
            },
        ) => {
            closures_goal(
                nbe,
                left_base,
                right_base,
                ClosureArity::from(1_usize),
                state,
                goals,
            )?;
            goals.push(Frame::Value(left_scrutinee, right_scrutinee, state));
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::Native {
                prim: left_prim,
                args: ref left_args,
            },
            &NeutralHead::Native {
                prim: right_prim,
                args: ref right_args,
            },
        ) => {
            if left_prim != right_prim || left_args.len() != right_args.len() {
                return Ok(ValueEquality::from(false));
            }
            for (left, right) in left_args.iter().zip(right_args.iter()).rev() {
                goals.push(Frame::Value(*left, *right, state));
            }
            Ok(ValueEquality::from(true))
        },
        // Three heads carry one value and nothing else, so one arm answers
        // for all three: forcing a neutral, and the two grade operations the
        // quarantine keeps stuck.
        | (&NeutralHead::Force(left), &NeutralHead::Force(right))
        | (&NeutralHead::Dup(left), &NeutralHead::Dup(right))
        | (&NeutralHead::Drop(left), &NeutralHead::Drop(right)) => {
            goals.push(Frame::Value(left, right, state));
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::Perform {
                source: left_source,
                payload: left_payload,
            },
            &NeutralHead::Perform {
                source: right_source,
                payload: right_payload,
            },
        ) => {
            // The signature and the operation name live on the source nodes, so
            // congruence reads them back rather than carrying copies.
            if !bool::from(same_effect_head(nbe, left_source, right_source)?) {
                return Ok(ValueEquality::from(false));
            }
            goals.push(Frame::Value(left_payload, right_payload, state));
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::Handle {
                source: left_source,
                scrutinee: left_scrutinee,
                ret: left_ret,
                ops: ref left_ops,
            },
            &NeutralHead::Handle {
                source: right_source,
                scrutinee: right_scrutinee,
                ret: right_ret,
                ops: ref right_ops,
            },
        ) => {
            // The signature and the clause labels live on the source nodes, so
            // congruence reads them back rather than carrying copies.
            if left_ops.len() != right_ops.len()
                || !bool::from(same_effect_head(nbe, left_source, right_source)?)
            {
                return Ok(ValueEquality::from(false));
            }
            for (left, right) in left_ops.iter().zip(right_ops.iter()).rev() {
                closures_goal(
                    nbe,
                    *left,
                    *right,
                    ClosureArity::from(2_usize),
                    state,
                    goals,
                )?;
            }
            closures_goal(
                nbe,
                left_ret,
                right_ret,
                ClosureArity::from(1_usize),
                state,
                goals,
            )?;
            closures_goal(
                nbe,
                left_scrutinee,
                right_scrutinee,
                ClosureArity::from(0_usize),
                state,
                goals,
            )?;
            Ok(ValueEquality::from(true))
        },
        | (
            &NeutralHead::Resume {
                value: left_value,
                body: left_body,
            },
            &NeutralHead::Resume {
                value: right_value,
                body: right_body,
            },
        ) => {
            closures_goal(
                nbe,
                left_body,
                right_body,
                ClosureArity::from(0_usize),
                state,
                goals,
            )?;
            goals.push(Frame::Value(left_value, right_value, state));
            Ok(ValueEquality::from(true))
        },
        | (&NeutralHead::Reset(left), &NeutralHead::Reset(right)) => {
            closures_goal(nbe, left, right, ClosureArity::from(0_usize), state, goals)?;
            Ok(ValueEquality::from(true))
        },
        | (&NeutralHead::Shift(left), &NeutralHead::Shift(right)) => {
            closures_goal(nbe, left, right, ClosureArity::from(1_usize), state, goals)?;
            Ok(ValueEquality::from(true))
        },
        | (&NeutralHead::Fix(left), &NeutralHead::Fix(right)) => {
            // Congruence under the self-reference binder, and nothing more:
            // the fixpoint is never unfolded here.
            closures_goal(nbe, left, right, ClosureArity::from(1_usize), state, goals)?;
            Ok(ValueEquality::from(true))
        },
        | (&NeutralHead::Hole(_), _) | (_, &NeutralHead::Hole(_)) => Ok(ValueEquality::from(true)),
        | (&NeutralHead::Mismatch(left), &NeutralHead::Mismatch(right)) => {
            goals.push(Frame::Comp(left, right, state));
            Ok(ValueEquality::from(true))
        },
        | _ => Ok(ValueEquality::from(false)),
    }
}

/// Whether two quarantined effect heads agree on their signature and labels.
///
/// Both are read off their source nodes: the semantic record names the node
/// rather than copying the signature out of it, so this is where the two are
/// compared.
///
/// # Errors
///
/// Returns [`SemError`] when a source node does not resolve.
fn same_effect_head(
    nbe: &Normalizer,
    lhs: gandr_core_term::syntax::CompNodeId,
    rhs: gandr_core_term::syntax::CompNodeId,
) -> Result<ValueEquality, SemError>
{
    if lhs == rhs {
        return Ok(ValueEquality::from(true));
    }
    let decided = match (syntax_comp(nbe, lhs)?, syntax_comp(nbe, rhs)?) {
        | (
            gandr_core_term::syntax::CompNode::Perform(left_sig, left_op, _),
            gandr_core_term::syntax::CompNode::Perform(right_sig, right_op, _),
        ) => left_sig == right_sig && left_op == right_op,
        | (
            gandr_core_term::syntax::CompNode::Handle {
                sig: left_sig,
                ops: left_ops,
                ..
            },
            gandr_core_term::syntax::CompNode::Handle {
                sig: right_sig,
                ops: right_ops,
                ..
            },
        ) => {
            left_sig == right_sig
                && left_ops.len() == right_ops.len()
                && left_ops
                    .iter()
                    .zip(right_ops.iter())
                    .all(|(left, right)| left.op == right.op)
        },
        | _ => false,
    };
    Ok(ValueEquality::from(decided))
}

/// Whether two frustrated package eliminations agree on their annotation half:
/// the atoms they minted and the signature they ascribe.
///
/// Both are read off their source nodes, exactly as [`same_effect_head`] reads
/// a signature and its labels. The atoms are compared by **identity**, which is
/// the whole point of minting them: a sealed atom is nominal, so two
/// eliminations that minted different atoms opened different abstractions and
/// congruence must not merge them.
///
/// # Errors
///
/// Returns [`SemError`] when a source node does not resolve.
fn same_package_head(
    nbe: &Normalizer,
    lhs: gandr_core_term::syntax::CompNodeId,
    rhs: gandr_core_term::syntax::CompNodeId,
) -> Result<ValueEquality, SemError>
{
    if lhs == rhs {
        return Ok(ValueEquality::from(true));
    }
    let decided = match (syntax_comp(nbe, lhs)?, syntax_comp(nbe, rhs)?) {
        | (
            gandr_core_term::syntax::CompNode::Unpack {
                signature: left_signature,
                atoms: left_atoms,
                ..
            },
            gandr_core_term::syntax::CompNode::Unpack {
                signature: right_signature,
                atoms: right_atoms,
                ..
            },
        ) => {
            let store = nbe.syntax();
            left_atoms == right_atoms
                && bool::from(canonically_equal_value_types(
                    store,
                    left_signature,
                    right_signature,
                ))
        },
        | _ => false,
    };
    Ok(ValueEquality::from(decided))
}

/// Unfolds a neutral computation's head and re-runs its spine, memoizing the
/// result on the neutral that owns the face.
///
/// Step five lives here: in the speculative state the unfolding is **declined**
/// when it makes no case-tree progress, and the neutral records that verdict so
/// the decision is not re-taken. In the full state the gate is not consulted,
/// so declining costs completeness nowhere — the retry always spends the
/// unfolding.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn unfold_comp(
    nbe: &mut Normalizer,
    id: SemCompId,
    state: ConvState,
) -> Result<Option<SemCompId>, SemError>
{
    let SemCompNode::Neutral(stuck) = *nbe.arena().comp(id)?.node()
    else {
        return Ok(None);
    };
    let (head, spine, face) = {
        let neutral = nbe.arena().neutral(stuck)?;
        (
            neutral.head().clone(),
            neutral.spine().to_vec(),
            neutral.unfold(),
        )
    };
    match face {
        | CompUnfold::Forced(forced) => return Ok(Some(forced)),
        | CompUnfold::Rigid => return Ok(None),
        | CompUnfold::Blocked(_) if matches!(state, ConvState::Flex) => return Ok(None),
        | CompUnfold::Blocked(_) | CompUnfold::Pending(_) => {},
    }
    let NeutralHead::Force(carried) = head
    else {
        return Ok(None);
    };
    let unfolded = force_value(nbe, carried, ForceMode::Unfold)?;
    if unfolded == carried {
        return Ok(None);
    }
    let base = force_head(nbe, unfolded, ForceMode::Unfold)?;
    let result = rerun_spine(nbe, base, &spine, ForceMode::Unfold)?;
    if matches!(state, ConvState::Flex) && !bool::from(made_progress(nbe, result)?) {
        if let Some(height) = face.height() {
            nbe.arena_mut()
                .set_unfold_face(stuck, CompUnfold::Blocked(height))?;
        }
        return Ok(None);
    }
    nbe.arena_mut()
        .set_unfold_face(stuck, CompUnfold::Forced(result))?;
    Ok(Some(result))
}

/// Whether unfolding produced case-tree progress.
///
/// Case is first-class here, so progress is decidable directly on the result:
/// an unfolding that lands on a frustrated elimination reduced nothing a
/// comparison can use, which is exactly the stuck-recursor gas the smart
/// unfolding rule exists to avoid spending.
///
/// # Errors
///
/// Returns [`SemError`] when an id does not resolve.
fn made_progress(
    nbe: &Normalizer,
    id: SemCompId,
) -> Result<ProgressStatus, SemError>
{
    let SemCompNode::Neutral(stuck) = *nbe.arena().comp(id)?.node()
    else {
        return Ok(ProgressStatus::from(true));
    };
    let frustrated = matches!(
        *nbe.arena().neutral(stuck)?.head(),
        NeutralHead::Case { .. }
            | NeutralHead::DataCase { .. }
            | NeutralHead::ListCase { .. }
            | NeutralHead::Split { .. }
            | NeutralHead::Walk { .. }
    );
    Ok(ProgressStatus::from(!frustrated))
}

/// Decides definitional equality of two **value types**.
///
/// A module signature is a record type, so this is where signature conversion
/// lands: label sets compared over the canonical order, field types compared
/// pointwise, and no width or permutation rule anywhere.
///
/// # Contract
/// - ensures: returns whether the two types are definitionally equal; the
///   unknown type is consistent with every type in both directions, matching
///   the gradual discipline subsumption runs; types embedding **values** — an
///   identity type's endpoints, a dependent pair's components — compare those
///   values through the normalizer, so definitional equality inside a type is
///   the same relation as outside it.
/// - provides: the invariant comparison the subtyping relation needs where
///   widening would be unsound.
/// - fails: never; an arena error is absorbed into a distinct verdict.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the decision surfaces are the former match, the record
///   label comparison, the unknown wildcard, and the two value-embedding
///   formers, each separated by one pair that differs in exactly it.
/// - witness: `crate::tests::signature_conversion_is_label_exact`
/// - witness: `crate::tests::identity_endpoints_convert_up_to_beta`
#[must_use]
#[inline]
pub fn type_converts(
    nbe: &mut Normalizer,
    lhs: &ValueType,
    rhs: &ValueType,
) -> ValueEquality
{
    let opened = nbe.begin_run();
    let decision = type_converts_run(nbe, lhs, rhs);
    nbe.finish_run(opened);
    decision
}

/// Decides definitional equality of two **computation types**.
///
/// The negative-sort sibling of [`type_converts`], sharing its worklist and so
/// its relation. It exists because a dependent function type is compared as a
/// whole rather than decomposed — the binder alignment a `Π` comparison needs
/// lives in this module, and a caller that decomposed the pair itself would be
/// a second place deciding when two function types are the same type.
///
/// # Contract
/// - ensures: returns whether the two computation types are definitionally
///   equal, under the same unknown-type and value-embedding discipline
///   [`type_converts`] documents; a `Π` whose binder does not occur is equal to
///   the corresponding plain arrow, and two `Π`s are compared up to the name of
///   their binder.
/// - provides: the invariant comparison the subtyping relation needs at a
///   dependent function type, where variance would be unsound.
/// - fails: never; an arena error is absorbed into a distinct verdict.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the decision surfaces are the former match and the three
///   binder-alignment cases, each separated by one pair that differs in exactly
///   it.
/// - witness: `crate::tests::pi_converts_up_to_binder_name`
/// - witness: `crate::tests::vacuous_pi_converts_with_the_plain_arrow`
/// - witness: `crate::tests::dependent_pi_does_not_convert_with_the_plain_arrow`
#[must_use]
#[inline]
pub fn comp_type_converts(
    nbe: &mut Normalizer,
    lhs: &CompType,
    rhs: &CompType,
) -> ValueEquality
{
    let opened = nbe.begin_run();
    let mut goals = alloc::vec![TypeGoal::Comp(Rc::new(lhs.clone()), Rc::new(rhs.clone()))];
    let decision = drain_type_goals(nbe, &mut goals);
    nbe.finish_run(opened);
    decision
}

/// One pending type-comparison goal.
enum TypeGoal
{
    /// Two value types to compare.
    Value(Rc<ValueType>, Rc<ValueType>),
    /// Two computation types to compare.
    Comp(Rc<CompType>, Rc<CompType>),
}

/// The worklist core of [`type_converts`].
///
/// It is total: the only place a type comparison can reach the semantic arena
/// is through [`converts`], which already absorbs an arena error into a
/// distinct verdict.
///
/// # Termination
/// - reason: the walk drains an explicit goal stack over finite type trees.
/// - measure: pending goals.
/// - boundedness: every goal decomposes into strictly smaller type pairs.
/// - input recursion: none.
fn type_converts_run(
    nbe: &mut Normalizer,
    lhs: &ValueType,
    rhs: &ValueType,
) -> ValueEquality
{
    let mut goals = alloc::vec![TypeGoal::Value(Rc::new(lhs.clone()), Rc::new(rhs.clone()))];
    drain_type_goals(nbe, &mut goals)
}

/// Drains a seeded type-comparison goal stack to a verdict.
///
/// The shared core of the two entries, so the positive and negative sorts
/// decide **one** relation rather than two that have to be kept in step.
///
/// # Termination
/// - reason: the walk drains an explicit goal stack over finite type trees.
/// - measure: pending goals.
/// - boundedness: every goal decomposes into strictly smaller type pairs.
/// - input recursion: none.
fn drain_type_goals(
    nbe: &mut Normalizer,
    goals: &mut Vec<TypeGoal>,
) -> ValueEquality
{
    while let Some(goal) = goals.pop() {
        match goal {
            | TypeGoal::Value(lhs, rhs) => {
                if !bool::from(value_type_goal(nbe, &lhs, &rhs, goals)) {
                    return ValueEquality::from(false);
                }
            },
            | TypeGoal::Comp(lhs, rhs) => {
                if !bool::from(comp_type_goal(&lhs, &rhs, goals)) {
                    return ValueEquality::from(false);
                }
            },
        }
    }
    ValueEquality::from(true)
}

/// Compares one value-type pair.
fn value_type_goal(
    nbe: &mut Normalizer,
    lhs: &Rc<ValueType>,
    rhs: &Rc<ValueType>,
    goals: &mut Vec<TypeGoal>,
) -> ValueEquality
{
    if Rc::ptr_eq(lhs, rhs)
        || matches!(**lhs, ValueType::Unknown)
        || matches!(**rhs, ValueType::Unknown)
    {
        return ValueEquality::from(true);
    }
    match (&**lhs, &**rhs) {
        | (&ValueType::Atom(ref left), &ValueType::Atom(ref right)) => {
            ValueEquality::from(left == right)
        },
        | (&ValueType::Unit, &ValueType::Unit) | (&ValueType::Universe, &ValueType::Universe) => {
            ValueEquality::from(true)
        },
        | (&ValueType::Sealed(ref left), &ValueType::Sealed(ref right)) => {
            ValueEquality::from(left == right)
        },
        | (
            &ValueType::Prod(ref left_fst, ref left_snd),
            &ValueType::Prod(ref right_fst, ref right_snd),
        )
        | (
            &ValueType::Sum(ref left_fst, ref left_snd),
            &ValueType::Sum(ref right_fst, ref right_snd),
        ) => {
            goals.push(TypeGoal::Value(Rc::clone(left_snd), Rc::clone(right_snd)));
            goals.push(TypeGoal::Value(Rc::clone(left_fst), Rc::clone(right_fst)));
            ValueEquality::from(true)
        },
        | (&ValueType::List(ref left), &ValueType::List(ref right)) => {
            goals.push(TypeGoal::Value(Rc::clone(left), Rc::clone(right)));
            ValueEquality::from(true)
        },
        | (&ValueType::Record(ref left), &ValueType::Record(ref right)) => {
            if left.len() != right.len() {
                return ValueEquality::from(false);
            }
            for ((left_label, left_field), (right_label, right_field)) in
                left.iter().zip(right.iter())
            {
                if left_label != right_label {
                    return ValueEquality::from(false);
                }
                goals.push(TypeGoal::Value(
                    Rc::clone(left_field),
                    Rc::clone(right_field),
                ));
            }
            ValueEquality::from(true)
        },
        | (
            &ValueType::Thunk(left_grade, ref left_body),
            &ValueType::Thunk(right_grade, ref right_body),
        ) => {
            if left_grade != right_grade {
                return ValueEquality::from(false);
            }
            goals.push(TypeGoal::Comp(Rc::clone(left_body), Rc::clone(right_body)));
            ValueEquality::from(true)
        },
        | (
            &ValueType::Stk(ref left_consumes, ref left_delivers),
            &ValueType::Stk(ref right_consumes, ref right_delivers),
        ) => {
            goals.push(TypeGoal::Comp(
                Rc::clone(left_delivers),
                Rc::clone(right_delivers),
            ));
            goals.push(TypeGoal::Comp(
                Rc::clone(left_consumes),
                Rc::clone(right_consumes),
            ));
            ValueEquality::from(true)
        },
        // A neutral type spine: heads first, then arguments pointwise. The
        // arguments are values, so this is where a type comparison descends
        // into terms — the same descent a `Path` endpoint makes, through the
        // same relation, rather than a second kind of descent.
        //
        // Arity is compared before the arguments because two spines with
        // different arities are different types whatever their common prefix
        // says.
        | (
            &ValueType::Family {
                head: ref left_head,
                args: ref left_args,
            },
            &ValueType::Family {
                head: ref right_head,
                args: ref right_args,
            },
        ) => {
            if left_head != right_head || left_args.len() != right_args.len() {
                return ValueEquality::from(false);
            }
            for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
                if !bool::from(converts(nbe, left_arg, right_arg)) {
                    return ValueEquality::from(false);
                }
            }
            ValueEquality::from(true)
        },
        | (
            &ValueType::Path {
                ty: ref left_ty,
                lhs: ref left_lhs,
                rhs: ref left_rhs,
            },
            &ValueType::Path {
                ty: ref right_ty,
                lhs: ref right_lhs,
                rhs: ref right_rhs,
            },
        ) => {
            if !bool::from(converts(nbe, left_lhs, right_lhs))
                || !bool::from(converts(nbe, left_rhs, right_rhs))
            {
                return ValueEquality::from(false);
            }
            goals.push(TypeGoal::Value(Rc::clone(left_ty), Rc::clone(right_ty)));
            ValueEquality::from(true)
        },
        | (
            &ValueType::Data {
                id: ref left_id,
                args: ref left_args,
            },
            &ValueType::Data {
                id: ref right_id,
                args: ref right_args,
            },
        ) => {
            if left_id != right_id {
                return ValueEquality::from(false);
            }
            if left_args.is_empty() || right_args.is_empty() {
                return ValueEquality::from(true);
            }
            if left_args.len() != right_args.len() {
                return ValueEquality::from(false);
            }
            for (left, right) in left_args.iter().zip(right_args.iter()) {
                goals.push(TypeGoal::Value(Rc::clone(left), Rc::clone(right)));
            }
            ValueEquality::from(true)
        },
        | (
            &ValueType::Sigma {
                fst: ref left_fst,
                binder: ref left_binder,
                snd: ref left_snd,
            },
            &ValueType::Sigma {
                fst: ref right_fst,
                binder: ref right_binder,
                snd: ref right_snd,
            },
        ) => {
            let left_snd = if left_binder == right_binder {
                Rc::clone(left_snd)
            }
            else {
                Rc::new(subst_valuetype(
                    left_snd,
                    NameRef::from(left_binder.as_str()),
                    &Value::var(NameRef::from(right_binder.as_str())),
                ))
            };
            goals.push(TypeGoal::Value(left_snd, Rc::clone(right_snd)));
            goals.push(TypeGoal::Value(Rc::clone(left_fst), Rc::clone(right_fst)));
            ValueEquality::from(true)
        },
        | _ => ValueEquality::from(false),
    }
}

/// Brings two function types' codomains into one binder scope, so the ordinary
/// congruence can compare them, or reports that no such scope exists.
///
/// This is the **α-equivalence and degeneracy** half of the `Π` congruence, and
/// it is the single place in the tree that decides when a dependent function
/// type and a plain arrow classify the same functions. Three cases:
///
/// * **Both binders written.** Renaming the right codomain's binder to the
///   left's puts the two in one scope. Identical names need no renaming, which
///   is the common case and costs nothing.
/// * **Neither binder written.** Two plain arrows are already in one scope.
/// * **One binder written.** `Π(x : A). B` and `A → B′` classify the same
///   functions exactly when `x` does not occur free in `B`. The occurrence is
///   *decided here* rather than asserted at construction, which is what lets
///   the elaborator write binders freely without a canonicity obligation.
///
/// # Why the question is not "are the binder names equal"
///
/// Binder names are not observable: they come from source, and two signatures
/// naming the same function type with different variable names must convert.
/// Comparing the names would make conversion depend on spelling, which is
/// exactly the bug α-equivalence exists to prevent.
///
/// # Contract
/// - ensures: returns the right codomain relocated into the left's binder scope
///   when the two function types can be related, and `None` when a written
///   binder occurs free on one side and has no counterpart on the other.
/// - panics: none.
fn align_binders(
    left_binder: &Option<String>,
    right_binder: &Option<String>,
    left_res: &Rc<CompType>,
    right_res: &Rc<CompType>,
) -> Option<Rc<CompType>>
{
    match (left_binder.as_deref(), right_binder.as_deref()) {
        | (None, None) => Some(Rc::clone(right_res)),
        | (Some(left), Some(right)) => {
            if left == right {
                return Some(Rc::clone(right_res));
            }
            Some(Rc::new(subst_comptype(
                right_res,
                NameRef::from(right),
                &Value::Var(String::from(left)),
            )))
        },
        // One side quantifies and the other does not: they agree exactly when
        // the quantification is vacuous.
        | (Some(left), None) => {
            (!occurs_free_comptype(left_res, NameRef::from(left))).then(|| Rc::clone(right_res))
        },
        | (None, Some(right)) => {
            (!occurs_free_comptype(right_res, NameRef::from(right))).then(|| Rc::clone(right_res))
        },
    }
}

/// Compares one computation-type pair.
///
/// No arm reaches a value, so this one never consults the normalizer.
fn comp_type_goal(
    lhs: &Rc<CompType>,
    rhs: &Rc<CompType>,
    goals: &mut Vec<TypeGoal>,
) -> ValueEquality
{
    if Rc::ptr_eq(lhs, rhs)
        || matches!(**lhs, CompType::Unknown)
        || matches!(**rhs, CompType::Unknown)
    {
        return ValueEquality::from(true);
    }
    match (&**lhs, &**rhs) {
        | (&CompType::F(ref left_of, ref left_row), &CompType::F(ref right_of, ref right_row)) => {
            if left_row != right_row {
                return ValueEquality::from(false);
            }
            goals.push(TypeGoal::Value(Rc::clone(left_of), Rc::clone(right_of)));
            ValueEquality::from(true)
        },
        | (
            &CompType::Arrow {
                binder: ref left_binder,
                arg: ref left_arg,
                res: ref left_res,
            },
            &CompType::Arrow {
                binder: ref right_binder,
                arg: ref right_arg,
                res: ref right_res,
            },
        ) => {
            let Some(right_res) = align_binders(left_binder, right_binder, left_res, right_res)
            else {
                return ValueEquality::from(false);
            };
            goals.push(TypeGoal::Comp(Rc::clone(left_res), right_res));
            goals.push(TypeGoal::Value(Rc::clone(left_arg), Rc::clone(right_arg)));
            ValueEquality::from(true)
        },
        | (
            &CompType::With(ref left_fst, ref left_snd),
            &CompType::With(ref right_fst, ref right_snd),
        ) => {
            goals.push(TypeGoal::Comp(Rc::clone(left_snd), Rc::clone(right_snd)));
            goals.push(TypeGoal::Comp(Rc::clone(left_fst), Rc::clone(right_fst)));
            ValueEquality::from(true)
        },
        | _ => ValueEquality::from(false),
    }
}
