//! The per-environment append-only term arena (D1(C), gandr-5t3): the single
//! owner of every S1 term/type node, addressed by four typed `u32`-backed node
//! ids ([`ValueId`], [`ComputationId`], [`ValueTypeId`], [`CompTypeId`]).
//!
//! # Why an arena, not owned trees (impl-models §2.1, §5.1(1))
//!
//! The predecessor representation was mutually-recursive owned `Box` trees.
//! Their derived `Drop`/`Clone` recurse on term depth and overflow the stack on
//! adversarial input (decode can build an arbitrarily deep term from bytes,
//! gandr-i3i), which forced hand-written iterative `Drop`/`Clone` worklists
//! into the TCB. The arena **eliminates** that hazard family rather than
//! managing it: a node's children are `Copy` ids, so the node enums derive a
//! **shallow** `Clone`/`Drop`/`PartialEq`/`Hash`, and arena teardown is a flat
//! `Vec` drop — no recursive drop glue, no manual worklist to audit. This is
//! the Idris-2 context / smalltt int-indexed-array discipline (impl-models
//! §1.4, §4.1) and the Lean intrusive-word end-state (impl-models §2.1, §5.6
//! #5: the cached word is **S2-deferred** — no per-node slot is reserved yet).
//!
//! # The K1-weakening and its mitigations
//!
//! An owned tree cannot be ill-formed; a `u32` id *can* name no node or a node
//! in another arena (a **K1 weakening**, honestly recorded). The mitigations
//! that keep it fail-closed:
//!
//! * **Constructor-only minting.** An id is produced *only* by a [`TermArena`]
//!   constructor over already-allocated children, so a child id always resolves
//!   and is always **strictly less** than its parent's id (acyclic by
//!   construction — the same strictly-earlier invariant the export subterm
//!   table relies on).
//! * **One arena per environment.** Every id in a declaration resolves against
//!   the one [`Environment`](crate::Environment) arena; ids never cross arenas.
//! * **Checked `get`.** Resolution is [`TermArena::value`] &c. — a
//!   bounds-checked [`Option`] read (never indexing, per
//!   docs/workflow/rust.md), so a dangling id fails closed (the checker maps it
//!   to [`KernelError::ArenaFault`], the conversion walk to
//!   [`Convertibility::Distinct`](crate::Convertibility)) — the
//!   [`KernelError::CheckerRegisterFault`] "surface, never trust" posture.
//! * **Decode-side validation.** The export reader (commit 2) validates every
//!   child reference (strictly-earlier and polarity-correct) before it mints,
//!   so a decoded arena is well-formed by the same invariant.
//!
//! # Admission watermark discipline (the Idris staging overlay, impl-models §1.4/§5.3)
//!
//! A declaration's content is built into the environment arena by a
//! [`DeclarationBuilder`], which records the arena's lengths at the point
//! building began (the **content-start** watermark).
//! Admission ([`Environment::add_decl`](crate::Environment::add_decl)) reads a
//! second mark on entry (the **content-end**, past which the checker's
//! synthesized intermediates allocate) and truncates after the verdict: to
//! content-end on success (dropping intermediates, keeping content) or to
//! content-start on rejection (dropping both). This is the transactional
//! commit-on-success overlay of the Idris-2 `branchDepth`/`staging` context,
//! adapted to the flat arena.
//!
//! [`DeclarationBuilder`]: crate::DeclarationBuilder
//! [`KernelError::ArenaFault`]: crate::KernelError::ArenaFault
//! [`KernelError::CheckerRegisterFault`]: crate::KernelError::CheckerRegisterFault

use alloc::vec::Vec;

use gandr_kernel_strata::Level;

use crate::base::Literal;
use crate::term::Computation;
use crate::term::ConstantIndex;
use crate::term::DeBruijnIndex;
use crate::term::Side;
use crate::term::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// The id of a [`Value`] node in a [`TermArena`].
///
/// Minted only by a [`TermArena`] constructor over already-allocated children,
/// so it always resolves and is strictly greater than every child id.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ValueId(u32);

/// The id of a [`Computation`] node in a [`TermArena`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComputationId(u32);

/// The id of a [`ValueType`] node in a [`TermArena`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ValueTypeId(u32);

/// The id of a [`CompType`] node in a [`TermArena`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompTypeId(u32);

/// Widen an arena length to the `u32` an id wraps, saturating at the ceiling.
///
/// # Contract
/// - requires: `length` is a `Vec` length within an arena.
/// - ensures: the equal `u32`, or `u32::MAX` if the arena exceeded the id space
///   (~4 billion nodes — far above `MAX_TABLE_ENTRIES`, the decode cap; an
///   in-memory arena beyond it would alias, a documented ceiling).
/// - provides: the total, panic-free length→id-index widening.
/// - fails: never (saturates).
/// - panics: none.
#[inline]
fn id_index(length: usize) -> u32
{
    u32::try_from(length).unwrap_or(u32::MAX)
}

/// Narrow an id's wrapped `u32` to the `usize` a `Vec::get` takes.
///
/// # Contract
/// - requires: nothing.
/// - ensures: the equal `usize` (lossless on every supported ≥32-bit platform).
/// - provides: the total, panic-free id-index→offset narrowing.
/// - fails: never (saturates at `usize::MAX`, which `get` rejects).
/// - panics: none.
#[inline]
fn id_offset(index: u32) -> usize
{
    usize::try_from(index).unwrap_or(usize::MAX)
}

/// A snapshot of the four family lengths — the admission watermark.
///
/// Restoring an arena to a watermark ([`TermArena::truncate_to`]) drops exactly
/// the nodes allocated after it, in flat `Vec` truncations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArenaWatermark
{
    /// The [`Value`] family length.
    values: usize,
    /// The [`Computation`] family length.
    computations: usize,
    /// The [`ValueType`] family length.
    value_types: usize,
    /// The [`CompType`] family length.
    comp_types: usize,
}

/// The append-only arena owning every S1 term and type node of one environment.
///
/// The four families are parallel append-only `Vec`s; a node's children are ids
/// into the same arena. The node enums derive shallow `Clone`/`Drop`, so
/// cloning or dropping a whole arena is a flat per-family `Vec` operation,
/// total on any term depth.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TermArena
{
    /// The value nodes, in allocation (child-before-parent) order.
    values: Vec<Value>,
    /// The computation nodes.
    computations: Vec<Computation>,
    /// The value-type nodes.
    value_types: Vec<ValueType>,
    /// The computation-type nodes.
    comp_types: Vec<CompType>,
}

impl TermArena
{
    /// An empty arena.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// The current watermark (the four family lengths).
    #[inline]
    #[must_use]
    pub(crate) fn watermark(&self) -> ArenaWatermark
    {
        ArenaWatermark {
            values: self.values.len(),
            computations: self.computations.len(),
            value_types: self.value_types.len(),
            comp_types: self.comp_types.len(),
        }
    }

    /// Truncate every family back to `watermark`, dropping later allocations.
    ///
    /// # Contract
    /// - requires: `watermark` was taken from this arena and no family has
    ///   since shrunk below it.
    /// - ensures: each family holds exactly its watermark-many leading nodes;
    ///   every id minted after the watermark now dangles (and must be
    ///   unreachable, per the admission discipline).
    /// - provides: the rejection/intermediate truncation of `add_decl`.
    /// - fails: never (a `Vec::truncate` past the end is a no-op).
    /// - panics: none.
    #[inline]
    pub(crate) fn truncate_to(
        &mut self,
        watermark: ArenaWatermark,
    )
    {
        self.values.truncate(watermark.values);
        self.computations.truncate(watermark.computations);
        self.value_types.truncate(watermark.value_types);
        self.comp_types.truncate(watermark.comp_types);
    }

    /// Resolve a value id to its node, or `None` if it dangles (fail-closed).
    #[inline]
    #[must_use]
    pub(crate) fn value(
        &self,
        id: ValueId,
    ) -> Option<&Value>
    {
        self.values.get(id_offset(id.0))
    }

    /// Resolve a computation id to its node, or `None` if it dangles.
    #[inline]
    #[must_use]
    pub(crate) fn computation(
        &self,
        id: ComputationId,
    ) -> Option<&Computation>
    {
        self.computations.get(id_offset(id.0))
    }

    /// Resolve a value-type id to its node, or `None` if it dangles.
    #[inline]
    #[must_use]
    pub(crate) fn value_type(
        &self,
        id: ValueTypeId,
    ) -> Option<&ValueType>
    {
        self.value_types.get(id_offset(id.0))
    }

    /// Resolve a computation-type id to its node, or `None` if it dangles.
    #[inline]
    #[must_use]
    pub(crate) fn comp_type(
        &self,
        id: CompTypeId,
    ) -> Option<&CompType>
    {
        self.comp_types.get(id_offset(id.0))
    }

    /// Append a value node and return its fresh id.
    #[inline]
    pub(crate) fn alloc_value(
        &mut self,
        value: Value,
    ) -> ValueId
    {
        let id = ValueId(id_index(self.values.len()));
        self.values.push(value);
        id
    }

    /// Append a computation node and return its fresh id.
    #[inline]
    pub(crate) fn alloc_computation(
        &mut self,
        computation: Computation,
    ) -> ComputationId
    {
        let id = ComputationId(id_index(self.computations.len()));
        self.computations.push(computation);
        id
    }

    /// Append a value-type node and return its fresh id.
    #[inline]
    pub(crate) fn alloc_value_type(
        &mut self,
        value_type: ValueType,
    ) -> ValueTypeId
    {
        let id = ValueTypeId(id_index(self.value_types.len()));
        self.value_types.push(value_type);
        id
    }

    /// Append a computation-type node and return its fresh id.
    #[inline]
    pub(crate) fn alloc_comp_type(
        &mut self,
        comp_type: CompType,
    ) -> CompTypeId
    {
        let id = CompTypeId(id_index(self.comp_types.len()));
        self.comp_types.push(comp_type);
        id
    }

    // ----- Value constructors (mint over already-allocated children) -----

    /// Mint a bound value variable.
    #[inline]
    pub fn value_variable(
        &mut self,
        index: DeBruijnIndex,
    ) -> ValueId
    {
        self.alloc_value(Value::Variable(index))
    }

    /// Mint a constant reference to a prior declaration.
    #[inline]
    pub fn value_constant(
        &mut self,
        index: ConstantIndex,
    ) -> ValueId
    {
        self.alloc_value(Value::Constant(index))
    }

    /// Mint the unit value.
    #[inline]
    pub fn value_unit(&mut self) -> ValueId
    {
        self.alloc_value(Value::Unit)
    }

    /// Mint a base-type literal value.
    #[inline]
    pub fn value_literal(
        &mut self,
        literal: Literal,
    ) -> ValueId
    {
        self.alloc_value(Value::Literal(literal))
    }

    /// Mint a pair over two already-allocated value children.
    #[inline]
    pub fn value_pair(
        &mut self,
        first: ValueId,
        second: ValueId,
    ) -> ValueId
    {
        self.alloc_value(Value::Pair(first, second))
    }

    /// Mint a sum injection over an already-allocated value body.
    #[inline]
    pub fn value_injection(
        &mut self,
        side: Side,
        body: ValueId,
    ) -> ValueId
    {
        self.alloc_value(Value::Injection(side, body))
    }

    /// Mint a thunk over an already-allocated computation body.
    #[inline]
    pub fn value_thunk(
        &mut self,
        body: ComputationId,
    ) -> ValueId
    {
        self.alloc_value(Value::Thunk(body))
    }

    /// Mint a value lift over an already-allocated value body.
    #[inline]
    pub fn value_lift(
        &mut self,
        target: Level,
        body: ValueId,
    ) -> ValueId
    {
        self.alloc_value(Value::Lift { target, body })
    }

    // ----- Computation constructors -----

    /// Mint a lambda over an already-allocated computation body.
    #[inline]
    pub fn computation_lambda(
        &mut self,
        body: ComputationId,
    ) -> ComputationId
    {
        self.alloc_computation(Computation::Lambda(body))
    }

    /// Mint an application over an already-allocated head and argument.
    #[inline]
    pub fn computation_application(
        &mut self,
        head: ComputationId,
        argument: ValueId,
    ) -> ComputationId
    {
        self.alloc_computation(Computation::Application(head, argument))
    }

    /// Mint a returner over an already-allocated value.
    #[inline]
    pub fn computation_return(
        &mut self,
        value: ValueId,
    ) -> ComputationId
    {
        self.alloc_computation(Computation::Return(value))
    }

    /// Mint a bind over an already-allocated bound computation and body.
    #[inline]
    pub fn computation_bind(
        &mut self,
        bound: ComputationId,
        body: ComputationId,
    ) -> ComputationId
    {
        self.alloc_computation(Computation::Bind(bound, body))
    }

    /// Mint a force over an already-allocated value.
    #[inline]
    pub fn computation_force(
        &mut self,
        value: ValueId,
    ) -> ComputationId
    {
        self.alloc_computation(Computation::Force(value))
    }

    /// Mint a case over an already-allocated scrutinee and two branches.
    #[inline]
    pub fn computation_case(
        &mut self,
        scrutinee: ValueId,
        on_left: ComputationId,
        on_right: ComputationId,
    ) -> ComputationId
    {
        self.alloc_computation(Computation::Case {
            scrutinee,
            on_left,
            on_right,
        })
    }

    // ----- Value-type constructors -----

    /// Mint a base value type.
    #[inline]
    pub fn value_type_base(
        &mut self,
        base: crate::base::BaseType,
    ) -> ValueTypeId
    {
        self.alloc_value_type(ValueType::Base(base))
    }

    /// Mint the unit value type.
    #[inline]
    pub fn value_type_unit(&mut self) -> ValueTypeId
    {
        self.alloc_value_type(ValueType::Unit)
    }

    /// Mint a product over two already-allocated value types.
    #[inline]
    pub fn value_type_product(
        &mut self,
        first: ValueTypeId,
        second: ValueTypeId,
    ) -> ValueTypeId
    {
        self.alloc_value_type(ValueType::Product(first, second))
    }

    /// Mint a sum over two already-allocated value types.
    #[inline]
    pub fn value_type_sum(
        &mut self,
        first: ValueTypeId,
        second: ValueTypeId,
    ) -> ValueTypeId
    {
        self.alloc_value_type(ValueType::Sum(first, second))
    }

    /// Mint a thunk type over an already-allocated computation type.
    #[inline]
    pub fn value_type_thunk(
        &mut self,
        body: CompTypeId,
    ) -> ValueTypeId
    {
        self.alloc_value_type(ValueType::Thunk(body))
    }

    /// Mint a universe value type at a canonical level.
    #[inline]
    pub fn value_type_universe(
        &mut self,
        level: Level,
    ) -> ValueTypeId
    {
        self.alloc_value_type(ValueType::Universe(level))
    }

    /// Mint a value-type lift over an already-allocated inner value type.
    #[inline]
    pub fn value_type_lift(
        &mut self,
        inner: ValueTypeId,
        target: Level,
    ) -> ValueTypeId
    {
        self.alloc_value_type(ValueType::Lift { inner, target })
    }

    // ----- Computation-type constructors -----

    /// Mint a returner type over an already-allocated value type.
    #[inline]
    pub fn comp_type_returner(
        &mut self,
        result: ValueTypeId,
    ) -> CompTypeId
    {
        self.alloc_comp_type(CompType::Returner(result))
    }

    /// Mint an arrow type over an already-allocated domain and codomain.
    #[inline]
    pub fn comp_type_arrow(
        &mut self,
        domain: ValueTypeId,
        codomain: CompTypeId,
    ) -> CompTypeId
    {
        self.alloc_comp_type(CompType::Arrow { domain, codomain })
    }
}

/// A cross-family node reference — the work item of the reachable-closure copy.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum AnyId
{
    /// A value node.
    Value(ValueId),
    /// A computation node.
    Computation(ComputationId),
    /// A value-type node.
    ValueType(ValueTypeId),
    /// A computation-type node.
    CompType(CompTypeId),
}

impl TermArena
{
    /// The immediate child references of `node`, in order (empty for a leaf).
    ///
    /// # Contract
    /// - requires: `node` resolves in this arena (else an empty list, the
    ///   fail-closed reading).
    /// - ensures: the node's child ids as [`AnyId`]s, each strictly less than
    ///   `node`'s own id (the minting invariant).
    /// - provides: the edge relation the reachable-closure copy walks.
    /// - fails: never — a dangling id yields no children.
    /// - panics: none.
    fn any_children(
        &self,
        node: AnyId,
    ) -> Vec<AnyId>
    {
        let mut children: Vec<AnyId> = Vec::new();
        match node {
            | AnyId::Value(id) => {
                if let Some(value) = self.value(id) {
                    match *value {
                        | Value::Variable(_)
                        | Value::Constant(_)
                        | Value::Unit
                        | Value::Literal(_) => {},
                        | Value::Pair(first, second) => {
                            children.push(AnyId::Value(first));
                            children.push(AnyId::Value(second));
                        },
                        | Value::Injection(_, body) | Value::Lift { body, .. } => {
                            children.push(AnyId::Value(body));
                        },
                        | Value::Thunk(body) => children.push(AnyId::Computation(body)),
                    }
                }
            },
            | AnyId::Computation(id) => {
                if let Some(computation) = self.computation(id) {
                    match *computation {
                        | Computation::Lambda(body) => children.push(AnyId::Computation(body)),
                        | Computation::Application(head, argument) => {
                            children.push(AnyId::Computation(head));
                            children.push(AnyId::Value(argument));
                        },
                        | Computation::Return(value) | Computation::Force(value) => {
                            children.push(AnyId::Value(value));
                        },
                        | Computation::Bind(bound, body) => {
                            children.push(AnyId::Computation(bound));
                            children.push(AnyId::Computation(body));
                        },
                        | Computation::Case {
                            scrutinee,
                            on_left,
                            on_right,
                        } => {
                            children.push(AnyId::Value(scrutinee));
                            children.push(AnyId::Computation(on_left));
                            children.push(AnyId::Computation(on_right));
                        },
                    }
                }
            },
            | AnyId::ValueType(id) => {
                if let Some(value_type) = self.value_type(id) {
                    match *value_type {
                        | ValueType::Base(_) | ValueType::Unit | ValueType::Universe(_) => {},
                        | ValueType::Product(first, second) | ValueType::Sum(first, second) => {
                            children.push(AnyId::ValueType(first));
                            children.push(AnyId::ValueType(second));
                        },
                        | ValueType::Thunk(body) => children.push(AnyId::CompType(body)),
                        | ValueType::Lift { inner, .. } => children.push(AnyId::ValueType(inner)),
                    }
                }
            },
            | AnyId::CompType(id) => {
                if let Some(comp_type) = self.comp_type(id) {
                    match *comp_type {
                        | CompType::Returner(result) => children.push(AnyId::ValueType(result)),
                        | CompType::Arrow { domain, codomain } => {
                            children.push(AnyId::ValueType(domain));
                            children.push(AnyId::CompType(codomain));
                        },
                    }
                }
            },
        }
        children
    }
}

/// A memo from source [`AnyId`] to the id it was copied to in the destination.
type ReifyMemo = alloc::collections::BTreeMap<AnyId, AnyId>;

/// Copy the node `source` names — with its already-copied children remapped
/// through `memo` — into `destination`, returning its new [`AnyId`].
///
/// # Contract
/// - requires: every child of `source` is already keyed in `memo`; `source`
///   resolves in `origin`.
/// - ensures: `destination` gains the structurally-identical node and its new
///   id is returned; a dangling id or an absent child remaps to a unit leaf
///   (the fail-closed reading — unreachable under the minting invariant).
/// - provides: the assembly step of [`TermArena::reify`].
/// - fails: never.
/// - panics: none.
#[expect(
    clippy::too_many_lines,
    reason = "the flat assembly enumerates every former across the four families in one match; splitting it would obscure the single copy step"
)]
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of the borrowed source node against value patterns; every binding is a shared reference by intent"
)]
fn reify_assemble(
    origin: &TermArena,
    destination: &mut TermArena,
    memo: &ReifyMemo,
    source: AnyId,
) -> AnyId
{
    /// Remap a source value child through the memo, or a unit leaf if absent.
    fn value_of(
        memo: &ReifyMemo,
        destination: &mut TermArena,
        child: AnyId,
    ) -> ValueId
    {
        match memo.get(&child) {
            | Some(&AnyId::Value(id)) => id,
            | _ => destination.value_unit(),
        }
    }
    /// Remap a source computation child, or a `return unit` leaf if absent.
    fn computation_of(
        memo: &ReifyMemo,
        destination: &mut TermArena,
        child: AnyId,
    ) -> ComputationId
    {
        match memo.get(&child) {
            | Some(&AnyId::Computation(id)) => id,
            | _ => {
                let unit = destination.value_unit();
                destination.computation_return(unit)
            },
        }
    }
    /// Remap a source value-type child, or a unit leaf if absent.
    fn value_type_of(
        memo: &ReifyMemo,
        destination: &mut TermArena,
        child: AnyId,
    ) -> ValueTypeId
    {
        match memo.get(&child) {
            | Some(&AnyId::ValueType(id)) => id,
            | _ => destination.value_type_unit(),
        }
    }
    /// Remap a source computation-type child, or a `F Unit` leaf if absent.
    fn comp_type_of(
        memo: &ReifyMemo,
        destination: &mut TermArena,
        child: AnyId,
    ) -> CompTypeId
    {
        match memo.get(&child) {
            | Some(&AnyId::CompType(id)) => id,
            | _ => {
                let unit = destination.value_type_unit();
                destination.comp_type_returner(unit)
            },
        }
    }

    match source {
        | AnyId::Value(id) => {
            let value = origin.value(id);
            let new = match value {
                | Some(&Value::Variable(index)) => destination.value_variable(index),
                | Some(&Value::Constant(index)) => destination.value_constant(index),
                | Some(&Value::Unit) | None => destination.value_unit(),
                | Some(Value::Literal(literal)) => destination.value_literal(literal.clone()),
                | Some(&Value::Pair(first, second)) => {
                    let first = value_of(memo, destination, AnyId::Value(first));
                    let second = value_of(memo, destination, AnyId::Value(second));
                    destination.value_pair(first, second)
                },
                | Some(&Value::Injection(side, body)) => {
                    let body = value_of(memo, destination, AnyId::Value(body));
                    destination.value_injection(side, body)
                },
                | Some(&Value::Thunk(body)) => {
                    let body = computation_of(memo, destination, AnyId::Computation(body));
                    destination.value_thunk(body)
                },
                | Some(Value::Lift { target, body }) => {
                    let target = target.clone();
                    let body = value_of(memo, destination, AnyId::Value(*body));
                    destination.value_lift(target, body)
                },
            };
            AnyId::Value(new)
        },
        | AnyId::Computation(id) => {
            let computation = origin.computation(id);
            let new = match computation {
                | Some(&Computation::Lambda(body)) => {
                    let body = computation_of(memo, destination, AnyId::Computation(body));
                    destination.computation_lambda(body)
                },
                | Some(&Computation::Application(head, argument)) => {
                    let head = computation_of(memo, destination, AnyId::Computation(head));
                    let argument = value_of(memo, destination, AnyId::Value(argument));
                    destination.computation_application(head, argument)
                },
                | Some(&Computation::Return(value)) => {
                    let value = value_of(memo, destination, AnyId::Value(value));
                    destination.computation_return(value)
                },
                | Some(&Computation::Force(value)) => {
                    let value = value_of(memo, destination, AnyId::Value(value));
                    destination.computation_force(value)
                },
                | Some(&Computation::Bind(bound, body)) => {
                    let bound = computation_of(memo, destination, AnyId::Computation(bound));
                    let body = computation_of(memo, destination, AnyId::Computation(body));
                    destination.computation_bind(bound, body)
                },
                | Some(&Computation::Case {
                    scrutinee,
                    on_left,
                    on_right,
                }) => {
                    let scrutinee = value_of(memo, destination, AnyId::Value(scrutinee));
                    let on_left = computation_of(memo, destination, AnyId::Computation(on_left));
                    let on_right = computation_of(memo, destination, AnyId::Computation(on_right));
                    destination.computation_case(scrutinee, on_left, on_right)
                },
                | None => {
                    let unit = destination.value_unit();
                    destination.computation_return(unit)
                },
            };
            AnyId::Computation(new)
        },
        | AnyId::ValueType(id) => {
            let value_type = origin.value_type(id);
            let new = match value_type {
                | Some(&ValueType::Base(base)) => destination.value_type_base(base),
                | Some(&ValueType::Unit) | None => destination.value_type_unit(),
                | Some(ValueType::Universe(level)) => {
                    destination.value_type_universe(level.clone())
                },
                | Some(&ValueType::Product(first, second)) => {
                    let first = value_type_of(memo, destination, AnyId::ValueType(first));
                    let second = value_type_of(memo, destination, AnyId::ValueType(second));
                    destination.value_type_product(first, second)
                },
                | Some(&ValueType::Sum(first, second)) => {
                    let first = value_type_of(memo, destination, AnyId::ValueType(first));
                    let second = value_type_of(memo, destination, AnyId::ValueType(second));
                    destination.value_type_sum(first, second)
                },
                | Some(&ValueType::Thunk(body)) => {
                    let body = comp_type_of(memo, destination, AnyId::CompType(body));
                    destination.value_type_thunk(body)
                },
                | Some(ValueType::Lift { inner, target }) => {
                    let target = target.clone();
                    let inner = value_type_of(memo, destination, AnyId::ValueType(*inner));
                    destination.value_type_lift(inner, target)
                },
            };
            AnyId::ValueType(new)
        },
        | AnyId::CompType(id) => {
            let comp_type = origin.comp_type(id);
            let new = match comp_type {
                | Some(&CompType::Returner(result)) => {
                    let result = value_type_of(memo, destination, AnyId::ValueType(result));
                    destination.comp_type_returner(result)
                },
                | Some(&CompType::Arrow { domain, codomain }) => {
                    let domain = value_type_of(memo, destination, AnyId::ValueType(domain));
                    let codomain = comp_type_of(memo, destination, AnyId::CompType(codomain));
                    destination.comp_type_arrow(domain, codomain)
                },
                | None => {
                    let unit = destination.value_type_unit();
                    destination.comp_type_returner(unit)
                },
            };
            AnyId::CompType(new)
        },
    }
}

impl TermArena
{
    /// Copy the sub-DAG reachable from `roots` into a fresh arena, preserving
    /// sharing, and return the fresh arena with each root's new [`AnyId`].
    ///
    /// # Contract
    /// - requires: every root resolves in this arena.
    /// - ensures: the returned arena holds exactly the reachable closure with
    ///   structural sharing intact; the roots map positionally to the returned
    ///   ids. The walk is a resumable-frame post-order over an explicit heap
    ///   stack, so it is total on any depth (no input-scaled recursion).
    /// - provides: the self-contained snapshot an error payload owns after the
    ///   working arena is truncated on rejection.
    /// - fails: never — a dangling reference copies as a unit leaf.
    /// - panics: none.
    fn reify(
        &self,
        roots: &[AnyId],
    ) -> (Self, Vec<AnyId>)
    {
        let mut destination = Self::new();
        let mapped = self.reify_into(&mut destination, roots);
        (destination, mapped)
    }

    /// Copy the sub-DAG reachable from `roots` into an existing `destination`
    /// arena (appending), returning each root's new [`AnyId`].
    ///
    /// # Contract
    /// - requires: every root resolves in `self`.
    /// - ensures: `destination` gains the reachable closure with sharing intact
    ///   (relative to `destination`'s own ids); the roots map positionally. The
    ///   walk is a resumable-frame post-order over an explicit heap stack: a
    ///   node is assembled only once every child is memoized, and a child is
    ///   scheduled at most once, so it is O(reachable nodes + edges), total on
    ///   any depth.
    /// - provides: the reachable-copy engine of [`Self::reify`] and the export
    ///   reader's content import ([`Self::import_def`]/[`Self::import_axiom`]).
    /// - fails: never — a dangling reference copies as a unit leaf.
    /// - panics: none.
    fn reify_into(
        &self,
        destination: &mut Self,
        roots: &[AnyId],
    ) -> Vec<AnyId>
    {
        let mut memo: ReifyMemo = ReifyMemo::new();
        for &root in roots {
            let mut stack: Vec<(AnyId, usize)> = Vec::new();
            stack.push((root, 0_usize));
            while let Some((node, cursor)) = stack.pop() {
                if memo.contains_key(&node) {
                    continue;
                }
                let children = self.any_children(node);
                let mut next = cursor;
                while let Some(&child) = children.get(next) {
                    if memo.contains_key(&child) {
                        next = next.saturating_add(1);
                    }
                    else {
                        stack.push((node, next.saturating_add(1)));
                        stack.push((child, 0_usize));
                        break;
                    }
                }
                if children.get(next).is_none() {
                    let copied = reify_assemble(self, destination, &memo, node);
                    let _prior = memo.insert(node, copied);
                }
            }
        }
        roots
            .iter()
            .map(|root| memo.get(root).copied().unwrap_or(*root))
            .collect()
    }

    /// Import a `Def`'s declared-type and body closures from `self` into
    /// `destination`, returning their new roots.
    ///
    /// # Contract
    /// - requires: `declared` and `body` resolve in `self`.
    /// - ensures: `destination` gains both closures (sharing between them
    ///   intact) and the two new roots are returned; on a family/dangling
    ///   mismatch the original ids are echoed (unreachable under the reader's
    ///   validation).
    /// - provides: the export reader's per-declaration content import into the
    ///   environment arena.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub(crate) fn import_def(
        &self,
        destination: &mut Self,
        declared: ValueTypeId,
        body: ValueId,
    ) -> (ValueTypeId, ValueId)
    {
        let mapped = self.reify_into(destination, &[
            AnyId::ValueType(declared),
            AnyId::Value(body),
        ]);
        let declared = match mapped.first() {
            | Some(&AnyId::ValueType(id)) => id,
            | _ => declared,
        };
        let body = match mapped.get(1) {
            | Some(&AnyId::Value(id)) => id,
            | _ => body,
        };
        (declared, body)
    }

    /// Import an `Axiom`'s declared-type closure from `self` into
    /// `destination`, returning its new root.
    ///
    /// # Contract
    /// - requires: `declared` resolves in `self`.
    /// - ensures: `destination` gains the closure and the new root is returned.
    /// - provides: the export reader's axiom content import.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub(crate) fn import_axiom(
        &self,
        destination: &mut Self,
        declared: ValueTypeId,
    ) -> ValueTypeId
    {
        let mapped = self.reify_into(destination, &[AnyId::ValueType(declared)]);
        match mapped.first() {
            | Some(&AnyId::ValueType(id)) => id,
            | _ => declared,
        }
    }

    /// Snapshot the closure of one value type into a fresh arena.
    #[inline]
    #[must_use]
    pub(crate) fn snapshot_value_type(
        &self,
        root: ValueTypeId,
    ) -> (Self, ValueTypeId)
    {
        let (arena, mapped) = self.reify(&[AnyId::ValueType(root)]);
        let root = match mapped.first() {
            | Some(&AnyId::ValueType(id)) => id,
            | _ => root,
        };
        (arena, root)
    }

    /// Snapshot the closure of two value types into one fresh arena.
    #[inline]
    #[must_use]
    pub(crate) fn snapshot_value_types(
        &self,
        first: ValueTypeId,
        second: ValueTypeId,
    ) -> (Self, ValueTypeId, ValueTypeId)
    {
        let (arena, mapped) = self.reify(&[AnyId::ValueType(first), AnyId::ValueType(second)]);
        let first = match mapped.first() {
            | Some(&AnyId::ValueType(id)) => id,
            | _ => first,
        };
        let second = match mapped.get(1) {
            | Some(&AnyId::ValueType(id)) => id,
            | _ => second,
        };
        (arena, first, second)
    }

    /// Snapshot the closure of one computation type into a fresh arena.
    #[inline]
    #[must_use]
    pub(crate) fn snapshot_comp_type(
        &self,
        root: CompTypeId,
    ) -> (Self, CompTypeId)
    {
        let (arena, mapped) = self.reify(&[AnyId::CompType(root)]);
        let root = match mapped.first() {
            | Some(&AnyId::CompType(id)) => id,
            | _ => root,
        };
        (arena, root)
    }

    /// Snapshot the closure of two computation types into one fresh arena.
    #[inline]
    #[must_use]
    pub(crate) fn snapshot_comp_types(
        &self,
        first: CompTypeId,
        second: CompTypeId,
    ) -> (Self, CompTypeId, CompTypeId)
    {
        let (arena, mapped) = self.reify(&[AnyId::CompType(first), AnyId::CompType(second)]);
        let first = match mapped.first() {
            | Some(&AnyId::CompType(id)) => id,
            | _ => first,
        };
        let second = match mapped.get(1) {
            | Some(&AnyId::CompType(id)) => id,
            | _ => second,
        };
        (arena, first, second)
    }
}
