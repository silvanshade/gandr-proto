//! Authoritative mold-definition and interned regex-context tables.
//!
//! A mold is `{rctx, prec, sort}` — a zipper into the grammar — never an opaque
//! shape code. This module owns the tables the `Pbg` exposes: one [`MoldDef`]
//! per tile occurrence, keyed by an interned regex-zipper context ([`RCtxId`]),
//! with precomputed precedence bounds and zipper steps. The compact
//! [`gandr_surface_syntax::MoldId`] carried by a CST indexes the mold table.
//!
//! Mold identity is assigned deterministically at `Pbg` build in canonical
//! table order (rules in input order, each rule's regex walked left to right).
//! Two occurrences that intern to the same `(label, rctx)` are genuine
//! redundancy and surface as [`PbgError::DuplicateTile`]; occurrences at
//! structurally distinct regex positions receive distinct contexts, so the
//! `comma1`/`repeat1` element-clone class resolves per-occurrence.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;

use gandr_surface_syntax::ClosingClass;
use gandr_surface_syntax::DelimSpelling;
use gandr_surface_syntax::GrammarFingerprint;
pub use gandr_surface_syntax::MoldId;
use gandr_theory_graphs::Bound;
use gandr_theory_graphs::Dir;
use gandr_theory_graphs::EdgeSource;
use gandr_theory_graphs::NodeCount;
use gandr_theory_graphs::NodeId;
use gandr_theory_graphs::Prec;
use gandr_theory_graphs::condensation;

use crate::model::CandidateCount;
use crate::model::MoldCount;
use crate::model::PbgError;
use crate::model::Regex;
use crate::model::Rule;
use crate::model::Sort;
use crate::model::Sym;
use crate::model::TileLabel;

/// FNV-1a offset basis for stable mold-table fingerprints.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a prime for stable mold-table fingerprints.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Frame byte separating the mold-table region of the PBG fingerprint.
const FRAME_MOLD: u8 = b'M';

/// Interned regex-zipper context into a rule's contribution to `G(sort, prec)`.
///
/// Two tile occurrences at structurally identical contexts (e.g. identical
/// branches of an alternation) share an `RCtxId`; occurrences at distinct
/// positions receive distinct ids. Because gandr's `G(sort, prec)` is the
/// labelled disjunction of its rules, a context is scoped to the rule whose
/// regex it indexes.
///
/// # Contract
/// - requires: `raw` indexes the interned-context table built with the owning
///   `Pbg`.
/// - ensures: preserves `raw` exactly.
/// - provides: the mold's zipper identity, distinct from precedence and sort.
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 — a newtype with no branches; interning distinctness is
///   witnessed through the declared candidate inventory.
/// - witness: `gandr_surface_grammar::contracts::declared_mold_candidate_inventory_is_exact`
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RCtxId(u32);

impl From<u32> for RCtxId
{
    #[inline]
    fn from(index: u32) -> Self
    {
        Self(index)
    }
}

impl From<RCtxId> for u32
{
    #[inline]
    fn from(id: RCtxId) -> Self
    {
        id.0
    }
}

/// One symbol the zipper crosses when stepping out of a context.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StepSym
{
    /// A recursive sort symbol faces the context on this side.
    Sort(Sort),
    /// A terminal tile with this label faces the context on this side.
    Tile(&'static str),
}

/// One precomputed zipper step (tylr `RZipper.step`).
///
/// Advancing a mold's regex zipper crosses the adjacent symbol as a stance; the
/// generative walk front-end consumes these precomputed steps.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RCtxStep
{
    /// The symbol crossed by this step.
    pub crossed: StepSym,
}

/// An authoritative mold definition: a zipper into the grammar.
///
/// # Contract
/// - requires: fields come from a checked PBG rule occurrence.
/// - ensures: preserves the tile label, interned context, precedence, and sort
///   exactly; ids are canonical and fingerprint-scoped.
/// - provides: the replacement for the retired opaque `Mold { sort, tips }`.
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the four fields distinguish a mold from same-label molds
///   at other contexts, precedences, and sorts.
/// - witness: `gandr_surface_grammar::contracts::declared_mold_candidate_inventory_is_exact`
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoldDef
{
    /// Textual tile form.
    pub label: &'static str,
    /// The interned regex-zipper context into `G(sort, prec)`.
    pub rctx: RCtxId,
    /// Precedence group of the producing form.
    pub prec: Prec,
    /// Grammar sort of the producing form.
    pub sort: Sort,
}

/// Precomputed properties of one interned regex-zipper context.
#[derive(Clone, Debug, Eq, PartialEq)]
struct RCtxData
{
    /// Whether a recursive sort faces the context on the left.
    left_faces_sort: bool,
    /// Whether a recursive sort faces the context on the right.
    right_faces_sort: bool,
    /// Symbols the zipper crosses stepping left, in deterministic order.
    left_steps: Vec<RCtxStep>,
    /// Symbols the zipper crosses stepping right, in deterministic order.
    right_steps: Vec<RCtxStep>,
}

/// The authoritative mold and interned-context tables for a checked PBG.
#[derive(Clone, Debug)]
pub struct MoldTable
{
    /// Mold definitions indexed by [`MoldId`], in canonical table order.
    molds: Vec<MoldDef>,
    /// Interned regex-context data indexed by [`RCtxId`].
    rctxs: Vec<RCtxData>,
    /// Precomputed precedence bounds indexed by [`MoldId`].
    bounds: Vec<(Bound<Prec>, Bound<Prec>)>,
    /// Per-label candidate menus, sorted by [`MoldId`].
    candidates: BTreeMap<&'static str, Vec<MoldId>>,
    /// Per-label **fresh-slot** candidate menus: the subset of each label's
    /// molds that can be admissible when no form is open — every mold that has
    /// no `≐`-predecessor (an operand, operator, or form-start) plus any
    /// form-first tile. A form-mid / form-end with a `≐`-predecessor is
    /// inadmissible with no open frontier, so the molder skips the (wide)
    /// tail of the menu at the overwhelmingly common fresh-operand
    /// position, collapsing the ~130-mold `identifier` menu to its two
    /// atoms there (`proposal-parser-interaction-core` §5.2 wide-menu
    /// gather). Sorted by [`MoldId`], like [`Self::candidates`].
    fresh: BTreeMap<&'static str, Vec<MoldId>>,
    /// Consecutive same-form tile pairs (the `≐` adjacency), sorted and unique.
    adjacencies: Vec<(MoldId, MoldId)>,
    /// Molds that can be a form's first tile (its regex FIRST set, holes
    /// skipped), sorted and unique.
    form_first: Vec<MoldId>,
    /// Molds that can be a form's **last** tile (its regex LAST set, holes
    /// skipped), sorted and unique — the dual of
    /// [`form_first`](Self::form_first). A form-start / form-mid whose
    /// remaining tail is nullable (e.g. `?` before an optional `hole_name`)
    /// is in this set: the form is already complete at that tile, so the
    /// melder closes it cleanly rather than force-closing with a ghost end
    /// Not folded into the fingerprint (derived, like
    /// `form_first`).
    form_last: Vec<MoldId>,
    /// Each mold's **form-level closing class**, indexed by [`MoldId`].
    ///
    /// `Some(c)` when every completion path from that mold ends in a paired
    /// closer of class `c`, `None` otherwise — the datum a force-close needs to
    /// say which closer its minted ghost stood in for. Derived per rule, so a
    /// completion path never leaves the form it started in. Not folded into the
    /// fingerprint (derived, like `form_first` / `form_last`).
    closing: Vec<Option<ClosingClass>>,
    /// Deterministic PBG fingerprint folding the precedence DAG and the
    /// mold/context tables; scopes the CST identity.
    fingerprint: GrammarFingerprint,
}

impl MoldTable
{
    /// Build the mold and interned-context tables for `rules`.
    ///
    /// # Contract
    /// - requires: `rules` are the validated rules of a PBG candidate and
    ///   `dag_fingerprint` is the producing precedence DAG's fingerprint.
    /// - ensures: assigns one [`MoldId`] per tile occurrence in canonical order
    ///   and folds the tables (with `dag_fingerprint`) into a deterministic PBG
    ///   fingerprint.
    /// - provides: the `mold`/`bounds`/`step`/`candidates` surface and the CST
    ///   fingerprint scope.
    /// - fails: returns [`PbgError::DuplicateTile`] when two occurrences intern
    ///   to the same `(label, rctx)` — genuine redundancy.
    /// - panics: none.
    /// - intension: rules are walked in input order; each rule's regex is
    ///   walked left to right; contexts intern in first-seen order.
    ///
    /// # Errors
    /// Returns [`PbgError::DuplicateTile`] for the first duplicated
    /// `(label, rctx)` occurrence.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the alternation-of-identical-branches witness kills
    ///   the duplicate branch while the built-in surface witnesses the accepted
    ///   path.
    /// - witness: `gandr_surface_grammar::contracts::pbg_rejects_duplicate_rctx_tile`
    /// - witness: `gandr_surface_grammar::contracts::declared_mold_candidate_inventory_is_exact`
    pub(crate) fn build(
        rules: &[Rule],
        dag_fingerprint: GrammarFingerprint,
    ) -> Result<Self, PbgError>
    {
        let mut interner = ContextInterner::new();
        let mut molds: Vec<MoldDef> = Vec::new();
        let mut identity: BTreeMap<TileKey, &'static str> = BTreeMap::new();
        let mut candidates: BTreeMap<&'static str, Vec<MoldId>> = BTreeMap::new();
        let mut adjacent_keys: BTreeSet<(TileKey, TileKey)> = BTreeSet::new();
        let mut first_keys: BTreeSet<TileKey> = BTreeSet::new();
        let mut last_keys: BTreeSet<TileKey> = BTreeSet::new();
        let mut closing: Vec<Option<ClosingClass>> = Vec::new();

        for rule in rules {
            let mut occurrences = Vec::new();
            let facet = collect_occurrences(rule, &mut interner, &mut occurrences);
            first_keys.extend(facet.first.iter().copied());
            last_keys.extend(facet.last.iter().copied());
            // Derived per rule, before the adjacency set is drained into the
            // table-wide one: the class is a property of THIS form's completion
            // paths, and a table-wide walk would cross into other rules.
            closing.extend(closing_classes(&occurrences, &facet));
            adjacent_keys.extend(facet.adjacent);
            for occurrence in occurrences {
                let rctx = occurrence.rctx;
                if let Some(first_rule) =
                    identity.get(&TileKey::new(TileLabel(occurrence.label), rctx))
                {
                    return Err(PbgError::DuplicateTile {
                        label: occurrence.label,
                        sort: rule.sort,
                        prec: rule.prec,
                        first_rule,
                        second_rule: rule.name,
                    });
                }
                let mold_index =
                    u32::try_from(molds.len()).map_err(|_error| PbgError::MoldOverflow)?;
                let mold_id = MoldId::from(mold_index);
                identity.insert(TileKey::new(TileLabel(occurrence.label), rctx), rule.name);
                molds.push(MoldDef {
                    label: occurrence.label,
                    rctx,
                    prec: rule.prec,
                    sort: rule.sort,
                });
                candidates
                    .entry(occurrence.label)
                    .or_default()
                    .push(mold_id);
            }
        }

        let rctxs = interner.finish();
        let bounds = molds
            .iter()
            .map(|mold| bounds_for(mold, &rctxs))
            .collect::<Vec<_>>();
        let adjacencies = resolve_adjacencies(&molds, &adjacent_keys);
        let form_first = resolve_keys(&molds, &first_keys);
        let form_last = resolve_keys(&molds, &last_keys);
        let has_pred: BTreeSet<MoldId> = adjacencies.iter().map(|&(_, right)| right).collect();
        let first: BTreeSet<MoldId> = form_first.iter().copied().collect();
        let fresh = candidates
            .iter()
            .map(|(&label, label_molds)| {
                let fresh = label_molds
                    .iter()
                    .copied()
                    .filter(|mold| !has_pred.contains(mold) || first.contains(mold))
                    .collect::<Vec<_>>();
                (label, fresh)
            })
            .collect();
        let fingerprint = fold_fingerprint(dag_fingerprint, &molds, &rctxs);
        Ok(Self {
            molds,
            rctxs,
            bounds,
            candidates,
            fresh,
            adjacencies,
            form_first,
            form_last,
            closing,
            fingerprint,
        })
    }

    /// Return the mold definition for `id`.
    ///
    /// # Contract
    /// - requires: `id` was assigned by this table.
    /// - ensures: returns the exact [`MoldDef`] at `id` when in range.
    /// - provides: the authoritative mold lookup.
    /// - fails: returns [`PbgError::UnknownMold`] when `id` is out of range.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PbgError::UnknownMold`] when `id` is outside the mold table.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a valid id and the first out-of-range id kill the
    ///   only branch.
    /// - witness: `gandr_surface_grammar::contracts::mold_lookup_checks_bounds`
    #[inline]
    pub(crate) fn mold(
        &self,
        id: MoldId,
    ) -> Result<&MoldDef, PbgError>
    {
        let index =
            usize::try_from(u32::from(id)).map_err(|_error| PbgError::UnknownMold { id })?;
        self.molds.get(index).ok_or(PbgError::UnknownMold { id })
    }

    /// Return the precomputed precedence bounds for `id`.
    ///
    /// # Contract
    /// - requires: `id` was assigned by this table.
    /// - ensures: returns the `(left, right)` bounds derived from the context's
    ///   sort-facing nullability and the mold precedence.
    /// - provides: the tylr `Mold.bounds` surface, precomputed at build.
    /// - fails: returns [`PbgError::UnknownMold`] when `id` is out of range.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PbgError::UnknownMold`] when `id` is outside the mold table.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a sort-facing and a tile-facing occurrence
    ///   distinguish `Value(prec)` from `Root` on each side.
    /// - witness: `gandr_surface_grammar::contracts::mold_bounds_follow_context_nullability`
    #[inline]
    pub(crate) fn bounds(
        &self,
        id: MoldId,
    ) -> Result<(Bound<Prec>, Bound<Prec>), PbgError>
    {
        let index =
            usize::try_from(u32::from(id)).map_err(|_error| PbgError::UnknownMold { id })?;
        self.bounds
            .get(index)
            .copied()
            .ok_or(PbgError::UnknownMold { id })
    }

    /// Return the precomputed zipper steps for `rctx` in direction `dir`.
    ///
    /// # Contract
    /// - requires: `rctx` was interned by this table.
    /// - ensures: returns the crossed-symbol steps for that side.
    /// - provides: the tylr `RZipper.step` surface, precomputed at build.
    /// - fails: returns [`PbgError::UnknownRCtx`] when `rctx` is out of range.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PbgError::UnknownRCtx`] when `rctx` is outside the context
    /// table.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a left step and a right step over an infix context
    ///   kill the direction branch.
    /// - witness: `gandr_surface_grammar::contracts::rctx_steps_cross_adjacent_symbols`
    #[inline]
    pub(crate) fn step(
        &self,
        rctx: RCtxId,
        dir: Dir,
    ) -> Result<&[RCtxStep], PbgError>
    {
        let index =
            usize::try_from(u32::from(rctx)).map_err(|_error| PbgError::UnknownRCtx { rctx })?;
        let data = self
            .rctxs
            .get(index)
            .ok_or(PbgError::UnknownRCtx { rctx })?;
        Ok(match dir {
            | Dir::Left => &data.left_steps,
            | Dir::Right => &data.right_steps,
        })
    }

    /// Return the per-label candidate menu.
    #[inline]
    #[must_use]
    pub(crate) fn candidates(
        &self,
        label: TileLabel,
    ) -> &[MoldId]
    {
        self.candidates
            .get(label.as_ref())
            .map_or(&[], Vec::as_slice)
    }

    /// Return the per-label fresh-slot candidate menu (see [`Self::fresh`]).
    #[inline]
    #[must_use]
    pub(crate) fn fresh_candidates(
        &self,
        label: TileLabel,
    ) -> &[MoldId]
    {
        self.fresh.get(label.as_ref()).map_or(&[], Vec::as_slice)
    }

    /// Return every candidate label with its declared candidate count.
    #[inline]
    #[must_use]
    pub(crate) fn candidate_counts(&self) -> Vec<(TileLabel, CandidateCount)>
    {
        self.candidates
            .iter()
            .map(|(&label, molds)| (TileLabel(label), CandidateCount(molds.len())))
            .collect()
    }

    /// Return the consecutive same-form tile pairs (the `≐` adjacency).
    ///
    /// Each pair `(left, right)` names two tile occurrences that are
    /// consecutive within one form occurrence, skipping any recursive-sort
    /// holes between them (tylr's same-form `≐` relation). Pairs are sorted
    /// and unique.
    #[inline]
    #[must_use]
    pub(crate) fn adjacencies(&self) -> &[(MoldId, MoldId)]
    {
        &self.adjacencies
    }

    /// Return the molds that can be a form's first tile (its regex FIRST set,
    /// recursive-sort holes skipped), sorted and unique.
    ///
    /// A mold in this set opens its form even when it also carries a same-form
    /// `≐`-predecessor reachable only through a nullable prefix (a `def` behind
    /// an optional `@[…]` attribute block), which distinguishes a legitimate
    /// form-start from a genuine mid whose predecessor is required.
    #[inline]
    #[must_use]
    pub(crate) fn form_first(&self) -> &[MoldId]
    {
        &self.form_first
    }

    /// Return the molds that can be a form's **last** tile (its regex LAST set,
    /// recursive-sort holes skipped), sorted and unique — the dual of
    /// [`form_first`](Self::form_first).
    ///
    /// A mold in this set can end its form even when it also carries a
    /// same-form `≐`-successor reachable only through a nullable tail (a
    /// `?` before an optional `hole_name`): the form is already complete at
    /// that tile, so the melder closes it cleanly rather than force-closing
    /// with a ghost end and a spurious missing-tile obligation.
    #[inline]
    #[must_use]
    pub(crate) fn form_last(&self) -> &[MoldId]
    {
        &self.form_last
    }

    /// Return `id`'s form-level closing class, if its completions agree on one.
    #[inline]
    pub(crate) fn closing_class(
        &self,
        id: MoldId,
    ) -> Option<ClosingClass>
    {
        let index = usize::try_from(u32::from(id)).ok()?;
        self.closing.get(index).copied().flatten()
    }

    /// Return the number of mold definitions.
    #[inline]
    #[must_use]
    pub(crate) fn len(&self) -> MoldCount
    {
        MoldCount(self.molds.len())
    }

    /// Iterate every mold definition with its id, in canonical order.
    #[inline]
    pub(crate) fn iter(&self) -> impl Iterator<Item = (MoldId, &MoldDef)>
    {
        self.molds.iter().enumerate().filter_map(|(index, def)| {
            let id = MoldId::try_from(index).ok()?;
            Some((id, def))
        })
    }

    /// Return the PBG fingerprint folding the DAG and mold/context tables.
    #[inline]
    #[must_use]
    pub(crate) const fn fingerprint(&self) -> GrammarFingerprint
    {
        self.fingerprint
    }
}

/// One tile occurrence discovered while walking a rule's regex.
struct Occurrence
{
    /// Tile label at the occurrence.
    label: &'static str,
    /// Interned regex-zipper context for the occurrence.
    rctx: RCtxId,
}

/// Interner assigning [`RCtxId`]s to canonical context keys.
struct ContextInterner
{
    /// Canonical key to dense context id.
    keys: BTreeMap<String, u32>,
    /// Context data in dense id order.
    data: Vec<RCtxData>,
}

impl ContextInterner
{
    /// Create an empty interner.
    fn new() -> Self
    {
        Self {
            keys: BTreeMap::new(),
            data: Vec::new(),
        }
    }

    /// Intern one context, returning its dense id and recording its data.
    fn intern(
        &mut self,
        key: String,
        data: RCtxData,
    ) -> RCtxId
    {
        if let Some(existing) = self.keys.get(&key) {
            return RCtxId::from(*existing);
        }
        let raw = u32::try_from(self.data.len()).unwrap_or(u32::MAX);
        self.keys.insert(key, raw);
        self.data.push(data);
        RCtxId::from(raw)
    }

    /// Consume the interner and return the dense context table.
    fn finish(self) -> Vec<RCtxData>
    {
        self.data
    }
}

/// Symbol-level FIRST/LAST summary over a regex's crossable neighbours.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct FaceCtx
{
    /// Whether the regex derives the empty sequence.
    nullable: bool,
    /// Symbols that can appear first in a generated sequence.
    first: BTreeSet<StepSym>,
    /// Symbols that can appear last in a generated sequence.
    last: BTreeSet<StepSym>,
}

impl FaceCtx
{
    /// The empty (fully nullable, no-symbol) context.
    fn empty() -> Self
    {
        Self {
            nullable: true,
            first: BTreeSet::new(),
            last: BTreeSet::new(),
        }
    }

    /// A single-symbol context.
    fn leaf(sym: StepSym) -> Self
    {
        let mut set = BTreeSet::new();
        set.insert(sym);
        Self {
            nullable: false,
            first: set.clone(),
            last: set,
        }
    }
}

/// Walk one rule's regex, collecting molded tile occurrences and the tile
/// adjacency facet used to derive the `≐` relation.
fn collect_occurrences(
    rule: &Rule,
    interner: &mut ContextInterner,
    out: &mut Vec<Occurrence>,
) -> TileFacet
{
    let left = FaceCtx::empty();
    let right = FaceCtx::empty();
    walk_regex(
        rule.regex(),
        &left,
        &right,
        RegexPath(rule.name),
        interner,
        out,
    )
}

/// A component's folded ending constraint for the closing-class derivation.
///
/// The three-way shape mirrors the three ways to be unsure: no ending at all,
/// endings that agree, and endings that do not. Both unsure answers surface as
/// `None` for the occurrence, because an unclassed minted close pairs with
/// nothing and the failure mode is a suppression not applied.
#[derive(Clone, Copy, Eq, PartialEq)]
enum EndingVerdict
{
    /// No reachable ending: a pure interior cycle or an exit-free tail.
    Empty,
    /// Every reachable ending agrees on one closing class.
    Agree(ClosingClass),
    /// A reachable ending closes nothing the rule opened, or two endings
    /// disagree.
    Divergent,
}

impl EndingVerdict
{
    /// Fold one more reachable ending's class into the verdict.
    fn merge_class(
        self,
        class: ClosingClass,
    ) -> Self
    {
        match self {
            | Self::Empty => Self::Agree(class),
            | Self::Agree(held) if held == class => self,
            | Self::Agree(_) | Self::Divergent => Self::Divergent,
        }
    }

    /// Fold an already-final successor component's verdict into this one.
    fn merge_successor(
        self,
        successor: Self,
    ) -> Self
    {
        match successor {
            | Self::Empty => self,
            | Self::Agree(class) => self.merge_class(class),
            | Self::Divergent => Self::Divergent,
        }
    }
}

/// Dense successor rows over one rule's tile-adjacency graph: the
/// [`EdgeSource`] the condensation algorithm runs against.
#[repr(transparent)]
struct TileGraph
{
    /// Outgoing successors by dense node.
    rows: Vec<Vec<NodeId>>,
}

impl EdgeSource for TileGraph
{
    type Successors<'successors>
        = core::iter::Copied<core::slice::Iter<'successors, NodeId>>
    where
        Self: 'successors;

    #[inline]
    fn node_count(&self) -> NodeCount
    {
        NodeCount::from(u32::try_from(self.rows.len()).unwrap_or(u32::MAX))
    }

    #[inline]
    fn successors(
        &self,
        node: NodeId,
    ) -> Self::Successors<'_>
    {
        let empty: &[NodeId] = &[];
        usize::try_from(u32::from(node))
            .ok()
            .and_then(|index| self.rows.get(index))
            .map_or_else(|| empty.iter().copied(), |row| row.iter().copied())
    }
}

/// Derive each occurrence's **form-level closing class** within one rule.
///
/// `Some(c)` exactly when every completion path from that occurrence ends in a
/// paired closer of class `c`; `None` otherwise. Read the three conditions as
/// three ways to be unsure, because unsure is the safe answer here — a minted
/// close that names no class simply never pairs, and the cost of `None` is a
/// suppression not applied rather than one applied wrongly.
///
/// - **Terminals, not neighbours.** The completions of an occurrence are the
///   rule's LAST tiles reachable from it through the rule's own tile adjacency.
///   A repeat is therefore interior by construction: stepping `; → def → … → ;`
///   around a member list changes nothing about where the paths end, so a
///   member's `=` inside `module M { … }` still reaches only `}`.
/// - **Alternatives intersect.** Reaching two different terminal classes, or
///   any terminal that spells no closer at all, is divergence and yields
///   `None`. This is what keeps `def name = E ;` unclassed: its completions end
///   at `;`, which closes nothing.
/// - **Paired, not merely closing.** A terminal `}` counts only when the rule
///   also writes an opener of that class, so a rule that closes something it
///   never opened claims no class.
///
/// The evaluation runs on the rule's condensation, not per occurrence. Every
/// key in one strongly-connected component reaches the same endings — the
/// component's own ending members plus whatever its exit components reach — so
/// the fold is computed once per component in sinks-first order and shared by
/// every occurrence starting inside it. A per-key memo with a visiting set is
/// NOT equivalent and is deliberately not used: on a repeat-with-exit shape
/// (`a → b`, `b → a`, `b → )`), a search that reaches the cycle through `a`
/// would memoize `b` without the exit's class, and a later query starting at
/// `b` would read the poisoned entry. Component membership decides sharing,
/// not search history, so the condensation has no such order dependence.
///
/// # Contract
/// - requires: `occurrences` and `facet` come from [`collect_occurrences`] on
///   one rule, so every occurrence key and adjacency key belongs to that rule.
/// - ensures: element `i` is `Some(c)` exactly when every completion path from
///   occurrence `i` ends at a paired closer of class `c`, and `None` on
///   divergence or when no completion path ends at a paired closer.
/// - provides: the per-occurrence form-level closing classes of one rule.
/// - fails: never; a graph-validation failure yields the safe `None` for every
///   occurrence rather than a guessed class.
/// - panics: none.
/// - intension: one condensation plus one sinks-first fold — O(keys +
///   adjacency) per rule, independent of the occurrence count.
///
/// # Adequacy
/// - hypothesis: L3 — a repeat-with-exit cycle distinguishes the condensation
///   fold from a visiting-set memo, and a divergent alternative distinguishes
///   the poison case from the agree case.
/// - witness: `gandr_surface_grammar::contracts::closing_class_is_form_level`
/// - witness: `gandr_surface_grammar::contracts::closing_class_repeat_with_exit_shares_its_component_answer`
fn closing_classes(
    occurrences: &[Occurrence],
    facet: &TileFacet,
) -> Vec<Option<ClosingClass>>
{
    // Classes the rule actually opens; a terminal closer of any other class is
    // unpaired within this form.
    let opened: BTreeSet<ClosingClass> = occurrences
        .iter()
        .filter_map(|occurrence| ClosingClass::opening(DelimSpelling(occurrence.label)))
        .collect();

    // The rule's tile graph, dense: every key the adjacency mentions, plus
    // every occurrence's start key (an isolated occurrence is its own ending).
    let mut all: BTreeSet<TileKey> = BTreeSet::new();
    for &(left, right) in &facet.adjacent {
        all.insert(left);
        all.insert(right);
    }
    for occurrence in occurrences {
        all.insert(TileKey::new(TileLabel(occurrence.label), occurrence.rctx));
    }
    let keys: Vec<TileKey> = all.into_iter().collect();
    let dense: BTreeMap<TileKey, u32> = keys
        .iter()
        .copied()
        .enumerate()
        .map(|(index, key)| (key, u32::try_from(index).unwrap_or(u32::MAX)))
        .collect();
    let mut rows: Vec<Vec<NodeId>> = Vec::new();
    rows.resize_with(keys.len(), Vec::new);
    for &(left, right) in &facet.adjacent {
        let (Some(&source), Some(&target)) = (dense.get(&left), dense.get(&right))
        else {
            continue;
        };
        let Ok(index) = usize::try_from(source)
        else {
            continue;
        };
        if let Some(row) = rows.get_mut(index) {
            row.push(NodeId::from(target));
        }
    }
    let graph = TileGraph { rows };

    let Ok(condensed) = condensation(&graph)
    else {
        // Validation cannot fail on a graph built from the rule's own facet;
        // if it ever does, every occurrence takes the safe answer rather than
        // a guessed class.
        return vec![None; occurrences.len()];
    };

    // Per-component successor and predecessor lists over the condensation DAG.
    let component_count = condensed.components.len();
    let mut successors: Vec<Vec<usize>> = Vec::new();
    let mut predecessors: Vec<Vec<usize>> = Vec::new();
    successors.resize_with(component_count, Vec::new);
    predecessors.resize_with(component_count, Vec::new);
    for edge in &condensed.edges {
        let Ok(source) = usize::try_from(u32::from(edge.source))
        else {
            continue;
        };
        let Ok(target) = usize::try_from(u32::from(edge.target))
        else {
            continue;
        };
        if let Some(row) = successors.get_mut(source) {
            row.push(target);
        }
        if let Some(row) = predecessors.get_mut(target) {
            row.push(source);
        }
    }

    // Sinks-first component order (Kahn from the sink side): a component is
    // folded only after every successor component's verdict is final, so each
    // component reads final inputs exactly once.
    let mut out_degree: Vec<usize> = Vec::new();
    out_degree.resize_with(component_count, usize::default);
    for (index, row) in successors.iter().enumerate() {
        if let Some(degree) = out_degree.get_mut(index) {
            *degree = row.len();
        }
    }
    let mut sinks: Vec<usize> = out_degree
        .iter()
        .enumerate()
        .filter_map(|(index, &degree)| (degree == 0).then_some(index))
        .collect();
    let mut order: Vec<usize> = Vec::new();
    while let Some(component) = sinks.pop() {
        order.push(component);
        let Some(row) = predecessors.get(component)
        else {
            continue;
        };
        for &predecessor in row {
            let Some(degree) = out_degree.get_mut(predecessor)
            else {
                continue;
            };
            *degree = degree.saturating_sub(1);
            if *degree == 0 {
                sinks.push(predecessor);
            }
        }
    }

    // Fold each component: its own ending members first, then its successor
    // components' verdicts. A member is an ending when it has no successors or
    // sits in the rule's LAST set — the search's halt condition, unchanged.
    let mut verdicts: Vec<EndingVerdict> = vec![EndingVerdict::Empty; component_count];
    for &component in &order {
        let mut verdict = EndingVerdict::Empty;
        if let Some(members) = condensed.components.get(component) {
            for &node in members {
                let Ok(index) = usize::try_from(u32::from(node))
                else {
                    continue;
                };
                let Some(&key) = keys.get(index)
                else {
                    continue;
                };
                let has_successors = graph.rows.get(index).is_some_and(|row| !row.is_empty());
                if has_successors && !facet.last.contains(&key) {
                    continue;
                }
                // A completion ends here. Every one of them must agree.
                verdict = match ClosingClass::closing(DelimSpelling(key.label().0))
                    .filter(|reached| opened.contains(reached))
                {
                    | Some(class) => verdict.merge_class(class),
                    | None => EndingVerdict::Divergent,
                };
            }
        }
        if let Some(row) = successors.get(component) {
            for &successor in row {
                if let Some(&found) = verdicts.get(successor) {
                    verdict = verdict.merge_successor(found);
                }
            }
        }
        if let Some(slot) = verdicts.get_mut(component) {
            *slot = verdict;
        }
    }

    // Every occurrence shares the answer of the component it starts in.
    let mut component_of: Vec<u32> = vec![u32::MAX; keys.len()];
    for (component, members) in condensed.components.iter().enumerate() {
        let Ok(component) = u32::try_from(component)
        else {
            continue;
        };
        for &node in members {
            let Ok(index) = usize::try_from(u32::from(node))
            else {
                continue;
            };
            if let Some(slot) = component_of.get_mut(index) {
                *slot = component;
            }
        }
    }
    occurrences
        .iter()
        .map(|occurrence| {
            let start = TileKey::new(TileLabel(occurrence.label), occurrence.rctx);
            let node = dense.get(&start)?;
            let index = usize::try_from(*node).ok()?;
            let component = component_of.get(index)?;
            let component = usize::try_from(*component).ok()?;
            let verdict = verdicts.get(component)?;
            match *verdict {
                | EndingVerdict::Agree(class) => Some(class),
                | EndingVerdict::Empty | EndingVerdict::Divergent => None,
            }
        })
        .collect()
}

/// Iteratively walk a regex, interning each tile occurrence's context and
/// returning the subtree's tile-adjacency facet.
///
/// Recursive-sort holes are treated as tile-transparent (nullable, no tiles):
/// two tiles separated only by holes are consecutive within their form, so
/// `( E )` yields the pair `( ≐ )` and `if E then` yields `if ≐ then`.
fn walk_regex(
    regex: &Regex,
    left: &FaceCtx,
    right: &FaceCtx,
    path: RegexPath<'_>,
    interner: &mut ContextInterner,
    out: &mut Vec<Occurrence>,
) -> TileFacet
{
    enum WalkFrame<'regex>
    {
        Enter
        {
            regex: &'regex Regex,
            left: FaceCtx,
            right: FaceCtx,
            path: String,
        },
        FinishSeq
        {
            count: usize,
        },
        FinishAlt
        {
            count: usize,
        },
        FinishOptional,
        FinishRepeat,
    }

    let mut frames = vec![WalkFrame::Enter {
        regex,
        left: left.clone(),
        right: right.clone(),
        path: path.0.to_owned(),
    }];
    let mut values = Vec::new();

    while let Some(frame) = frames.pop() {
        match frame {
            | WalkFrame::Enter {
                regex: node,
                left: node_left,
                right: node_right,
                path: node_path,
            } => match *node {
                | Regex::Empty | Regex::Sym(Sym::Sort(_)) => values.push(TileFacet::transparent()),
                | Regex::Sym(Sym::Tile(tile)) => {
                    let data = context_data(&node_left, &node_right);
                    let rctx = interner.intern(node_path, data);
                    out.push(Occurrence {
                        label: tile.label,
                        rctx,
                    });
                    values.push(TileFacet::leaf(TileKey::new(TileLabel(tile.label), rctx)));
                },
                | Regex::Seq(ref items) => {
                    frames.push(WalkFrame::FinishSeq { count: items.len() });
                    for (index, item) in items.iter().enumerate().rev() {
                        let before = items.get(.. index).unwrap_or(&[]);
                        let after = items.get(index.saturating_add(1) ..).unwrap_or(&[]);
                        let child_left = compose_seq(&node_left, &face_of_slice(before));
                        let child_right = compose_seq(&face_of_slice(after), &node_right);
                        let frame_tag =
                            format!("Q{}\x1e{}", canon_slice(before), canon_slice(after));
                        let child_path = format!("{node_path}\x1f{frame_tag}");
                        frames.push(WalkFrame::Enter {
                            regex: item,
                            left: child_left,
                            right: child_right,
                            path: child_path,
                        });
                    }
                },
                | Regex::Alt(ref items) => {
                    frames.push(WalkFrame::FinishAlt { count: items.len() });
                    for (index, item) in items.iter().enumerate().rev() {
                        let mut siblings = Vec::new();
                        for (other_index, other) in items.iter().enumerate() {
                            if other_index != index {
                                siblings.push(canon(other));
                            }
                        }
                        siblings.sort();
                        let frame_tag = format!("A{}", siblings.join("\x1d"));
                        let child_path = format!("{node_path}\x1f{frame_tag}");
                        frames.push(WalkFrame::Enter {
                            regex: item,
                            left: node_left.clone(),
                            right: node_right.clone(),
                            path: child_path,
                        });
                    }
                },
                | Regex::Optional(ref inner) => {
                    frames.push(WalkFrame::FinishOptional);
                    frames.push(WalkFrame::Enter {
                        regex: inner,
                        left: node_left,
                        right: node_right,
                        path: format!("{node_path}\x1fO"),
                    });
                },
                | Regex::Repeat(ref inner) => {
                    frames.push(WalkFrame::FinishRepeat);
                    frames.push(WalkFrame::Enter {
                        regex: inner,
                        left: node_left,
                        right: node_right,
                        path: format!("{node_path}\x1fR"),
                    });
                },
            },
            | WalkFrame::FinishSeq { count } => {
                let split = values.len().saturating_sub(count);
                let children = values.split_off(split);
                let mut acc = TileFacet::empty();
                for child in children {
                    acc = seq_facet(&acc, &child);
                }
                values.push(acc);
            },
            | WalkFrame::FinishAlt { count } => {
                let split = values.len().saturating_sub(count);
                let children = values.split_off(split);
                let mut acc = TileFacet::void();
                for child in children {
                    acc = alt_facet(&acc, &child);
                }
                values.push(acc);
            },
            | WalkFrame::FinishOptional => {
                let mut child = values.pop().unwrap_or_else(TileFacet::transparent);
                child.nullable = true;
                values.push(child);
            },
            | WalkFrame::FinishRepeat => {
                let child = values.pop().unwrap_or_else(TileFacet::transparent);
                values.push(repeat_facet(&child));
            },
        }
    }

    values.pop().unwrap_or_else(TileFacet::transparent)
}

/// Resolve a set of `(label, rctx)` tile keys to sorted, unique [`MoldId`]s —
/// the shared resolver for both the form-first (FIRST) and form-last (LAST)
/// mold sets.
fn resolve_keys(
    molds: &[MoldDef],
    keys: &BTreeSet<TileKey>,
) -> Vec<MoldId>
{
    let index = tile_index(molds);
    let mut ids: Vec<MoldId> = keys
        .iter()
        .filter_map(|key| index.get(key).copied())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Resolve `(label, rctx)` adjacency pairs to sorted, unique [`MoldId`] pairs.
fn resolve_adjacencies(
    molds: &[MoldDef],
    keys: &BTreeSet<(TileKey, TileKey)>,
) -> Vec<(MoldId, MoldId)>
{
    let index = tile_index(molds);
    let mut pairs: Vec<(MoldId, MoldId)> = keys
        .iter()
        .filter_map(|&(left, right)| {
            let left_id = *index.get(&left)?;
            let right_id = *index.get(&right)?;
            Some((left_id, right_id))
        })
        .collect();
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// Index `molds` by their `(label, rctx)` tile keys.
///
/// # Contract
/// - ensures: every mold whose position fits the checked `u32` table bound is
///   present under its [`TileKey`]; positions beyond the bound are skipped,
///   which the build-time length check rules out.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — adjacency and context-step resolution read the index
///   across labels that repeat between regex contexts.
/// - witness: `gandr_surface_grammar::contracts::rctx_steps_cross_adjacent_symbols`
fn tile_index(molds: &[MoldDef]) -> BTreeMap<TileKey, MoldId>
{
    let mut index: BTreeMap<TileKey, MoldId> = BTreeMap::new();
    for (position, mold) in molds.iter().enumerate() {
        if let Ok(id) = MoldId::try_from(position) {
            index.insert(TileKey::new(TileLabel(mold.label), mold.rctx), id);
        }
    }
    index
}

/// Build the precomputed context data from a tile's left/right faces.
fn context_data(
    left: &FaceCtx,
    right: &FaceCtx,
) -> RCtxData
{
    RCtxData {
        left_faces_sort: left.last.iter().any(|sym| matches!(sym, StepSym::Sort(_))),
        right_faces_sort: right
            .first
            .iter()
            .any(|sym| matches!(sym, StepSym::Sort(_))),
        left_steps: left
            .last
            .iter()
            .copied()
            .map(|crossed| RCtxStep { crossed })
            .collect(),
        right_steps: right
            .first
            .iter()
            .copied()
            .map(|crossed| RCtxStep { crossed })
            .collect(),
    }
}

/// A tile occurrence identity used by the adjacency facet: `(label, rctx)`.
///
/// This is the pre-resolution key; it is mapped to a [`MoldId`] after
/// mold-table assignment, when `(label, rctx)` is known to be unique (the
/// Unique Tiles gate).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TileKey((TileLabel, RCtxId));

impl TileKey
{
    /// Pair a tile label with its interned regex-context id.
    #[inline]
    #[must_use]
    const fn new(
        label: TileLabel,
        rctx: RCtxId,
    ) -> Self
    {
        Self((label, rctx))
    }

    /// The tile label this key pairs.
    #[inline]
    #[must_use]
    const fn label(self) -> TileLabel
    {
        self.0.0
    }
}

/// Borrowed regex-context path while deriving mold zipper identities.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RegexPath<'path>(&'path str);

/// Whether a context side faces a recursive sort.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SortFacing(bool);

/// Bottom-up tile-adjacency summary of a regex subtree.
///
/// Recursive-sort holes are tile-transparent, so `first`/`last` are the tiles
/// that can begin or end the subtree once holes are skipped, and `adjacent`
/// records the consecutive tile pairs (across holes) that seed the `≐`
/// relation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct TileFacet
{
    /// Whether the subtree can contribute no tiles (holes are transparent).
    nullable: bool,
    /// Tiles that can be the subtree's first tile.
    first: BTreeSet<TileKey>,
    /// Tiles that can be the subtree's last tile.
    last: BTreeSet<TileKey>,
    /// Consecutive tile pairs within the subtree.
    adjacent: BTreeSet<(TileKey, TileKey)>,
}

impl TileFacet
{
    /// The identity element for sequential composition (tile-empty).
    fn empty() -> Self
    {
        Self {
            nullable: true,
            first: BTreeSet::new(),
            last: BTreeSet::new(),
            adjacent: BTreeSet::new(),
        }
    }

    /// A tile-transparent hole (`Empty` or a recursive sort): nullable, no
    /// tiles.
    fn transparent() -> Self
    {
        Self::empty()
    }

    /// The identity element for alternation (matches nothing).
    fn void() -> Self
    {
        Self {
            nullable: false,
            first: BTreeSet::new(),
            last: BTreeSet::new(),
            adjacent: BTreeSet::new(),
        }
    }

    /// A single tile occurrence.
    fn leaf(key: TileKey) -> Self
    {
        let mut set = BTreeSet::new();
        set.insert(key);
        Self {
            nullable: false,
            first: set.clone(),
            last: set,
            adjacent: BTreeSet::new(),
        }
    }
}

/// Compose two contexts as a concatenation `left · right`.
fn compose_seq(
    left: &FaceCtx,
    right: &FaceCtx,
) -> FaceCtx
{
    let mut first = left.first.clone();
    if left.nullable {
        first.extend(right.first.iter().copied());
    }
    let mut last = right.last.clone();
    if right.nullable {
        last.extend(left.last.iter().copied());
    }
    FaceCtx {
        nullable: left.nullable && right.nullable,
        first,
        last,
    }
}

/// Summarise the concatenation of a regex slice.
fn face_of_slice(items: &[Regex]) -> FaceCtx
{
    let mut acc = FaceCtx::empty();
    for item in items {
        acc = compose_seq(&acc, &face_of(item));
    }
    acc
}

/// Summarise a regex's first/last crossable symbols and nullability.
fn face_of(regex: &Regex) -> FaceCtx
{
    enum FaceFrame<'regex>
    {
        Enter(&'regex Regex),
        FinishSeq
        {
            count: usize,
        },
        FinishAlt
        {
            count: usize,
        },
        FinishNullable,
    }

    let mut frames = vec![FaceFrame::Enter(regex)];
    let mut values = Vec::new();

    while let Some(frame) = frames.pop() {
        match frame {
            | FaceFrame::Enter(current) => match *current {
                | Regex::Empty => values.push(FaceCtx::empty()),
                | Regex::Sym(Sym::Sort(sort)) => values.push(FaceCtx::leaf(StepSym::Sort(sort))),
                | Regex::Sym(Sym::Tile(tile)) => {
                    values.push(FaceCtx::leaf(StepSym::Tile(tile.label)));
                },
                | Regex::Seq(ref items) => {
                    frames.push(FaceFrame::FinishSeq { count: items.len() });
                    for item in items.iter().rev() {
                        frames.push(FaceFrame::Enter(item));
                    }
                },
                | Regex::Alt(ref items) => {
                    frames.push(FaceFrame::FinishAlt { count: items.len() });
                    for item in items.iter().rev() {
                        frames.push(FaceFrame::Enter(item));
                    }
                },
                | Regex::Optional(ref inner) | Regex::Repeat(ref inner) => {
                    frames.push(FaceFrame::FinishNullable);
                    frames.push(FaceFrame::Enter(inner));
                },
            },
            | FaceFrame::FinishSeq { count } => {
                let split = values.len().saturating_sub(count);
                let children = values.split_off(split);
                let mut acc = FaceCtx::empty();
                for child in children {
                    acc = compose_seq(&acc, &child);
                }
                values.push(acc);
            },
            | FaceFrame::FinishAlt { count } => {
                let split = values.len().saturating_sub(count);
                let children = values.split_off(split);
                let mut acc = FaceCtx::default();
                for current in children {
                    acc.nullable = acc.nullable || current.nullable;
                    acc.first.extend(current.first);
                    acc.last.extend(current.last);
                }
                values.push(acc);
            },
            | FaceFrame::FinishNullable => {
                let mut summary = values.pop().unwrap_or_else(FaceCtx::empty);
                summary.nullable = true;
                values.push(summary);
            },
        }
    }

    values.pop().unwrap_or_else(FaceCtx::empty)
}

/// Derive the precedence bounds for a mold from its context nullability.
///
/// The precedence bound applies on a side exactly when a recursive sort faces
/// the context there (tylr `Mold.bounds`); otherwise the side is unbounded.
fn bounds_for(
    mold: &MoldDef,
    rctxs: &[RCtxData],
) -> (Bound<Prec>, Bound<Prec>)
{
    let index = usize::try_from(u32::from(mold.rctx)).unwrap_or(usize::MAX);
    match rctxs.get(index) {
        | Some(data) => (
            side_bound(SortFacing(data.left_faces_sort), mold.prec),
            side_bound(SortFacing(data.right_faces_sort), mold.prec),
        ),
        | None => (Bound::Root, Bound::Root),
    }
}

/// Map one side's sort-facing to a precedence bound.
const fn side_bound(
    faces_sort: SortFacing,
    prec: Prec,
) -> Bound<Prec>
{
    if faces_sort.0 {
        Bound::Value(prec)
    }
    else {
        Bound::Root
    }
}

/// Canonically serialise a regex slice as a comma-joined sequence.
fn canon_slice(items: &[Regex]) -> String
{
    items.iter().map(canon).collect::<Vec<_>>().join(",")
}

/// Canonically serialise a regex, treating alternation branches as unordered.
fn canon(regex: &Regex) -> String
{
    enum CanonFrame<'regex>
    {
        Enter(&'regex Regex),
        FinishSeq
        {
            count: usize,
        },
        FinishAlt
        {
            count: usize,
        },
        FinishOptional,
        FinishRepeat,
    }

    let mut frames = vec![CanonFrame::Enter(regex)];
    let mut values = Vec::new();

    while let Some(frame) = frames.pop() {
        match frame {
            | CanonFrame::Enter(current) => match *current {
                | Regex::Empty => values.push("e".to_owned()),
                | Regex::Sym(Sym::Sort(sort)) => {
                    values.push(format!("s{}", u16::from(sort.as_u16())));
                },
                | Regex::Sym(Sym::Tile(tile)) => {
                    values.push(format!("t{}:{}", tile.label.len(), tile.label));
                },
                | Regex::Seq(ref items) => {
                    frames.push(CanonFrame::FinishSeq { count: items.len() });
                    for item in items.iter().rev() {
                        frames.push(CanonFrame::Enter(item));
                    }
                },
                | Regex::Alt(ref items) => {
                    frames.push(CanonFrame::FinishAlt { count: items.len() });
                    for item in items.iter().rev() {
                        frames.push(CanonFrame::Enter(item));
                    }
                },
                | Regex::Optional(ref inner) => {
                    frames.push(CanonFrame::FinishOptional);
                    frames.push(CanonFrame::Enter(inner));
                },
                | Regex::Repeat(ref inner) => {
                    frames.push(CanonFrame::FinishRepeat);
                    frames.push(CanonFrame::Enter(inner));
                },
            },
            | CanonFrame::FinishSeq { count } => {
                let split = values.len().saturating_sub(count);
                let parts = values.split_off(split);
                values.push(format!("Q[{}]", parts.join(",")));
            },
            | CanonFrame::FinishAlt { count } => {
                let split = values.len().saturating_sub(count);
                let mut parts = values.split_off(split);
                parts.sort();
                values.push(format!("A[{}]", parts.join(",")));
            },
            | CanonFrame::FinishOptional => {
                let inner = values.pop().unwrap_or_default();
                values.push(format!("O[{inner}]"));
            },
            | CanonFrame::FinishRepeat => {
                let inner = values.pop().unwrap_or_default();
                values.push(format!("R[{inner}]"));
            },
        }
    }

    values.pop().unwrap_or_default()
}

/// Compose two facets as a concatenation `left · right`.
fn seq_facet(
    left: &TileFacet,
    right: &TileFacet,
) -> TileFacet
{
    let mut adjacent = left.adjacent.clone();
    adjacent.extend(right.adjacent.iter().copied());
    for tail in &left.last {
        for head in &right.first {
            adjacent.insert((*tail, *head));
        }
    }
    let mut first = left.first.clone();
    if left.nullable {
        first.extend(right.first.iter().copied());
    }
    let mut last = right.last.clone();
    if right.nullable {
        last.extend(left.last.iter().copied());
    }
    TileFacet {
        nullable: left.nullable && right.nullable,
        first,
        last,
        adjacent,
    }
}

/// Compose two facets as an alternation `left | right`.
fn alt_facet(
    left: &TileFacet,
    right: &TileFacet,
) -> TileFacet
{
    let mut first = left.first.clone();
    first.extend(right.first.iter().copied());
    let mut last = left.last.clone();
    last.extend(right.last.iter().copied());
    let mut adjacent = left.adjacent.clone();
    adjacent.extend(right.adjacent.iter().copied());
    TileFacet {
        nullable: left.nullable || right.nullable,
        first,
        last,
        adjacent,
    }
}

/// Summarise a `repeat(inner)`: nullable, with the repetition seam
/// `last·first`.
fn repeat_facet(inner: &TileFacet) -> TileFacet
{
    let mut facet = inner.clone();
    facet.nullable = true;
    for tail in &inner.last {
        for head in &inner.first {
            facet.adjacent.insert((*tail, *head));
        }
    }
    facet
}

/// Fold the precedence DAG fingerprint and the mold/context tables.
fn fold_fingerprint(
    dag_fingerprint: GrammarFingerprint,
    molds: &[MoldDef],
    rctxs: &[RCtxData],
) -> GrammarFingerprint
{
    let mut hasher = Fnv64::new();
    hasher.write_byte(HashByte(FRAME_MOLD));
    hasher.write_u64(HashU64(dag_fingerprint.0));
    hasher.write_u64(HashU64(u64::try_from(molds.len()).unwrap_or(u64::MAX)));
    for mold in molds {
        hasher.write_bytes(HashBytes(mold.label.as_bytes()));
        hasher.write_byte(HashByte(0));
        hasher.write_u32(HashU32(u32::from(mold.rctx)));
        hasher.write_u16(HashU16(u16::from(mold.prec.index())));
        hasher.write_u16(HashU16(mold.sort.as_u16().into()));
    }
    hasher.write_u64(HashU64(u64::try_from(rctxs.len()).unwrap_or(u64::MAX)));
    for data in rctxs {
        hasher.write_byte(HashByte(u8::from(data.left_faces_sort)));
        hasher.write_byte(HashByte(u8::from(data.right_faces_sort)));
        fold_steps(&mut hasher, &data.left_steps);
        fold_steps(&mut hasher, &data.right_steps);
    }
    hasher.finish()
}

/// Fold a step slice into the fingerprint.
fn fold_steps(
    hasher: &mut Fnv64,
    steps: &[RCtxStep],
)
{
    hasher.write_u64(HashU64(u64::try_from(steps.len()).unwrap_or(u64::MAX)));
    for step in steps {
        match step.crossed {
            | StepSym::Sort(sort) => {
                hasher.write_byte(HashByte(b'S'));
                hasher.write_u16(HashU16(sort.as_u16().into()));
            },
            | StepSym::Tile(label) => {
                hasher.write_byte(HashByte(b'T'));
                hasher.write_bytes(HashBytes(label.as_bytes()));
                hasher.write_byte(HashByte(0));
            },
        }
    }
}

/// Single byte fed into the mold-table fingerprint.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HashByte(u8);

/// Borrowed byte slice fed into the mold-table fingerprint.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HashBytes<'bytes>(&'bytes [u8]);

/// Little-endian `u16` fed into the mold-table fingerprint.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HashU16(u16);

/// Little-endian `u32` fed into the mold-table fingerprint.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HashU32(u32);

/// Little-endian `u64` fed into the mold-table fingerprint.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HashU64(u64);

/// Minimal framed FNV-1a accumulator for the mold-table fingerprint.
#[repr(transparent)]
struct Fnv64
{
    /// Current hash state.
    state: u64,
}

impl Fnv64
{
    /// Create a fresh accumulator.
    fn new() -> Self
    {
        Self { state: FNV_OFFSET }
    }

    /// Mix one byte.
    fn write_byte(
        &mut self,
        byte: HashByte,
    )
    {
        self.state ^= u64::from(byte.0);
        self.state = self.state.wrapping_mul(FNV_PRIME);
    }

    /// Mix a byte slice.
    fn write_bytes(
        &mut self,
        bytes: HashBytes<'_>,
    )
    {
        for byte in bytes.0 {
            self.write_byte(HashByte(*byte));
        }
    }

    /// Mix a little-endian `u16`.
    fn write_u16(
        &mut self,
        value: HashU16,
    )
    {
        self.write_bytes(HashBytes(&value.0.to_le_bytes()));
    }

    /// Mix a little-endian `u32`.
    fn write_u32(
        &mut self,
        value: HashU32,
    )
    {
        self.write_bytes(HashBytes(&value.0.to_le_bytes()));
    }

    /// Mix a little-endian `u64`.
    fn write_u64(
        &mut self,
        value: HashU64,
    )
    {
        self.write_bytes(HashBytes(&value.0.to_le_bytes()));
    }

    /// Return the accumulated hash.
    fn finish(self) -> GrammarFingerprint
    {
        GrammarFingerprint(self.state)
    }
}
