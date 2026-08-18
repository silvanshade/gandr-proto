//! The **command-pattern IL** — patterns over the polarized command IL with
//! pattern metavariables (the sequent-machines design's §7.3, the
//! `CommandPattern` of the `Cell` sketch).
//!
//! A [`Cell`](crate::cell::Cell) rewrites `lhs ~> rhs` where each side is a
//! [`CmdPat`] — the §2.1 command grammar `⟨p |ε c⟩` restricted to the
//! **cell-visible fragment** (§7.4: natives are opaque, so `prim`/`jump` carry
//! no seam a cell reads through) and extended with the two fusion-layer
//! consumer frames the running example needs: an **operation frame** `f(p̄; c)`
//! (§7.1, the reification of a datatype `op`) and a **return-side constructor
//! frame** `K⁻(c)` (§7.1, `Succ⁻(α) := μ̃x.⟨Succ(x) | α⟩` — "definable, not
//! primitive").
//!
//! Leaves are either a concrete constructor / operation symbol ([`Sym`]) or a
//! **metavariable** ([`MetaVar`]) carrying its producer/consumer [`Cat`]. The
//! [`Polarity`] on a cut is reused verbatim from [`gandr_core_sequent::il`]
//! (the read-only consumption of the L0 IL vocabulary; K2's per-cut orientation
//! is what the η discipline of [`crate::sequent::EtaKind`] checks).
//!
//! # Content addressing
//!
//! Cells are content-addressed by **structural identity** (§7.3.1): two cells
//! are the same cell when their [`CmdPat`]s are structurally equal ([`Eq`]).
//! [`CmdPat`] derives total structural equality, so the cell store dedups on it
//! (node-hash keying is the eventual efficiency refinement; the
//! semantics are these structural keys). Iteration order everywhere is
//! insertion/positional, never hash order, so the engine is deterministic.
//!
//! # Positions
//!
//! [`Pos`] addresses a subterm as a path of child indices through the uniform
//! [`Node`] view ([`Node::children`] / [`Node::with_children`]); overlap
//! enumeration (`gandr_theory_coherent_resolutions::overlap`) unifies a subterm
//! at a [`Pos`] and splices the reduct back with [`splice_at`]. Position paths
//! are bounded by pattern depth (rule sizes are author-controlled and the
//! generated ground configs are depth-capped), and the navigation recurses only
//! on the *path*, not on user-sized input.

use alloc::boxed::Box;
use alloc::vec::Vec;

use gandr_core_sequent::il::Polarity;

use crate::boundary::GroundPatternStatus;
use crate::boundary::PatternSize;
use crate::boundary::PositionRootStatus;
use crate::boundary::PositionStep;

/// A constructor or operation **symbol** — an interned datatype-declared name
/// (`Zero`, `Succ`, `add`, `double`, `Nil`, `Cons`, …).
///
/// Symbols are the bridge to `gandr_theory_levitation`: elaboration
/// (`gandr_theory_computads::elaborate`) maps a
/// `gandr_theory_levitation::FreeTerm::Ctor` /
/// `gandr_theory_levitation::FreeTerm::Op` name onto a `Sym`. Equality is by
/// name, so the symbol is its own content key.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Sym(pub Box<str>);

impl Sym
{
    /// A symbol from any string-like name.
    #[inline]
    #[must_use]
    pub fn new<N>(name: N) -> Self
    where
        N: Into<Box<str>>,
    {
        Self(name.into())
    }
}

impl AsRef<str> for Sym
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

/// The **category** a metavariable ranges over — the producer/consumer split of
/// the sequent grammar (`proposal-sequent-kernel.md` §2.1).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Cat
{
    /// A producer metavariable `x` — ranges over [`ProdPat`].
    Producer,
    /// A consumer / covariable metavariable `α` — ranges over [`ConsPat`].
    Consumer,
}

/// A **pattern metavariable** — a hole in a cell pattern, tagged with the
/// [`Cat`] it ranges over (`proposal-sequent-kernel.md` §7.3, "patterns = node
/// ids + pattern vars").
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MetaVar
{
    /// The metavariable's name.
    pub name: Box<str>,
    /// The category the metavariable ranges over.
    pub cat: Cat,
}

impl MetaVar
{
    /// A producer metavariable with the given name.
    #[inline]
    #[must_use]
    pub fn producer<N>(name: N) -> Self
    where
        N: Into<Box<str>>,
    {
        Self {
            name: name.into(),
            cat: Cat::Producer,
        }
    }

    /// A consumer metavariable with the given name.
    #[inline]
    #[must_use]
    pub fn consumer<N>(name: N) -> Self
    where
        N: Into<Box<str>>,
    {
        Self {
            name: name.into(),
            cat: Cat::Consumer,
        }
    }
}

/// A **producer pattern** `p` (`proposal-sequent-kernel.md` §2.1, positive
/// intro) — the value-side of a cut, over the fusion-visible fragment.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ProdPat
{
    /// A producer metavariable `x`.
    Meta(MetaVar),
    /// A constructor application `K(p̄)` — a positive intro (the datatype's
    /// declared constructor; consumer arguments are empty for every frozen-core
    /// and datatype constructor, so they are elided from the pattern).
    Ctor
    {
        /// The constructor symbol `K`.
        ctor: Sym,
        /// The producer arguments `p̄`.
        args: Box<[Self]>,
    },
}

impl ProdPat
{
    /// A producer metavariable pattern.
    #[inline]
    #[must_use]
    pub fn meta<N>(name: N) -> Self
    where
        N: Into<Box<str>>,
    {
        Self::Meta(MetaVar::producer(name))
    }

    /// A constructor-application pattern.
    #[inline]
    #[must_use]
    pub fn ctor<S, A>(
        ctor: S,
        args: A,
    ) -> Self
    where
        S: Into<Box<str>>,
        A: Into<Box<[Self]>>,
    {
        Self::Ctor {
            ctor: Sym::new(ctor),
            args: args.into(),
        }
    }
}

/// A **consumer pattern** `c` (`proposal-sequent-kernel.md` §2.1, negative elim
/// and the §7.1 fusion frames) — the covalue-side of a cut.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ConsPat
{
    /// A consumer / covariable metavariable `α`.
    Meta(MetaVar),
    /// An **operation frame** `f(p̄; c)` (§7.1) — the reification of a datatype
    /// `op` as "an f-application waiting at the seam", carrying its remaining
    /// producer arguments `p̄` and its return continuation `c`.
    Op
    {
        /// The operation symbol `f`.
        op: Sym,
        /// The remaining producer arguments `p̄`.
        args: Box<[ProdPat]>,
        /// The return continuation `c`.
        ret: Box<Self>,
    },
    /// A **return-side constructor frame** `K⁻(c)` (§7.1) — the definable frame
    /// `K⁻(c) := μ̃x.⟨K(x) | c⟩`; its defining cell is
    /// [`crate::sequent::frame_defining_cell`].
    Frame
    {
        /// The constructor symbol `K` the frame re-wraps.
        ctor: Sym,
        /// The return continuation `c`.
        ret: Box<Self>,
    },
    /// The terminal consumer `★` (top level).
    Top,
}

impl ConsPat
{
    /// A consumer metavariable pattern.
    #[inline]
    #[must_use]
    pub fn meta<N>(name: N) -> Self
    where
        N: Into<Box<str>>,
    {
        Self::Meta(MetaVar::consumer(name))
    }

    /// An operation-frame pattern.
    #[inline]
    #[must_use]
    pub fn op<S, A>(
        op: S,
        args: A,
        ret: Self,
    ) -> Self
    where
        S: Into<Box<str>>,
        A: Into<Box<[ProdPat]>>,
    {
        Self::Op {
            op: Sym::new(op),
            args: args.into(),
            ret: Box::new(ret),
        }
    }

    /// A return-side constructor-frame pattern `K⁻(ret)`.
    #[inline]
    #[must_use]
    pub fn frame<S>(
        ctor: S,
        ret: Self,
    ) -> Self
    where
        S: Into<Box<str>>,
    {
        Self::Frame {
            ctor: Sym::new(ctor),
            ret: Box::new(ret),
        }
    }
}

/// A **command pattern** `s` (`proposal-sequent-kernel.md` §2.1) — a machine
/// state, restricted to the fusion-visible cut form.
///
/// The `prim`/`jump` commands of §2.1 are deliberately absent: a `prim` is the
/// opaque host seam (§7.4, "natives are opaque"), and a `jump` is outside the
/// cell-visible fragment. Matches over this enum are total by policy: a later
/// `Case`/`Cocase` extension is a compile-visible change at every match site,
/// which is the intended review tripwire.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CmdPat
{
    /// A cut `⟨p |ε c⟩` carrying its polarity orientation `ε` (K2).
    Cut
    {
        /// The cut's evaluation-strategy orientation `ε`.
        pol: Polarity,
        /// The producer half `p`.
        prod: ProdPat,
        /// The consumer half `c`.
        cons: ConsPat,
    },
}

impl CmdPat
{
    /// A cut pattern `⟨prod |pol cons⟩`.
    #[inline]
    #[must_use]
    pub fn cut(
        pol: Polarity,
        prod: ProdPat,
        cons: ConsPat,
    ) -> Self
    {
        Self::Cut { pol, prod, cons }
    }

    /// The cut's polarity, if this command is a cut.
    #[inline]
    #[must_use]
    pub fn polarity(&self) -> Polarity
    {
        match *self {
            | Self::Cut { pol, .. } => pol,
        }
    }
}

/// A **position path** into a pattern tree (`proposal-sequent-kernel.md`
/// §7.3.2) — a sequence of child indices through the uniform [`Node`] view.
///
/// The empty path is the root. Overlap enumeration walks every non-metavariable
/// position; the "cuts make overlaps shallow" observation (§7.3.2) is why the
/// interesting positions sit near the root, not a restriction encoded here.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Pos(pub Box<[usize]>);

impl Pos
{
    /// The root position (the empty path).
    #[inline]
    #[must_use]
    pub fn root() -> Self
    {
        Self(Box::from([]))
    }

    /// A position from a slice of child indices.
    #[inline]
    #[must_use]
    pub fn from_indices<I>(indices: I) -> Self
    where
        I: Into<Box<[usize]>>,
    {
        Self(indices.into())
    }

    /// Whether this is the root position.
    #[inline]
    #[must_use]
    pub fn is_root(&self) -> PositionRootStatus
    {
        PositionRootStatus::from(self.0.is_empty())
    }

    /// This position extended by one child step.
    #[inline]
    #[must_use]
    pub fn child(
        &self,
        step: PositionStep,
    ) -> Self
    {
        let step = usize::from(step);
        let mut next = Vec::with_capacity(self.0.len().saturating_add(1));
        next.extend_from_slice(&self.0);
        next.push(step);
        Self(next.into_boxed_slice())
    }
}

impl AsRef<[usize]> for Pos
{
    #[inline]
    fn as_ref(&self) -> &[usize]
    {
        &self.0
    }
}

/// A **uniform subtree view** over the three pattern categories — the carrier
/// [`Pos`] navigation and [`splice_at`] operate on.
///
/// The command IL is a heterogeneous tree (a cut's children are a producer and
/// a consumer); [`Node`] erases the category distinction just enough to give a
/// single child-indexed navigation, re-checking category agreement on
/// [`Node::with_children`] so a splice can never graft a consumer where a
/// producer belongs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Node
{
    /// A producer subtree.
    Prod(ProdPat),
    /// A consumer subtree.
    Cons(ConsPat),
    /// A command subtree.
    Cmd(CmdPat),
}

impl Node
{
    /// This node's immediate children, in canonical left-to-right order.
    ///
    /// # Contract
    /// - ensures: a metavariable / [`ConsPat::Top`] leaf has no children; a cut
    ///   yields `[Prod(prod), Cons(cons)]`; a constructor / operation / frame
    ///   yields its producer arguments (as [`Node::Prod`]) followed by its
    ///   return consumer (as [`Node::Cons`]) where present.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn children(&self) -> Vec<Self>
    {
        match *self {
            | Self::Prod(ProdPat::Meta(_)) | Self::Cons(ConsPat::Meta(_) | ConsPat::Top) => {
                Vec::new()
            },
            | Self::Prod(ProdPat::Ctor { ref args, .. }) => {
                args.iter().cloned().map(Self::Prod).collect()
            },
            | Self::Cons(ConsPat::Op {
                ref args, ref ret, ..
            }) => {
                let mut out: Vec<Self> = args.iter().cloned().map(Self::Prod).collect();
                out.push(Self::Cons((**ret).clone()));
                out
            },
            | Self::Cons(ConsPat::Frame { ref ret, .. }) => {
                alloc::vec![Self::Cons((**ret).clone())]
            },
            | Self::Cmd(CmdPat::Cut {
                ref prod, ref cons, ..
            }) => {
                alloc::vec![Self::Prod(prod.clone()), Self::Cons(cons.clone())]
            },
        }
    }

    /// Rebuild this node with `new_children` replacing its immediate children,
    /// preserving the node's own shape and each child's category.
    ///
    /// # Contract
    /// - requires: `new_children.len()` equals [`Node::children`]'s length and
    ///   each replacement's category ([`Node::Prod`] / [`Node::Cons`]) matches
    ///   the slot it fills.
    /// - ensures: `Some(rebuilt)` on a category- and arity-matching
    ///   replacement; `None` otherwise (a mis-categorized or mis-counted splice
    ///   is rejected, never grafted).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn with_children(
        &self,
        new_children: &[Self],
    ) -> Option<Self>
    {
        match *self {
            | Self::Prod(ProdPat::Meta(_)) | Self::Cons(ConsPat::Meta(_) | ConsPat::Top) => {
                new_children.is_empty().then(|| self.clone())
            },
            | Self::Prod(ProdPat::Ctor { ref ctor, ref args }) => {
                let rebuilt = collect_prods(new_children, PatternSize::from(args.len()))?;
                Some(Self::Prod(ProdPat::Ctor {
                    ctor: ctor.clone(),
                    args: rebuilt,
                }))
            },
            | Self::Cons(ConsPat::Op {
                ref op, ref args, ..
            }) => {
                let (arg_slots, ret_slot) = new_children.split_at(args.len());
                let rebuilt_args = collect_prods(arg_slots, PatternSize::from(args.len()))?;
                let rebuilt_ret = single_cons(ret_slot)?;
                Some(Self::Cons(ConsPat::Op {
                    op: op.clone(),
                    args: rebuilt_args,
                    ret: Box::new(rebuilt_ret),
                }))
            },
            | Self::Cons(ConsPat::Frame { ref ctor, .. }) => {
                let rebuilt_ret = single_cons(new_children)?;
                Some(Self::Cons(ConsPat::Frame {
                    ctor: ctor.clone(),
                    ret: Box::new(rebuilt_ret),
                }))
            },
            | Self::Cmd(CmdPat::Cut { pol, .. }) => {
                if new_children.len() != 2 {
                    return None;
                }
                let first = new_children.first()?;
                let second = new_children.get(1)?;
                let prod = first.as_prod()?;
                let prod = prod.clone();
                let cons = second.as_cons()?;
                let cons = cons.clone();
                Some(Self::Cmd(CmdPat::Cut { pol, prod, cons }))
            },
        }
    }

    /// This node as a producer subtree, if it is one.
    ///
    /// # Contract
    /// - ensures: `Some` iff this is a [`Node::Prod`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn as_prod(&self) -> Option<&ProdPat>
    {
        match *self {
            | Self::Prod(ref prod) => Some(prod),
            | Self::Cons(_) | Self::Cmd(_) => None,
        }
    }

    /// This node as a consumer subtree, if it is one.
    ///
    /// # Contract
    /// - ensures: `Some` iff this is a [`Node::Cons`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn as_cons(&self) -> Option<&ConsPat>
    {
        match *self {
            | Self::Cons(ref cons) => Some(cons),
            | Self::Prod(_) | Self::Cmd(_) => None,
        }
    }
}

/// Collect exactly `n` [`Node::Prod`] slots into a producer argument vector.
///
/// # Contract
/// - ensures: `Some` iff `slots.len() == n` and every slot is a [`Node::Prod`].
/// - panics: none.
#[inline]
fn collect_prods(
    slots: &[Node],
    n: PatternSize,
) -> Option<Box<[ProdPat]>>
{
    let n = usize::from(n);
    if slots.len() != n {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    for slot in slots {
        match *slot {
            | Node::Prod(ref p) => out.push(p.clone()),
            | Node::Cons(_) | Node::Cmd(_) => return None,
        }
    }
    Some(out.into_boxed_slice())
}

/// Extract a single [`Node::Cons`] from a one-element slot slice.
///
/// # Contract
/// - ensures: `Some(cons)` iff `slots` is exactly one [`Node::Cons`].
/// - panics: none.
#[inline]
fn single_cons(slots: &[Node]) -> Option<ConsPat>
{
    if slots.len() != 1 {
        return None;
    }
    let slot = slots.first()?;
    let cons = slot.as_cons()?;
    Some(cons.clone())
}

/// The subtree at `pos`, or `None` if the path leaves the tree.
///
/// # Contract
/// - ensures: `Some(node)` when `pos` addresses an existing subtree of `root`;
///   `None` when any step indexes past a node's children.
/// - panics: none.
#[inline]
#[must_use]
pub fn subterm_at(
    root: &Node,
    pos: &Pos,
) -> Option<Node>
{
    let mut cursor = root.clone();
    for &step in pos.as_ref() {
        let children = cursor.children();
        let child = children.get(step)?;
        cursor = child.clone();
    }
    Some(cursor)
}

/// Splice `replacement` into `root` at `pos`, returning the rebuilt tree.
///
/// # Contract
/// - requires: `pos` addresses an existing subtree and `replacement` matches
///   its category.
/// - ensures: `Some(rebuilt)` with the subtree at `pos` replaced; `None` when
///   the path leaves the tree or a category mismatch is spliced.
/// - panics: none.
#[inline]
#[must_use]
pub fn splice_at(
    root: &Node,
    pos: &Pos,
    replacement: Node,
) -> Option<Node>
{
    let mut cursor = root.clone();
    let mut frames: Vec<(Node, Vec<Node>, usize)> = Vec::new();
    for &step in pos.as_ref() {
        let children = cursor.children();
        let child = children.get(step)?;
        let child = child.clone();
        frames.push((cursor, children, step));
        cursor = child;
    }

    let mut rebuilt = replacement;
    for (parent, mut children, step) in frames.into_iter().rev() {
        let destination = children.get_mut(step)?;
        *destination = rebuilt;
        rebuilt = parent.with_children(&children)?;
    }
    Some(rebuilt)
}

/// Append a producer pattern's metavariables onto `into` (see
/// [`collect_cmd_metavars`]).
///
/// # Contract
/// - ensures: as [`collect_cmd_metavars`], for a producer subtree.
/// - panics: none.
#[inline]
pub fn collect_prod_metavars(
    prod: &ProdPat,
    into: &mut Vec<MetaVar>,
)
{
    collect_node_metavars(Node::Prod(prod.clone()), into);
}

/// Append a consumer pattern's metavariables onto `into` (see
/// [`collect_cmd_metavars`]).
///
/// # Contract
/// - ensures: as [`collect_cmd_metavars`], for a consumer subtree.
/// - panics: none.
#[inline]
pub fn collect_cons_metavars(
    cons: &ConsPat,
    into: &mut Vec<MetaVar>,
)
{
    collect_node_metavars(Node::Cons(cons.clone()), into);
}

/// Whether a command pattern is **ground** (metavariable-free) — the shape a
/// differential instance and a machine configuration take.
///
/// # Contract
/// - ensures: `true` iff the pattern contains no [`MetaVar`] leaf.
/// - panics: none.
#[inline]
#[must_use]
pub fn is_ground_cmd(cmd: &CmdPat) -> GroundPatternStatus
{
    let mut vars = Vec::new();
    collect_cmd_metavars(cmd, &mut vars);
    GroundPatternStatus::from(vars.is_empty())
}

/// Append `cmd`'s metavariables, in left-to-right order with repeats, onto
/// `into` (`proposal-sequent-kernel.md` §7.3, the `CellMeta` derivation
/// source).
///
/// # Contract
/// - ensures: every metavariable leaf is pushed once per occurrence in
///   left-to-right order, so linearity is judged by counting.
/// - panics: none.
#[inline]
pub fn collect_cmd_metavars(
    cmd: &CmdPat,
    into: &mut Vec<MetaVar>,
)
{
    collect_node_metavars(Node::Cmd(cmd.clone()), into);
}

/// Append metavariables from one erased node tree, preserving left-to-right
/// order with an explicit worklist.
#[inline]
fn collect_node_metavars(
    root: Node,
    into: &mut Vec<MetaVar>,
)
{
    let mut stack = alloc::vec![root];
    while let Some(node) = stack.pop() {
        match node {
            | Node::Prod(ProdPat::Meta(mv)) | Node::Cons(ConsPat::Meta(mv)) => into.push(mv),
            | other => {
                let children = other.children();
                stack.extend(children.into_iter().rev());
            },
        }
    }
}

/// The size of a producer pattern (see [`cmd_size`]).
///
/// # Contract
/// - ensures: a positive node count.
/// - panics: none.
#[inline]
#[must_use]
pub fn prod_size(prod: &ProdPat) -> PatternSize
{
    node_size(Node::Prod(prod.clone()))
}

/// The size of a consumer pattern (see [`cmd_size`]).
///
/// # Contract
/// - ensures: a positive node count.
/// - panics: none.
#[inline]
#[must_use]
pub fn cons_size(cons: &ConsPat) -> PatternSize
{
    node_size(Node::Cons(cons.clone()))
}

/// The **size** of a command pattern (its node count) — the well-founded
/// measure the reduction order ([`crate::order::reduction_cmp`]) compares
/// before it consults the path order.
///
/// # Contract
/// - ensures: a positive count, monotone under subterm inclusion.
/// - panics: none.
#[inline]
#[must_use]
pub fn cmd_size(cmd: &CmdPat) -> PatternSize
{
    node_size(Node::Cmd(cmd.clone()))
}

/// Count nodes in one erased tree with an explicit worklist.
#[inline]
fn node_size(root: Node) -> PatternSize
{
    let mut count = 0_usize;
    let mut stack = alloc::vec![root];
    while let Some(node) = stack.pop() {
        count = count.saturating_add(1);
        stack.extend(node.children());
    }
    PatternSize::from(count)
}

/// Transform an erased pattern tree bottom-up without recursive descent.
#[inline]
pub(crate) fn transform_node<F>(
    root: Node,
    mut transform: F,
) -> Option<Node>
where
    F: FnMut(Node) -> Option<Node>,
{
    enum Frame
    {
        Enter(Node),
        Exit(Node, usize),
    }

    let mut stack = alloc::vec![Frame::Enter(root)];
    let mut rebuilt: Vec<Node> = Vec::new();
    while let Some(frame) = stack.pop() {
        match frame {
            | Frame::Enter(node) => {
                let children = node.children();
                let child_count = children.len();
                stack.push(Frame::Exit(node, child_count));
                stack.extend(children.into_iter().rev().map(Frame::Enter));
            },
            | Frame::Exit(node, child_count) => {
                let split_at = rebuilt.len().checked_sub(child_count)?;
                let children = rebuilt.split_off(split_at);
                let with_children = node.with_children(&children)?;
                let transformed = transform(with_children)?;
                rebuilt.push(transformed);
            },
        }
    }
    rebuilt.pop()
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn metavars_are_collected_in_order()
    {
        let mut vars = Vec::new();
        collect_cmd_metavars(&peano_add_s(), &mut vars);
        let names: Vec<&str> = vars.iter().map(|mv| &*mv.name).collect();
        assert_eq!(
            &["m", "n", "alpha"][..],
            names.as_slice(),
            "producer then op-arg then ret"
        );
    }

    #[test]
    fn ground_and_size_track_structure()
    {
        assert!(
            !bool::from(is_ground_cmd(&peano_add_s())),
            "the rule LHS has metavars"
        );
        let ground = CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top);
        assert!(
            bool::from(is_ground_cmd(&ground)),
            "Zero cut against Top is ground"
        );
        assert_eq!(
            PatternSize::from(3_usize),
            cmd_size(&ground),
            "cut + Zero + Top"
        );
    }

    #[test]
    fn subterm_and_splice_round_trip()
    {
        let root = Node::Cmd(peano_add_s());
        // Position [1, 0] = the op's first producer argument `n`.
        let pos = Pos::from_indices([1_usize, 0_usize]);
        let sub = subterm_at(&root, &pos).expect("n is addressable");
        assert_eq!(sub, Node::Prod(ProdPat::meta("n")), "reached the op arg");
        let spliced = splice_at(&root, &pos, Node::Prod(ProdPat::ctor("Zero", [])))
            .expect("a producer splices into a producer slot");
        let expected = Node::Cmd(CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::meta("alpha")),
        ));
        assert_eq!(spliced, expected, "the op arg was replaced by Zero");
    }

    #[test]
    fn a_miscategorized_splice_is_rejected()
    {
        let root = Node::Cmd(peano_add_s());
        let pos = Pos::from_indices([1_usize, 0_usize]);
        // A consumer cannot fill a producer slot.
        assert_eq!(
            None,
            splice_at(&root, &pos, Node::Cons(ConsPat::Top)),
            "grafting a consumer where a producer belongs is declined"
        );
    }

    #[test]
    fn the_root_position_is_the_only_one_that_reports_root()
    {
        assert!(
            bool::from(Pos::root().is_root()),
            "the empty path is the root"
        );
        assert!(
            !bool::from(Pos::from_indices([0_usize]).is_root()),
            "and one step down is not"
        );
    }

    #[test]
    fn the_per_category_sizes_count_their_own_subtree()
    {
        // The three size faces agree on one term: a cut is its two halves plus
        // itself, so the whole exceeds each part by more than one only because
        // the parts are counted whole.
        let cmd = peano_add_s();
        let CmdPat::Cut {
            ref prod, ref cons, ..
        } = cmd;
        let producer = prod_size(prod);
        let consumer = cons_size(cons);
        assert_eq!(
            PatternSize::from(2_usize),
            producer,
            "`Succ(m)` is the constructor and its argument"
        );
        assert_eq!(
            PatternSize::from(3_usize),
            consumer,
            "`add(n; α)` is the frame, its argument, and its return continuation"
        );
        assert_eq!(
            PatternSize::from(
                usize::from(producer)
                    .saturating_add(usize::from(consumer))
                    .saturating_add(1_usize)
            ),
            cmd_size(&cmd),
            "and the cut is both halves plus itself"
        );
    }

    fn peano_add_s() -> CmdPat
    {
        // ⟨Succ(m) | add(n; α)⟩
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
        )
    }
}
