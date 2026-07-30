//! Generative walk-index front-end over the delivered `gandr-theory-graphs`
//! engine.
//!
//! The delivered walk engine is a declarative finite walk machine: callers
//! supply exact ends and direct steps, and [`WalkIndex::build`] computes the
//! deterministic closure, filters, and total order. This module is the
//! generative front-end (proposal P5) that instantiates the
//! machine for the real gandr PBG, replacing the placeholder direct-row
//! projection:
//!
//! 1. **Vertices** — every [`MoldId`] (one tile occurrence in some `G(sort,
//!    prec)` form) becomes an [`End::Node`], plus [`End::Root`].
//! 2. **Reachability rows** — a molded terminal is reachable from the root
//!    boundary. Every gandr form is enterable through some unbounded cross-sort
//!    hole, so the whole mold table is reachable; one flat root row per mold
//!    keeps each terminal a closure sink, which is what makes the index
//!    tractable at gandr's ~1000-tile scale. [`WalkIndex::molds`] is then the
//!    real label→mold reachability projection.
//! 3. **Comparison rows** — the operator-precedence relation (paper Fig. 15),
//!    read off `PrecDag` *checks* only (the checks seam, never level
//!    arithmetic). The relation is intrinsically a `(sort, prec)` form-group
//!    matrix, so it is carried on one representative tile per group: `t_L ⋖
//!    t_R` when `t_R`'s group is tighter in the shared sort, `t_L ⋗ t_R`
//!    dually. These land as `lt`/`gt` direct rows over ~20 representatives, so
//!    the closure stays bounded by the shallow precedence bands rather than the
//!    layered per-tile fan-out that makes the eager transitive closure blow up.
//!    The `≐` face (consecutive tiles of one form) is the same-form adjacency
//!    `Pbg::adjacencies`, consumed directly by [`comparison_table`] rather than
//!    materialised as chainable rows. Grout sits at `⊥`, comparable to
//!    everything, so incomparable tile pairs simply have no comparison and
//!    repair routes through grout (Degrout totality).
//!
//! **Design note (W3′ perf profile).** The literal per-tile operator comparison
//! feeds the engine's eager full transitive closure an exponentially-pathed
//! layered DAG (many tiles per band across a deep band) and is intractable at
//! this scale; it was profiled and attributed to the eager closure, and the
//! front-end was optimized to the group-representative form above with
//! `gandr-theory-graphs` left unmodified (`docs/METRICS.md`).

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;

use gandr_theory_graphs::Dir;
use gandr_theory_graphs::End;
use gandr_theory_graphs::Prec;
use gandr_theory_graphs::SeenKeyVerdict;
use gandr_theory_graphs::StanceTileSorted;
use gandr_theory_graphs::Swing;
use gandr_theory_graphs::Walk;
use gandr_theory_graphs::WalkChainLength;
use gandr_theory_graphs::WalkIndex;
use gandr_theory_graphs::WalkSpec;
use gandr_theory_graphs::WalkSym;
use gandr_theory_graphs::WalkSymbolKey;

use crate::model::Pbg;
use crate::model::PbgError;
use crate::model::Sort;
use crate::model::TileLabel;
use crate::mold::MoldId;

/// The maximum alternating chain length materialised by the walk index.
///
/// The generated direct rows are single-swing (`chain_len == 1`); the closure
/// combines them through the shallow same-sort precedence bands. This cap is an
/// engineering ceiling checked by the engine's `guard_cap` `debug_assert` — a
/// materialised walk that exceeds it surfaces as
/// [`gandr_theory_graphs::WalkBuildError::ChainLengthExceeded`], never a silent
/// truncation.
pub const MAX_WALK_CHAIN_LEN: u32 = 64;

/// Gandr-specific symbol vocabulary for the generic walk index.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the grammar walk-symbol marker is a fixed zero-field vocabulary for the generic walk index"
)]
pub struct GrammarWalkSym;

/// A precedence-bounded grammar nonterminal.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a grammar nonterminal is exactly its (sort, prec) PBG form coordinates"
)]
pub struct GrammarNonterminal
{
    /// Surface grammar sort.
    pub sort: Sort,
    /// Precedence node constraining the form.
    pub prec: Prec,
}

impl GrammarNonterminal
{
    /// Construct a nonterminal from its exact PBG form key.
    #[inline]
    #[must_use]
    pub const fn new(
        sort: Sort,
        prec: Prec,
    ) -> Self
    {
        Self { sort, prec }
    }
}

/// Exact tile stance carried by walk boundaries.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a walk tile stance is exactly its (label, mold, sort) coordinates"
)]
pub struct GrammarTile
{
    /// Static tree-sitter label for the tile.
    pub label: TileLabel,
    /// Mold-table reference for the tile occurrence.
    pub mold_id: MoldId,
    /// Checked PBG sort of the producing form.
    pub sort: Sort,
}

impl GrammarTile
{
    /// Construct a walk stance from a declared mold.
    #[inline]
    #[must_use]
    pub const fn new(
        label: TileLabel,
        mold_id: MoldId,
        sort: Sort,
    ) -> Self
    {
        Self {
            label,
            mold_id,
            sort,
        }
    }
}

impl WalkSym for GrammarWalkSym
{
    type Nonterminal = GrammarNonterminal;
    type Stance = GrammarTile;
    type Sort = Sort;
    type Bounds = Prec;
    type Label = TileLabel;
    type Mold = MoldId;

    #[inline]
    fn nonterminal_sort(nonterminal: &Self::Nonterminal) -> Self::Sort
    {
        nonterminal.sort
    }

    #[inline]
    fn nonterminal_bounds(nonterminal: &Self::Nonterminal) -> Self::Bounds
    {
        nonterminal.prec
    }

    #[inline]
    fn stance_sort(stance: &Self::Stance) -> Self::Sort
    {
        stance.sort
    }

    #[inline]
    fn stance_tile_sorted(_stance: &Self::Stance) -> StanceTileSorted
    {
        // Every gandr stance is a molded terminal; the minimality filter treats
        // them all as tile-sorted so no shorter same-level walk is masked.
        StanceTileSorted::from(true)
    }

    #[inline]
    fn label_mold(stance: &Self::Stance) -> Option<(Self::Label, Self::Mold)>
    {
        Some((stance.label, stance.mold_id))
    }

    #[inline]
    fn nonterminal_key(nonterminal: &Self::Nonterminal) -> WalkSymbolKey
    {
        let sort = StableHash(u64::from(u16::from(nonterminal.sort.as_u16())));
        let prec = StableHash(u64::from(u16::from(nonterminal.prec.index())));
        WalkSymbolKey::from(stable_mix(stable_mix(StableHash(FNV_OFFSET), sort), prec).0)
    }

    #[inline]
    fn stance_key(stance: &Self::Stance) -> WalkSymbolKey
    {
        let mut hash = stable_mix(
            StableHash(FNV_OFFSET),
            StableHash(u64::from(u16::from(stance.sort.as_u16()))),
        );
        hash = stable_mix(hash, StableHash(u64::from(u32::from(stance.mold_id))));
        for byte in stance.label.as_ref().as_bytes() {
            hash = stable_mix(hash, StableHash(u64::from(*byte)));
        }
        WalkSymbolKey::from(hash.0)
    }
}

/// One derivable operator-precedence comparison (paper Fig. 15).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Relation
{
    /// `t_L ⋖ t_R`: the left tile yields; the right tile's form nests to the
    /// right.
    Yields,
    /// `t_L ≐ t_R`: the tiles belong to one form (same-precedence).
    Equal,
    /// `t_L ⋗ t_R`: the left tile takes precedence; its form nests to the left.
    Takes,
}

/// One row of the generated comparison table, indexed by the mediating sort.
///
/// The optional nonterminal of paper Fig. 15 is the sort `ρ` at which the two
/// tiles' forms are compared; gandr's DAG makes it a concrete [`Sort`], which
/// is what restores the soundness Floyd's unindexed relation lacks.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a comparison row is exactly the ordered mold pair, relation, and mediating sort"
)]
pub struct ComparisonRow
{
    /// The left tile occurrence.
    pub left: MoldId,
    /// The right tile occurrence.
    pub right: MoldId,
    /// The derived relation.
    pub cmp: Comparison,
    /// The sort `ρ` mediating the comparison.
    pub sort: Sort,
}

/// Precomputed per-mold facts the front-end reads from the PBG.
#[derive(Clone, Copy, Debug)]
struct MoldFacts
{
    /// The mold's stance vertex.
    stance: GrammarTile,
    /// The mold's producing form nonterminal.
    nonterminal: GrammarNonterminal,
}

/// Build the checked gandr walk index for a PBG (the generative front-end).
///
/// # Contract
/// - requires: `pbg` was built by the checked PBG builder.
/// - ensures: every declared mold is reachable from [`End::Root`] and projects
///   under its label through [`WalkIndex::molds`]; the `lt`/`gt` faces carry
///   the form-group operator-precedence relation, generated by `PrecDag`
///   checks.
/// - provides: the generative walk front-end (proposal P5) that replaces the
///   placeholder direct-row projection.
/// - fails: returns [`PbgError`] for walk construction failures, including
///   [`gandr_theory_graphs::WalkBuildError::ChainLengthExceeded`] when a
///   materialised walk exceeds [`MAX_WALK_CHAIN_LEN`].
/// - panics: none.
/// - intension: molds are traversed in canonical table order via
///   [`Pbg::iter_molds`]; comparison rows iterate ordered form-group
///   representatives and filter through `PrecDag` checks; all extraction is
///   sorted.
///
/// # Errors
/// Returns [`PbgError`] when the walk builder rejects a constructed row.
///
/// # Adequacy
/// - hypothesis: L3 generative — the built-in surface plus the synthetic infix
///   PBG witness reachability, the three comparison faces, coherence with the
///   precedence DAG, and conflict-freedom.
/// - witness: `gandr_surface_grammar::contracts::walk_index_projects_every_mold_once`
/// - witness: `gandr_surface_grammar::contracts::comparison_table_coheres_with_precedence`
/// - witness: `gandr_surface_grammar::contracts::comparison_table_is_conflict_free`
#[inline]
pub fn walk_index(pbg: &Pbg) -> Result<WalkIndex<GrammarWalkSym>, PbgError>
{
    let spec = build_spec(pbg)?;
    WalkIndex::build(&spec).map_err(PbgError::from)
}

/// Report the `(sort, bounds)`-vs-sort-only seen-key verdict on the real PBG.
///
/// This is the diagnostic comparison recorded against
/// the real gandr spec. The generative front-end emits explicit direct rows and
/// no swing seeds or arcs, so there is no swing closure to diverge — the
/// verdict is [`SeenKeyVerdict::Equivalent`], and the §5.2 sort-only-dedup
/// hazard is not exercised by this construction.
///
/// # Contract
/// - requires: `pbg` was built by the checked PBG builder.
/// - ensures: returns the seen-key comparison verdict for the real gandr spec.
/// - provides: the recorded verdict for `docs/METRICS.md`.
/// - fails: returns [`PbgError`] for spec or comparison construction failures.
/// - panics: none.
///
/// # Errors
/// Returns [`PbgError`] when the spec cannot be built or the comparison fails.
///
/// # Adequacy
/// - hypothesis: L1 — a deterministic diagnostic over the real spec; the value
///   is witnessed by the metrics test.
/// - witness: `gandr_surface_grammar::contracts::seen_key_verdict_is_recorded`
#[inline]
pub fn seen_key_verdict(pbg: &Pbg) -> Result<SeenKeyVerdict, PbgError>
{
    let spec = build_spec(pbg)?;
    WalkIndex::compare_seen_keys(&spec).map_err(PbgError::from)
}

/// Build the generative walk specification for a PBG.
fn build_spec(pbg: &Pbg) -> Result<WalkSpec<GrammarWalkSym>, PbgError>
{
    let facts = mold_facts(pbg);
    let mut spec = WalkSpec::<GrammarWalkSym>::new(WalkChainLength::from(MAX_WALK_CHAIN_LEN))
        .map_err(PbgError::from)?;

    if let Some(first) = facts.first() {
        spec.set_root_entry(first.nonterminal);
    }

    // Reachability rows: every molded terminal is reachable from the root
    // boundary. Every gandr form is enterable through some unbounded cross-sort
    // hole, so the whole mold table is reachable; a direct root row per mold
    // keeps the closure flat (each terminal is a sink), which is what makes the
    // index tractable at gandr's ~1000-tile scale — the same-form `≐` adjacency
    // is dense with alternation fan-out and is instead consumed directly by the
    // comparison table rather than materialised as chainable rows.
    for fact in &facts {
        let walk = level_walk(fact.nonterminal)?;
        spec.insert_direct(Dir::Left, End::Root, End::Node(fact.stance), walk);
    }

    // Comparison rows: the operator-precedence relation (paper Fig. 15) between
    // `(sort, prec)` form groups, carried on one representative tile per group
    // so the closure stays bounded by the shallow precedence bands rather than
    // the layered per-tile fan-out. Every comparable same-sort group pair is a
    // direct row, so the `lt`/`gt` faces are complete without deep transitive
    // combination.
    let reps = group_reps(&facts);
    let dag = pbg.dag();
    for left in reps.values() {
        for right in reps.values() {
            if left.stance.sort != right.stance.sort {
                continue;
            }
            let p_l = left.nonterminal.prec;
            let p_r = right.nonterminal.prec;
            if bool::from(dag.lt(p_l, p_r, None)) {
                let walk = descent_walk(left.nonterminal, right.nonterminal)?;
                spec.insert_direct(
                    Dir::Left,
                    End::Node(left.stance),
                    End::Node(right.stance),
                    walk,
                );
            }
            else if bool::from(dag.gt(p_l, p_r, None)) {
                let walk = descent_walk(right.nonterminal, left.nonterminal)?;
                spec.insert_direct(
                    Dir::Right,
                    End::Node(right.stance),
                    End::Node(left.stance),
                    walk,
                );
            }
        }
    }

    Ok(spec)
}

/// Derive the operator-precedence comparison table for a built walk index.
///
/// The table has two parts:
///
/// - `⋖`/`⋗` between form-group representative tiles, read off the index
///   `lt`/`gt` faces (the group precedence matrix, paper Fig. 15). The
///   mediating sort `ρ` is the shared form sort — the index restores the
///   soundness Floyd's unindexed relation lacks by keying the relation on it.
/// - `≐` between consecutive same-form tiles, taken from `Pbg::adjacencies`
///   (the same-form relation is a grammar fact, not a precedence walk, so it is
///   read directly rather than materialised as chainable index rows).
///
/// # Contract
/// - requires: `index` was built by [`walk_index`] for `pbg`.
/// - ensures: returns sorted [`ComparisonRow`]s with at most one relation per
///   ordered tile pair; only same-sort pairs appear, so incomparable
///   (cross-sort / cross-band) pairs have no row.
/// - provides: the Fig. 15 comparison table indexed by the mediating sort, as a
///   projection of the walk index.
/// - fails: never.
/// - panics: none.
/// - intension: group representatives are read in `(sort, prec)` order and each
///   ordered same-sort representative pair is probed against `lt` then `gt`;
///   adjacencies are read in canonical order.
///
/// # Adequacy
/// - hypothesis: L3 — the built-in surface distinguishes yields/equal/takes
///   rows and the absence of any row for incomparable pairs.
/// - witness: `gandr_surface_grammar::contracts::comparison_table_coheres_with_precedence`
#[inline]
#[must_use]
pub fn comparison_table(
    pbg: &Pbg,
    index: &WalkIndex<GrammarWalkSym>,
) -> Vec<ComparisonRow>
{
    let facts = mold_facts(pbg);
    let reps = group_reps(&facts);
    let mut rows = Vec::new();

    for left in reps.values() {
        let left_end = End::Node(left.stance);
        for right in reps.values() {
            if left.stance.sort != right.stance.sort {
                continue;
            }
            let right_end = End::Node(right.stance);
            let cmp = if !index.lt(&left_end, &right_end).is_empty() {
                Some(Comparison::Yields)
            }
            else if !index.gt(&left_end, &right_end).is_empty() {
                Some(Comparison::Takes)
            }
            else {
                None
            };
            if let Some(cmp) = cmp {
                rows.push(ComparisonRow {
                    left: left.stance.mold_id,
                    right: right.stance.mold_id,
                    cmp,
                    sort: left.stance.sort,
                });
            }
        }
    }

    for &(left, right) in pbg.adjacencies() {
        let Some(left_fact) = fact_at(&facts, left)
        else {
            continue;
        };
        rows.push(ComparisonRow {
            left,
            right,
            cmp: Comparison::Equal,
            sort: left_fact.stance.sort,
        });
    }

    rows.sort_unstable();
    rows
}

/// Read the per-mold facts the generative front-end needs.
fn mold_facts(pbg: &Pbg) -> Vec<MoldFacts>
{
    let mut facts = Vec::with_capacity(pbg.mold_count().0);
    for (mold_id, def) in pbg.iter_molds() {
        facts.push(MoldFacts {
            stance: GrammarTile::new(TileLabel(def.label), mold_id, def.sort),
            nonterminal: GrammarNonterminal::new(def.sort, def.prec),
        });
    }
    facts
}

/// Return per-label reachable mold sets from the walk index projection.
///
/// # Contract
/// - requires: `index` was built by [`walk_index`].
/// - ensures: returns `label -> reachable molds`, one entry per label that
///   projects at least one mold, sorted by label with sorted mold sets.
/// - provides: the reachable multi-mold metric source.
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 — a deterministic projection over `molds`; contents are
///   witnessed by the reachability test.
/// - witness: `gandr_surface_grammar::contracts::walk_index_projects_every_mold_once`
#[inline]
#[must_use]
pub fn reachable_molds(
    pbg: &Pbg,
    index: &WalkIndex<GrammarWalkSym>,
) -> BTreeMap<TileLabel, BTreeSet<MoldId>>
{
    let mut labels: BTreeSet<TileLabel> = BTreeSet::new();
    for (_mold_id, def) in pbg.iter_molds() {
        labels.insert(TileLabel(def.label));
    }
    let mut out: BTreeMap<TileLabel, BTreeSet<MoldId>> = BTreeMap::new();
    for label in labels {
        let mut molds = BTreeSet::new();
        for &(_, mold) in index.molds(&label) {
            molds.insert(mold);
        }
        if !molds.is_empty() {
            out.insert(label, molds);
        }
    }
    out
}

/// A same-level (height zero) walk that stays within one form.
fn level_walk(
    nonterminal: GrammarNonterminal
) -> Result<Walk<GrammarNonterminal, GrammarTile>, PbgError>
{
    let swing = Swing::new(vec![nonterminal]).map_err(PbgError::from)?;
    Walk::new(vec![swing], Vec::new()).map_err(PbgError::from)
}

/// Pick one representative mold per `(sort, prec)` form group.
///
/// The representative is the canonical (smallest [`MoldId`]) tile of the group;
/// it carries the group's operator-precedence comparison so the walk closure is
/// bounded by the number of form groups, not the far larger tile count.
fn group_reps(facts: &[MoldFacts]) -> BTreeMap<(Sort, Prec), MoldFacts>
{
    let mut reps: BTreeMap<(Sort, Prec), MoldFacts> = BTreeMap::new();
    for fact in facts {
        reps.entry((fact.stance.sort, fact.nonterminal.prec))
            .or_insert(*fact);
    }
    reps
}

/// Return the mold facts for a [`MoldId`], fail-closed on out-of-range ids.
fn fact_at(
    facts: &[MoldFacts],
    id: MoldId,
) -> Option<&MoldFacts>
{
    let index = usize::try_from(u32::from(id)).ok()?;
    facts.get(index)
}

/// A one-level descent walk (height one) from an outer form into a nested form.
fn descent_walk(
    outer: GrammarNonterminal,
    inner: GrammarNonterminal,
) -> Result<Walk<GrammarNonterminal, GrammarTile>, PbgError>
{
    let swing = Swing::new(vec![outer, inner]).map_err(PbgError::from)?;
    Walk::new(vec![swing], Vec::new()).map_err(PbgError::from)
}

/// FNV-1a offset basis for stable walk symbol keys.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a prime for stable walk symbol keys.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Stable framed FNV-1a hash used as a walk symbol key.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StableHash(u64);

/// Mix one framed value into a stable hash.
fn stable_mix(
    hash: StableHash,
    value: StableHash,
) -> StableHash
{
    StableHash(core::ops::BitXor::bitxor(hash.0, value.0).wrapping_mul(FNV_PRIME))
}
