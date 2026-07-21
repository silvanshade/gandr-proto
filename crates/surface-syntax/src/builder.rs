//! Checked construction for the flat gandr CST arena.

use core::error::Error;
use core::fmt;
use core::num::TryFromIntError;

use crate::model::ChildCount;
use crate::model::ChildStart;
use crate::model::Cst;
use crate::model::GrammarFingerprint;
use crate::model::HashByte;
use crate::model::Material;
use crate::model::MoldPayload;
use crate::model::NodeData;
use crate::model::NodeId;
use crate::model::NodeIndex;
use crate::model::NodeKind;
use crate::model::SignificantChildCount;
use crate::model::SourceSlice;
use crate::model::SourceText;
use crate::model::StableHash;
use crate::model::TextRange;

/// FNV-1a offset basis for the stable CST hash.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a prime for the stable CST hash.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Frame byte for a space token hash.
const FRAME_SPACE: u8 = b'S';

/// Frame byte for a grout token hash.
const FRAME_GROUT: u8 = b'G';

/// Frame byte for a tile token hash.
const FRAME_TILE: u8 = b'T';

/// Frame byte for an interior node hash.
const FRAME_INTERIOR: u8 = b'I';

/// Errors surfaced by checked CST construction and read-side arena validation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "BuildError enumerates the closed, contract-fixed set of arena-construction and validation failures"
)]
pub enum BuildError
{
    /// A text range ended before it started or exceeded the source length.
    InvalidTextRange
    {
        start: usize, end: usize
    },
    /// A node's text range crossed a UTF-8 boundary.
    InvalidTextBoundary
    {
        node: NodeId,
        start: usize,
        end: usize,
    },
    /// A model accessor was asked for a node outside the arena.
    NodeOutOfBounds
    {
        id: NodeId, len: usize
    },
    /// A model accessor found a malformed flattened child span.
    ChildRangeOutOfBounds
    {
        node: NodeId,
        start: usize,
        len: usize,
        children_len: usize,
    },
    /// The model arena length cannot be represented by dense node ids.
    NodeCountOverflow
    {
        len: usize
    },
    /// A material carried a mold payload of the wrong variant.
    MoldPayloadMismatch
    {
        material: Material
    },
    /// An interior node was requested with `NodeKind::Token`.
    TokenKindForInterior,
    /// A non-token node was requested with tile material.
    TileInterior
    {
        kind: NodeKind
    },
    /// A child appeared more than once in a single parent request.
    DuplicateChild
    {
        child: NodeId
    },
    /// A child already belonged to another parent.
    ChildAlreadyParented
    {
        child: NodeId, parent: NodeId
    },
    /// A flattened child span exceeded the stored `u32` coordinate space.
    TooManyChildren
    {
        count: usize
    },
    /// A machine integer conversion required by arena construction failed.
    IntegerConversion,
    /// The requested root node does not exist.
    UnknownRoot
    {
        root: NodeId
    },
    /// The requested root is already owned by another parent.
    RootHasParent
    {
        root: NodeId, parent: NodeId
    },
    /// A non-root node was left unreachable from the root ownership tree.
    OrphanNode
    {
        node: NodeId
    },
}

impl fmt::Display for BuildError
{
    #[inline]
    #[expect(
        clippy::use_debug,
        reason = "the error text embeds the NodeId, Material, and NodeKind identity fields via their Debug forms, which are the canonical inspectable identities; a bespoke Display would duplicate that vocabulary and risk drifting from the arena types"
    )]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::InvalidTextRange { start, end } => {
                write!(
                    f,
                    "text range {start}..{end} is out of order or outside the source"
                )
            },
            | Self::InvalidTextBoundary { node, start, end } => write!(
                f,
                "node {:?} text range {start}..{end} does not lie on UTF-8 boundaries",
                node,
            ),
            | Self::NodeOutOfBounds { id, len } => {
                write!(f, "node {:?} is outside arena length {len}", id)
            },
            | Self::ChildRangeOutOfBounds {
                node,
                start,
                len,
                children_len,
            } => write!(
                f,
                "node {:?} child span {start}+{len} exceeds flattened child length {children_len}",
                node,
            ),
            | Self::NodeCountOverflow { len } => {
                write!(
                    f,
                    "arena length {len} cannot be represented by u32 node ids"
                )
            },
            | Self::MoldPayloadMismatch { material } => write!(
                f,
                "material {material:?} carries a mold payload of the wrong variant: Space requires MoldPayload::Space, Grout requires MoldPayload::Grout, and Tile requires MoldPayload::Tile",
            ),
            | Self::TokenKindForInterior => {
                f.write_str("interior node construction cannot use NodeKind::Token")
            },
            | Self::TileInterior { kind } => {
                write!(f, "interior node {kind:?} cannot use tile material")
            },
            | Self::DuplicateChild { child } => {
                write!(f, "child {:?} appears more than once", child)
            },
            | Self::ChildAlreadyParented { child, parent } => {
                write!(f, "child {:?} is already parented by {:?}", child, parent)
            },
            | Self::TooManyChildren { count } => write!(
                f,
                "flattened child arena has {count} entries, exceeding u32 spans",
            ),
            | Self::IntegerConversion => f.write_str("checked integer conversion failed"),
            | Self::UnknownRoot { root } => write!(f, "root {:?} does not exist", root),
            | Self::RootHasParent { root, parent } => {
                write!(f, "root {:?} is already parented by {:?}", root, parent)
            },
            | Self::OrphanNode { node } => {
                write!(f, "node {:?} has no parent and is not the root", node)
            },
        }
    }
}

impl Error for BuildError
{
}

impl From<TryFromIntError> for BuildError
{
    #[inline]
    fn from(_: TryFromIntError) -> Self
    {
        Self::IntegerConversion
    }
}

/// Checked builder for the flattened CST arena.
///
/// # Contract
/// - requires: callers add children before the parent that owns them.
/// - ensures: allocated node ids are dense `u32` identities in construction
///   order.
/// - provides: a single-parent CST whose nodes, flattened children, and hashes
///   are validated before `finish` returns.
/// - fails: returns `BuildError` for invalid ranges, malformed token/interior
///   combinations, unknown children, duplicate parents, integer overflow, or an
///   incomplete parent closure.
/// - panics: none.
/// - intension: construction is append-only; `finish` performs an iterative
///   ownership-closure check over the dense parent state.
///
/// # Adequacy
/// - hypothesis: L3 — each invariant has a boundary witness at the violated
///   constructor call and a success witness through model accessors.
/// - witness: `gandr_surface_syntax::builder::tests::builder_rejects_malformed_ranges`
/// - witness: `gandr_surface_syntax::builder::tests::builder_round_trips_tile_text`
/// - witness: `gandr_surface_syntax::builder::tests::builder_rejects_duplicate_parents`
pub struct CstBuilder
{
    /// Source text shared with the final CST.
    source: SourceText,
    /// Dense node arena under construction.
    nodes: Vec<NodeData>,
    /// Flattened child storage under construction.
    children: Vec<NodeId>,
    /// Parent for each dense node, or `None` while unowned.
    parents: Vec<Option<NodeId>>,
    /// Fingerprint of the grammar whose mold table the tiles reference.
    grammar_fingerprint: GrammarFingerprint,
}

impl CstBuilder
{
    /// Create an empty CST builder over shared source text.
    ///
    /// # Contract
    /// - requires: `source` is the exact source backing all byte ranges later
    ///   supplied to this builder, and `grammar_fingerprint` identifies the
    ///   grammar whose mold table the tiles' [`crate::MoldId`]s index.
    /// - ensures: the returned builder has no allocated nodes and records the
    ///   grammar fingerprint for the finished [`Cst`].
    /// - provides: append-only construction state for a single CST.
    /// - fails: never.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — construction is a direct field initialization with no
    ///   branches; later constructor witnesses observe the empty initial state.
    /// - witness: `gandr_surface_syntax::builder::tests::builder_round_trips_tile_text`
    #[inline]
    #[must_use]
    pub fn new(
        source: SourceText,
        grammar_fingerprint: GrammarFingerprint,
    ) -> Self
    {
        Self {
            source,
            nodes: Vec::new(),
            children: Vec::new(),
            parents: Vec::new(),
            grammar_fingerprint,
        }
    }
    /// Allocate a token node.
    ///
    /// # Contract
    /// - requires: `range` denotes bytes in this builder's source text.
    /// - ensures: the returned id names a new `NodeKind::Token` with no
    ///   children.
    /// - provides: a token hash framed by `material`: space is non-significant
    ///   to parents, grout ignores source bytes, and tile includes exact UTF-8
    ///   bytes.
    /// - fails: returns `BuildError` when the range is out of bounds, crosses a
    ///   UTF-8 boundary, or the dense arena exceeds `u32`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `BuildError::InvalidTextRange` for out-of-source ranges or
    /// `BuildError::InvalidTextBoundary` for non-boundary ranges, and
    /// `BuildError::NodeCountOverflow` when the dense id space is exhausted.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — token mutants are distinguished by the three material
    ///   classes and range-boundary failures.
    /// - witness: `gandr_surface_syntax::builder::tests::builder_rejects_malformed_ranges`
    /// - witness: `gandr_surface_syntax::builder::tests::builder_round_trips_tile_text`
    #[inline]
    pub fn token(
        &mut self,
        material: Material,
        payload: MoldPayload,
        range: TextRange,
    ) -> Result<NodeId, BuildError>
    {
        let stored_payload = validate_material_payload(material, payload)?;
        let id = self.next_node_id()?;
        let text = self.validate_range(range, id)?;
        let hash = hash_token(material, stored_payload, text);
        Ok(self.push_node(
            id,
            NodeKind::Token,
            material,
            stored_payload,
            range,
            ChildStart(0),
            ChildCount(0),
            hash,
        ))
    }

    /// Allocate an interior node and assign ownership of its children.
    ///
    /// # Contract
    /// - requires: each child id names an existing parentless node in this
    ///   builder.
    /// - ensures: the returned id names a new interior node owning exactly the
    ///   supplied children in iteration order.
    /// - provides: an interior hash framed by `kind`, the material-governed
    ///   optional `mold`, and the significant child hashes, omitting direct
    ///   children whose material is `Space`.
    /// - fails: returns `BuildError` for invalid ranges, token-kind interiors,
    ///   tile interiors, unknown children, duplicate children, already-parented
    ///   children, or integer overflow.
    /// - panics: none.
    /// - intension: validation completes before any parent state is mutated.
    ///
    /// # Errors
    /// Returns `BuildError::InvalidTextRange`,
    /// `BuildError::InvalidTextBoundary`,
    /// `BuildError::TokenKindForInterior`, `BuildError::TileInterior`,
    /// `BuildError::NodeOutOfBounds`, `BuildError::DuplicateChild`,
    /// `BuildError::ChildAlreadyParented`, `BuildError::NodeCountOverflow`, or
    /// `BuildError::TooManyChildren` for the corresponding invariant
    /// violation.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — parent ownership mutants are killed by duplicate,
    ///   already-parented, unknown-child, and successful multi-child
    ///   construction observations.
    /// - witness: `gandr_surface_syntax::builder::tests::builder_rejects_duplicate_parents`
    /// - witness: `gandr_surface_syntax::builder::tests::builder_round_trips_tile_text`
    #[inline]
    pub fn node<I: IntoIterator<Item = NodeId>>(
        &mut self,
        kind: NodeKind,
        material: Material,
        payload: MoldPayload,
        range: TextRange,
        children: I,
    ) -> Result<NodeId, BuildError>
    {
        Self::validate_interior_shape(kind, material)?;
        let id = self.next_node_id()?;
        self.validate_range(range, id)?;

        let collected = self.collect_children(children)?;
        let stored_payload = validate_material_payload(material, payload)?;
        let child_start = ChildStart::try_from(self.children.len()).map_err(BuildError::from)?;
        let child_len = ChildCount::try_from(collected.len()).map_err(BuildError::from)?;
        let final_child_len = self
            .children
            .len()
            .checked_add(collected.len())
            .ok_or(BuildError::TooManyChildren { count: usize::MAX })?;
        u32::try_from(final_child_len).map_err(|_error| BuildError::TooManyChildren {
            count: final_child_len,
        })?;

        let hash = self.hash_interior(kind, stored_payload, collected.iter().copied())?;
        self.assign_parent_links(&collected, id)?;
        let parent = self.push_node(
            id,
            kind,
            material,
            stored_payload,
            range,
            child_start,
            child_len,
            hash,
        );
        self.children.extend(collected);
        Ok(parent)
    }

    /// Finish construction and return the immutable CST.
    ///
    /// # Contract
    /// - requires: `root` names the sole parentless root of the builder's
    ///   arena.
    /// - ensures: every non-root node has exactly one parent and the returned
    ///   CST owns the source, node arena, flattened children, and root id.
    /// - provides: a closed flat arena suitable for read-only model accessors
    ///   and deterministic tree diffing.
    /// - fails: returns `BuildError` when the root is unknown, parented, or any
    ///   other node remains parentless.
    /// - panics: none.
    /// - intension: closure validation scans dense parent state iteratively.
    ///
    /// # Errors
    /// Returns `BuildError::UnknownRoot`, `BuildError::RootHasParent`,
    /// `BuildError::OrphanNode`, or a validation error from `Cst::from_parts`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — root closure is separated by unknown-root,
    ///   parented-root, orphan, and closed-tree observations.
    /// - witness: `gandr_surface_syntax::builder::tests::builder_rejects_orphan_nodes`
    /// - witness: `gandr_surface_syntax::builder::tests::builder_round_trips_tile_text`
    #[inline]
    pub fn finish(
        self,
        root: NodeId,
    ) -> Result<Cst, BuildError>
    {
        let root_index = self
            .node_index(root)
            .map_err(|_error| BuildError::UnknownRoot { root })?;
        match *self
            .parents
            .get(usize::from(root_index))
            .ok_or(BuildError::UnknownRoot { root })?
        {
            | Some(parent) => Err(BuildError::RootHasParent { root, parent }),
            | None => {
                self.validate_parent_closure(root)?;
                Cst::from_parts(
                    self.source,
                    self.nodes,
                    self.children,
                    root,
                    self.grammar_fingerprint,
                )
            },
        }
    }

    /// Validate that an interior node is not using token-only shape.
    fn validate_interior_shape(
        kind: NodeKind,
        material: Material,
    ) -> Result<(), BuildError>
    {
        if kind == NodeKind::Token {
            return Err(BuildError::TokenKindForInterior);
        }
        if material == Material::Tile {
            return Err(BuildError::TileInterior { kind });
        }
        Ok(())
    }

    /// Validate a source range and return its text.
    fn validate_range(
        &self,
        range: TextRange,
        node: NodeId,
    ) -> Result<SourceSlice<'_>, BuildError>
    {
        let start_index = range.start_index()?;
        let end_index = range.end_index()?;
        let start = usize::from(start_index);
        let end = usize::from(end_index);
        if start > end || end > self.source.as_ref().len() {
            return Err(BuildError::InvalidTextRange { start, end });
        }
        let text = self
            .source
            .as_ref()
            .get(start .. end)
            .ok_or(BuildError::InvalidTextBoundary { node, start, end })?;
        Ok(SourceSlice::from(text))
    }

    /// Collect and validate children without mutating parent links.
    fn collect_children(
        &self,
        children: impl IntoIterator<Item = NodeId>,
    ) -> Result<Vec<NodeId>, BuildError>
    {
        let mut collected = Vec::new();
        let mut seen = Vec::new();
        for child in children {
            let raw = child.raw();
            if seen.contains(&raw) {
                return Err(BuildError::DuplicateChild { child });
            }
            let node_index = self.node_index(child)?;
            let index = usize::from(node_index);
            if let Some(&Some(parent)) = self.parents.get(index) {
                return Err(BuildError::ChildAlreadyParented { child, parent });
            }
            seen.push(raw);
            collected.push(child);
        }
        Ok(collected)
    }

    /// Assign a parent id to validated children.
    fn assign_parent_links(
        &mut self,
        children: &[NodeId],
        parent: NodeId,
    ) -> Result<(), BuildError>
    {
        for child in children {
            let node_index = self.node_index(*child)?;
            let index = usize::from(node_index);
            let len = self.nodes.len();
            match self.parents.get_mut(index) {
                | Some(slot) => {
                    *slot = Some(parent);
                },
                | None => return Err(BuildError::NodeOutOfBounds { id: *child, len }),
            }
            match self.nodes.get_mut(index) {
                | Some(node) => node.set_parent(parent),
                | None => return Err(BuildError::NodeOutOfBounds { id: *child, len }),
            }
        }
        Ok(())
    }

    /// Push an already-validated node into every parallel arena.
    #[expect(
        clippy::too_many_arguments,
        reason = "push_node threads the eight validated arena fields into the parallel node/child/parent arenas; a parameter struct would only relocate the same arity"
    )]
    fn push_node(
        &mut self,
        id: NodeId,
        kind: NodeKind,
        material: Material,
        payload: MoldPayload,
        range: TextRange,
        child_start: ChildStart,
        child_len: ChildCount,
        hash: StableHash,
    ) -> NodeId
    {
        self.nodes.push(NodeData::new(
            kind,
            material,
            payload,
            range,
            None,
            child_start,
            child_len,
            hash,
        ));
        self.parents.push(None);
        id
    }

    /// Return the next dense node id.
    fn next_node_id(&self) -> Result<NodeId, BuildError>
    {
        NodeId::from_index(NodeIndex(self.nodes.len()))
    }

    /// Convert a node id into an arena index after checking existence.
    fn node_index(
        &self,
        node: NodeId,
    ) -> Result<NodeIndex, BuildError>
    {
        let node_index = node.index()?;
        let index = usize::from(node_index);
        if self.nodes.get(index).is_some() {
            Ok(NodeIndex(index))
        }
        else {
            Err(BuildError::NodeOutOfBounds {
                id: node,
                len: self.nodes.len(),
            })
        }
    }

    /// Hash an interior node from significant direct child hashes.
    fn hash_interior(
        &self,
        kind: NodeKind,
        payload: MoldPayload,
        children: impl IntoIterator<Item = NodeId>,
    ) -> Result<StableHash, BuildError>
    {
        let mut state = StableHasher::new();
        state.write_byte(HashByte(FRAME_INTERIOR));
        state.write_byte(kind_tag(kind));
        state.write_payload(payload);
        let mut significant_count = SignificantChildCount::zero();
        for child in children {
            let node_index = self.node_index(child)?;
            let index = usize::from(node_index);
            let child_node = self.nodes.get(index).ok_or(BuildError::NodeOutOfBounds {
                id: child,
                len: self.nodes.len(),
            })?;
            if child_node.material() != Material::Space {
                let child_hash = child_node.hash();
                significant_count = significant_count.increment()?;
                state.write_hash(child_hash);
            }
        }
        state.write_significant_count(significant_count);
        Ok(state.finish())
    }

    /// Validate that every non-root node is parented.
    fn validate_parent_closure(
        &self,
        root: NodeId,
    ) -> Result<(), BuildError>
    {
        for (raw, parent) in self.parents.iter().enumerate() {
            let node = NodeId::from_index(NodeIndex(raw)).map_err(|_error| {
                BuildError::NodeCountOverflow {
                    len: self.parents.len(),
                }
            })?;
            if node != root && parent.is_none() {
                return Err(BuildError::OrphanNode { node });
            }
        }
        Ok(())
    }
}

/// Validate that a mold payload variant matches its material.
fn validate_material_payload(
    material: Material,
    payload: MoldPayload,
) -> Result<MoldPayload, BuildError>
{
    match (material, payload) {
        | (Material::Space, MoldPayload::Space)
        | (Material::Grout, MoldPayload::Grout { .. })
        | (Material::Tile, MoldPayload::Tile(_)) => Ok(payload),
        | (Material::Space | Material::Grout | Material::Tile, _) => {
            Err(BuildError::MoldPayloadMismatch { material })
        },
    }
}

/// Compute a token hash under the token material rules.
///
/// Tiles hash `(TILE, MoldId, source bytes)`; grout hashes
/// `(GROUT, GroutShape, sort)`; space is framed but carries no mold identity.
fn hash_token(
    material: Material,
    payload: MoldPayload,
    text: SourceSlice<'_>,
) -> StableHash
{
    let mut state = StableHasher::new();
    match (material, payload) {
        | (Material::Space, _) => {
            state.write_byte(HashByte(FRAME_SPACE));
        },
        | (Material::Grout, MoldPayload::Grout { shape, sort }) => {
            state.write_byte(HashByte(FRAME_GROUT));
            state.write_byte(shape.tag());
            state.write_grout_sort(sort);
        },
        | (Material::Tile, MoldPayload::Tile(mold)) => {
            state.write_byte(HashByte(FRAME_TILE));
            state.write_mold_id(mold);
            for byte in text.as_ref().as_bytes() {
                state.write_byte(HashByte(*byte));
            }
        },
        | (Material::Grout | Material::Tile, _) => {
            // Pairing is validated before hashing; this arm is unreachable in
            // practice but keeps the hash total without a partial function.
            state.write_byte(HashByte(FRAME_SPACE));
        },
    }
    state.finish()
}

/// Map node kinds to stable frame tags.
fn kind_tag(kind: NodeKind) -> HashByte
{
    match kind {
        | NodeKind::Cell => HashByte(b'C'),
        | NodeKind::Meld => HashByte(b'M'),
        | NodeKind::Wald => HashByte(b'W'),
        | NodeKind::Token => HashByte(b'T'),
    }
}

/// Small framed FNV-1a hasher for CST stable hashes.
#[repr(transparent)]
struct StableHasher
{
    /// Current FNV state.
    state: u64,
}

impl StableHasher
{
    /// Create a fresh FNV-1a state.
    fn new() -> Self
    {
        Self { state: FNV_OFFSET }
    }

    /// Mix a single byte.
    fn write_byte(
        &mut self,
        byte: HashByte,
    )
    {
        self.state ^= u64::from(byte.0);
        self.state = self.state.wrapping_mul(FNV_PRIME);
    }

    /// Mix a grout sort tag in little-endian order.
    fn write_grout_sort(
        &mut self,
        sort: crate::GroutSort,
    )
    {
        for byte in sort.0.to_le_bytes() {
            self.write_byte(HashByte(byte));
        }
    }

    /// Mix a mold identifier in little-endian order.
    fn write_mold_id(
        &mut self,
        mold: crate::MoldId,
    )
    {
        for byte in u32::from(mold).to_le_bytes() {
            self.write_byte(HashByte(byte));
        }
    }

    /// Mix a stable child hash in little-endian order.
    fn write_hash(
        &mut self,
        hash: StableHash,
    )
    {
        for byte in hash.0.to_le_bytes() {
            self.write_byte(HashByte(byte));
        }
    }

    /// Mix the significant-child count in little-endian order.
    fn write_significant_count(
        &mut self,
        count: SignificantChildCount,
    )
    {
        for byte in count.0.to_le_bytes() {
            self.write_byte(HashByte(byte));
        }
    }

    /// Mix a material-governed mold payload frame.
    ///
    /// Space carries no identity; grout mixes its shape and sort; a tile mixes
    /// its opaque [`crate::MoldId`], preserving mold-sensitive hashing over the
    /// mold reference.
    fn write_payload(
        &mut self,
        payload: MoldPayload,
    )
    {
        match payload {
            | MoldPayload::Space => self.write_byte(HashByte(0)),
            | MoldPayload::Grout { shape, sort } => {
                self.write_byte(HashByte(1));
                self.write_byte(shape.tag());
                self.write_grout_sort(sort);
            },
            | MoldPayload::Tile(mold) => {
                self.write_byte(HashByte(2));
                self.write_mold_id(mold);
            },
        }
    }

    /// Return the current hash state.
    fn finish(self) -> StableHash
    {
        StableHash(self.state)
    }
}
