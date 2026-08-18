//! The **reduction order** completion orients critical pairs by: a size
//! comparison guarded by hole occurrence, with a lexicographic path order
//! deciding the ties.
//!
//! `gandr_theory_coherent_resolutions::completion` turns a divergent critical
//! pair into a derived cell by putting the larger side on the left, and takes
//! [`core::cmp::Ordering::Equal`] as an honest obstruction rather than guessing
//! an orientation. What counts as larger is this module's whole subject.
//!
//! # Why size alone leaves an obstruction, and why it also needs a guard
//!
//! A pure node-count order cannot orient a pair whose two sides have the same
//! size, which is the common shape at a cut: both faces of a rule are one cut
//! with a producer and a consumer half, so rearranging where the work sits
//! moves nodes around without adding any. Every such pair was left unoriented.
//!
//! Size also is not, by itself, **stable under substitution**, and a reduction
//! order must be: if `l ≻ r` then `lσ ≻ rσ` for every `σ`, or orienting `l → r`
//! proves nothing about the instances that actually rewrite. A side that is
//! smaller today grows faster under substitution when it repeats a hole the
//! other side does not — `f(x, a, a)` outsizes `g(x, x)` until `x` is
//! instantiated by anything of size three. So the size comparison is admitted
//! only when the larger side **dominates** the smaller one hole by hole, which
//! is exactly the condition under which substitution cannot reverse it. Cell
//! patterns are linear on the left ([`crate::linearity`]) but not on the right,
//! so the shape this guard excludes is reachable from a written rule.
//!
//! # The path order that decides the ties
//!
//! The tie-break is the standard lexicographic path order over the pattern
//! grammar's erased [`Node`] view, taken with respect to a total, well-founded
//! precedence on head symbols. It is a simplification order, hence stable,
//! monotone under contexts, and well-founded — the three properties the size
//! comparison needs a guard to keep.
//!
//! The precedence is `cut > K⁻ > f > K > ★`, and the middle of it is chosen
//! rather than arbitrary. Ranking a **return-side constructor frame above an
//! operation frame** is what orients the design's own worked fusion cell in the
//! direction it is written:
//!
//! ```text
//! ⟨v | Succ⁻(add(n; α))⟩  ~>  ⟨v | add(n; Succ⁻(α))⟩
//! ```
//!
//! That is deforestation — the intermediate `Succ` allocation is gone — and it
//! is precisely a same-size pair, so the size comparison left it unoriented and
//! the completion loop could not synthesize it. Under the ranking above the
//! `K⁻`-headed side is the larger one, so the derived cell comes out pushing
//! the constructor frame inward. The opposite ranking orients it backwards,
//! which is why the choice is stated here rather than left to taste.
//!
//! The remaining tiers are ordinary: a cut is the frame every other head sits
//! inside, and a constructor is the value form nothing rewrites away. Within
//! one head kind the precedence is the symbol's own order, then the arity; a
//! cut orders by polarity. Those two are arbitrary but **total and
//! deterministic**, which is all a path order asks of a precedence, and
//! determinism is what keeps the engine's orientations reproducible.
//!
//! # What this order does not orient, and why that is not a defect
//!
//! A path order is a simplification order, so it cannot orient a rule whose
//! right-hand side buries the left's hole under new structure. The
//! frame-defining cell ([`crate::sequent::frame_defining_cell`]) is exactly
//! that shape — `⟨v | K⁻(β)⟩ ~> ⟨K(v) | β⟩` puts `v` under `K` — so no
//! precedence orients it in the shipped direction. That costs nothing here: a
//! polarity-derived cell's orientation is fixed by the calculus and never
//! passes through this order, which orients **critical pairs** and nothing
//! else. What it does mean is that this order alone is not a termination proof
//! for a store containing polarity-derived cells, and it is not offered as one.
//!
//! # Implementation shape
//!
//! Both terms are flattened into post-order node tables, so every node's
//! children carry strictly smaller indices. The order relation is then filled
//! by two nested loops over `(left index, right index)` in increasing order,
//! because a path order at one pair reads only pairs with a smaller left index,
//! a smaller right index, or both. There is no recursion at all, which is what
//! the interpreter's depth policy asks for ([`crate::pattern`] carries the same
//! discipline for its own traversals).

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cmp::Ordering;

use gandr_core_sequent::il::Polarity;

use crate::pattern::CmdPat;
use crate::pattern::ConsPat;
use crate::pattern::MetaVar;
use crate::pattern::Node;
use crate::pattern::ProdPat;
use crate::pattern::Sym;
use crate::pattern::cmd_size;
use crate::pattern::collect_cmd_metavars;

/// The **reduction order** — the orientation
/// `gandr_theory_coherent_resolutions::completion` reads.
///
/// # Contract
/// - ensures: [`Ordering::Greater`] or [`Ordering::Less`] only for a pair the
///   order can orient **stably** — the named side is larger by node count and
///   carries every hole of the other at least as often, or the two are equal by
///   node count with equal hole counts and separated by the lexicographic path
///   order. [`Ordering::Equal`] everywhere else, which
///   `gandr_theory_coherent_resolutions::completion` reads as an honest
///   obstruction.
/// - provides: a well-founded, substitution-stable, context-monotone
///   orientation of critical pairs.
/// - panics: none.
/// - intension: the path order is consulted only on a size tie, so every
///   orientation the size comparison already decided is unchanged.
///
/// # Adequacy
/// - hypothesis: L3 — the three decision surfaces are separated pointwise: a
///   size-decided pair with hole domination, an equal-size pair the path order
///   orients, and the two obstruction routes (a size difference the hole counts
///   do not license, and an equal-size pair the path order cannot separate).
/// - witness: `order::tests::a_size_difference_orients_when_the_larger_side_dominates`
/// - witness: `order::tests::a_size_difference_that_substitution_could_reverse_is_an_obstruction`
/// - witness: `order::tests::an_equal_size_pair_is_oriented_by_the_path_order`
/// - witness: `order::tests::an_equal_size_pair_the_path_order_cannot_separate_stays_an_obstruction`
/// - witness: `order::tests::the_order_is_a_strict_order_over_generated_patterns`
#[inline]
#[must_use]
pub fn reduction_cmp(
    lhs: &CmdPat,
    rhs: &CmdPat,
) -> Ordering
{
    let left_holes = hole_counts(lhs);
    let right_holes = hole_counts(rhs);
    match cmd_size(lhs).cmp(&cmd_size(rhs)) {
        | Ordering::Greater if dominates(&left_holes, &right_holes).0 => Ordering::Greater,
        | Ordering::Less if dominates(&right_holes, &left_holes).0 => Ordering::Less,
        | Ordering::Equal if left_holes == right_holes => path_order_cmp(lhs, rhs),
        | _ => Ordering::Equal,
    }
}

/// The **lexicographic path order** on two command patterns.
///
/// # Contract
/// - ensures: [`Ordering::Greater`] when the left strictly exceeds the right in
///   the path order, [`Ordering::Less`] in the mirror case, and
///   [`Ordering::Equal`] when neither does — which covers both syntactic
///   equality and genuine incomparability, since
///   `gandr_theory_coherent_resolutions::completion` treats the two the same
///   way.
/// - provides: a simplification order, so stable under substitution, monotone
///   under contexts, and well-founded.
/// - panics: none.
#[inline]
#[must_use]
pub fn path_order_cmp(
    lhs: &CmdPat,
    rhs: &CmdPat,
) -> Ordering
{
    let left = FlatTerm::flatten(lhs);
    let right = FlatTerm::flatten(rhs);
    if bool::from(strictly_greater(&left, &right)) {
        return Ordering::Greater;
    }
    if bool::from(strictly_greater(&right, &left)) {
        return Ordering::Less;
    }
    Ordering::Equal
}

/// How many times each metavariable occurs in a pattern.
///
/// Occurrence is counted per [`MetaVar`] — the `(name, category)` pair a
/// substitution binds — rather than per hole name, because a name worn at two
/// polarities is two independent substitution targets.
///
/// # Contract
/// - ensures: one entry per distinct metavariable, mapped to its occurrence
///   count.
/// - panics: none.
#[inline]
fn hole_counts(cmd: &CmdPat) -> BTreeMap<MetaVar, HoleOccurrences>
{
    let mut occurrences = Vec::new();
    collect_cmd_metavars(cmd, &mut occurrences);
    let mut counts: BTreeMap<MetaVar, HoleOccurrences> = BTreeMap::new();
    for var in occurrences {
        let entry = counts.entry(var).or_insert_with(|| HoleOccurrences(0_u32));
        entry.0 = entry.0.saturating_add(1_u32);
    }
    counts
}

/// Whether `larger` carries every metavariable of `smaller` at least as often.
///
/// This is the condition that makes a node-count comparison survive
/// substitution: instantiating a metavariable adds its image's size once per
/// occurrence, so a side that never repeats a hole less often than the other
/// cannot be overtaken.
///
/// # Contract
/// - ensures: positive exactly when every entry of `smaller` has an entry in
///   `larger` with a count at least as high.
/// - panics: none.
#[inline]
fn dominates(
    larger: &BTreeMap<MetaVar, HoleOccurrences>,
    smaller: &BTreeMap<MetaVar, HoleOccurrences>,
) -> HoleDomination
{
    HoleDomination(
        smaller
            .iter()
            .all(|(var, count)| larger.get(var).is_some_and(|held| held.0 >= count.0)),
    )
}

/// Whether one side carries every hole of the other at least as often — the
/// condition that makes a node-count comparison survive substitution.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HoleDomination(bool);

/// An occurrence count for one metavariable.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HoleOccurrences(u32);

/// A verdict of the path order's strict relation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PathOrderStrict(bool);

impl From<bool> for PathOrderStrict
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<PathOrderStrict> for bool
{
    #[inline]
    fn from(value: PathOrderStrict) -> Self
    {
        value.0
    }
}

/// A dense index into a [`FlatTerm`]'s node table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FlatIndex(usize);

/// The rank of a head symbol's **kind** in the precedence.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HeadRank(u8);

/// A node's number of immediate children.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ChildCount(usize);

/// The **head** of a pattern node — a function symbol, or a metavariable.
///
/// A path order treats the two differently: a metavariable is never greater
/// than anything, and is exceeded only by a term that properly contains it.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Head
{
    /// A cut `⟨p |ε c⟩`, carrying its polarity.
    Cut(Polarity),
    /// An operation frame `f(p̄; c)`.
    Op(Sym),
    /// A return-side constructor frame `K⁻(c)`.
    Frame(Sym),
    /// A constructor application `K(p̄)`.
    Ctor(Sym),
    /// The terminal consumer `★`.
    Top,
    /// A metavariable leaf.
    Var(MetaVar),
}

/// A pattern flattened into a **post-order** node table: every node's children
/// carry strictly smaller indices, and the root is the last entry.
#[repr(transparent)]
struct FlatTerm
{
    /// The nodes, in post-order.
    nodes: Vec<FlatNode>,
}

/// One node of a [`FlatTerm`].
struct FlatNode
{
    /// The node's head.
    head: Head,
    /// The node's children, in left-to-right order.
    children: Vec<FlatIndex>,
}

impl FlatTerm
{
    /// Flatten a command pattern into a post-order node table.
    ///
    /// # Contract
    /// - ensures: every node appears after all of its children, so a child's
    ///   index is strictly smaller than its parent's; the last entry is the
    ///   root.
    /// - panics: none.
    /// - intension: one explicit worklist pass, never recursion.
    #[inline]
    fn flatten(cmd: &CmdPat) -> Self
    {
        /// A step of the flattening worklist.
        enum Frame
        {
            /// Visit a node, scheduling its children first.
            Enter(Node),
            /// Emit a node whose children have been emitted.
            Exit(Node, ChildCount),
        }

        let mut stack = alloc::vec![Frame::Enter(Node::Cmd(cmd.clone()))];
        let mut emitted: Vec<FlatIndex> = Vec::new();
        let mut nodes: Vec<FlatNode> = Vec::new();
        while let Some(frame) = stack.pop() {
            match frame {
                | Frame::Enter(node) => {
                    let children = node.children();
                    stack.push(Frame::Exit(node, ChildCount(children.len())));
                    stack.extend(children.into_iter().rev().map(Frame::Enter));
                },
                | Frame::Exit(node, count) => {
                    let Some(split_at) = emitted.len().checked_sub(count.0)
                    else {
                        continue;
                    };
                    let children = emitted.split_off(split_at);
                    let head = head_of(&node);
                    emitted.push(FlatIndex(nodes.len()));
                    nodes.push(FlatNode { head, children });
                },
            }
        }
        Self { nodes }
    }
}

/// The head of one erased pattern node.
///
/// # Contract
/// - ensures: the head constructor matching the node's own shape, carrying its
///   symbol, polarity or metavariable.
/// - panics: none.
#[inline]
fn head_of(node: &Node) -> Head
{
    match *node {
        | Node::Cmd(CmdPat::Cut { pol, .. }) => Head::Cut(pol),
        | Node::Prod(ProdPat::Meta(ref var)) | Node::Cons(ConsPat::Meta(ref var)) => {
            Head::Var(var.clone())
        },
        | Node::Prod(ProdPat::Ctor { ref ctor, .. }) => Head::Ctor(ctor.clone()),
        | Node::Cons(ConsPat::Op { ref op, .. }) => Head::Op(op.clone()),
        | Node::Cons(ConsPat::Frame { ref ctor, .. }) => Head::Frame(ctor.clone()),
        | Node::Cons(ConsPat::Top) => Head::Top,
    }
}

/// The precedence rank of a head's kind, or `None` for a metavariable — which
/// is not a function symbol and takes no precedence at all.
///
/// # Contract
/// - ensures: cuts above constructor frames above operation frames above
///   constructors above `★`; `None` for [`Head::Var`]. The
///   frame-above-operation tier is what orients the worked fusion cell forwards
///   (see the module header).
/// - panics: none.
#[inline]
fn head_rank(head: &Head) -> Option<HeadRank>
{
    match *head {
        | Head::Cut(_) => Some(HeadRank(4_u8)),
        | Head::Frame(_) => Some(HeadRank(3_u8)),
        | Head::Op(_) => Some(HeadRank(2_u8)),
        | Head::Ctor(_) => Some(HeadRank(1_u8)),
        | Head::Top => Some(HeadRank(0_u8)),
        | Head::Var(_) => None,
    }
}

/// The polarity's rank inside the cut kind — arbitrary, total, deterministic.
#[inline]
fn polarity_rank(polarity: Polarity) -> HeadRank
{
    match polarity {
        | Polarity::Positive => HeadRank(0_u8),
        | Polarity::Negative => HeadRank(1_u8),
    }
}

/// The **precedence** comparison of two heads at their arities, or `None` when
/// either is a metavariable.
///
/// # Contract
/// - ensures: kind rank first, then the kind's own payload (a symbol's order, a
///   cut's polarity rank), then the arity; the result is a total order on
///   function symbols.
/// - panics: none.
#[inline]
fn precedence_cmp(
    left: &Head,
    left_arity: ChildCount,
    right: &Head,
    right_arity: ChildCount,
) -> Option<Ordering>
{
    let left_rank = head_rank(left)?;
    let right_rank = head_rank(right)?;
    let by_kind = left_rank.cmp(&right_rank);
    if by_kind != Ordering::Equal {
        return Some(by_kind);
    }
    let by_payload = match (left, right) {
        | (&Head::Cut(left_pol), &Head::Cut(right_pol)) => {
            polarity_rank(left_pol).cmp(&polarity_rank(right_pol))
        },
        | (&Head::Op(ref left_sym), &Head::Op(ref right_sym))
        | (&Head::Frame(ref left_sym), &Head::Frame(ref right_sym))
        | (&Head::Ctor(ref left_sym), &Head::Ctor(ref right_sym)) => left_sym.cmp(right_sym),
        | _ => Ordering::Equal,
    };
    if by_payload != Ordering::Equal {
        return Some(by_payload);
    }
    Some(left_arity.cmp(&right_arity))
}

/// Whether `left`'s root strictly exceeds `right`'s root in the path order.
///
/// The relation is filled by two nested loops over `(left index, right index)`
/// in increasing order. That is sound because the path order at one pair reads
/// only pairs with a strictly smaller left index (the subterm case), a strictly
/// smaller right index (the "greater than every argument" side condition), or
/// both (the lexicographic comparison of arguments) — and the post-order
/// flattening makes every child's index strictly smaller than its parent's.
///
/// # Contract
/// - ensures: positive exactly when the two roots stand in the strict
///   lexicographic path order induced by [`precedence_cmp`].
/// - panics: none.
/// - intension: no recursion; time is quadratic in the two node counts times
///   the arity.
#[inline]
fn strictly_greater(
    left: &FlatTerm,
    right: &FlatTerm,
) -> PathOrderStrict
{
    let right_count = right.nodes.len();
    let mut equal = RelationTable::default();
    let mut greater = RelationTable::default();
    for left_node in &left.nodes {
        let mut equal_row: Vec<PathOrderStrict> = Vec::with_capacity(right_count);
        let mut greater_row: Vec<PathOrderStrict> = Vec::with_capacity(right_count);
        for right_node in &right.nodes {
            let same = left_node.head == right_node.head
                && left_node.children.len() == right_node.children.len()
                && left_node
                    .children
                    .iter()
                    .zip(right_node.children.iter())
                    .all(|(&child, &other)| equal.at(child, other).0);
            equal_row.push(PathOrderStrict(same));
            greater_row.push(exceeds(
                left_node,
                right_node,
                &equal,
                &greater,
                &greater_row,
            ));
        }
        equal.push_row(equal_row);
        greater.push_row(greater_row);
    }
    greater.roots()
}

/// A filled half of the path-order relation over one pair of flattened terms:
/// `rows[left index][right index]`.
#[repr(transparent)]
#[derive(Default)]
struct RelationTable
{
    /// One row per left node, each holding one answer per right node.
    rows: Vec<Vec<PathOrderStrict>>,
}

impl RelationTable
{
    /// The answer at `(left, right)`, negative outside the filled region.
    ///
    /// # Contract
    /// - ensures: the recorded answer when both indices are inside the filled
    ///   region, and a negative answer otherwise — a defensive default that a
    ///   correctly ordered fill never reaches.
    /// - panics: none.
    #[inline]
    fn at(
        &self,
        left: FlatIndex,
        right: FlatIndex,
    ) -> PathOrderStrict
    {
        self.rows
            .get(left.0)
            .and_then(|row| row.get(right.0))
            .copied()
            .unwrap_or(PathOrderStrict(false))
    }

    /// Append a filled row.
    #[inline]
    fn push_row(
        &mut self,
        row: Vec<PathOrderStrict>,
    )
    {
        self.rows.push(row);
    }

    /// The last entry of the last row — the two roots' answer, once the fill is
    /// complete.
    ///
    /// # Contract
    /// - ensures: the root pair's answer for a completely filled table, and a
    ///   negative answer for an empty one.
    /// - panics: none.
    #[inline]
    fn roots(&self) -> PathOrderStrict
    {
        self.rows
            .last()
            .and_then(|row| row.last())
            .copied()
            .unwrap_or(PathOrderStrict(false))
    }
}

/// Whether `left_node` exceeds `right_node`, reading the already-filled rows.
///
/// `current_row` holds the greater-than answers for `left_node` against every
/// right node already visited in this row, which is exactly the set the "and it
/// exceeds each of the right's arguments" side condition needs.
///
/// # Contract
/// - requires: `equal` and `greater` hold every row with a strictly smaller
///   left index, and `current_row` every column with a strictly smaller right
///   index.
/// - ensures: the strict path-order answer for this pair.
/// - panics: none.
#[inline]
fn exceeds(
    left_node: &FlatNode,
    right_node: &FlatNode,
    equal: &RelationTable,
    greater: &RelationTable,
    current_row: &[PathOrderStrict],
) -> PathOrderStrict
{
    // A metavariable exceeds nothing.
    if matches!(left_node.head, Head::Var(_)) {
        return PathOrderStrict(false);
    }
    let right_index = FlatIndex(current_row.len());
    // The subterm case: some argument of the left already reaches the right.
    let subterm = left_node
        .children
        .iter()
        .any(|&child| greater.at(child, right_index).0 || equal.at(child, right_index).0);
    if subterm {
        return PathOrderStrict(true);
    }
    // Both remaining cases need the left to exceed every argument of the right.
    let over_arguments = right_node.children.iter().all(|&arg| {
        current_row
            .get(arg.0)
            .copied()
            .unwrap_or(PathOrderStrict(false))
            .0
    });
    if !over_arguments {
        return PathOrderStrict(false);
    }
    let left_arity = ChildCount(left_node.children.len());
    let right_arity = ChildCount(right_node.children.len());
    match precedence_cmp(&left_node.head, left_arity, &right_node.head, right_arity) {
        | Some(Ordering::Greater) => PathOrderStrict(true),
        | Some(Ordering::Equal) => {
            lexicographically_greater(&left_node.children, &right_node.children, equal, greater)
        },
        | _ => PathOrderStrict(false),
    }
}

/// Whether the left argument list exceeds the right one lexicographically.
///
/// # Contract
/// - requires: `equal` and `greater` hold every row the two lists index.
/// - ensures: positive when the first position at which the two are not equal
///   has the left argument strictly greater; negative when the lists agree
///   throughout or the first difference goes the other way.
/// - panics: none.
#[inline]
fn lexicographically_greater(
    left: &[FlatIndex],
    right: &[FlatIndex],
    equal: &RelationTable,
    greater: &RelationTable,
) -> PathOrderStrict
{
    for (&left_arg, &right_arg) in left.iter().zip(right.iter()) {
        if equal.at(left_arg, right_arg).0 {
            continue;
        }
        return greater.at(left_arg, right_arg);
    }
    PathOrderStrict(false)
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use proptest::prelude::*;

    use super::*;

    /// The design's worked fusion cell, left face: the intermediate `Succ`
    /// allocation is still there.
    fn fusion_before() -> CmdPat
    {
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("v"),
            ConsPat::frame(
                "Succ",
                ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("a")),
            ),
        )
    }

    /// The design's worked fusion cell, right face: the constructor frame has
    /// been pushed inside the operation frame and the allocation is gone.
    fn fusion_after() -> CmdPat
    {
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("v"),
            ConsPat::op(
                "add",
                [ProdPat::meta("n")],
                ConsPat::frame("Succ", ConsPat::meta("a")),
            ),
        )
    }

    #[test]
    fn an_equal_size_pair_is_oriented_by_the_path_order()
    {
        // The pair the residual was about, and the one the design already
        // wanted derived: `(fuse-S⁻-add)`, whose two faces have the same node
        // count because fusing moves a frame rather than removing one. The size
        // comparison reported them equal and completion left an obstruction.
        let (before, after) = (fusion_before(), fusion_after());
        assert_eq!(
            cmd_size(&before),
            cmd_size(&after),
            "the hypothesis: the fusion cell's two faces are the same size"
        );
        assert_eq!(
            Ordering::Greater,
            reduction_cmp(&before, &after),
            "so the order puts the unfused face on the left, which is the cell the design writes"
        );
        assert_eq!(
            Ordering::Less,
            reduction_cmp(&after, &before),
            "and the relation is antisymmetric on it"
        );
    }

    #[test]
    fn a_size_difference_orients_when_the_larger_side_dominates()
    {
        // The unchanged half: where the node counts differ and the larger side
        // carries every hole of the smaller at least as often, the size
        // comparison decides exactly as it did before.
        let larger = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("n")]),
            ConsPat::op("add", [ProdPat::meta("m")], ConsPat::meta("a")),
        );
        let smaller = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("n"),
            ConsPat::op("add", [ProdPat::meta("m")], ConsPat::meta("a")),
        );
        assert!(
            cmd_size(&larger) > cmd_size(&smaller),
            "the hypothesis: the two differ in node count"
        );
        assert_eq!(Ordering::Greater, reduction_cmp(&larger, &smaller));
        assert_eq!(Ordering::Less, reduction_cmp(&smaller, &larger));
    }

    #[test]
    fn a_size_difference_that_substitution_could_reverse_is_an_obstruction()
    {
        // The guard. The left is larger by node count today and smaller under
        // any substitution that sends `x` to something of size three or more,
        // because the right repeats `x` and the left does not. A reduction
        // order that oriented this pair would be proving termination of a rule
        // whose instances grow.
        let larger_now = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Pair", [
                ProdPat::meta("x"),
                ProdPat::ctor("Zero", []),
                ProdPat::ctor("Zero", []),
            ]),
            ConsPat::Top,
        );
        let grows_faster = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Pair", [ProdPat::meta("x"), ProdPat::meta("x")]),
            ConsPat::Top,
        );
        assert!(
            cmd_size(&larger_now) > cmd_size(&grows_faster),
            "the hypothesis: the guarded side is the one a size order would pick"
        );
        assert_eq!(
            Ordering::Equal,
            reduction_cmp(&larger_now, &grows_faster),
            "the hole counts do not license the size comparison, so it is an honest obstruction"
        );
        assert_eq!(
            Ordering::Equal,
            reduction_cmp(&grows_faster, &larger_now),
            "in either direction"
        );
    }

    #[test]
    fn an_equal_size_pair_the_path_order_cannot_separate_stays_an_obstruction()
    {
        // The other obstruction route, and the reason the order still reports
        // one. Two cuts differing only in which of two holes sits where are the
        // same size with the same hole counts, and the path order reaches the
        // metavariable leaves without a precedence to separate them.
        let left = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("x"),
            ConsPat::op("f", [ProdPat::meta("y")], ConsPat::meta("a")),
        );
        let right = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("y"),
            ConsPat::op("f", [ProdPat::meta("x")], ConsPat::meta("a")),
        );
        assert_eq!(
            cmd_size(&left),
            cmd_size(&right),
            "the hypothesis: equal size"
        );
        assert_eq!(
            Ordering::Equal,
            reduction_cmp(&left, &right),
            "a path order does not order metavariables, so this pair is left to the engine"
        );
    }

    #[test]
    fn the_frame_defining_shape_is_not_oriented_forwards_and_that_is_stated()
    {
        // The documented limit, pinned so it cannot be forgotten. A path order
        // is a simplification order, so it cannot orient a rule whose right
        // side buries the left's hole under new structure — which is what the
        // frame-defining cell does. Its orientation comes from the calculus and
        // never passes through this order.
        let before = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("v"),
            ConsPat::frame("Succ", ConsPat::meta("b")),
        );
        let after = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("v")]),
            ConsPat::meta("b"),
        );
        assert_eq!(
            cmd_size(&before),
            cmd_size(&after),
            "the two faces are the same size"
        );
        assert_eq!(
            Ordering::Less,
            reduction_cmp(&before, &after),
            "the order reads the constructor-building side as larger, which is the reverse of the \
             calculus's own orientation"
        );
    }

    proptest! {
        /// The relation is a strict order: never both ways, and never a term
        /// against itself.
        #[test]
        fn the_order_is_a_strict_order_over_generated_patterns(
            left in any_cmd(),
            right in any_cmd(),
        ) {
            prop_assert_eq!(
                Ordering::Equal,
                reduction_cmp(&left, &left),
                "irreflexive: a pattern never exceeds itself"
            );
            let forward = reduction_cmp(&left, &right);
            let backward = reduction_cmp(&right, &left);
            match forward {
                | Ordering::Greater => prop_assert_eq!(Ordering::Less, backward),
                | Ordering::Less => prop_assert_eq!(Ordering::Greater, backward),
                | Ordering::Equal => prop_assert_eq!(Ordering::Equal, backward),
            }
        }

        /// The path order is **stable under substitution** on the fragment the
        /// reduction order admits: renaming or instantiating a hole uniformly
        /// on both sides cannot reverse a strict verdict.
        #[test]
        fn the_path_order_survives_a_uniform_hole_instantiation(
            left in any_cmd(),
            right in any_cmd(),
            filler in any_prod(),
        ) {
            let verdict = reduction_cmp(&left, &right);
            prop_assume!(verdict != Ordering::Equal);
            let hole = SubstitutedHole("x");
            let left_instance = substitute_producer(&left, hole, &filler);
            let right_instance = substitute_producer(&right, hole, &filler);
            prop_assert_eq!(
                verdict,
                reduction_cmp(&left_instance, &right_instance),
                "a strict verdict survives instantiating the hole `x` on both sides"
            );
        }
    }

    /// The producer hole a fixture instantiates.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct SubstitutedHole<'fixture>(&'fixture str);

    /// Replace every producer metavariable named `hole` with `filler`.
    ///
    /// Written over [`crate::pattern::transform_node`]'s explicit worklist, so
    /// the substitution is iterative like every other traversal in this crate.
    fn substitute_producer(
        cmd: &CmdPat,
        hole: SubstitutedHole<'_>,
        filler: &ProdPat,
    ) -> CmdPat
    {
        let replaced = crate::pattern::transform_node(Node::Cmd(cmd.clone()), |node| {
            Some(match node {
                | Node::Prod(ProdPat::Meta(ref var))
                    if &*var.name == hole.0 && var.cat == crate::pattern::Cat::Producer =>
                {
                    Node::Prod(filler.clone())
                },
                | other => other,
            })
        });
        let Some(Node::Cmd(out)) = replaced
        else {
            return cmd.clone();
        };
        out
    }

    /// Generated producer patterns over a small alphabet.
    fn any_prod() -> impl Strategy<Value = ProdPat>
    {
        let leaf = prop_oneof![
            Just(ProdPat::meta("x")),
            Just(ProdPat::meta("y")),
            Just(ProdPat::ctor("Zero", [])),
        ];
        leaf.prop_recursive(3_u32, 12_u32, 2_u32, |inner| {
            prop_oneof![
                inner.clone().prop_map(|arg| ProdPat::ctor("Succ", [arg])),
                proptest::collection::vec(inner, 2 ..= 2_usize)
                    .prop_map(|args| ProdPat::ctor("Pair", args)),
            ]
        })
    }

    /// Generated consumer patterns over a small alphabet.
    fn any_cons() -> impl Strategy<Value = ConsPat>
    {
        let leaf = prop_oneof![
            Just(ConsPat::meta("a")),
            Just(ConsPat::meta("b")),
            Just(ConsPat::Top),
        ];
        leaf.prop_recursive(3_u32, 12_u32, 2_u32, |inner| {
            prop_oneof![
                inner.clone().prop_map(|ret| ConsPat::frame("Succ", ret)),
                (any_prod(), inner).prop_map(|(arg, ret)| ConsPat::op("add", [arg], ret)),
            ]
        })
    }

    /// Generated command patterns over a small alphabet.
    fn any_cmd() -> impl Strategy<Value = CmdPat>
    {
        (any_prod(), any_cons())
            .prop_map(|(prod, cons)| CmdPat::cut(Polarity::Positive, prod, cons))
    }
}
