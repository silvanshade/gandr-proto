//! Reading a committed CST as a **flat tile run**: the shared cursor the
//! surface-directed passes walk declarations with.
//!
//! The checked PBG captures a declaration as a flat sequence — its members are
//! inline tiles under one Meld, and only compound sub-forms (a nested type, a
//! rule expression) nest into sub-Melds. A pass over such a declaration is
//! therefore not a tree walk but a **cursor over significant children**,
//! resolving each tile's label through the grammar the CST's `MoldId`s index.
//!
//! Two passes read that way today — the levitation stage-0 elaborator
//! ([`crate::desc_elab`], `data` / `codata` blocks) and the circuit surface
//! check ([`crate::circuit`], `sign` blocks and circuit declarations) — and the
//! machinery is here rather than in either of them because it belongs to
//! neither: it knows about tiles, spans, and commas, and nothing about
//! descriptions or circuits.
//!
//! This module deliberately stops at the label. Deciding what a run *means* —
//! which member family it belongs to, which sub-run is a type — is the
//! consuming pass's, and each adds its own inherent methods to [`Reader`].

use gandr_surface_grammar::Pbg;
use gandr_surface_syntax::Cst;
use gandr_surface_syntax::Material;
use gandr_surface_syntax::MoldPayload;
use gandr_surface_syntax::NodeId;
use gandr_surface_syntax::TextRange;
use gandr_theory_levitation::SurfaceSpan;
use gandr_theory_levitation::boundary::SurfaceByteOffset;

use crate::boundary::MatchDecision;
use crate::boundary::NodeText;
use crate::boundary::SourceRange;
use crate::boundary::TileSpelling;

/// The process-wide cached built-in grammar, for resolving the CST's `MoldId`s
/// to tile labels and sorts.
///
/// The one cache lives in [`crate::synnode::shared_grammar`]: `built_in` is
/// deterministic and fingerprint-pinned, so the ids in a [`SynTree::parse`]
/// CST resolve against that instance's table (both are the same checked
/// artifact).
///
/// [`SynTree::parse`]: crate::synnode::SynTree::parse
pub fn grammar() -> Option<&'static Pbg>
{
    crate::synnode::shared_grammar()
}

/// The empty provenance span used when a CST lookup misses unexpectedly, or
/// when a decline belongs to a member that carries no span.
pub fn empty_surface_span() -> SurfaceSpan
{
    SurfaceSpan::new(SurfaceByteOffset::from(0), SurfaceByteOffset::from(0))
}

/// Convert a syntax text range into a host byte range.
fn source_range(range: TextRange) -> SourceRange
{
    let start = usize::try_from(u32::from(range.start())).unwrap_or(0);
    let end = usize::try_from(u32::from(range.end())).unwrap_or(start);
    SourceRange(start .. end)
}

/// A borrowing reader over one parse's CST, resolving tile labels through the
/// grammar.
pub struct Reader<'tree>
{
    /// The grammar the CST's `MoldId`s index.
    pbg: &'tree Pbg,
    /// The concrete syntax tree.
    cst: &'tree Cst,
}

impl<'tree> Reader<'tree>
{
    /// A reader over `cst`, resolving mold ids against `pbg`.
    ///
    /// # Contract
    /// - requires: `pbg` is the grammar `cst` was committed over — the same
    ///   checked artifact, so its `MoldId`s index this table.
    /// - ensures: returns a reader borrowing both for the tree's lifetime.
    /// - provides: the entry point every flat-run pass starts from.
    /// - fails: never.
    /// - panics: none.
    pub fn new(
        pbg: &'tree Pbg,
        cst: &'tree Cst,
    ) -> Self
    {
        Self { pbg, cst }
    }

    /// The significant (non-space) children of `id`, in source order.
    pub fn sig_children(
        &self,
        id: NodeId,
    ) -> Vec<NodeId>
    {
        let Ok(children) = self.cst.children(id)
        else {
            return Vec::new();
        };
        children
            .iter()
            .copied()
            .filter(|&child| {
                self.cst
                    .node(child)
                    .is_ok_and(|view| !matches!(view.material(), Material::Space))
            })
            .collect()
    }

    /// The tile label of `id`, or [`None`] when `id` is not a tile.
    pub fn label(
        &self,
        id: NodeId,
    ) -> Option<TileSpelling>
    {
        match {
            let present = self.cst.node(id).ok()?;
            core::convert::identity(present)
        }
        .payload()
        {
            | MoldPayload::Tile(mold) => {
                self.pbg.mold(mold).ok().map(|def| TileSpelling(def.label))
            },
            | MoldPayload::Grout { .. } | MoldPayload::GhostClose { .. } | MoldPayload::Space => {
                None
            },
        }
    }

    /// Whether `id` is a Meld (a grouped sub-node, e.g. a compound type or rule
    /// expression) rather than a tile.
    pub fn is_meld(
        &self,
        id: NodeId,
    ) -> MatchDecision
    {
        MatchDecision(
            self.label(id).map(|label| label.0).is_none() && !self.sig_children(id).is_empty(),
        )
    }

    /// The source text `id` covers.
    pub fn text(
        &self,
        id: NodeId,
    ) -> NodeText<'tree>
    {
        NodeText(
            self.cst
                .node(id)
                .ok()
                .and_then(|view| {
                    let range = view.range();
                    self.cst.source().as_ref().get(source_range(range).0)
                })
                .unwrap_or(""),
        )
    }

    /// The half-open byte span `id` covers.
    pub fn span(
        &self,
        id: NodeId,
    ) -> SurfaceSpan
    {
        self.cst.node(id).map_or_else(
            |_error| empty_surface_span(),
            |view| {
                let range = source_range(view.range());
                SurfaceSpan::new(
                    SurfaceByteOffset::from(range.start),
                    SurfaceByteOffset::from(range.end),
                )
            },
        )
    }
}

/// A cursor over a run of significant CST children.
pub struct Cursor<'run, 'tree>
{
    /// The reader resolving tile labels.
    reader: &'run Reader<'tree>,
    /// The run being scanned.
    run: &'run [NodeId],
    /// The current position into `run`.
    pos: usize,
}

impl<'run, 'tree> Cursor<'run, 'tree>
{
    /// A cursor at the start of `run`.
    pub fn new(
        reader: &'run Reader<'tree>,
        run: &'run [NodeId],
    ) -> Self
    {
        Self {
            reader,
            run,
            pos: 0,
        }
    }

    /// The node at the cursor, without advancing.
    pub fn peek(&self) -> Option<NodeId>
    {
        self.run.get(self.pos).copied()
    }

    /// Advance past and return the node at the cursor.
    pub fn bump(&mut self) -> Option<NodeId>
    {
        let id = self.run.get(self.pos).copied();
        if id.is_some() {
            self.pos = self.pos.saturating_add(1);
        }
        id
    }

    /// Advance past the node at the cursor when its label is `label`, returning
    /// whether it matched.
    pub fn eat(
        &mut self,
        label: TileSpelling,
    ) -> MatchDecision
    {
        if self
            .peek()
            .and_then(|id| self.reader.label(id))
            .is_some_and(|actual| actual == label)
        {
            self.bump();
            MatchDecision(true)
        }
        else {
            MatchDecision(false)
        }
    }

    /// The remaining run up to (and consuming) the **matching** closing `}` —
    /// i.e. the member region of a declaration body.
    ///
    /// Nested braces are counted, so a member that opens its own block (a
    /// circuit member's filler) keeps its closer instead of ending the region
    /// early. A description block never has a flat nested brace — its
    /// brace-bearing sub-forms sit at sort holes and reach the run as Melds —
    /// so the count is inert there and the behaviour is unchanged.
    ///
    /// # Contract
    /// - requires: the cursor sits just past a declaration body's opening `{`,
    ///   and the run's brackets balance (the melder's commit guarantees this
    ///   for a clean parse).
    /// - ensures: returns every node up to the matching `}`, which is consumed
    ///   and excluded; an unbalanced or truncated run returns everything left,
    ///   with the cursor at the end.
    /// - provides: the member region a declaration's members are split out of.
    /// - fails: never.
    /// - panics: none.
    pub fn until_close_brace(&mut self) -> Vec<NodeId>
    {
        let mut region = Vec::new();
        let mut depth: u32 = 0;
        while let Some(id) = self.bump() {
            match self.reader.label(id).map(|label| label.0) {
                | Some(opener) if is_opener(BracketLabel(opener)).0 => {
                    depth = depth.saturating_add(1);
                    region.push(id);
                },
                | Some("}") if depth == 0 => break,
                | Some(closer) if is_closer(BracketLabel(closer)).0 => {
                    depth = depth.saturating_sub(1);
                    region.push(id);
                },
                | _ => region.push(id),
            }
        }
        region
    }
}

/// A tile label under bracket classification.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BracketLabel<'label>(pub &'label str);

/// Whether a tile label opens a bracketed region.
///
/// `#{` is an opener whose closer is the ordinary `}`, which is why the two
/// predicates are not symmetric in spelling.
pub fn is_opener(label: BracketLabel<'_>) -> MatchDecision
{
    MatchDecision(matches!(label.0, "(" | "[" | "{" | "#{"))
}

/// Whether a tile label closes a bracketed region.
pub fn is_closer(label: BracketLabel<'_>) -> MatchDecision
{
    MatchDecision(matches!(label.0, ")" | "]" | "}"))
}

/// Split a block's member region into per-member runs at bracket-depth-zero
/// member separators: the ruled `;` terminator, or the retired `,` separator
/// kept admissible so a stale declaration parses whole and reaches the
/// elaborator's migration decline (the retired-`~>` precedent).
///
/// Depth counts brackets exactly as [`split_at_top_level`] does, so a
/// separator nested inside a telescope, an attribute slot, or a nested body
/// stays within its member; a member's own terminator and a trailing
/// terminator yield no empty runs.
///
/// # Contract
/// - requires: `region` is a run of significant children of one block, with
///   balanced brackets (the melder's commit guarantees this).
/// - ensures: returns the members in source order, each WITHOUT its terminator.
/// - provides: the member split the nested generator block's reader takes.
/// - fails: never; an unbalanced region simply splits at the depths it has.
/// - panics: none.
pub fn member_runs(
    reader: &Reader<'_>,
    region: &[NodeId],
) -> Vec<Vec<NodeId>>
{
    let mut members: Vec<Vec<NodeId>> = Vec::new();
    let mut current: Vec<NodeId> = Vec::new();
    let mut depth: u32 = 0;
    for &id in region {
        let label = reader.label(id);
        if depth == 0 && matches!(label.map(|label| label.0), Some(";" | ",")) {
            if !current.is_empty() {
                members.push(core::mem::take(&mut current));
            }
            continue;
        }
        match label.map(|label| label.0) {
            | Some(bracket) if is_opener(BracketLabel(bracket)).0 => {
                depth = depth.saturating_add(1);
            },
            | Some(bracket) if is_closer(BracketLabel(bracket)).0 => {
                depth = depth.saturating_sub(1);
            },
            | _ => {},
        }
        current.push(id);
    }
    if !current.is_empty() {
        members.push(current);
    }
    members
}

/// Split a member region into per-member runs at bracket-depth-zero
/// occurrences of `separator`.
///
/// Depth counts `(` / `[` / `{` against their closers, so a separator nested
/// inside a parameter list, an attribute slot, or a nested body stays within
/// its member. A run of separators yields no empty members, and a trailing
/// separator yields none either.
///
/// # Contract
/// - requires: `region` is a run of significant children of one declaration,
///   with balanced brackets (the melder's commit guarantees this).
/// - ensures: returns the members in source order; concatenating them and the
///   elided separators reproduces `region`.
/// - provides: the member split both the description reader and the circuit
///   check take.
/// - fails: never; an unbalanced region simply splits at the depths it has.
/// - panics: none.
pub fn split_at_top_level(
    reader: &Reader<'_>,
    region: &[NodeId],
    separator: TileSpelling,
) -> Vec<Vec<NodeId>>
{
    let mut members: Vec<Vec<NodeId>> = Vec::new();
    let mut current: Vec<NodeId> = Vec::new();
    let mut depth: u32 = 0;
    for &id in region {
        let label = reader.label(id);
        if depth == 0 && label.is_some_and(|actual| actual == separator) {
            if !current.is_empty() {
                members.push(core::mem::take(&mut current));
            }
            continue;
        }
        match label.map(|label| label.0) {
            | Some(bracket) if is_opener(BracketLabel(bracket)).0 => {
                depth = depth.saturating_add(1);
            },
            | Some(bracket) if is_closer(BracketLabel(bracket)).0 => {
                depth = depth.saturating_sub(1);
            },
            | _ => {},
        }
        current.push(id);
    }
    if !current.is_empty() {
        members.push(current);
    }
    members
}
