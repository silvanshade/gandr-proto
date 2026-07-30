//! **Overlap enumeration at cut seams** — the multi-sum-shaped enumerator
//! (`proposal-sequent-kernel.md` §7.3.2; VDC addendum §7.4).
//!
//! For each ordered pair of cells the enumerator returns the *complete family*
//! of unifications at a cut, never a single chosen one (§7.3.2: "enumerate
//! every overlap"; the fan-out is the mathematically-forced multi-sum of the
//! addendum §7.4). Two flavors, both rooted at the seam ("cuts make overlaps
//! shallow — the seam is the root"):
//!
//! - [`OverlapKind::Confluence`] — the Knuth–Bendix **critical pair**: the two
//!   left-hand sides unify, so the peak `σ(lₐ)` reduces two ways (apply left,
//!   apply right). Completion (`crate::completion`) normalizes both reducts and
//!   either joins them (a coherence [`crate::tracelet::Tracelet`]) or orients
//!   the pair into a new cell.
//! - [`OverlapKind::Composition`] — the Behr–Harmer–Krivine **sequential
//!   composition**: the left cell's right-hand side unifies (at a command
//!   position) with the right cell's left-hand side, so applying left then
//!   right is a two-step derivation whose composite is the **fused cell**
//!   `σ(lₐ) ~> (σ(rₐ) with the right cell fired at the seam)` — deforestation
//!   as a derived 2-cell (§7.2), certified by the fused≡two-step tracelet.
//!
//! Both are single root/seam unifications, which is exactly why the enumeration
//! is tractable where tree rewriting would need full subterm traversal
//! (§7.3.2).

use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::boundary::PrimeNameRef;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::pattern::CmdPat;
use crate::pattern::ConsPat;
use crate::pattern::MetaVar;
use crate::pattern::Node;
use crate::pattern::Pos;
use crate::pattern::ProdPat;
use crate::pattern::collect_cmd_metavars;
use crate::pattern::splice_at;
use crate::pattern::subterm_at;
use crate::pattern::transform_node;
use crate::rewrite::command_positions;
use crate::subst::Subst;
use crate::subst::unify_cmd;

/// The kind of overlap (`proposal-sequent-kernel.md` §7.2–§7.3.2).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OverlapKind
{
    /// The two left-hand sides unify — a confluence critical pair (§7.3.3).
    Confluence,
    /// The left right-hand side unifies with the right left-hand side — a
    /// sequential composition / fusion overlap (§7.2).
    Composition,
}

/// One **overlap** between two cells at a seam (`proposal-sequent-kernel.md`
/// §7.3, the `Overlap` struct).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Overlap
{
    /// The left cell.
    pub left: CellId,
    /// The right cell.
    pub right: CellId,
    /// The overlap kind.
    pub kind: OverlapKind,
    /// The most general unifier at the seam.
    pub unifier: Subst,
    /// The seam position (the root for a confluence overlap; the command
    /// position in the left right-hand side for a composition overlap).
    pub seam: Pos,
    /// The superposition term `σ(lₐ)` the overlap is rooted at.
    pub peak: CmdPat,
    /// The right cell renamed apart from the left (so unification kept the two
    /// cells' metavariables disjoint); the reduct methods contract against it.
    right_renamed: Cell,
}

impl Overlap
{
    /// The left reduct — the peak contracted by the **left** cell at the root.
    ///
    /// # Contract
    /// - ensures: `Some(σ(rₗₑ𝒻ₜ))`, the one-step contraction of the peak by the
    ///   left cell; `None` only if the left id is stale.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn left_reduct(
        &self,
        store: &CellStore,
    ) -> Option<CmdPat>
    {
        let left = store.get(self.left)?;
        Some(self.unifier.apply_cmd(&left.rhs))
    }

    /// The right reduct of a **confluence** overlap — the peak contracted by
    /// the **right** cell at the root.
    ///
    /// # Contract
    /// - requires: `self.kind == OverlapKind::Confluence`.
    /// - ensures: `Some(σ(rᵣᵢ𝓰ₕₜ))`, the other one-step contraction of the
    ///   peak; the confluence critical pair is `(left_reduct, right_reduct)`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn right_reduct(
        &self,
        _store: &CellStore,
    ) -> Option<CmdPat>
    {
        Some(self.unifier.apply_cmd(&self.right_renamed.rhs))
    }

    /// The right cell **renamed apart** from the left — the span's right leg
    /// (TA-8: the enumerator's seam data, consumed by the crDC suite, the
    /// convolution face, and the overlap-support cache).
    ///
    /// The enumerator renames the right cell's metavariables until they are
    /// disjoint from the left cell's before unifying, so the unifier's bindings
    /// on the right side read against this renamed cell, never the original.
    /// The renaming is structure-preserving (same pattern shapes, occurrence-
    /// parallel metavariable lists), so the original right leg is recoverable
    /// by zipping the two cells' metavariable occurrences.
    ///
    /// # Contract
    /// - ensures: the apartness-renamed right cell the [`Self::unifier`] and
    ///   [`Self::right_reduct`] / [`Self::composite`] reducts contract against.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn right_renamed(&self) -> &Cell
    {
        &self.right_renamed
    }

    /// The composite of a **composition** overlap — apply the left cell at the
    /// root, then the right cell at the seam (the fused cell's right-hand
    /// side).
    ///
    /// # Contract
    /// - requires: `self.kind == OverlapKind::Composition`.
    /// - ensures: `Some(result)`, the two-step contraction `peak → (left) →
    ///   (right at seam)`; the fused cell is `peak ~> result`. `None` if the
    ///   seam no longer addresses a command after the left contraction (never,
    ///   for a well-formed composition overlap).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn composite(
        &self,
        store: &CellStore,
    ) -> Option<CmdPat>
    {
        let after_left = self.left_reduct(store)?;
        let right_rhs = self.unifier.apply_cmd(&self.right_renamed.rhs);
        match splice_at(&Node::Cmd(after_left), &self.seam, Node::Cmd(right_rhs))? {
            | Node::Cmd(result) => Some(result),
            | Node::Prod(_) | Node::Cons(_) => None,
        }
    }
}

/// Enumerate the **complete family** of overlaps among the store's cells
/// (`proposal-sequent-kernel.md` §7.3.2).
///
/// # Contract
/// - ensures: every confluence overlap (a unifiable left/left pair, excluding a
///   cell with itself) and every composition overlap (a unifiable right/left
///   pair at a command seam) among the store's cells, in a deterministic order.
///   The list is the multi-sum family — one entry per unifier, never collapsed.
/// - panics: none.
#[inline]
#[must_use]
pub fn enumerate_overlaps(store: &CellStore) -> Vec<Overlap>
{
    let mut out = Vec::new();
    for (id_left, cell_left) in store.iter() {
        for (id_right, cell_right) in store.iter() {
            let avoid = cmd_var_names(&cell_left.lhs, &cell_left.rhs);
            let renamed = rename_cell_apart(cell_right, &avoid);
            // Confluence: unify the two left-hand sides (skip the trivial
            // self-overlap, whose reducts are identical by construction).
            if id_left != id_right {
                let mut unifier = Subst::new();
                if bool::from(unify_cmd(&cell_left.lhs, &renamed.lhs, &mut unifier)) {
                    let peak = unifier.apply_cmd(&cell_left.lhs);
                    out.push(Overlap {
                        left: id_left,
                        right: id_right,
                        kind: OverlapKind::Confluence,
                        unifier,
                        seam: Pos::root(),
                        peak,
                        right_renamed: renamed.clone(),
                    });
                }
            }
            // Composition: unify the left RHS (at each command position) with the
            // right LHS.
            for seam in command_positions(&cell_left.rhs) {
                let Some(Node::Cmd(sub)) = subterm_at(&Node::Cmd(cell_left.rhs.clone()), &seam)
                else {
                    continue;
                };
                let mut unifier = Subst::new();
                if bool::from(unify_cmd(&sub, &renamed.lhs, &mut unifier)) {
                    let peak = unifier.apply_cmd(&cell_left.lhs);
                    out.push(Overlap {
                        left: id_left,
                        right: id_right,
                        kind: OverlapKind::Composition,
                        unifier,
                        seam,
                        peak,
                        right_renamed: renamed.clone(),
                    });
                }
            }
        }
    }
    out
}

/// Set of metavariable names reserved during apartness renaming.
#[repr(transparent)]
struct MetaVarNameSet(BTreeSet<Box<str>>);

/// Freshened metavariable name produced by deterministic priming.
#[repr(transparent)]
struct FreshMetaVarName(Box<str>);

/// The set of metavariable names occurring in two command patterns.
///
/// # Contract
/// - ensures: every metavariable name of `lhs` and `rhs`, deduplicated.
/// - panics: none.
#[inline]
fn cmd_var_names(
    lhs: &CmdPat,
    rhs: &CmdPat,
) -> MetaVarNameSet
{
    let mut occurrences = Vec::new();
    collect_cmd_metavars(lhs, &mut occurrences);
    collect_cmd_metavars(rhs, &mut occurrences);
    MetaVarNameSet(occurrences.into_iter().map(|mv| mv.name).collect())
}

/// Rename a cell's metavariables to be disjoint from `avoid`, priming each name
/// until fresh (a deterministic apartness renaming).
///
/// # Contract
/// - ensures: the returned cell is structurally the input with every
///   metavariable name replaced by a name absent from `avoid` and from the
///   other renamed names, preserving each metavariable's category and the
///   pattern shape; a cell already disjoint from `avoid` is unchanged.
/// - panics: none.
#[inline]
fn rename_cell_apart(
    cell: &Cell,
    avoid: &MetaVarNameSet,
) -> Cell
{
    let mut occurrences = Vec::new();
    collect_cmd_metavars(&cell.lhs, &mut occurrences);
    collect_cmd_metavars(&cell.rhs, &mut occurrences);
    let mut mapping: Vec<(MetaVar, MetaVar)> = Vec::new();
    let mut taken: BTreeSet<Box<str>> = avoid.0.clone();
    for mv in &occurrences {
        let mut already_mapped = false;
        for pair in &mapping {
            if pair.0 == *mv {
                already_mapped = true;
                break;
            }
        }
        if already_mapped {
            continue;
        }
        let mut fresh = mv.name.clone();
        while taken.contains(&fresh) {
            fresh = prime(fresh.as_ref().into()).0;
        }
        taken.insert(fresh.clone());
        mapping.push((mv.clone(), MetaVar {
            name: fresh,
            cat: mv.cat,
        }));
    }
    Cell::new(
        rename_cmd(&cell.lhs, &mapping),
        rename_cmd(&cell.rhs, &mapping),
        cell.orient,
        cell.provenance,
    )
}

/// Append a prime `'` to a name (the fresh-name step of [`rename_cell_apart`]).
///
/// # Contract
/// - ensures: a strictly longer name, so priming terminates against any finite
///   `avoid` set.
/// - panics: none.
#[inline]
fn prime(name: PrimeNameRef<'_>) -> FreshMetaVarName
{
    let name = name.as_ref();
    let mut out = alloc::string::String::with_capacity(name.len().saturating_add(1));
    out.push_str(name);
    out.push('\'');
    FreshMetaVarName(out.into_boxed_str())
}

/// Rename a command pattern's metavariables through `mapping`.
///
/// # Contract
/// - ensures: every metavariable in `mapping`'s domain is replaced by its
///   image; others are left in place.
/// - panics: none.
#[inline]
fn rename_cmd(
    cmd: &CmdPat,
    mapping: &[(MetaVar, MetaVar)],
) -> CmdPat
{
    let Some(Node::Cmd(cmd)) = transform_node(Node::Cmd(cmd.clone()), |node| {
        Some(rename_node(node, mapping))
    })
    else {
        return cmd.clone();
    };
    cmd
}

/// Rename one rebuilt node.
#[inline]
fn rename_node(
    node: Node,
    mapping: &[(MetaVar, MetaVar)],
) -> Node
{
    match node {
        | Node::Prod(ProdPat::Meta(mv)) => Node::Prod(ProdPat::Meta(rename_var(&mv, mapping))),
        | Node::Cons(ConsPat::Meta(mv)) => Node::Cons(ConsPat::Meta(rename_var(&mv, mapping))),
        | other => other,
    }
}

/// Look a metavariable up in `mapping`, returning its image or the variable
/// unchanged.
///
/// # Contract
/// - ensures: the mapped variable when present, else the input clone.
/// - panics: none.
#[inline]
fn rename_var(
    mv: &MetaVar,
    mapping: &[(MetaVar, MetaVar)],
) -> MetaVar
{
    for pair in mapping {
        if pair.0 == *mv {
            return pair.1.clone();
        }
    }
    mv.clone()
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;

    use super::*;
    use crate::cell::CellProvenance;
    use crate::cell::Orientation;
    use crate::cell::frame_defining_cell;
    use crate::pattern::Sym;

    #[test]
    fn the_frame_and_add_cells_compose_into_the_commutation_cell()
    {
        // (Succ⁻-def) then (add-S) compose to Succ⁻(add(n;α)) ~> add(n;Succ⁻(α)).
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let overlaps = enumerate_overlaps(&store);
        let composition = overlaps
            .iter()
            .find(|o| o.kind == OverlapKind::Composition && o.left == frame && o.right == add)
            .expect("the frame RHS ⟨Succ(v)|β⟩ unifies with add-S's LHS ⟨Succ(m)|add(n;α)⟩");
        let peak = &composition.peak;
        let composite = composition.composite(&store).expect("the composite exists");
        // Peak: ⟨v | Succ⁻(add(n;α))⟩ ; composite: ⟨v | add(n; Succ⁻(α))⟩.
        assert!(
            matches!(peak, CmdPat::Cut {
                cons: ConsPat::Frame { .. },
                ..
            }),
            "the peak feeds a Succ⁻ frame into add"
        );
        assert!(
            matches!(composite, CmdPat::Cut {
                cons: ConsPat::Op { .. },
                ..
            }),
            "the composite drops the intermediate Succ, leaving add with a pushed-in frame"
        );
    }

    #[test]
    fn overlaps_are_a_deterministic_family()
    {
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        store.insert(add_s());
        let first = enumerate_overlaps(&store);
        let second = enumerate_overlaps(&store);
        assert_eq!(first, second, "enumeration is deterministic");
    }

    /// (add-S): ⟨Succ(m) | add(n; α)⟩ ~> ⟨m | add(n; Succ⁻(α))⟩.
    fn add_s() -> Cell
    {
        let lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
        );
        let rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("m"),
            ConsPat::op(
                "add",
                [ProdPat::meta("n")],
                ConsPat::frame("Succ", ConsPat::meta("alpha")),
            ),
        );
        Cell::new(
            lhs,
            rhs,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }
}
