//! Deterministic structural diffing for flat Gandr CST arenas.

use crate::Cst;
use crate::Material;
use crate::MoldPayload;
use crate::NodeId;
use crate::NodeKind;
use crate::StableHash;

/// Predicate result for equal-hash candidate checks.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct HashCandidate(bool);

impl From<HashCandidate> for bool
{
    #[inline]
    fn from(value: HashCandidate) -> Self
    {
        value.0
    }
}

/// Predicate result for structural equality verification.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct StructuralEquality(bool);

impl From<StructuralEquality> for bool
{
    #[inline]
    fn from(value: StructuralEquality) -> Self
    {
        value.0
    }
}

/// Predicate result for source-text equality.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct TextEquality(bool);

impl From<TextEquality> for bool
{
    #[inline]
    fn from(value: TextEquality) -> Self
    {
        value.0
    }
}

/// Predicate result for child-signature alignment.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ChildAlignment(bool);

impl From<ChildAlignment> for bool
{
    #[inline]
    fn from(value: ChildAlignment) -> Self
    {
        value.0
    }
}

/// A pair of subtree roots proven equal by the CST stable hash.
///
/// # Contract
/// - ensures: `old_root` names the matched root in the old tree and `new_root`
///   names the matched root in the new tree.
/// - provides: the pruning witness emitted by [`diff`] for an unchanged
///   significant subtree.
/// - panics: none.
/// - intension: a match represents one whole pruned subtree; descendants are
///   intentionally not emitted as separate matches.
///
/// # Adequacy
/// - hypothesis: L3 only — root-equal, nested-equal, and hash-collision-shaped
///   inputs distinguish pair orientation, pruning granularity, and debug
///   verification routing.
/// - witness: downstream integration tests exercise the public diff contract
///   once the sibling model and builder modules land.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubtreeMatch
{
    /// Matched root in the old CST.
    old_root: NodeId,
    /// Matched root in the new CST.
    new_root: NodeId,
}

impl SubtreeMatch
{
    /// Create a matched-subtree witness.
    #[inline]
    #[must_use]
    pub const fn new(
        old_root: NodeId,
        new_root: NodeId,
    ) -> Self
    {
        Self { old_root, new_root }
    }

    /// Return the matched root in the old CST.
    #[inline]
    #[must_use]
    pub const fn old_root(self) -> NodeId
    {
        self.old_root
    }

    /// Return the matched root in the new CST.
    #[inline]
    #[must_use]
    pub const fn new_root(self) -> NodeId
    {
        self.new_root
    }
}

/// Top-down CST diff result.
///
/// # Contract
/// - ensures: `matches` contains hash-pruned equal subtree roots in traversal
///   discovery order.
/// - ensures: `unmatched_old` and `unmatched_new` contain conservative subtree
///   roots that could not be aligned or read.
/// - provides: a read-only summary of structural agreement and disagreement
///   between two CST arenas.
/// - panics: none.
/// - intension: equal subtrees are pruned before child alignment; differing
///   interiors align significant children by deterministic LCS with right-side
///   tie advancement.
///
/// # Adequacy
/// - hypothesis: L4 only — whitespace-only edits, single-token replacements,
///   sibling reordering, and malformed-node fixtures distinguish every public
///   projection and the conservative failure policy.
/// - witness: downstream integration tests exercise the public diff contract
///   once the sibling model and builder modules land.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Diff
{
    /// Hash-pruned subtree root pairs.
    matches: Vec<SubtreeMatch>,
    /// Old-side subtree roots not aligned to a new-side subtree.
    unmatched_old: Vec<NodeId>,
    /// New-side subtree roots not aligned to an old-side subtree.
    unmatched_new: Vec<NodeId>,
}

impl Diff
{
    /// Create an empty diff accumulator.
    #[must_use]
    fn new() -> Self
    {
        Self::default()
    }

    /// Return matched subtree roots.
    #[inline]
    #[must_use]
    pub fn matches(&self) -> &[SubtreeMatch]
    {
        &self.matches
    }

    /// Return old-side unmatched subtree roots.
    #[inline]
    #[must_use]
    pub fn unmatched_old(&self) -> &[NodeId]
    {
        &self.unmatched_old
    }

    /// Return new-side unmatched subtree roots.
    #[inline]
    #[must_use]
    pub fn unmatched_new(&self) -> &[NodeId]
    {
        &self.unmatched_new
    }

    /// Record a pruned subtree match.
    fn push_match(
        &mut self,
        old_root: NodeId,
        new_root: NodeId,
    )
    {
        self.matches.push(SubtreeMatch::new(old_root, new_root));
    }

    /// Record an unmatched old-side subtree root.
    fn push_unmatched_old(
        &mut self,
        root: NodeId,
    )
    {
        self.unmatched_old.push(root);
    }

    /// Record an unmatched new-side subtree root.
    fn push_unmatched_new(
        &mut self,
        root: NodeId,
    )
    {
        self.unmatched_new.push(root);
    }
}

/// Compute a deterministic, whitespace-insensitive CST diff.
///
/// # Contract
/// - requires: `old` and `new` are CST arenas produced by the syntax builder.
/// - ensures: equal `(kind, payload, hash)` subtree roots emit one
///   [`SubtreeMatch`] and prune traversal of that subtree.
/// - ensures: differing interior nodes align non-space children by LCS over
///   `(kind, payload, hash)` and recurse only into aligned child pairs.
/// - provides: unmatched roots for unaligned or unreadable subtrees without
///   exposing mutable CST storage.
/// - fails: malformed or unreadable nodes are represented conservatively as
///   unmatched roots because this API returns [`Diff`] rather than `Result`.
/// - panics: none in production; debug builds assert the crate-private
///   significant-content verifier for equal-hash candidates.
/// - intension: traversal is iterative and top-down; LCS ties advance the
///   new/right side first.
///
/// # Adequacy
/// - hypothesis: L4 only — unchanged roots, whitespace-only changes,
///   single-token edits, and ambiguous equal-length child alignments
///   distinguish pruning, space filtering, statement-local unmatched roots, and
///   the tie break.
/// - witness: downstream integration tests exercise the public diff contract
///   once the sibling model and builder modules land.
#[inline]
#[must_use]
pub fn diff(
    old: &Cst,
    new: &Cst,
) -> Diff
{
    let mut result = Diff::new();
    let mut work = Vec::new();
    work.push((old.root(), new.root()));

    while let Some((old_root, new_root)) = work.pop() {
        diff_pair(old, new, old_root, new_root, &mut result, &mut work);
    }

    result
}

/// Process one worklist pair.
fn diff_pair(
    old: &Cst,
    new: &Cst,
    old_root: NodeId,
    new_root: NodeId,
    result: &mut Diff,
    work: &mut Vec<(NodeId, NodeId)>,
)
{
    let old_data = match old.node_data(old_root) {
        | Ok(data) => data,
        | Err(_error) => {
            result.push_unmatched_old(old_root);
            result.push_unmatched_new(new_root);
            return;
        },
    };

    let new_data = match new.node_data(new_root) {
        | Ok(data) => data,
        | Err(_error) => {
            result.push_unmatched_old(old_root);
            result.push_unmatched_new(new_root);
            return;
        },
    };

    if bool::from(same_hash_candidate(
        old_data.kind(),
        old_data.payload(),
        old_data.hash(),
        new_data.kind(),
        new_data.payload(),
        new_data.hash(),
    )) {
        debug_assert!(
            bool::from(verified_equal(old, old_root, new, new_root)),
            "equal CST hashes must agree on significant structure"
        );
        result.push_match(old_root, new_root);
        return;
    }

    if old_data.kind() == NodeKind::Token || new_data.kind() == NodeKind::Token {
        result.push_unmatched_old(old_root);
        result.push_unmatched_new(new_root);
        return;
    }

    let old_children = significant_children(old, old_root, &mut result.unmatched_old);
    let new_children = significant_children(new, new_root, &mut result.unmatched_new);
    let aligned = lcs_pairs(&old_children, &new_children);
    mark_unaligned(&old_children, &new_children, &aligned, result);
    push_aligned_work(&aligned, work);
}

/// Return whether two nodes have the same hash-comparison key.
fn same_hash_candidate(
    old_kind: NodeKind,
    old_payload: MoldPayload,
    old_hash: StableHash,
    new_kind: NodeKind,
    new_payload: MoldPayload,
    new_hash: StableHash,
) -> HashCandidate
{
    HashCandidate(old_kind == new_kind && old_payload == new_payload && old_hash == new_hash)
}

/// Verify significant subtree equality with an iterative structural walk.
fn verified_equal(
    old: &Cst,
    old_root: NodeId,
    new: &Cst,
    new_root: NodeId,
) -> StructuralEquality
{
    let mut work = Vec::new();
    work.push((old_root, new_root));

    while let Some((old_id, new_id)) = work.pop() {
        let old_data = match old.node_data(old_id) {
            | Ok(data) => data,
            | Err(_error) => return StructuralEquality(false),
        };
        let new_data = match new.node_data(new_id) {
            | Ok(data) => data,
            | Err(_error) => return StructuralEquality(false),
        };

        if old_data.kind() != new_data.kind()
            || old_data.material() != new_data.material()
            || old_data.payload() != new_data.payload()
            || old_data.hash() != new_data.hash()
        {
            return StructuralEquality(false);
        }

        if old_data.material() == Material::Tile && !bool::from(same_text(old, old_id, new, new_id))
        {
            return StructuralEquality(false);
        }

        if old_data.kind() == NodeKind::Token {
            continue;
        }

        let Some(old_children) = significant_child_ids(old, old_id)
        else {
            return StructuralEquality(false);
        };
        let Some(new_children) = significant_child_ids(new, new_id)
        else {
            return StructuralEquality(false);
        };

        if old_children.len() != new_children.len() {
            return StructuralEquality(false);
        }

        for (old_child, new_child) in old_children
            .iter()
            .copied()
            .zip(new_children.iter().copied())
        {
            work.push((old_child, new_child));
        }
    }

    StructuralEquality(true)
}

/// Return whether two tile nodes expose identical source text.
fn same_text(
    old: &Cst,
    old_id: NodeId,
    new: &Cst,
    new_id: NodeId,
) -> TextEquality
{
    let old_data = match old.node_data(old_id) {
        | Ok(data) => data,
        | Err(_error) => return TextEquality(false),
    };
    let new_data = match new.node_data(new_id) {
        | Ok(data) => data,
        | Err(_error) => return TextEquality(false),
    };

    match (
        old.text_for_data(old_id, *old_data),
        new.text_for_data(new_id, *new_data),
    ) {
        | (Ok(old_text), Ok(new_text)) => TextEquality(old_text == new_text),
        | (Err(_error), _) | (_, Err(_error)) => TextEquality(false),
    }
}

/// Collect significant child ids for structural verification.
fn significant_child_ids(
    cst: &Cst,
    root: NodeId,
) -> Option<Vec<NodeId>>
{
    let mut significant = Vec::new();
    let data = match cst.node_data(root) {
        | Ok(data) => data,
        | Err(_error) => return None,
    };
    let children = match cst.children_for_data(root, *data) {
        | Ok(children) => children,
        | Err(_error) => return None,
    };

    for child in children.iter().copied() {
        let child_data = match cst.node_data(child) {
            | Ok(child_data) => child_data,
            | Err(_error) => return None,
        };
        if child_data.material() != Material::Space {
            significant.push(child);
        }
    }

    Some(significant)
}

/// A significant child and its LCS key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChildSig
{
    /// Child node identity.
    id: NodeId,
    /// Child syntactic kind.
    kind: NodeKind,
    /// Child material-governed mold payload.
    payload: MoldPayload,
    /// Child stable hash.
    hash: StableHash,
}

impl ChildSig
{
    /// Return whether two child signatures align exactly.
    fn aligns_with(
        self,
        other: Self,
    ) -> ChildAlignment
    {
        ChildAlignment(
            self.kind == other.kind && self.payload == other.payload && self.hash == other.hash,
        )
    }
}

/// Collect non-space children, marking unreadable children unmatched.
fn significant_children(
    cst: &Cst,
    root: NodeId,
    unreadable: &mut Vec<NodeId>,
) -> Vec<ChildSig>
{
    let data = match cst.node_data(root) {
        | Ok(data) => data,
        | Err(_error) => {
            unreadable.push(root);
            return Vec::new();
        },
    };
    let children = match cst.children_for_data(root, *data) {
        | Ok(children) => children,
        | Err(_error) => {
            unreadable.push(root);
            return Vec::new();
        },
    };
    let mut significant = Vec::new();

    for child in children.iter().copied() {
        match cst.node_data(child) {
            | Ok(child_data) => {
                if child_data.material() != Material::Space {
                    significant.push(ChildSig {
                        id: child,
                        kind: child_data.kind(),
                        payload: child_data.payload(),
                        hash: child_data.hash(),
                    });
                }
            },
            | Err(_error) => unreadable.push(child),
        }
    }

    significant
}

/// A pair of aligned child-vector positions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AlignedPos
{
    /// Old child position.
    old_pos: usize,
    /// New child position.
    new_pos: usize,
    /// Old child node identity.
    old_id: NodeId,
    /// New child node identity.
    new_id: NodeId,
}

/// Compute LCS child alignments over `(kind, payload, hash)`.
fn lcs_pairs(
    old: &[ChildSig],
    new: &[ChildSig],
) -> Vec<AlignedPos>
{
    let Some(width) = new.len().checked_add(1)
    else {
        return Vec::new();
    };
    let Some(height) = old.len().checked_add(1)
    else {
        return Vec::new();
    };
    let Some(cells) = width.checked_mul(height)
    else {
        return Vec::new();
    };
    let mut table = vec![0usize; cells];
    let table_index = |row: usize, col: usize| {
        let row_offset = row.checked_mul(width)?;
        row_offset.checked_add(col)
    };

    let mut old_cursor = old.len();
    while old_cursor > 0 {
        old_cursor = match old_cursor.checked_sub(1) {
            | Some(value) => value,
            | None => return Vec::new(),
        };
        let mut new_cursor = new.len();
        while new_cursor > 0 {
            new_cursor = match new_cursor.checked_sub(1) {
                | Some(value) => value,
                | None => return Vec::new(),
            };
            let old_child = match old.get(old_cursor) {
                | Some(child) => *child,
                | None => return Vec::new(),
            };
            let new_child = match new.get(new_cursor) {
                | Some(child) => *child,
                | None => return Vec::new(),
            };
            let value = if bool::from(old_child.aligns_with(new_child)) {
                let Some(row) = old_cursor.checked_add(1)
                else {
                    return Vec::new();
                };
                let Some(col) = new_cursor.checked_add(1)
                else {
                    return Vec::new();
                };
                let Some(index) = table_index(row, col)
                else {
                    return Vec::new();
                };
                let Some(tail) = table.get(index).copied()
                else {
                    return Vec::new();
                };
                let Some(sum) = tail.checked_add(1)
                else {
                    return Vec::new();
                };
                sum
            }
            else {
                let Some(old_row) = old_cursor.checked_add(1)
                else {
                    return Vec::new();
                };
                let Some(new_col) = new_cursor.checked_add(1)
                else {
                    return Vec::new();
                };
                let Some(advance_old_index) = table_index(old_row, new_cursor)
                else {
                    return Vec::new();
                };
                let Some(advance_new_index) = table_index(old_cursor, new_col)
                else {
                    return Vec::new();
                };
                let Some(advance_old) = table.get(advance_old_index).copied()
                else {
                    return Vec::new();
                };
                let Some(advance_new) = table.get(advance_new_index).copied()
                else {
                    return Vec::new();
                };
                advance_old.max(advance_new)
            };
            let Some(index) = table_index(old_cursor, new_cursor)
            else {
                return Vec::new();
            };
            if let Some(slot) = table.get_mut(index) {
                *slot = value;
            }
        }
    }

    let mut aligned = Vec::new();
    let mut old_pos = 0usize;
    let mut new_pos = 0usize;

    while old_pos < old.len() && new_pos < new.len() {
        let old_child = match old.get(old_pos) {
            | Some(child) => *child,
            | None => return aligned,
        };
        let new_child = match new.get(new_pos) {
            | Some(child) => *child,
            | None => return aligned,
        };
        if bool::from(old_child.aligns_with(new_child)) {
            aligned.push(AlignedPos {
                old_pos,
                new_pos,
                old_id: old_child.id,
                new_id: new_child.id,
            });
            old_pos = match old_pos.checked_add(1) {
                | Some(value) => value,
                | None => return aligned,
            };
            new_pos = match new_pos.checked_add(1) {
                | Some(value) => value,
                | None => return aligned,
            };
            continue;
        }

        let Some(old_row) = old_pos.checked_add(1)
        else {
            return aligned;
        };
        let Some(new_col) = new_pos.checked_add(1)
        else {
            return aligned;
        };
        let Some(advance_old_index) = table_index(old_row, new_pos)
        else {
            return aligned;
        };
        let Some(advance_new_index) = table_index(old_pos, new_col)
        else {
            return aligned;
        };
        let Some(advance_old) = table.get(advance_old_index).copied()
        else {
            return aligned;
        };
        let Some(advance_new) = table.get(advance_new_index).copied()
        else {
            return aligned;
        };

        if advance_old > advance_new {
            old_pos = match old_pos.checked_add(1) {
                | Some(value) => value,
                | None => return aligned,
            };
        }
        else {
            new_pos = match new_pos.checked_add(1) {
                | Some(value) => value,
                | None => return aligned,
            };
        }
    }

    aligned
}

/// Mark all significant children not selected by the LCS as unmatched roots.
fn mark_unaligned(
    old: &[ChildSig],
    new: &[ChildSig],
    aligned: &[AlignedPos],
    result: &mut Diff,
)
{
    let mut old_cursor = aligned.iter().copied();
    let mut old_current = old_cursor.next();

    for (old_pos, old_child) in old.iter().copied().enumerate() {
        if old_current.is_some_and(|pair| pair.old_pos == old_pos) {
            old_current = old_cursor.next();
            continue;
        }
        result.push_unmatched_old(old_child.id);
    }

    let mut new_cursor = aligned.iter().copied();
    let mut new_current = new_cursor.next();

    for (new_pos, new_child) in new.iter().copied().enumerate() {
        if new_current.is_some_and(|pair| pair.new_pos == new_pos) {
            new_current = new_cursor.next();
            continue;
        }
        result.push_unmatched_new(new_child.id);
    }
}

/// Push aligned child pairs onto the worklist in source order.
fn push_aligned_work(
    aligned: &[AlignedPos],
    work: &mut Vec<(NodeId, NodeId)>,
)
{
    for pair in aligned.iter().rev().copied() {
        work.push((pair.old_id, pair.new_id));
    }
}
