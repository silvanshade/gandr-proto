//! Decision-tree compilation for arm sets the tag walk cannot place, and the
//! join point that lets one arm body be reached from several branches.
//!
//! # What this module is for
//!
//! [`super::pattern`] compiles arms the declared-data eliminator can take one
//! at a time: each surface arm fills exactly one constructor tag, and its
//! nested sub-patterns become a chain of enclosing tests. That mapping is
//! total on the arm sets it accepts and cannot express three shapes, each for
//! the same reason — **one arm body has to be reached from more than one
//! branch**:
//!
//! * a top-level catch-all, whose body every tag no earlier arm settled must
//!   reach;
//! * an or-pattern with distinguishable alternatives, whose body each named tag
//!   must reach;
//! * two arms sharing one constructor head, told apart only by their arguments,
//!   so the tag's branch tests further and both arms live under it.
//!
//! Those three are what this module compiles. A column whose head domain this
//! eliminator does not switch on — a literal, tuple, record, or list — is a
//! missing *test* rather than a missing join, and is still declined by name.
//!
//! # The join point is a bound thunk, and the core stays frozen
//!
//! A join point is a **compilation device**, not a term former: it names a
//! continuation several branches share and has no meaning a user writes.
//! Call-by-push-value already names computations — `thunk` makes one a value
//! and `force` runs it — so a shared body binds once before the match and each
//! branch that reaches it forces the binding:
//!
//! ```text
//! run %j <- ret ((thunk { fn(%p) { body } }) : U_ω (? → ?));
//! case %s { … force %j %x … force %j %y … }
//! ```
//!
//! Grades sit on the thunk's type rather than on context entries, so `force`
//! asks only `1 ⊑ r`; and a `case`'s branches are alternatives rather than a
//! sequence. Those two facts are what let one binding be forced from several
//! branches with no accounting change, so **no core variant is added and none
//! is needed**. The precedent runs the same way: Idris 2 elaborates a case
//! block to a top-level function and calls it, and Lean's join points live in
//! its compiler IR rather than in its kernel term language.
//!
//! # What the encoding costs, and when it stops costing it
//!
//! A shared body is typed through the thunk's annotation, and the lowerer has
//! an answer type to put there only when the source wrote `case … -> B`.
//! Without one the annotation is the gradual unknown, so a shared body is
//! **checked against the unknown rather than against the match's answer
//! type**. Every error inside the body is still caught; what goes unchecked is
//! the agreement between that body's result and the match's answer.
//!
//! Two things bound the cost. A body reached from exactly one branch is
//! **inlined** rather than joined, so it keeps full precision — and that is
//! every arm of every match the tag walk already compiled. And the loss
//! reverses exactly: when the lowerer threads an expected answer type into
//! `case`, the annotation takes it and nothing else here changes.
//!
//! # Why the tree is data
//!
//! Compilation runs in two passes over an explicit [`TreeArena`]: the first
//! reads the pattern matrix into a decision tree, the second emits core from
//! it. Splitting them is what makes the join decision possible — an arm body
//! is inlined or bound before either is emitted, and only the finished tree
//! knows how many branches reach it. Both passes carry their own work stacks:
//! pattern depth, column count, and constructor arity are all
//! input-controlled, so neither may recurse.

use alloc::borrow::ToOwned as _;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_term::boundary::ConstructorTag;
use gandr_core_term::boundary::DataTypeName;
use gandr_core_term::grade::Grade;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::CompType;
use gandr_core_term::types::ValueType;

use super::COut;
use super::LowerError;
use super::LowerResult;
use super::Lowerer;
use super::entry;
use super::node_kinds;
use super::pattern::ArmVerdict;
use super::pattern::PatternPlan;
use super::pattern::StuckPattern;
use crate::boundary::ArmIndex;
use crate::boundary::ColumnIndex;
use crate::boundary::ConstructorArity;
use crate::boundary::ConstructorOwner;
use crate::boundary::CoreVariableName;
use crate::boundary::LeafCount;
use crate::boundary::MatrixRequired;
use crate::boundary::RowIndex;
use crate::boundary::SourceRange;
use crate::boundary::TypeName;
use crate::origin::ElabKind;
use crate::origin::OriginNode;
use crate::synnode::SynNode;

/// An index into [`TreeArena`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TreeId(usize);

/// A node of the compiled decision tree.
#[derive(Clone)]
enum TreeNode<'tree>
{
    /// Test one occurrence's constructor, one subtree per declared tag.
    Switch
    {
        /// The core variable holding the value under test.
        occurrence: String,
        /// The node the synthesized eliminator is attributed to.
        node: SynNode<'tree>,
        /// Per tag, in tag order: the names bound to that constructor's
        /// fields, and the subtree taken under it.
        arms: Vec<(Vec<String>, TreeId)>,
    },
    /// Reach one surface arm, supplying its parameters from occurrences.
    Leaf
    {
        /// Which arm of the source-ordered arm set.
        arm: ArmIndex,
        /// The occurrence holding each of the arm's parameters, in the arm's
        /// own parameter order.
        arguments: Vec<String>,
        /// The node the synthesized binders are attributed to.
        node: SynNode<'tree>,
    },
    /// Stop on an unfinished test: every decidable test above has passed, and
    /// this hole is what remains.
    Stuck(
        /// The hole the slot is stuck on.
        StuckPattern,
    ),
    /// No arm reaches this slot.
    Missing
    {
        /// The node the missing-arm hole is attributed to.
        node: SynNode<'tree>,
    },
}

/// The decision tree under construction.
///
/// # Contract
/// - ensures: an allocated node's index never moves, so a parent holds its
///   children's indices while later nodes are still being allocated.
#[repr(transparent)]
struct TreeArena<'tree>
{
    /// The allocated nodes, in allocation order.
    nodes: Vec<TreeNode<'tree>>,
}

impl<'tree> TreeArena<'tree>
{
    /// Allocates one node and returns its index.
    fn alloc(
        &mut self,
        node: TreeNode<'tree>,
    ) -> TreeId
    {
        let id = TreeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    /// The node at `id`, cloned so the caller may mutate the arena while
    /// reading it.
    fn get(
        &self,
        id: TreeId,
    ) -> Option<TreeNode<'tree>>
    {
        self.nodes.get(id.0).cloned()
    }
}

/// One surface arm, with what compilation needs to place its body.
struct MatrixArm<'tree>
{
    /// The arm node, for the synthesized nodes' origins.
    node: SynNode<'tree>,
    /// The binder names the body may read, in the order they are bound.
    parameters: Vec<String>,
    /// The lowered body, taken when it is emitted.
    body: Option<COut>,
    /// How many leaves of the finished tree reach this arm.
    leaves: LeafCount,
    /// The join label, minted when more than one leaf reaches the arm.
    label: Option<String>,
}

/// One row of the pattern matrix.
#[derive(Clone)]
struct Row<'tree>
{
    /// The remaining column patterns, one per live occurrence.
    columns: Vec<PatternPlan<'tree>>,
    /// The arm this row belongs to.
    arm: ArmIndex,
    /// Each binder name the row has settled, and the occurrence holding it.
    bindings: Vec<(String, String)>,
    /// The unfinished test this row has already met, if any.
    stuck: Option<StuckPattern>,
}

/// One step of the iterative tree construction.
enum BuildStep<'tree>
{
    /// Compile one sub-matrix.
    Compile
    {
        /// The occurrences the columns are matched against.
        occurrences: Vec<String>,
        /// The rows, in source order.
        rows: Vec<Row<'tree>>,
        /// The node an unreached slot beneath this matrix is attributed to.
        node: SynNode<'tree>,
    },
    /// Assemble a switch from the subtrees on top of the output stack.
    FinishSwitch
    {
        /// The tested occurrence.
        occurrence: String,
        /// The attribution node.
        node: SynNode<'tree>,
        /// Per tag, in tag order, the names bound to that constructor's
        /// fields.
        fields: Vec<Vec<String>>,
    },
}

/// One step of the iterative emission pass.
enum EmitStep
{
    /// Emit one tree node.
    Visit(TreeId),
    /// Assemble a `DataCase` from the arm bodies on top of the output stack.
    FinishSwitch(TreeId),
}

/// The compiler's state for one `case`.
struct Matrix<'tree>
{
    /// The arms, in source order.
    arms: Vec<MatrixArm<'tree>>,
    /// The decision tree.
    arena: TreeArena<'tree>,
    /// One stuck pattern per *written* hole, keyed by the hole's byte range,
    /// so a hole reached from several branches still reports one goal.
    stucks: Vec<(SourceRange, StuckPattern)>,
    /// The computation type a shared body is checked against.
    answer: CompType,
}

/// Every binder name a pattern binds, in the order compilation settles them.
///
/// The order is source pre-order — an as-binder before the pattern it names,
/// a constructor's arguments left to right — which is the order the
/// specialization walk settles them in, so a leaf's arguments line up with the
/// join point's parameters without either side sorting.
///
/// # Contract
/// - ensures: no name appears twice, and an or-pattern contributes only its
///   first alternative's names; a leaf reached through an alternative binding
///   less is declined rather than compiled against a missing parameter.
/// - panics: none.
///
/// # Intension
/// The walk carries an explicit stack: pattern nesting depth is
/// input-controlled.
fn binder_names(plan: &PatternPlan<'_>) -> Vec<String>
{
    let mut names: Vec<String> = Vec::new();
    let mut work: Vec<&PatternPlan<'_>> = alloc::vec![plan];
    while let Some(plan) = work.pop() {
        match *plan {
            | PatternPlan::Bind { ref name, .. } => {
                push_name(&mut names, CoreVariableName::from(name.as_str()));
            },
            | PatternPlan::As {
                ref pattern,
                ref binder,
                ..
            } => {
                push_name(&mut names, CoreVariableName::from(binder.as_str()));
                work.push(pattern);
            },
            | PatternPlan::Ctor { ref arguments, .. } => work.extend(arguments.iter().rev()),
            | PatternPlan::Or {
                ref alternatives, ..
            } => work.extend(alternatives.iter().take(1)),
            | PatternPlan::Hole { .. } | PatternPlan::Discard | PatternPlan::Declined { .. } => {},
        }
    }
    names
}

/// Appends `name` unless it is already present.
fn push_name(
    names: &mut Vec<String>,
    name: CoreVariableName<'_>,
)
{
    if !names.iter().any(|held| held.as_str() == name.as_ref()) {
        names.push(name.as_ref().to_owned());
    }
}

/// Reports every binder one arm's pattern introduces, once.
///
/// Recognition is told about binders here rather than during specialization,
/// because a binder beneath an or-pattern or a catch-all is settled once per
/// branch that reaches it and the report is about the *source*, not about the
/// branches.
///
/// # Contract
/// - ensures: one report per distinct binder name in the pattern.
/// - fails: propagates [`LowerError::ShadowedBuiltin`] under a refusing policy.
/// - panics: none.
///
/// # Intension
/// The walk carries an explicit stack.
fn report_binders(
    lowerer: &mut Lowerer<'_>,
    plan: &PatternPlan<'_>,
) -> LowerResult<()>
{
    let mut seen: Vec<String> = Vec::new();
    let mut work: Vec<&PatternPlan<'_>> = alloc::vec![plan];
    while let Some(plan) = work.pop() {
        match *plan {
            | PatternPlan::Bind { ref name, node } => {
                if !seen.iter().any(|held| held == name) {
                    seen.push(name.clone());
                    lowerer.note_binder(name.as_str().into(), node)?;
                }
            },
            | PatternPlan::As {
                ref pattern,
                ref binder,
                node,
            } => {
                if !seen.iter().any(|held| held == binder) {
                    seen.push(binder.clone());
                    lowerer.note_binder(binder.as_str().into(), node)?;
                }
                work.push(pattern);
            },
            | PatternPlan::Ctor { ref arguments, .. } => work.extend(arguments.iter().rev()),
            | PatternPlan::Or {
                ref alternatives, ..
            } => work.extend(alternatives.iter().rev()),
            | PatternPlan::Hole { .. } | PatternPlan::Discard | PatternPlan::Declined { .. } => {},
        }
    }
    Ok(())
}

/// Settles one row: peels every binding column and expands every or-column.
///
/// # What is semantics here and what is strategy
///
/// **Which scrutinees a column admits is the meaning of the match**, and it is
/// fixed here and nowhere else: a binder or wildcard column admits everything
/// and tests nothing, an as-binder names what its inner pattern tests, and an
/// or-column admits each alternative in source order. A later pass may not
/// read any of that as a choice point — changing it changes which scrutinee
/// reaches which body.
///
/// **Which column is tested first is strategy, and the strategy returns an
/// ORDER rather than a verdict.** It cannot decline a row, drop a tag, or stop
/// the walk — every column a row still holds is tested before any leaf is
/// reached, whatever order they are tested in — so changing it changes the
/// emitted tree's shape and size and not one scrutinee's outcome. That is why
/// relation-invariance under a different column choice is a property of the
/// shape rather than a rule someone has to keep: a strategy able to stop the
/// walk could turn a match into a miss, and then two strategies would compile
/// two different languages. [`Matrix::compile_step`] takes the first row's
/// leftmost constructor column, which is what makes every decidable test run
/// before an indeterminate one is reached.
///
/// # Contract
/// - requires: `occurrences` has one entry per column of every row.
/// - ensures: no surviving column is a binder, an as-pattern, or an or-pattern;
///   rows keep source order, with one row's alternatives contiguous and in the
///   alternatives' own order.
/// - panics: none.
///
/// # Intension
/// Iterative to a fixed point: peeling an as-binder can expose an or-pattern
/// and expanding an or-pattern can expose an as-binder.
fn settle<'tree>(
    occurrences: &[String],
    rows: Vec<Row<'tree>>,
) -> Vec<Row<'tree>>
{
    let mut pending = rows;
    loop {
        for row in &mut pending {
            for (position, column) in row.columns.iter_mut().enumerate() {
                let Some(occurrence) = occurrences.get(position)
                else {
                    continue;
                };
                loop {
                    match core::mem::replace(column, PatternPlan::Discard) {
                        | PatternPlan::Bind { name, .. } => {
                            row.bindings.push((name, occurrence.clone()));
                            break;
                        },
                        | PatternPlan::As {
                            pattern, binder, ..
                        } => {
                            row.bindings.push((binder, occurrence.clone()));
                            *column = *pattern;
                        },
                        // The catch-all is safe by construction rather than
                        // by inspection: this loop peels bindings and nothing
                        // else, so every other column form is meant to survive
                        // untouched. A pattern form added to the grammar is
                        // still forced through `specialize`, which matches
                        // every variant by name and would stop compiling.
                        | held => {
                            *column = held;
                            break;
                        },
                    }
                }
            }
        }
        let Some(split) = pending.iter().enumerate().find_map(|(index, row)| {
            row.columns
                .iter()
                .position(|column| matches!(*column, PatternPlan::Or { .. }))
                .map(|column| (RowIndex::from(index), ColumnIndex::from(column)))
        })
        else {
            return pending;
        };
        pending = expand_one(pending, split);
    }
}

/// Replaces the row at `index` with one row per alternative of its or-column.
fn expand_one<'tree>(
    rows: Vec<Row<'tree>>,
    split: (RowIndex, ColumnIndex),
) -> Vec<Row<'tree>>
{
    let (index, column) = (usize::from(split.0), usize::from(split.1));
    let mut expanded: Vec<Row<'tree>> = Vec::with_capacity(rows.len());
    for (position, row) in rows.into_iter().enumerate() {
        let alternatives = match row.columns.get(column) {
            | Some(&PatternPlan::Or {
                ref alternatives, ..
            }) if position == index => alternatives.clone(),
            | _ => {
                expanded.push(row);
                continue;
            },
        };
        for alternative in alternatives {
            let mut columns = row.columns.clone();
            if let Some(slot) = columns.get_mut(column) {
                *slot = alternative;
            }
            expanded.push(Row {
                columns,
                arm: row.arm,
                bindings: row.bindings.clone(),
                stuck: row.stuck.clone(),
            });
        }
    }
    expanded
}

impl<'tree> Matrix<'tree>
{
    /// Builds the compiler over one `case`'s arms and their lowered bodies.
    ///
    /// # Contract
    /// - requires: `arms` and `bodies` are the same length and in source order.
    /// - ensures: each arm's parameter list is its pattern's binder names in
    ///   the order specialization settles them.
    /// - fails: propagates the binder report under a refusing policy.
    /// - panics: none.
    fn new(
        lowerer: &mut Lowerer<'_>,
        arms: &[(SynNode<'tree>, PatternPlan<'tree>)],
        bodies: Vec<COut>,
        answer: CompType,
    ) -> LowerResult<Self>
    {
        let mut held: Vec<MatrixArm<'tree>> = Vec::with_capacity(arms.len());
        for (arm, body) in arms.iter().zip(bodies) {
            report_binders(lowerer, &arm.1)?;
            held.push(MatrixArm {
                node: arm.0,
                parameters: binder_names(&arm.1),
                body: Some(body),
                leaves: 0_usize.into(),
                label: None,
            });
        }
        Ok(Self {
            arms: held,
            arena: TreeArena { nodes: Vec::new() },
            stucks: Vec::new(),
            answer,
        })
    }

    /// The stuck pattern one written hole carries, minted once per hole.
    ///
    /// # Contract
    /// - ensures: two leaves stopped by one written `?` share one hole
    ///   identity, so a single unfinished test reports a single goal however
    ///   many branches reach it.
    /// - panics: none.
    fn stuck_for(
        &mut self,
        lowerer: &mut Lowerer<'_>,
        node: SynNode<'_>,
        name: Option<String>,
    ) -> StuckPattern
    {
        let key = node.byte_range();
        if let Some(held) = self.stucks.iter().find(|entry| entry.0 == key) {
            return held.1.clone();
        }
        let minted = lowerer.mint_stuck(node, name);
        self.stucks.push((key, minted.clone()));
        minted
    }

    /// Reads the pattern matrix into the decision tree.
    ///
    /// # Contract
    /// - ensures: every leaf of the returned tree is an arm, a stuck hole, or
    ///   an unreached slot, and every arm's leaf count is the number of leaves
    ///   that reach it.
    /// - fails: [`LowerError::Unsupported`] naming the first column form the
    ///   matrix does not switch on, an unresolvable constructor, an arity
    ///   mismatch, a cross-datatype column, or an or-alternative that binds
    ///   less than its arm's parameters.
    /// - panics: none.
    ///
    /// # Intension
    /// One work stack drives the whole construction, so column count and
    /// pattern depth cost heap and never host stack.
    fn build(
        &mut self,
        lowerer: &mut Lowerer<'_>,
        node: SynNode<'tree>,
        occurrence: String,
        rows: Vec<Row<'tree>>,
    ) -> LowerResult<TreeId>
    {
        let mut work: Vec<BuildStep<'tree>> = alloc::vec![BuildStep::Compile {
            occurrences: alloc::vec![occurrence],
            rows,
            node,
        }];
        let mut built: Vec<TreeId> = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                | BuildStep::Compile {
                    occurrences,
                    rows,
                    node,
                } => self.compile_step(
                    lowerer,
                    occurrences.as_slice(),
                    rows,
                    node,
                    &mut work,
                    &mut built,
                )?,
                | BuildStep::FinishSwitch {
                    occurrence,
                    node,
                    fields,
                } => {
                    let taken = built.len().saturating_sub(fields.len());
                    let subtrees: Vec<TreeId> = built.split_off(taken);
                    let arms: Vec<(Vec<String>, TreeId)> =
                        fields.into_iter().zip(subtrees).collect();
                    let id = self.arena.alloc(TreeNode::Switch {
                        occurrence,
                        node,
                        arms,
                    });
                    built.push(id);
                },
            }
        }
        built.pop().ok_or_else(|| LowerError::Unsupported {
            kind: node.kind(),
            byte_range: node.byte_range(),
        })
    }

    /// Compiles one sub-matrix: a leaf, a stuck slot, an unreached slot, or a
    /// switch whose subtrees are queued.
    ///
    /// # Contract
    /// - ensures: rows after the first irrefutable row are dropped, because no
    ///   scrutinee reaches them.
    /// - fails: as [`Self::build`].
    /// - panics: none.
    fn compile_step(
        &mut self,
        lowerer: &mut Lowerer<'_>,
        occurrences: &[String],
        rows: Vec<Row<'tree>>,
        node: SynNode<'tree>,
        work: &mut Vec<BuildStep<'tree>>,
        built: &mut Vec<TreeId>,
    ) -> LowerResult<()>
    {
        let mut rows = settle(occurrences, rows);
        truncate_after_irrefutable(&mut rows);
        let Some(first) = rows.first()
        else {
            let id = self.arena.alloc(TreeNode::Missing { node });
            built.push(id);
            return Ok(());
        };
        if let Some(column) = first
            .columns
            .iter()
            .position(|column| matches!(*column, PatternPlan::Ctor { .. }))
        {
            return self.queue_switch(
                lowerer,
                occurrences,
                rows.as_slice(),
                ColumnIndex::from(column),
                node,
                work,
            );
        }
        let leaf = self.settle_leaf(lowerer, rows.as_slice(), node)?;
        built.push(leaf);
        Ok(())
    }

    /// The tree node one irrefutable-or-stuck row reaches.
    ///
    /// # Contract
    /// - ensures: a row that met an unfinished test reaches [`TreeNode::Stuck`]
    ///   after every decidable test above it has passed, never before; a row
    ///   that settled every column reaches its arm.
    /// - fails: [`LowerError::Unsupported`] for a column form outside the
    ///   switched set, or for a row missing one of its arm's parameters.
    /// - panics: none.
    fn settle_leaf(
        &mut self,
        lowerer: &mut Lowerer<'_>,
        rows: &[Row<'tree>],
        node: SynNode<'tree>,
    ) -> LowerResult<TreeId>
    {
        let Some(first) = rows.first()
        else {
            return Ok(self.arena.alloc(TreeNode::Missing { node }));
        };
        for column in &first.columns {
            if let PatternPlan::Declined {
                kind,
                ref byte_range,
            } = *column
            {
                return Err(ArmVerdict::declined_error(kind, byte_range.clone()));
            }
        }
        let hole = first.columns.iter().find_map(|column| match *column {
            | PatternPlan::Hole { node, ref name } => Some((node, name.clone())),
            | _ => None,
        });
        if let Some(stuck) = first.stuck.clone() {
            return Ok(self.arena.alloc(TreeNode::Stuck(stuck)));
        }
        if let Some((hole_node, name)) = hole {
            let stuck = self.stuck_for(lowerer, hole_node, name);
            return Ok(self.arena.alloc(TreeNode::Stuck(stuck)));
        }
        let arm = first.arm;
        let Some(held) = self.arms.get_mut(usize::from(arm))
        else {
            return Err(LowerError::Unsupported {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        };
        let mut arguments: Vec<String> = Vec::with_capacity(held.parameters.len());
        for parameter in &held.parameters {
            let Some(binding) = first
                .bindings
                .iter()
                .find(|binding| binding.0 == *parameter)
            else {
                return Err(LowerError::Unsupported {
                    kind: held.node.kind(),
                    byte_range: held.node.byte_range(),
                });
            };
            arguments.push(binding.1.clone());
        }
        held.leaves = usize::from(held.leaves).saturating_add(1).into();
        let arm_node = held.node;
        Ok(self.arena.alloc(TreeNode::Leaf {
            arm,
            arguments,
            node: arm_node,
        }))
    }
}

/// Drops every row after the first that tests nothing.
///
/// A row all of whose columns are wildcards matches every scrutinee that
/// reaches it, so no later row is reachable. Keeping them would compile
/// branches nothing can take.
fn truncate_after_irrefutable(rows: &mut Vec<Row<'_>>)
{
    let irrefutable = rows.iter().position(|row| {
        row.columns
            .iter()
            .all(|column| matches!(*column, PatternPlan::Discard))
    });
    if let Some(position) = irrefutable {
        rows.truncate(position.saturating_add(1));
    }
}

impl<'tree> Matrix<'tree>
{
    /// Queues one constructor test and the sub-matrix under every tag.
    ///
    /// # Contract
    /// - ensures: one subtree per declared constructor of the column's
    ///   datatype, in tag order, each over the occurrences the tested column's
    ///   fields introduce.
    /// - fails: [`LowerError::Unsupported`] for an unresolvable constructor, an
    ///   arity mismatch, a cross-datatype column, or an unknown datatype.
    /// - panics: none.
    fn queue_switch(
        &mut self,
        lowerer: &mut Lowerer<'_>,
        occurrences: &[String],
        rows: &[Row<'tree>],
        column: ColumnIndex,
        node: SynNode<'tree>,
        work: &mut Vec<BuildStep<'tree>>,
    ) -> LowerResult<()>
    {
        let index = usize::from(column);
        let Some(occurrence) = occurrences.get(index).cloned()
        else {
            return Err(LowerError::Unsupported {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        };
        let data = Self::column_datatype(lowerer, rows, column, node)?;
        let count = usize::from(lowerer.constructor_count(TypeName::from(data.as_str())));
        if count == 0 {
            return Err(LowerError::Unsupported {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        }
        let mut fields: Vec<Vec<String>> = Vec::with_capacity(count);
        let mut plans: Vec<(Vec<String>, Vec<Row<'tree>>)> = Vec::with_capacity(count);
        for slot in 0 .. count {
            let tag = ConstructorTag::from(slot);
            let arity = usize::from(lowerer.field_arity(DataTypeName::from(data.as_str()), tag));
            let mut names: Vec<String> = Vec::with_capacity(arity);
            names.resize_with(arity, || lowerer.fresh_name());
            let specialized = self.specialize(
                lowerer,
                rows,
                column,
                ConstructorOwner::from(data.as_str()),
                tag,
                ConstructorArity::from(arity),
            )?;
            fields.push(names.clone());
            plans.push((names, specialized));
        }
        work.push(BuildStep::FinishSwitch {
            occurrence,
            node,
            fields,
        });
        // Pushed in reverse tag order so the subtrees pop, and so land on the
        // output stack, in tag order.
        for (names, specialized) in plans.into_iter().rev() {
            let mut sub: Vec<String> =
                Vec::with_capacity(occurrences.len().saturating_add(names.len()));
            sub.extend(occurrences.iter().take(index).cloned());
            sub.extend(names);
            sub.extend(occurrences.iter().skip(index.saturating_add(1)).cloned());
            work.push(BuildStep::Compile {
                occurrences: sub,
                rows: specialized,
                node,
            });
        }
        Ok(())
    }

    /// The datatype a column switches on: the datatype of the first
    /// constructor appearing in it.
    ///
    /// # Contract
    /// - ensures: every constructor in the column belongs to the returned
    ///   datatype.
    /// - fails: [`LowerError::Unsupported`] for an unresolvable constructor or
    ///   a column mixing two datatypes.
    /// - panics: none.
    fn column_datatype(
        lowerer: &Lowerer<'_>,
        rows: &[Row<'tree>],
        column: ColumnIndex,
        node: SynNode<'tree>,
    ) -> LowerResult<String>
    {
        let mut owner: Option<String> = None;
        for row in rows {
            let Some(&PatternPlan::Ctor {
                ref name,
                node: ctor_node,
                ..
            }) = row.columns.get(usize::from(column))
            else {
                continue;
            };
            let Some(resolved) = lowerer.constructors.get(name)
            else {
                return Err(ArmVerdict::declined_error(
                    ctor_node.kind(),
                    ctor_node.byte_range(),
                ));
            };
            let data = &resolved.0;
            match owner {
                | Some(ref held) if held != data => {
                    return Err(ArmVerdict::declined_error(
                        ctor_node.kind(),
                        ctor_node.byte_range(),
                    ));
                },
                | Some(_) => {},
                | None => owner = Some(data.clone()),
            }
        }
        owner.ok_or_else(|| LowerError::Unsupported {
            kind: node.kind(),
            byte_range: node.byte_range(),
        })
    }

    /// The rows that survive a constructor test at `tag`.
    ///
    /// **This is the meaning of the match rather than a strategy for finding
    /// it.** A constructor column admits its own tag alone and contributes its
    /// arguments as new columns; a wildcard column admits every tag and
    /// contributes wildcards; a hole column admits every tag *indeterminately*
    /// — it neither matches nor refutes — so the row survives carrying the
    /// hole, which is what stops the arms the hole shadows. A later pass may
    /// not read any of the three as a choice point.
    ///
    /// # Contract
    /// - ensures: the surviving rows keep source order, and each has `arity`
    ///   columns spliced in where the tested column was.
    /// - fails: [`LowerError::Unsupported`] for an unresolvable constructor, an
    ///   arity mismatch, or a column form outside the switched set.
    /// - panics: none.
    fn specialize(
        &mut self,
        lowerer: &mut Lowerer<'_>,
        rows: &[Row<'tree>],
        column: ColumnIndex,
        data: ConstructorOwner<'_>,
        tag: ConstructorTag,
        arity: ConstructorArity,
    ) -> LowerResult<Vec<Row<'tree>>>
    {
        let index = usize::from(column);
        let arity = usize::from(arity);
        let mut survivors: Vec<Row<'tree>> = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(tested) = row.columns.get(index)
            else {
                continue;
            };
            let (replacement, stuck) = match *tested {
                | PatternPlan::Discard => {
                    (wildcards(ConstructorArity::from(arity)), row.stuck.clone())
                },
                | PatternPlan::Hole { node, ref name } => {
                    let held = match row.stuck {
                        | Some(ref stuck) => stuck.clone(),
                        | None => self.stuck_for(lowerer, node, name.clone()),
                    };
                    (wildcards(ConstructorArity::from(arity)), Some(held))
                },
                | PatternPlan::Ctor {
                    ref name,
                    node,
                    ref arguments,
                } => {
                    let Some(&(ref owner, slot)) = lowerer.constructors.get(name)
                    else {
                        return Err(ArmVerdict::declined_error(node.kind(), node.byte_range()));
                    };
                    if owner.as_str() != data.as_ref() {
                        return Err(ArmVerdict::declined_error(node.kind(), node.byte_range()));
                    }
                    if ConstructorTag::from(slot) != tag {
                        continue;
                    }
                    if arguments.len() != arity {
                        return Err(ArmVerdict::declined_error(node.kind(), node.byte_range()));
                    }
                    (arguments.clone(), row.stuck.clone())
                },
                | PatternPlan::Declined {
                    kind,
                    ref byte_range,
                } => {
                    return Err(ArmVerdict::declined_error(kind, byte_range.clone()));
                },
                | PatternPlan::Bind { node, .. }
                | PatternPlan::As { node, .. }
                | PatternPlan::Or { node, .. } => {
                    return Err(ArmVerdict::declined_error(node.kind(), node.byte_range()));
                },
            };
            let mut columns: Vec<PatternPlan<'tree>> =
                Vec::with_capacity(row.columns.len().saturating_add(arity));
            columns.extend(row.columns.iter().take(index).cloned());
            columns.extend(replacement);
            columns.extend(row.columns.iter().skip(index.saturating_add(1)).cloned());
            survivors.push(Row {
                columns,
                arm: row.arm,
                bindings: row.bindings.clone(),
                stuck,
            });
        }
        Ok(survivors)
    }
}

/// `count` wildcard columns.
fn wildcards<'tree>(count: ConstructorArity) -> Vec<PatternPlan<'tree>>
{
    let count = usize::from(count);
    let mut columns: Vec<PatternPlan<'tree>> = Vec::with_capacity(count);
    columns.resize_with(count, || PatternPlan::Discard);
    columns
}

impl<'tree> Matrix<'tree>
{
    /// Mints a join label for every arm more than one leaf reaches.
    ///
    /// # Contract
    /// - ensures: an arm has a label exactly when its body will be bound rather
    ///   than inlined.
    /// - panics: none.
    fn label_shared(
        &mut self,
        lowerer: &mut Lowerer<'_>,
    )
    {
        for arm in &mut self.arms {
            if usize::from(arm.leaves) > 1 {
                arm.label = Some(lowerer.fresh_name());
            }
        }
    }

    /// Emits the decision tree as core, then binds the shared arm bodies
    /// around it.
    ///
    /// # Contract
    /// - ensures: the returned computation runs exactly the tests the tree
    ///   holds, in the tree's own order, and reaches each arm's body through
    ///   its parameters.
    /// - fails: propagates readback, hole-construction, and arm-placement
    ///   failures.
    /// - panics: none.
    ///
    /// # Intension
    /// One work stack drives emission, so tree depth costs heap and never host
    /// stack.
    fn emit(
        &mut self,
        lowerer: &mut Lowerer<'_>,
        node: SynNode<'tree>,
        root: TreeId,
    ) -> LowerResult<COut>
    {
        self.label_shared(lowerer);
        let mut work: Vec<EmitStep> = alloc::vec![EmitStep::Visit(root)];
        let mut built: Vec<COut> = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                | EmitStep::Visit(id) => {
                    let Some(node) = self.arena.get(id)
                    else {
                        return Err(LowerError::Unsupported {
                            kind: node.kind(),
                            byte_range: node.byte_range(),
                        });
                    };
                    match node {
                        | TreeNode::Switch { ref arms, .. } => {
                            work.push(EmitStep::FinishSwitch(id));
                            for &(_, child) in arms.iter().rev() {
                                work.push(EmitStep::Visit(child));
                            }
                        },
                        | TreeNode::Leaf {
                            arm,
                            ref arguments,
                            node: leaf_node,
                        } => {
                            let leaf = self.emit_leaf(arm, arguments.as_slice(), leaf_node)?;
                            built.push(leaf);
                        },
                        | TreeNode::Stuck(ref stuck) => built.push(Lowerer::stuck_slot(stuck)?),
                        | TreeNode::Missing { node: missing_node } => {
                            let (_, hole) = lowerer.missing_arm(missing_node)?;
                            built.push(hole);
                        },
                    }
                },
                | EmitStep::FinishSwitch(id) => {
                    let Some(TreeNode::Switch {
                        occurrence,
                        node,
                        arms,
                    }) = self.arena.get(id)
                    else {
                        return Err(LowerError::Unsupported {
                            kind: node.kind(),
                            byte_range: node.byte_range(),
                        });
                    };
                    let taken = built.len().saturating_sub(arms.len());
                    let bodies: Vec<COut> = built.split_off(taken);
                    let switch = Self::emit_switch(
                        lowerer,
                        CoreVariableName::from(occurrence.as_str()),
                        node,
                        &arms,
                        bodies,
                    )?;
                    built.push(switch);
                },
            }
        }
        let Some(tree) = built.pop()
        else {
            return Err(LowerError::Unsupported {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        };
        self.bind_joins(tree)
    }

    /// Emits one constructor test over the already-emitted tag bodies.
    fn emit_switch(
        lowerer: &mut Lowerer<'_>,
        occurrence: CoreVariableName<'_>,
        node: SynNode<'_>,
        arms: &[(Vec<String>, TreeId)],
        bodies: Vec<COut>,
    ) -> LowerResult<COut>
    {
        let elab = Some(ElabKind::PatternNest);
        let mut placed: Vec<(String, Rc<Comp>)> = Vec::with_capacity(arms.len());
        let mut origins: Vec<OriginNode> = alloc::vec![OriginNode::leaf(entry(node, elab))];
        for (arm, body) in arms.iter().zip(bodies) {
            let (binder, wrapped) = lowerer.bind_payload(node, arm.0.as_slice(), body)?;
            placed.push((
                binder,
                Rc::new({
                    let readback_comp = wrapped.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            ));
            origins.push(wrapped.origin);
        }
        COut::from_legacy_comp(
            &Comp::DataCase(Rc::new(Value::var(occurrence.as_ref())), placed),
            OriginNode::new(entry(node, elab), origins),
        )
    }

    /// Emits one leaf: the arm's body inlined, or a jump to its join point.
    ///
    /// # Contract
    /// - ensures: an arm exactly one leaf reaches is inlined with its
    ///   parameters bound to the occurrences that settled them; an arm several
    ///   leaves reach becomes a forced application of its join label.
    /// - fails: propagates readback failures, and reports an arm whose body was
    ///   already taken.
    /// - panics: none.
    fn emit_leaf(
        &mut self,
        arm: ArmIndex,
        arguments: &[String],
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let elab = Some(ElabKind::PatternJoin);
        let Some(held) = self.arms.get_mut(usize::from(arm))
        else {
            return Err(LowerError::Unsupported {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        };
        if let Some(ref label) = held.label {
            let mut acc = COut::from_legacy_comp(
                &Comp::Force(Rc::new(Value::var(label.as_str()))),
                OriginNode::new(entry(node, elab), alloc::vec![OriginNode::leaf(entry(
                    node, elab
                ))]),
            )?;
            for argument in arguments {
                acc = COut::from_legacy_comp(
                    &Comp::App(
                        Rc::new({
                            let readback_comp = acc.readback_comp()?;
                            core::convert::identity(readback_comp)
                        }),
                        Rc::new(Value::var(argument.as_str())),
                    ),
                    OriginNode::new(entry(node, elab), alloc::vec![
                        acc.origin,
                        OriginNode::leaf(entry(node, elab))
                    ]),
                )?;
            }
            return Ok(acc);
        }
        let Some(body) = held.body.take()
        else {
            return Err(LowerError::Unsupported {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        };
        let mut acc = body;
        for (parameter, argument) in held.parameters.iter().zip(arguments).rev() {
            acc = Lowerer::alias_value(
                node,
                CoreVariableName::from(argument.as_str()),
                parameter.clone(),
                acc,
            )?;
        }
        Ok(acc)
    }

    /// Binds every shared arm body as a join point around `body`.
    ///
    /// # Contract
    /// - ensures: one binding per arm more than one leaf reaches, outermost for
    ///   the earliest arm, each `run %j <- ret ((thunk …) : U_ω τ)` with `τ`
    ///   the parameters' arrows onto the match's answer type.
    /// - fails: propagates readback failures.
    /// - panics: none.
    fn bind_joins(
        &mut self,
        body: COut,
    ) -> LowerResult<COut>
    {
        let elab = Some(ElabKind::PatternJoin);
        let answer = self.answer.clone();
        let mut acc = body;
        for arm in self.arms.iter_mut().rev() {
            let Some(ref label) = arm.label
            else {
                continue;
            };
            let Some(arm_body) = arm.body.take()
            else {
                continue;
            };
            let node = arm.node;
            let mut comp = arm_body.readback_comp()?;
            let mut origin = arm_body.origin;
            let mut ty = answer.clone();
            for parameter in arm.parameters.iter().rev() {
                comp = Comp::Abs(parameter.clone(), None, Rc::new(comp));
                origin = OriginNode::new(entry(node, elab), alloc::vec![origin]);
                ty = CompType::arrow(ValueType::Unknown, ty);
            }
            let annotated = Value::annot(
                Value::thunk(Grade::OMEGA, comp),
                ValueType::thunk(Grade::OMEGA, ty),
            );
            let thunk = OriginNode::new(entry(node, elab), alloc::vec![origin]);
            let annot = OriginNode::new(entry(node, elab), alloc::vec![thunk]);
            let returned = OriginNode::new(entry(node, elab), alloc::vec![annot]);
            let bound = Comp::Bind(
                Rc::new(Comp::Ret(Rc::new(annotated))),
                label.clone(),
                Rc::new({
                    let readback_comp = acc.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            );
            acc = COut::from_legacy_comp(
                &bound,
                OriginNode::new(entry(node, elab), alloc::vec![returned, acc.origin]),
            )?;
        }
        Ok(acc)
    }
}

impl Lowerer<'_>
{
    /// Whether a `case`'s arm set needs the matrix compiler, and whether this
    /// compiler can take it.
    ///
    /// Both questions are answered here because the answer is one decision:
    /// the matrix path is taken exactly when the tag walk would decline *and*
    /// every column is a form the matrix switches on. An arm set the matrix
    /// cannot take is left to the tag walk, which declines it by name — so a
    /// form outside both stays declined exactly as it was.
    ///
    /// # Contract
    /// - ensures: false for every arm set the tag walk already compiles, so no
    ///   term that lowered before this module existed changes shape.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the three triggers (a catch-all, an or-pattern with a
    ///   constructor alternative, two arms at one head) and the supported-form
    ///   gate are independent decision surfaces, so a witness per trigger plus
    ///   one unsupported-column witness separates them. A trigger dropped would
    ///   send its arm set back to the tag walk and restore the decline the
    ///   witness asserts is gone.
    /// - witness: `matrix::tests::a_catch_all_reaches_every_tag_no_arm_named`
    /// - witness: `matrix::tests::an_or_pattern_of_heads_settles_both_tags`
    /// - witness: `matrix::tests::two_arms_at_one_head_are_told_apart_by_their_arguments`
    /// - witness: `matrix::tests::a_literal_column_is_still_declined_by_name`
    pub(super) fn arm_set_needs_matrix(
        &self,
        node: SynNode<'_>,
    ) -> MatrixRequired
    {
        // The matrix is a strict extension of the DECLARED-DATA eliminator and
        // claims nothing the other two dispatch to. This is the meaning of the
        // match rather than a scheduling preference: an arm set naming no
        // declared constructor names no constructor family at all, so reading
        // its lowercase arms as binders would turn another eliminator's
        // constructor — `here` on an identity type, say — into a variable and
        // accept a program that eliminator must refuse. The K-derivation
        // corpus witness is what catches that, and it is why this predicate
        // asks for a declared constructor first.
        if !bool::from(self.case_arms_are_data(node)) {
            return false.into();
        }
        let plans: Vec<PatternPlan<'_>> = Self::case_arm_patterns(node);
        if plans.is_empty()
            || !plans
                .iter()
                .all(|plan| bool::from(self.matrix_can_compile(plan)))
        {
            return false.into();
        }
        let mut claimed: Vec<(String, usize)> = Vec::new();
        let mut needed = false;
        for plan in &plans {
            match *peel_binders(plan) {
                | PatternPlan::Discard | PatternPlan::Bind { .. } => needed = true,
                | PatternPlan::Or {
                    ref alternatives, ..
                } => {
                    if alternatives.iter().any(|alternative| {
                        matches!(*peel_binders(alternative), PatternPlan::Ctor { .. })
                    }) {
                        needed = true;
                    }
                },
                | PatternPlan::Ctor { ref name, .. } => {
                    let Some(resolved) = self.constructors.get(name)
                    else {
                        continue;
                    };
                    if claimed.contains(resolved) {
                        needed = true;
                    }
                    else {
                        claimed.push(resolved.clone());
                    }
                },
                | PatternPlan::Hole { .. }
                | PatternPlan::As { .. }
                | PatternPlan::Declined { .. } => {},
            }
        }
        needed.into()
    }

    /// Every arm pattern of a `case`, read into the compiler's view, in source
    /// order.
    fn case_arm_patterns(node: SynNode<'_>) -> Vec<PatternPlan<'_>>
    {
        Self::case_arm_nodes(node)
            .into_iter()
            .filter_map(|arm| arm.child_by_field_name(node_kinds::FIELD_PATTERN))
            .map(super::pattern::read_pattern)
            .collect()
    }

    /// Every `arm` child of a `case`, in source order.
    fn case_arm_nodes(node: SynNode<'_>) -> Vec<SynNode<'_>>
    {
        super::named_non_extra_children(node)
            .into_iter()
            .filter(|arm| arm.kind() == node_kinds::ARM)
            .collect()
    }

    /// Whether every column of one pattern is a form the matrix switches on.
    ///
    /// # Intension
    /// The walk carries an explicit stack.
    fn matrix_can_compile(
        &self,
        plan: &PatternPlan<'_>,
    ) -> MatrixRequired
    {
        let mut work: Vec<&PatternPlan<'_>> = alloc::vec![plan];
        while let Some(plan) = work.pop() {
            match *plan {
                | PatternPlan::Discard | PatternPlan::Bind { .. } | PatternPlan::Hole { .. } => {},
                | PatternPlan::As { ref pattern, .. } => work.push(pattern),
                | PatternPlan::Declined { .. } => return false.into(),
                | PatternPlan::Ctor {
                    ref name,
                    ref arguments,
                    ..
                } => {
                    let Some(&(ref data, tag)) = self.constructors.get(name)
                    else {
                        return false.into();
                    };
                    let arity =
                        usize::from(self.field_arity(
                            DataTypeName::from(data.as_str()),
                            ConstructorTag::from(tag),
                        ));
                    if arguments.len() != arity {
                        return false.into();
                    }
                    work.extend(arguments.iter());
                },
                | PatternPlan::Or {
                    ref alternatives, ..
                } => {
                    let Some(first) = alternatives.first()
                    else {
                        return false.into();
                    };
                    let expected = binder_names(first);
                    for alternative in alternatives {
                        let held = binder_names(alternative);
                        if held.len() != expected.len()
                            || !expected.iter().all(|name| held.contains(name))
                        {
                            return false.into();
                        }
                        work.push(alternative);
                    }
                },
            }
        }
        true.into()
    }

    /// Compiles a `case` through the pattern matrix.
    ///
    /// # Contract
    /// - ensures: the scrutinee is evaluated once, bound to one occurrence, and
    ///   every test reads that occurrence or an occurrence derived from it.
    /// - ensures: each arm's body is lowered exactly once, in source order, so
    ///   the hole identities a match reports are the holes its author wrote.
    /// - fails: [`LowerError::Unsupported`] naming a column form the matrix
    ///   does not switch on; propagates scrutinee, body, and readback failures.
    /// - panics: none.
    /// # Termination
    /// - reason: each arm lowers only its proper pattern and body descendants.
    /// - measure: remaining source nodes beneath the case expression.
    /// - boundedness: the parsed CST is finite.
    /// - input recursion: none.
    pub(super) fn matrix_case(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let mut hoists = Vec::new();
        let scrut_node = super::required_field(node, node_kinds::FIELD_VALUE)?;
        let scrut = self.value_expr(scrut_node, &mut hoists)?;
        let scrut_value = scrut.readback_value()?;
        self.record_match_site(node, &scrut_value);

        let arm_nodes = Self::case_arm_nodes(node);
        let mut arms: Vec<(SynNode<'_>, PatternPlan<'_>)> = Vec::with_capacity(arm_nodes.len());
        for arm_node in arm_nodes {
            let pattern = super::required_field(arm_node, node_kinds::FIELD_PATTERN)?;
            arms.push((arm_node, super::pattern::read_pattern(pattern)));
        }
        let mut bodies: Vec<COut> = Vec::with_capacity(arms.len());
        for arm in &arms {
            let body_node = super::required_field(arm.0, node_kinds::FIELD_BODY)?;
            bodies.push(self.comp_expr(body_node)?);
        }
        let answer = self
            .computation_annotation(node, node_kinds::FIELD_ANSWER, |byte_range| {
                LowerError::EliminatorAnswerNotComputation { byte_range }
            })?
            .unwrap_or(CompType::Unknown);

        let mut matrix = Matrix::new(self, arms.as_slice(), bodies, answer)?;
        let occurrence = self.fresh_name();
        let rows: Vec<Row<'_>> = arms
            .iter()
            .enumerate()
            .map(|(index, arm)| Row {
                columns: alloc::vec![arm.1.clone()],
                arm: ArmIndex::from(index),
                bindings: Vec::new(),
                stuck: None,
            })
            .collect();
        let root = matrix.build(self, node, occurrence.clone(), rows)?;
        let tree = matrix.emit(self, node, root)?;

        let elab = Some(ElabKind::PatternScrutinee);
        let bound = Comp::Bind(
            Rc::new(Comp::Ret(Rc::new(scrut_value))),
            occurrence,
            Rc::new({
                let readback_comp = tree.readback_comp()?;
                core::convert::identity(readback_comp)
            }),
        );
        let body = COut::from_legacy_comp(
            &bound,
            OriginNode::new(entry(node, elab), alloc::vec![
                OriginNode::new(entry(node, elab), alloc::vec![scrut.origin]),
                tree.origin
            ]),
        )?;
        Self::wrap_hoists(hoists, body, node)
    }
}

/// The pattern beneath every as-binder.
fn peel_binders<'plan, 'tree>(plan: &'plan PatternPlan<'tree>) -> &'plan PatternPlan<'tree>
{
    let mut current = plan;
    while let PatternPlan::As { ref pattern, .. } = *current {
        current = pattern;
    }
    current
}
