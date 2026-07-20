//! The **cell store** — oriented 2-cells over the command IL, content-addressed
//! and carrying derived metadata (`proposal-sequent-kernel.md` §7.3; VDC
//! addendum §A).
//!
//! A [`Cell`] is an oriented rewrite `lhs ~> rhs` between two [`CmdPat`]s,
//! with:
//!
//! - an [`Orientation`] — fixed by the cut polarity (K2) or chosen by the
//!   completion reduction order (§7.3.3);
//! - a [`CellProvenance`] — where the cell came from (a surface `rule`, the μ/μ̃
//!   critical pair, a return-side frame's defining cell, an η law, or
//!   completion);
//! - a derived [`CellMeta`] — per-metavariable [`CellVariance`] (Producer /
//!   Consumer / `Mixed`, derived **live** across both faces) and linearity,
//!   plus the `invertible` flag that distinguishes a completion-emitted
//!   joinability certificate from an oriented optimization cell (VDC addendum
//!   §A; the flag and variance are what [`crate::compose::compose_invertible`]
//!   / [`crate::compose::compose_directed`] branch on).
//!
//! The store ([`CellStore`]) dedups on **structural identity** ([`Eq`] over the
//! pattern pair — the content-addressing of §7.3.1) and hands out dense
//! [`CellId`]s in insertion order, so iteration and completion are
//! deterministic.

use alloc::boxed::Box;
use alloc::vec::Vec;

use gandr_core_sequent::il::Polarity;

use crate::boundary::CellCount;
use crate::boundary::CellInvertibility;
use crate::boundary::CellLinearity;
use crate::boundary::CellStoreEmptyStatus;
use crate::pattern::Cat;
use crate::pattern::CmdPat;
use crate::pattern::ConsPat;
use crate::pattern::MetaVar;
use crate::pattern::ProdPat;
use crate::pattern::Sym;
use crate::pattern::collect_cmd_metavars;

/// Which η law a cell encodes — strategy-tied per K2 (§5).
///
/// Data η is valid only at a **positive** cut (call-by-value), codata η only at
/// a **negative** cut (call-by-name); see `proposal-sequent-kernel.md` §5,
/// §7.4.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the data/codata split is the closed two-way η vocabulary of the polarized calculus; each η law is one or the other and the pair is fixed by K2"
)]
pub enum EtaKind
{
    /// Data η — a positive-intro extensionality law; valid only at a positive
    /// cut.
    Data,
    /// Codata η — a negative-intro extensionality law; valid only at a negative
    /// cut.
    Codata,
}

impl EtaKind
{
    /// The cut polarity this η law requires (`proposal-sequent-kernel.md` §5).
    ///
    /// # Contract
    /// - ensures: [`Polarity::Positive`] for [`EtaKind::Data`],
    ///   [`Polarity::Negative`] for [`EtaKind::Codata`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn required_polarity(self) -> Polarity
    {
        match self {
            | Self::Data => Polarity::Positive,
            | Self::Codata => Polarity::Negative,
        }
    }
}

/// How a cell's orientation was fixed (`proposal-sequent-kernel.md` §7.3).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "an orientation is fixed either by the cut polarity (K2) or by the completion reduction order (§7.3.3); this closed pair is the whole vocabulary"
)]
pub enum Orientation
{
    /// The orientation is fixed by the cut polarity (K2, the μ/μ̃ pair oriented
    /// by `ε`).
    PolarityDerived,
    /// The orientation is chosen by the completion reduction order
    /// ([`crate::pattern::reduction_cmp`]).
    CompletionDerived,
}

/// Where a cell came from (`proposal-sequent-kernel.md` §7.3, the `provenance`
/// field: "surface `rule`, μ/μ̃, derived-by-completion, …").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum CellProvenance
{
    /// Elaborated from a surface `rule lhs ~> rhs` (a
    /// [`gandr_theory_levitation::CellFace`]).
    SurfaceRule,
    /// The μ/μ̃ critical pair — the fundamental strategy cell (§5).
    MuMuTilde,
    /// A return-side constructor frame's defining cell `⟨v | K⁻(β)⟩ ~> ⟨K(v) |
    /// β⟩` (§7.1; [`frame_defining_cell`]).
    FrameDefining,
    /// An η law, tied to the polarity its [`EtaKind`] requires (§5).
    Eta(EtaKind),
    /// Synthesized by the completion loop (§7.3.3) — a derived / fused cell.
    DerivedByCompletion,
}

/// The **variance** role a cell metavariable occupies (VDC addendum §A;
/// `proposal-vdc-reflection.md` §4.2).
///
/// Derived **live** by [`CellMeta::derive`] from where the hole (identified by
/// *name*) occurs across **both** faces: a hole seen only in producer positions
/// is [`CellVariance::Producer`], only in consumer positions
/// [`CellVariance::Consumer`], and one spanning **both** a producer and a
/// consumer position is [`CellVariance::Mixed`] — the dinaturality-shaped
/// metavariable (`μ`/`μ̃` and cocase create it) that the two-mode certificate
/// composition guards (ADR-69 D3). The engine's own apartness
/// renaming ([`crate::overlap`]) already keys freshness by name, so a name worn
/// by both a producer and a consumer metavariable *is* one hole at two
/// polarities; [`CellVariance::from_cat`] classifies a single occurrence, and
/// `derive` promotes the pair to `Mixed`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "producer/consumer/mixed is the closed variance vocabulary of the reflected judgment layer (ADR-68/69); it ships whole so the composition lane refines rather than migrates"
)]
pub enum CellVariance
{
    /// A producer-side (introduction) position.
    Producer,
    /// A consumer-side (observation) position.
    Consumer,
    /// A hole spanning both positions — reserved for the composition lane.
    Mixed,
}

impl CellVariance
{
    /// The variance implied by a **single** metavariable category — the
    /// building block [`CellMeta::derive`] joins across occurrences.
    ///
    /// # Contract
    /// - ensures: [`CellVariance::Producer`] for [`Cat::Producer`],
    ///   [`CellVariance::Consumer`] for [`Cat::Consumer`]; never `Mixed` (one
    ///   occurrence has one category — `Mixed` is the *join* of a producer and
    ///   a consumer occurrence, computed by [`CellMeta::derive`]).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn from_cat(cat: Cat) -> Self
    {
        match cat {
            | Cat::Producer => Self::Producer,
            | Cat::Consumer => Self::Consumer,
        }
    }
}

/// Derived **metadata for one cell metavariable** (VDC addendum §A): its
/// metavariable, variance role, and linearity (whether it occurs exactly once
/// in the left-hand side).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the per-variable metadata schema is exactly {var, variance, linear}; it is derived by CellMeta::derive and read by the composition gate"
)]
pub struct CellVarMeta
{
    /// The metavariable this metadata describes — the hole's first occurrence,
    /// the representative for a `Mixed` hole whose category is not
    /// single-valued.
    pub var: MetaVar,
    /// The variance role, joined across both faces (`Mixed` when the hole spans
    /// producer and consumer positions).
    pub variance: CellVariance,
    /// Whether the hole occurs exactly once in the left-hand side.
    pub linear: CellLinearity,
}

/// Derived **cell metadata** (VDC addendum §A) — the per-metavariable variance
/// and linearity, plus the `invertible` flag.
///
/// Computed at construction ([`CellMeta::derive`]), never user-declared. The
/// `invertible` flag distinguishes a **completion-emitted joinability
/// certificate** (invertible — the coherence-lane cells) from an **oriented
/// optimization cell** (directed): the two `compose_*` operations of the
/// composition lane branch on it. Shaping it here is the
/// "bolt-on" the addendum §C asks of this lane.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cell metadata is exactly {per-variable metadata, invertible flag}; it is derived and other engine stages read it"
)]
pub struct CellMeta
{
    /// The per-metavariable metadata, in left-to-right first-occurrence order.
    pub vars: Box<[CellVarMeta]>,
    /// Whether the cell is an invertible joinability certificate (as opposed to
    /// an oriented optimization cell).
    pub invertible: CellInvertibility,
}

impl CellMeta
{
    /// Derive the metadata **live** from a cell's two faces (VDC addendum §A,
    /// ADR-69 D2) — the variance is read from where each hole occurs across the
    /// faces, never from a stage-0 constant.
    ///
    /// Holes are identified by **name** (the engine's own apartness renaming
    /// keys freshness by name, so a name worn by a producer *and* a consumer
    /// metavariable is one hole at two polarities). A hole seen only in
    /// producer positions is [`CellVariance::Producer`], only in consumer
    /// positions [`CellVariance::Consumer`], and one seen in both is
    /// [`CellVariance::Mixed`] — the dinaturality case the composition gate
    /// (`compose_directed`, [`crate::compose`]) reads.
    ///
    /// # Contract
    /// - ensures: one [`CellVarMeta`] per distinct hole name, in left-to-right
    ///   first-occurrence order over `lhs` then `rhs`; `variance` is `Producer`
    ///   / `Consumer` / `Mixed` according to the categories the name is worn
    ///   with across both faces; `linear` true iff the name occurs exactly once
    ///   in `lhs` (the redex-side occurrence count matching consults); `var` is
    ///   the first occurrence's [`MetaVar`] as the representative, `invertible`
    ///   carried through verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn derive(
        lhs: &CmdPat,
        rhs: &CmdPat,
        invertible: CellInvertibility,
    ) -> Self
    {
        let mut occurrences = Vec::new();
        collect_cmd_metavars(lhs, &mut occurrences);
        let lhs_occurrences = occurrences.len();
        collect_cmd_metavars(rhs, &mut occurrences);
        let mut vars: Vec<CellVarMeta> = Vec::new();
        for mv in &occurrences {
            if vars.iter().any(|existing| existing.var.name == mv.name) {
                continue;
            }
            let seen_producer = occurrences
                .iter()
                .any(|other| other.name == mv.name && other.cat == Cat::Producer);
            let seen_consumer = occurrences
                .iter()
                .any(|other| other.name == mv.name && other.cat == Cat::Consumer);
            let variance = if seen_producer && seen_consumer {
                CellVariance::Mixed
            }
            else if seen_consumer {
                CellVariance::Consumer
            }
            else {
                CellVariance::Producer
            };
            let lhs_count = occurrences
                .iter()
                .take(lhs_occurrences)
                .filter(|other| other.name == mv.name)
                .count();
            vars.push(CellVarMeta {
                var: mv.clone(),
                variance,
                linear: CellLinearity::from(lhs_count == 1),
            });
        }
        Self {
            vars: vars.into_boxed_slice(),
            invertible,
        }
    }
}

/// An **oriented 2-cell** over the command IL (`proposal-sequent-kernel.md`
/// §7.3, the `Cell` struct).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct Cell
{
    /// The left-hand side (the redex pattern).
    pub lhs: CmdPat,
    /// The right-hand side (the contractum pattern).
    pub rhs: CmdPat,
    /// How the orientation was fixed.
    pub orient: Orientation,
    /// Where the cell came from.
    pub provenance: CellProvenance,
    /// The derived metadata.
    pub meta: CellMeta,
}

impl Cell
{
    /// A cell from its two sides, orientation, and provenance; the metadata is
    /// derived live from both faces and the `invertible` flag set from the
    /// provenance (completion-emitted cells are invertible joinability
    /// certificates).
    ///
    /// # Contract
    /// - ensures: `meta == CellMeta::derive(&lhs, &rhs, provenance ==
    ///   CellProvenance::DerivedByCompletion)`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        lhs: CmdPat,
        rhs: CmdPat,
        orient: Orientation,
        provenance: CellProvenance,
    ) -> Self
    {
        let invertible =
            CellInvertibility::from(matches!(provenance, CellProvenance::DerivedByCompletion));
        let meta = CellMeta::derive(&lhs, &rhs, invertible);
        Self {
            lhs,
            rhs,
            orient,
            provenance,
            meta,
        }
    }

    /// The polarity a cut cell applies at (its left-hand side's cut
    /// orientation).
    ///
    /// # Contract
    /// - ensures: the cut polarity of `lhs`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn polarity(&self) -> Polarity
    {
        self.lhs.polarity()
    }

    /// The cut polarity this cell's η law requires, if it is an η cell
    /// (`proposal-sequent-kernel.md` §5): a data-η cell must fire at a positive
    /// cut, a codata-η cell at a negative cut. A non-η cell has no η
    /// requirement.
    ///
    /// # Contract
    /// - ensures: `Some(pol)` iff the provenance is [`CellProvenance::Eta`],
    ///   with `pol` the [`EtaKind::required_polarity`]; `None` otherwise.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn eta_requirement(&self) -> Option<Polarity>
    {
        match self.provenance {
            | CellProvenance::Eta(kind) => Some(kind.required_polarity()),
            | _ => None,
        }
    }
}

/// A dense **cell identifier** — an index into a [`CellStore`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[expect(
    clippy::exhaustive_structs,
    reason = "a cell id is exactly its store index; the newtype keeps it from being confused with any other index"
)]
pub struct CellId(pub usize);

/// The **cell store** — a content-addressed, insertion-ordered collection of
/// cells (`proposal-sequent-kernel.md` §7.3.1).
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct CellStore
{
    /// The cells, in insertion order; the index is the [`CellId`].
    cells: Vec<Cell>,
}

impl CellStore
{
    /// An empty store.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Insert a cell, deduplicating on structural identity, and return its id.
    ///
    /// # Contract
    /// - ensures: a cell structurally equal (same `lhs`/`rhs`/orientation/
    ///   provenance) to one already present returns that cell's existing id and
    ///   does not grow the store; otherwise the cell is appended and its fresh
    ///   id returned. Content-addressing is thus by structural identity
    ///   (§7.3.1).
    /// - panics: none.
    #[inline]
    pub fn insert(
        &mut self,
        cell: Cell,
    ) -> CellId
    {
        if let Some(index) = self.cells.iter().position(|existing| *existing == cell) {
            return CellId(index);
        }
        let id = CellId(self.cells.len());
        self.cells.push(cell);
        id
    }

    /// The cell with the given id.
    ///
    /// # Contract
    /// - ensures: `Some` iff `id` was handed out by [`CellStore::insert`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        id: CellId,
    ) -> Option<&Cell>
    {
        self.cells.get(id.0)
    }

    /// The number of cells in the store.
    #[inline]
    #[must_use]
    pub fn len(&self) -> CellCount
    {
        CellCount::from(self.cells.len())
    }

    /// Whether the store is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> CellStoreEmptyStatus
    {
        CellStoreEmptyStatus::from(self.cells.is_empty())
    }

    /// The cells with their ids, in insertion order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (CellId, &Cell)>
    {
        self.cells
            .iter()
            .enumerate()
            .map(|(index, cell)| (CellId(index), cell))
    }
}

/// The **defining cell** of a return-side constructor frame `K⁻` (§7.1).
///
/// The cell is `⟨v | K⁻(β)⟩ ~> ⟨K(v) | β⟩` — the μ̃ reduction that makes `K⁻(β)
/// := μ̃x.⟨K(x) | β⟩` "definable, not primitive" (`proposal-sequent-kernel.md`
/// §7.1).
///
/// # Contract
/// - ensures: a positive, polarity-derived cell over fresh metavariables `v`
///   (producer) and `β` (consumer), both linear, with provenance
///   [`CellProvenance::FrameDefining`].
/// - panics: none.
#[inline]
#[must_use]
pub fn frame_defining_cell(ctor: &Sym) -> Cell
{
    let lhs = CmdPat::cut(
        Polarity::Positive,
        ProdPat::Meta(MetaVar::producer("v")),
        ConsPat::Frame {
            ctor: ctor.clone(),
            ret: Box::new(ConsPat::Meta(MetaVar::consumer("beta"))),
        },
    );
    let rhs = CmdPat::cut(
        Polarity::Positive,
        ProdPat::Ctor {
            ctor: ctor.clone(),
            args: Box::from([ProdPat::Meta(MetaVar::producer("v"))]),
        },
        ConsPat::Meta(MetaVar::consumer("beta")),
    );
    Cell::new(
        lhs,
        rhs,
        Orientation::PolarityDerived,
        CellProvenance::FrameDefining,
    )
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn metadata_tracks_variance_and_linearity()
    {
        // ⟨Succ(m) | add(n; α)⟩ ~> ⟨m | add(n; Succ⁻(α))⟩ — all three vars linear.
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
        let meta = CellMeta::derive(&lhs, &rhs, CellInvertibility::from(false));
        assert_eq!(3, meta.vars.len(), "m, n, alpha");
        assert!(
            meta.vars.iter().all(|v| bool::from(v.linear)),
            "each occurs once in the LHS"
        );
        assert_eq!(
            CellVariance::Producer,
            meta.vars[0].variance,
            "m is a producer var (producer positions only)"
        );
        assert_eq!(
            CellVariance::Consumer,
            meta.vars[2].variance,
            "alpha is a consumer var (consumer positions only)"
        );
    }

    #[test]
    fn a_repeated_metavariable_is_nonlinear()
    {
        // ⟨Pair(x; x) | α⟩ — x occurs twice.
        let lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Pair", [ProdPat::meta("x"), ProdPat::meta("x")]),
            ConsPat::meta("alpha"),
        );
        let meta = CellMeta::derive(&lhs, &lhs, CellInvertibility::from(false));
        assert_eq!(2, meta.vars.len(), "x (deduped) and alpha");
        let x = meta
            .vars
            .iter()
            .find(|v| &*v.var.name == "x")
            .expect("x present");
        assert!(!bool::from(x.linear), "x occurs twice, so it is non-linear");
    }

    #[test]
    fn a_hole_at_both_polarities_is_mixed()
    {
        // ⟨r | seam(; r)⟩ — the name `r` is worn by a producer metavariable and a
        // consumer metavariable: one hole at two polarities, so the LIVE
        // derivation promotes it to `Mixed` (ADR-69 D2/D3; the dinaturality case
        // the composition gate reads). This never arises from the stage-0
        // elaborator, which keeps the categories disjoint — it is the reachable
        // shape `μ`/`μ̃` and cocase produce.
        let lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("r"),
            ConsPat::op("seam", [], ConsPat::meta("r")),
        );
        let meta = CellMeta::derive(&lhs, &lhs, CellInvertibility::from(false));
        let r = meta
            .vars
            .iter()
            .find(|v| &*v.var.name == "r")
            .expect("r present");
        assert_eq!(
            CellVariance::Mixed,
            r.variance,
            "r spans a producer and a consumer position"
        );
    }

    #[test]
    fn the_store_dedups_on_structural_identity()
    {
        let cell = frame_defining_cell(&Sym::new("Succ"));
        let mut store = CellStore::new();
        let a = store.insert(cell.clone());
        let b = store.insert(cell);
        assert_eq!(a, b, "the same cell inserts once");
        assert_eq!(
            CellCount::from(1_usize),
            store.len(),
            "the store did not grow"
        );
    }

    #[test]
    fn completion_cells_are_invertible_certificates()
    {
        let lhs = CmdPat::cut(Polarity::Positive, ProdPat::meta("x"), ConsPat::Top);
        let cell = Cell::new(
            lhs.clone(),
            lhs,
            Orientation::CompletionDerived,
            CellProvenance::DerivedByCompletion,
        );
        assert!(
            bool::from(cell.meta.invertible),
            "a completion-emitted cell is an invertible certificate"
        );
    }
}
