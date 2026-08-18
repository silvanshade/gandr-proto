//! The **sequent-kernel command-pattern alphabet** — the first
//! [`CellAlphabet`] inhabitant.
//!
//! This module wires the command-pattern IL ([`crate::pattern`]) and its
//! substitution machinery ([`crate::subst`]) into the generic engines.
//!
//! Everything sequent-specific the generic layer deliberately does *not* know
//! lives here: the orientation and provenance tags ([`Orientation`],
//! [`CellProvenance`], [`EtaKind`]), the live per-metavariable metadata
//! ([`CellMeta`], [`CellVarMeta`], [`CellVariance`] — the producer/consumer /
//! `Mixed` variance of the VDC addendum §A), the η-polarity firing discipline
//! (§5, K2), the name-priming apartness renaming, the `$k$` skolem constants
//! of tracelet replay, and the return-side frame's defining cell
//! ([`frame_defining_cell`]).
//!
//! The narrowing tripwire survives the lift at this level: [`CmdPat`] matches
//! stay total by policy, so a grammar extension is still a compile-visible
//! change at every match site in this module and [`crate::pattern`].
//!
//! [`CmdPat`]: crate::pattern::CmdPat

use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_sequent::il::Polarity;

use crate::alphabet::CellAlphabet;
use crate::alphabet::ConvexityDischarge;
use crate::alphabet::PositionOrder;
use crate::alphabet::SeamRole;
use crate::alphabet::path_order;
use crate::boundary::CellInvertibility;
use crate::boundary::CellLinearity;
use crate::boundary::FiringPermission;
use crate::boundary::PositionStep;
use crate::boundary::PrimeNameRef;
use crate::boundary::SubstitutionDecision;
use crate::cell::Cell;
use crate::cell::CellStore;
use crate::order::reduction_cmp;
use crate::pattern::Cat;
use crate::pattern::CmdPat;
use crate::pattern::ConsPat;
use crate::pattern::MetaVar;
use crate::pattern::Node;
use crate::pattern::Pos;
use crate::pattern::ProdPat;
use crate::pattern::Sym;
use crate::pattern::collect_cmd_metavars;
use crate::pattern::splice_at;
use crate::pattern::subterm_at;
use crate::pattern::transform_node;
use crate::rewrite::command_positions;
use crate::subst::Subst;
use crate::subst::match_cmd as match_sequent_cmd;
use crate::subst::unify_cmd as unify_sequent_cmd;

/// The **sequent-kernel command-pattern alphabet** — a stateless marker (the
/// alphabet carries no state; every operation is a static method over the
/// [`crate::pattern`] / [`crate::subst`] machinery).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SequentAlphabet;

/// A metavariable **hole name** — the identity a hole is keyed by across a
/// cell's two faces (the composition gate's seam vocabulary; a name worn at
/// two polarities is one hole).
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HoleName(pub Box<str>);

impl AsRef<str> for HoleName
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

/// Which η law a cell encodes — strategy-tied per K2 (§5).
///
/// Data η is valid only at a **positive** cut (call-by-value), codata η only at
/// a **negative** cut (call-by-name); see `proposal-sequent-kernel.md` §5,
/// §7.4.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
pub enum Orientation
{
    /// The orientation is fixed by the cut polarity (K2, the μ/μ̃ pair oriented
    /// by `ε`).
    PolarityDerived,
    /// The orientation is chosen by the completion reduction order
    /// ([`crate::order::reduction_cmp`]).
    CompletionDerived,
}

/// Where a cell came from (`proposal-sequent-kernel.md` §7.3, the `provenance`
/// field: "surface `rule`, μ/μ̃, derived-by-completion, …").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CellProvenance
{
    /// Elaborated from a surface `rule lhs ~> rhs` (a
    /// [`gandr_theory_levitation::RuleFace`]).
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
/// renaming ([`rename_apart`]) already keys freshness by name, so a name worn
/// by both a producer and a consumer metavariable *is* one hole at two
/// polarities; [`CellVariance::from_cat`] classifies a single occurrence, and
/// `derive` promotes the pair to `Mixed`.
///
/// [`rename_apart`]: CellAlphabet::rename_apart
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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

/// How a hole is used on a cell's **contractum side** — the step-growth half
/// of the strict-linearity discipline, complementing [`CellVarMeta::linear`]'s
/// redex-side count.
///
/// The classes are the deep-inference reading of what a rewrite does to
/// information: `Once` preserves it, `Erased` weakens it away, `Repeated`
/// contracts (duplicates) it. Only the last two change a term's instance
/// content, and the duplication class is the one the sharing overlay's
/// policy layer is meant to own — which is why this derivation *reports*
/// rather than refuses: the admission boundary
/// ([`crate::linearity::admit_linear_cell`]) governs the redex side alone,
/// and whether contractum growth should also be governed is a separate
/// question this report makes visible.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CellContractumUse
{
    /// The `(name, category)` pair does not occur on the right-hand side: the
    /// step drops the hole — weakening.
    Erased,
    /// The pair occurs exactly once on the right-hand side: the step
    /// preserves the hole.
    Once,
    /// The pair occurs more than once on the right-hand side: the step
    /// duplicates the hole — contraction.
    Repeated,
}

/// The whole-step classification [`CellMeta::step_growth`] reports — the join
/// of the per-hole [`CellContractumUse`] verdicts.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StepGrowth
{
    /// Every hole is used exactly once on each side: the step is
    /// information-preserving — the strictly-linear fragment on which
    /// per-step amplification bounds are trivially dischargable.
    StrictlyLinear,
    /// No hole is duplicated, and at least one is dropped: the step weakens.
    Erasing,
    /// At least one hole is duplicated: the step grows — in the
    /// strictly-linear reading this is exactly the growth that belongs in an
    /// isolated expansion layer (the sharing overlay's policy rungs), not in
    /// the rule vocabulary.
    Duplicating,
}

/// Derived **metadata for one cell metavariable** (VDC addendum §A): its
/// metavariable, variance role, and linearity (whether it occurs exactly once
/// in the left-hand side).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CellVarMeta
{
    /// The metavariable this metadata describes — the hole's first occurrence,
    /// the representative for a `Mixed` hole whose category is not
    /// single-valued.
    pub var: MetaVar,
    /// The variance role, joined across both faces (`Mixed` when the hole spans
    /// producer and consumer positions).
    pub variance: CellVariance,
    /// Whether `var`'s own `(name, category)` pair occurs exactly once in the
    /// left-hand side. Counting per pair rather than per name is what keeps a
    /// hole worn at two polarities linear: that is one occurrence at each
    /// polarity — the dinaturality seam — and not a copy.
    pub linear: CellLinearity,
    /// How `var`'s own `(name, category)` pair is used on the right-hand
    /// side — dropped, preserved, or duplicated. The contractum-side
    /// complement of [`CellVarMeta::linear`]: linearity asks what the redex
    /// *copies*, this asks what the step *does with* the hole, and the join
    /// over all holes is [`CellMeta::step_growth`].
    pub contractum: CellContractumUse,
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
    /// **Linearity is counted per `(name, category)` pair, variance per name.**
    /// The two questions are different: variance asks which polarities a hole
    /// is worn at, so it joins across them; linearity asks whether the hole
    /// is *copied*, and a hole worn once as a producer and once as a
    /// consumer is the seam, not a copy. Counting bare name occurrences
    /// conflates them and reports the reachable `μ`/`μ̃` seam shape as
    /// non-linear (`spec:implementation/circuit-terms.md` §"The
    /// design questions", `circuit-terms-question-17`; owner decision,
    /// 2026-08-02). This derivation records; the refusal is the separate
    /// admission boundary [`crate::linearity::admit_linear_cell`].
    ///
    /// # Contract
    /// - ensures: one [`CellVarMeta`] per distinct hole name, in left-to-right
    ///   first-occurrence order over `lhs` then `rhs`; `variance` is `Producer`
    ///   / `Consumer` / `Mixed` according to the categories the name is worn
    ///   with across both faces; `linear` true iff the `(name, category)` pair
    ///   of `var` occurs exactly once in `lhs` (the redex-side occurrence count
    ///   matching consults, taken per pair rather than per name); `contractum`
    ///   is [`CellContractumUse::Erased`] / [`CellContractumUse::Once`] /
    ///   [`CellContractumUse::Repeated`] as the same pair occurs zero, one, or
    ///   more times in `rhs`; `var` is the first occurrence's [`MetaVar`] as
    ///   the representative, `invertible` carried through verbatim.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 only — the variance join and the per-pair linearity
    ///   count are separated by three shapes: an all-linear cell with a
    ///   producer-only and a consumer-only hole, a same-polarity repeat, and a
    ///   two-polarity seam that must come out `Mixed` **and** linear; the
    ///   contractum classification is separated by one cell carrying all three
    ///   uses.
    /// - witness: `sequent::tests::metadata_tracks_variance_and_linearity`
    /// - witness: `sequent::tests::a_repeated_metavariable_is_nonlinear`
    /// - witness: `sequent::tests::a_hole_at_both_polarities_is_a_linear_seam`
    /// - witness: `sequent::tests::the_contractum_use_reports_erased_once_and_repeated`
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
            // Per `(name, category)`, never per name: one hole worn at two
            // polarities is the dinaturality seam, not a copy.
            let lhs_count = occurrences
                .iter()
                .take(lhs_occurrences)
                .filter(|other| other.name == mv.name && other.cat == mv.cat)
                .count();
            let rhs_count = occurrences
                .iter()
                .skip(lhs_occurrences)
                .filter(|other| other.name == mv.name && other.cat == mv.cat)
                .count();
            let contractum = match rhs_count {
                | 0 => CellContractumUse::Erased,
                | 1 => CellContractumUse::Once,
                | _ => CellContractumUse::Repeated,
            };
            vars.push(CellVarMeta {
                var: mv.clone(),
                variance,
                linear: CellLinearity::from(lhs_count == 1),
                contractum,
            });
        }
        Self {
            vars: vars.into_boxed_slice(),
            invertible,
        }
    }

    /// The **whole-step growth classification** — the join of the per-hole
    /// contractum uses: [`StepGrowth::Duplicating`] when any hole is repeated
    /// on the right-hand side, else [`StepGrowth::Erasing`] when any is
    /// dropped, else [`StepGrowth::StrictlyLinear`].
    ///
    /// Duplication dominates erasure in the join because it dominates in
    /// cost: a dropped hole shrinks the term, a repeated one can amplify it,
    /// and the strictly-linear discipline's budget question is exactly
    /// "where can a step grow".
    ///
    /// # Contract
    /// - ensures: the join over [`CellVarMeta::contractum`] as above; a cell
    ///   with no metavariables at all is [`StepGrowth::StrictlyLinear`]
    ///   (vacuously information-preserving).
    /// - provides: the one-word verdict an admission report or a budget
    ///   discharge reads instead of re-walking the per-hole metadata.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the three outcomes own separate decision surfaces,
    ///   separated by the Peano successor cell (strictly linear), a cell
    ///   dropping one of two holes (erasing), and a cell duplicating one hole
    ///   (duplicating, with the erasure-domination case covered by a cell both
    ///   dropping and duplicating).
    /// - witness: `sequent::tests::the_step_growth_join_names_duplication_erasure_and_strict_linearity`
    #[inline]
    #[must_use]
    pub fn step_growth(&self) -> StepGrowth
    {
        let mut growth = StepGrowth::StrictlyLinear;
        for var in &self.vars {
            match var.contractum {
                | CellContractumUse::Repeated => return StepGrowth::Duplicating,
                | CellContractumUse::Erased => growth = StepGrowth::Erasing,
                | CellContractumUse::Once => {},
            }
        }
        growth
    }
}

impl Cell<SequentAlphabet>
{
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
pub fn frame_defining_cell(ctor: &Sym) -> Cell<SequentAlphabet>
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

/// **Skolemize** a command pattern — replace each metavariable with a fresh,
/// reserved-name constant so the schematic peak becomes a ground configuration
/// the replay can rewrite.
///
/// # Contract
/// - ensures: every producer metavariable `x` becomes a nullary constructor
///   `$k$x` and every consumer metavariable `α` an opaque nullary operation
///   frame `$k$α(; ★)`; the reserved `$k$` prefix never collides with a real
///   constructor / operation, so the constants are irreducible. The mapping is
///   name-deterministic, so shared metavariables skolemize identically across
///   peak and join.
/// - panics: none.
#[inline]
#[must_use]
pub fn skolemize_cmd(cmd: &CmdPat) -> CmdPat
{
    let Some(Node::Cmd(cmd)) =
        transform_node(Node::Cmd(cmd.clone()), |node| Some(skolemize_node(node)))
    else {
        return cmd.clone();
    };
    cmd
}

/// Skolemize one rebuilt node.
#[inline]
fn skolemize_node(node: Node) -> Node
{
    match node {
        | Node::Prod(ProdPat::Meta(mv)) => Node::Prod(ProdPat::Ctor {
            ctor: skolem_sym(&mv),
            args: Box::from([]),
        }),
        | Node::Cons(ConsPat::Meta(mv)) => Node::Cons(ConsPat::Op {
            op: skolem_sym(&mv),
            args: Box::from([]),
            ret: Box::new(ConsPat::Top),
        }),
        | other => other,
    }
}

/// The reserved skolem constant name for a metavariable.
///
/// # Contract
/// - ensures: the name `$k$<metavariable-name>`, whose `$k$` prefix a real
///   datatype symbol never carries.
/// - panics: none.
#[inline]
fn skolem_sym(mv: &MetaVar) -> Sym
{
    let mut name = String::with_capacity(mv.name.len().saturating_add(3));
    name.push_str("$k$");
    name.push_str(&mv.name);
    Sym::new(name)
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
/// - ensures: the returned pair is structurally the input with every
///   metavariable name replaced by a name absent from `avoid` and from the
///   other renamed names, preserving each metavariable's category and the
///   pattern shape; a cell already disjoint from `avoid` is unchanged.
/// - panics: none.
#[inline]
fn rename_sides_apart(
    lhs: &CmdPat,
    rhs: &CmdPat,
    avoid: &MetaVarNameSet,
) -> (CmdPat, CmdPat)
{
    let mut occurrences = Vec::new();
    collect_cmd_metavars(lhs, &mut occurrences);
    collect_cmd_metavars(rhs, &mut occurrences);
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
    (rename_cmd(lhs, &mapping), rename_cmd(rhs, &mapping))
}

/// Append a prime `'` to a name (the fresh-name step of
/// [`rename_sides_apart`]).
///
/// # Contract
/// - ensures: a strictly longer name, so priming terminates against any finite
///   `avoid` set.
/// - panics: none.
#[inline]
fn prime(name: PrimeNameRef<'_>) -> FreshMetaVarName
{
    let name = name.as_ref();
    let mut out = String::with_capacity(name.len().saturating_add(1));
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

impl CellAlphabet for SequentAlphabet
{
    type Cmd = CmdPat;
    type Hole = HoleName;
    type Meta = CellMeta;
    type Orientation = Orientation;
    type Pos = Pos;
    type Provenance = CellProvenance;
    type Subst = Subst;
    type Var = MetaVar;

    #[inline]
    fn match_cmd(
        pattern: &Self::Cmd,
        target: &Self::Cmd,
        subst: &mut Self::Subst,
    ) -> SubstitutionDecision
    {
        match_sequent_cmd(pattern, target, subst)
    }

    #[inline]
    fn unify_cmd(
        lhs: &Self::Cmd,
        rhs: &Self::Cmd,
        subst: &mut Self::Subst,
    ) -> SubstitutionDecision
    {
        unify_sequent_cmd(lhs, rhs, subst)
    }

    #[inline]
    fn apply_subst(
        subst: &Self::Subst,
        cmd: &Self::Cmd,
    ) -> Self::Cmd
    {
        subst.apply_cmd(cmd)
    }

    #[inline]
    fn metavariables(cmd: &Self::Cmd) -> Vec<Self::Var>
    {
        let mut out = Vec::new();
        collect_cmd_metavars(cmd, &mut out);
        out
    }

    #[inline]
    fn command_positions(cmd: &Self::Cmd) -> Vec<Self::Pos>
    {
        command_positions(cmd)
    }

    #[inline]
    fn root_position() -> Self::Pos
    {
        Pos::root()
    }

    #[inline]
    fn position_at_path(path: &[PositionStep]) -> Self::Pos
    {
        Pos::from_indices(
            path.iter()
                .copied()
                .map(usize::from)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }

    #[inline]
    fn position_order(
        left: &Self::Pos,
        right: &Self::Pos,
    ) -> PositionOrder
    {
        path_order(
            left.as_ref().iter().copied().map(PositionStep::from),
            right.as_ref().iter().copied().map(PositionStep::from),
        )
    }

    #[inline]
    fn convexity_discharge(_store: &CellStore<Self>) -> ConvexityDischarge
    {
        // The store is not consulted, and that is the finding rather than a
        // shortcut: every left-hand side this alphabet can express is a
        // `CmdPat` — one cut whose consumer half is a linear spine with a
        // single terminal — so strong connectedness is forced by the grammar,
        // not by which cells happen to be present. Targets are command-pattern
        // trees, hence acyclic. The re-check is therefore constant-true and is
        // skipped rather than run (`circuit-terms-spike-07`). An alphabet that
        // adds multi-output or disconnected left-hand sides breaks the forcing
        // argument and must answer `ReCheckRequired`.
        ConvexityDischarge::LeftConnectedOverAcyclicTarget
    }

    #[inline]
    fn subterm_cmd_at(
        cmd: &Self::Cmd,
        pos: &Self::Pos,
    ) -> Option<Self::Cmd>
    {
        match subterm_at(&Node::Cmd(cmd.clone()), pos) {
            | Some(Node::Cmd(sub)) => Some(sub),
            | Some(Node::Prod(_) | Node::Cons(_)) | None => None,
        }
    }

    #[inline]
    fn splice_cmd_at(
        cmd: &Self::Cmd,
        pos: &Self::Pos,
        replacement: Self::Cmd,
    ) -> Option<Self::Cmd>
    {
        match splice_at(&Node::Cmd(cmd.clone()), pos, Node::Cmd(replacement)) {
            | Some(Node::Cmd(rebuilt)) => Some(rebuilt),
            | Some(Node::Prod(_) | Node::Cons(_)) | None => None,
        }
    }

    #[inline]
    fn reduction_cmp(
        lhs: &Self::Cmd,
        rhs: &Self::Cmd,
    ) -> core::cmp::Ordering
    {
        reduction_cmp(lhs, rhs)
    }

    #[inline]
    fn rename_apart(
        anchor: (&Self::Cmd, &Self::Cmd),
        renamed: (&Self::Cmd, &Self::Cmd),
    ) -> (Self::Cmd, Self::Cmd)
    {
        let avoid = cmd_var_names(anchor.0, anchor.1);
        rename_sides_apart(renamed.0, renamed.1, &avoid)
    }

    #[inline]
    fn skolemize(cmd: &Self::Cmd) -> Self::Cmd
    {
        skolemize_cmd(cmd)
    }

    #[inline]
    fn hole_of(var: &Self::Var) -> Self::Hole
    {
        HoleName(var.name.clone())
    }

    #[inline]
    fn completion_certificate(provenance: &Self::Provenance) -> CellInvertibility
    {
        CellInvertibility::from(matches!(provenance, CellProvenance::DerivedByCompletion))
    }

    #[inline]
    fn derive_meta(
        lhs: &Self::Cmd,
        rhs: &Self::Cmd,
        invertible: CellInvertibility,
    ) -> Self::Meta
    {
        CellMeta::derive(lhs, rhs, invertible)
    }

    #[inline]
    fn hole_flow(
        meta: &Self::Meta,
        hole: &Self::Hole,
    ) -> Vec<(Self::Var, SeamRole)>
    {
        let mut out = Vec::new();
        for var_meta in &meta.vars {
            if var_meta.var.name == hole.0 {
                let role = match var_meta.variance {
                    | CellVariance::Producer => SeamRole::Forward,
                    | CellVariance::Consumer => SeamRole::Backward,
                    | CellVariance::Mixed => SeamRole::Both,
                };
                out.push((var_meta.var.clone(), role));
            }
        }
        out
    }

    #[inline]
    fn may_fire(
        provenance: &Self::Provenance,
        target: &Self::Cmd,
    ) -> FiringPermission
    {
        match *provenance {
            | CellProvenance::Eta(kind) => {
                FiringPermission::from(target.polarity() == kind.required_polarity())
            },
            | _ => FiringPermission::from(true),
        }
    }

    #[inline]
    fn derived_orientation() -> Self::Orientation
    {
        Orientation::CompletionDerived
    }

    #[inline]
    fn derived_provenance() -> Self::Provenance
    {
        CellProvenance::DerivedByCompletion
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::cell::CellStore;

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
        // ⟨Pair(x; x) | α⟩ — the producer hole `x` occurs twice at the SAME
        // polarity: a genuine copy, which the derivation records as non-linear
        // and the admission boundary refuses
        // ([`crate::linearity::admit_linear_cell`]).
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
        assert!(
            !bool::from(x.linear),
            "the producer x occurs twice, so it is non-linear"
        );
        let alpha = meta
            .vars
            .iter()
            .find(|v| &*v.var.name == "alpha")
            .expect("alpha present");
        assert!(
            bool::from(alpha.linear),
            "alpha occurs once, so the copy does not spread to its neighbours"
        );
    }

    #[test]
    fn a_hole_at_both_polarities_is_a_linear_seam()
    {
        // ⟨r | seam(; r)⟩ — the name `r` is worn by a producer metavariable and a
        // consumer metavariable: one hole at two polarities, so the LIVE
        // derivation promotes it to `Mixed` (ADR-69 D2/D3; the dinaturality case
        // the composition gate reads). This never arises from the stage-0
        // elaborator, which keeps the categories disjoint — it is the reachable
        // shape `μ`/`μ̃` and cocase produce.
        //
        // It is a SEAM, not a copy: `r` occurs once at each polarity, so nothing
        // is duplicated on a wire and the pattern stays linear. Counting bare
        // name occurrences instead of `(name, category)` pairs reported it as
        // non-linear, which would have made the admission boundary refuse the
        // very shape the composition gate is built to read.
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
        assert!(
            bool::from(r.linear),
            "one occurrence at each polarity is a seam, not a copy"
        );
    }

    #[test]
    fn the_contractum_use_reports_erased_once_and_repeated()
    {
        // One cell carrying all three uses: `x` preserved once, `y`
        // duplicated, `z` dropped, and `alpha` kept.
        let lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("T", [
                ProdPat::meta("x"),
                ProdPat::meta("y"),
                ProdPat::meta("z"),
            ]),
            ConsPat::meta("alpha"),
        );
        let rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("U", [
                ProdPat::meta("x"),
                ProdPat::meta("y"),
                ProdPat::meta("y"),
            ]),
            ConsPat::meta("alpha"),
        );
        let meta = CellMeta::derive(&lhs, &rhs, CellInvertibility::from(false));
        let use_of = |name: &str| {
            meta.vars
                .iter()
                .find(|v| &*v.var.name == name)
                .map(|v| v.contractum)
                .expect("the hole is present")
        };
        assert_eq!(CellContractumUse::Once, use_of("x"), "preserved once");
        assert_eq!(CellContractumUse::Repeated, use_of("y"), "duplicated");
        assert_eq!(CellContractumUse::Erased, use_of("z"), "dropped");
        assert_eq!(
            CellContractumUse::Once,
            use_of("alpha"),
            "the continuation is kept, per pair as ever"
        );
    }

    #[test]
    fn the_step_growth_join_names_duplication_erasure_and_strict_linearity()
    {
        // The Peano successor cell: every hole used exactly once on each
        // side — the strictly-linear fragment.
        let add_s_lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
        );
        let add_s_rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("m"),
            ConsPat::op(
                "add",
                [ProdPat::meta("n")],
                ConsPat::op("Succ⁻", [], ConsPat::meta("alpha")),
            ),
        );
        let linear = CellMeta::derive(&add_s_lhs, &add_s_rhs, CellInvertibility::from(false));
        assert_eq!(
            StepGrowth::StrictlyLinear,
            linear.step_growth(),
            "every hole preserved exactly once"
        );
        // Dropping one of two holes, duplicating none: erasing.
        let drop_lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Pair", [ProdPat::meta("x"), ProdPat::meta("y")]),
            ConsPat::meta("alpha"),
        );
        let drop_rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("x"),
            ConsPat::meta("alpha"),
        );
        let erasing = CellMeta::derive(&drop_lhs, &drop_rhs, CellInvertibility::from(false));
        assert_eq!(
            StepGrowth::Erasing,
            erasing.step_growth(),
            "a dropped hole with no duplication weakens"
        );
        // Duplicating one hole while dropping another: duplication dominates,
        // because it dominates in cost.
        let dup_rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Pair", [ProdPat::meta("x"), ProdPat::meta("x")]),
            ConsPat::meta("alpha"),
        );
        let duplicating = CellMeta::derive(&drop_lhs, &dup_rhs, CellInvertibility::from(false));
        assert_eq!(
            StepGrowth::Duplicating,
            duplicating.step_growth(),
            "a repeated hole makes the step growth, whatever else it drops"
        );
    }

    #[test]
    fn completion_cells_are_invertible_certificates()
    {
        let lhs = CmdPat::cut(Polarity::Positive, ProdPat::meta("x"), ConsPat::Top);
        let cell: Cell = Cell::new(
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

    #[test]
    fn skolemization_is_name_stable()
    {
        let peak = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("v"),
            ConsPat::frame("Succ", ConsPat::meta("v_cons")),
        );
        assert_eq!(
            skolemize_cmd(&peak),
            skolemize_cmd(&peak),
            "skolemization is deterministic"
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
        assert_eq!(1, store.iter().count(), "the store did not grow");
    }
}
