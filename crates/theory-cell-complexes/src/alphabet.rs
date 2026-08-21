//! The **cell-alphabet abstraction** (metatheory roadmap spike **S1**; the
//! alphabet-polymorphic-enumerator item of the implementation roadmap).
//!
//! The trait the overlap enumerator, the completion loop, and the normalizer
//! quantify over.
//!
//! # Why this lift is a design change, not a refactor
//!
//! This crate was **intentionally monomorphic** over the sequent-kernel
//! command-pattern alphabet — no trait, no type parameter — as a review
//! tripwire: the cell-visible fragment ([`crate::pattern::CmdPat`]) is
//! deliberately narrower than the full IL, matches over it are total by
//! policy, and any grammar extension was meant to be a compile-visible change
//! at every match site. Lifting the engines over an alphabet loosens that
//! tripwire on purpose: the shape layer and the directed rule layers need the
//! completion machinery (the four-layer exchange identity and the shape-layer
//! interchange equation are exactly the hand completion work the enumerator
//! exists to mechanize, per the executed
//! meta-spike-01), and a second copy of the engine per
//! alphabet is not a maintainable answer. The narrowing discipline survives
//! the lift one level down: each alphabet decides its own fragment, and the
//! sequent alphabet ([`crate::sequent`]) keeps its total matches verbatim.
//!
//! # The interface
//!
//! One deep trait, not a spray of knobs. An **alphabet** fixes:
//!
//! - the **pattern grammar** — the term type [`CellAlphabet::Cmd`] cells
//!   rewrite, its metavariables [`CellAlphabet::Var`] (with the hole identity
//!   [`CellAlphabet::Hole`] the composition gate keys on), its seam positions
//!   [`CellAlphabet::Pos`], navigation and splicing
//!   ([`CellAlphabet::subterm_cmd_at`] / [`CellAlphabet::splice_cmd_at`]), the
//!   well-founded [`CellAlphabet::reduction_cmp`] completion orients by, the
//!   deterministic apartness renaming [`CellAlphabet::rename_apart`], and the
//!   replay [`CellAlphabet::skolemize`];
//! - the **ordered-map substitution** — one-sided matching
//!   ([`CellAlphabet::match_cmd`], cell application) and two-sided unification
//!   ([`CellAlphabet::unify_cmd`], overlap superposition), both over the
//!   deterministic [`CellAlphabet::Subst`] carrier;
//! - the **cell vocabulary** — orientation and provenance tags, the derived
//!   per-cell metadata [`CellAlphabet::Meta`], the firing discipline
//!   ([`CellAlphabet::may_fire`], the sequent η-polarity gate's slot), and the
//!   provenance-to-certificate reading
//!   ([`CellAlphabet::completion_certificate`]).
//!
//! The two intended **future consumers** are the **shape layer** (whose
//! interchange equation is a completion problem over shape cells) and the
//! **directed rule layers** (whose rule alphabets need the same budgeted
//! Knuth–Bendix/Squier machinery); the sequent-kernel command-pattern alphabet
//! ([`crate::sequent::SequentAlphabet`]) is the first inhabitant and the
//! reference implementation.
//!
//! # Trust placement (standing warning, verbatim)
//!
//! off-TCB applies to the **enumerator only**; the `cells_equal` normal-form
//! fast path is TCB-adjacent and needs a guard plus a soundness witness,
//! never documentation.
//!
//! Concretely for this abstraction:
//! `gandr_theory_coherent_resolutions::overlap::enumerate_overlaps`
//! asserts nothing — it emits seam data and stays off-TCB. The equality the
//! engines decide with — the completion join check `normal == normal` and the
//! store's structural dedup — rides [`CellAlphabet::Cmd`]'s [`Eq`], so that
//! equality MUST be **structural content identity**: no arena addresses,
//! generation stamps, or session state may enter hashed or compared term
//! identity (this is also why an alphabet is a stateless marker type, never a
//! handle to an interner). Any future normal-form fast path for cell equality
//! lands behind a guard plus a soundness witness, per the warning — never as
//! an unchecked [`Eq`] shortcut.
//!
//! # Certificate identity versus term identity
//!
//! The abstraction never conflates the two: terms are identified structurally
//! ([`Eq`]); certificates (tracelets) are identified by **replay-equivalence**
//! (`gandr_theory_coherent_resolutions::tracelet::replay_equivalent`) —
//! proof-irrelevant up to replay. No `Path`/`Flow`-style conflation is admitted
//! anywhere in the interface: an alphabet contributes the term plane; the
//! certificate plane lives in `gandr_theory_coherent_resolutions::tracelet` and
//! `gandr_theory_decomposition_spaces::compose` over it. The two critical-pair
//! kinds (confluence Knuth–Bendix and composition sequential fusion) stay a
//! complete family in
//! `gandr_theory_coherent_resolutions::overlap::OverlapKind`, never collapsed,
//! and budgeted decline-and-report stays the completion contract in
//! `gandr_theory_coherent_resolutions::completion`, including the three honest
//! obstruction classes.

use alloc::vec::Vec;

use crate::boundary::CellInvertibility;
use crate::boundary::FiringPermission;
use crate::boundary::PositionStep;
use crate::boundary::SubstitutionDecision;
use crate::cell::CellStore;

/// The **flow role** a metavariable hole contributes across a composed seam.
///
/// This is the alphabet-neutral dinaturality vocabulary the directed
/// composition gate reads.
///
/// The sequent alphabet maps its [`crate::sequent::CellVariance`] onto these
/// roles (producer → [`SeamRole::Forward`], consumer → [`SeamRole::Backward`],
/// mixed → [`SeamRole::Both`]); an alphabet without a polarity split reports
/// the roles its own metadata supports.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SeamRole
{
    /// Forward flow across the seam, `a`-side → `b`-side.
    Forward,
    /// Backward flow across the seam, `b`-side → `a`-side.
    Backward,
    /// Both directions — the dinaturality-shaped hole that closes a loop when
    /// shared across the seam.
    Both,
}

/// How two positions in one term relate.
///
/// This is the vocabulary the shift guard's **first conjunct** reads
/// (`gandr_theory_deep_inference::shift`; the decided guard's "the positions
/// are incomparable — neither position a prefix of the other").
///
/// An alphabet answers it through [`CellAlphabet::position_order`]. Only
/// [`PositionOrder::Incomparable`] licenses a shift: it is the relation that
/// makes the two applications address **disjoint subtrees**, which is what
/// makes a term-shaped store free of the cross edges the convexity hazard is
/// built out of.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PositionOrder
{
    /// The same position — one application, not two.
    Same,
    /// The left position is a proper prefix of the right: the left addresses a
    /// subtree strictly containing the right's.
    Encloses,
    /// The right position is a proper prefix of the left.
    EnclosedBy,
    /// Neither is a prefix of the other — the two address **disjoint**
    /// subtrees.
    Incomparable,
}

/// The **warrant** an alphabet's store carries for the shift guard's *third*
/// conjunct — "each match image is still convex in the other's reduct"
/// (`gandr_theory_deep_inference::shift`).
///
/// The conjunct is stated on a pair of applications in one term and would cost
/// a directed reachability sweep per query. On a store certified
/// **left-connected** over an **acyclic** target it is provably constant-true,
/// so it is *skipped rather than run*, and the witness carries this datum as
/// the name of the warrant it was skipped under. That certificate is the first
/// inhabitant of the tractability witness the engine-metatheory contract owes:
/// a record per tractability reason, never a documented assumption. The
/// left-connectedness definition and its automatic-convexity theorem are cited
/// at their statement numbers in
/// `spec:implementation/circuit-terms.md`,
/// `circuit-terms-spike-07`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConvexityDischarge
{
    /// Discharged outright and never run: every left-hand side the alphabet can
    /// express is strongly connected and every target is acyclic, so **no**
    /// match can be made non-convex and the re-check is constant-true.
    LeftConnectedOverAcyclicTarget,
    /// Not discharged: the per-pair convexity re-check would have to be run,
    /// and it is not built — so the shift is refused rather than assumed.
    ReCheckRequired,
}

/// The **path order** of two child-index paths.
///
/// This is the shared implementation of [`CellAlphabet::position_order`] for
/// every alphabet whose positions are paths of child indices — which every
/// alphabet's are, by the [`CellAlphabet::Pos`] contract.
///
/// # Contract
/// - requires: both iterators yield the position's child indices from the root
///   outward.
/// - ensures: [`PositionOrder::Same`] on equal paths;
///   [`PositionOrder::Encloses`] when `left` is a proper prefix of `right`;
///   [`PositionOrder::EnclosedBy`] when `right` is a proper prefix of `left`;
///   [`PositionOrder::Incomparable`] at the first differing step, which is
///   exactly when the two paths address disjoint subtrees.
/// - provides: the first conjunct of the shift guard, in one pass over the
///   shorter path.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the four outcomes are separated pointwise by an
///   equal pair, a proper-prefix pair in each direction, and a pair diverging
///   at a shared depth.
/// - witness: `shift::tests::the_path_order_separates_its_four_outcomes`
#[inline]
#[must_use]
pub fn path_order<Left, Right>(
    left: Left,
    right: Right,
) -> PositionOrder
where
    Left: IntoIterator<Item = PositionStep>,
    Right: IntoIterator<Item = PositionStep>,
{
    let mut right = right.into_iter();
    for step in left {
        match right.next() {
            | None => return PositionOrder::EnclosedBy,
            | Some(other) if other == step => {},
            | Some(_) => return PositionOrder::Incomparable,
        }
    }
    match right.next() {
        | Some(_) => PositionOrder::Encloses,
        | None => PositionOrder::Same,
    }
}

/// The **cell alphabet** the engines quantify over — the pattern grammar, the
/// ordered-map substitution, and the seam/overlap vocabulary of one term
/// language.
///
/// An alphabet is a **stateless marker type** (a ZST: the supertraits require
/// [`Copy`] and [`Default`]): every operation is a static method, so no arena,
/// generation, or session state can leak into term identity or the engines'
/// decisions.
///
/// # Contract
/// - requires: implementors keep [`CellAlphabet::Cmd`] equality and hashing
///   **structural** (content identity only); [`CellAlphabet::match_cmd`] is
///   one-sided (binds the pattern's metavariables against a ground target),
///   [`CellAlphabet::unify_cmd`] two-sided (a most general unifier), and both
///   leave `subst` possibly partially extended on failure;
///   [`CellAlphabet::rename_apart`] is deterministic and structure-preserving.
/// - requires: every field whose value can distinguish two
///   [`crate::cell::Cell`] values reaches the local ordering digest used by
///   `gandr_theory_deep_inference::normal_form::prim_address`; this is a
///   content-faithfulness obligation, not an injectivity or portability
///   promise.
/// - requires: two occurrences of one primitive at one position remain
///   dependent: [`CellAlphabet::position_order`] must return
///   [`PositionOrder::Same`] for a position paired with itself, so causal
///   layering can separate repeated occurrences by depth in the graded
///   quotient.
/// - ensures: the engines (`gandr_theory_coherent_resolutions::overlap`,
///   `gandr_theory_coherent_resolutions::completion`,
///   `gandr_theory_coherent_resolutions::rewrite`,
///   `gandr_theory_coherent_resolutions::tracelet`,
///   `gandr_theory_decomposition_spaces::compose`) are total over any
///   implementor — every decline path is data, never a panic or divergence.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the sequent alphabet's existing gate suite runs
///   unmodified through the generic interface (behavior preserved), and a
///   second, minimal toy alphabet drives all three engines through the same
///   generic entry points (the lift is genuinely polymorphic, not sequent
///   special-casing).
/// - witness: `toy_alphabet::tests::the_enumerator_finds_the_toy_composition_overlap`
/// - witness: `toy_alphabet::tests::the_normalizer_runs_over_the_toy_alphabet`
/// - witness: `toy_alphabet::tests::completion_orients_and_certifies_over_the_toy_alphabet`
/// - witness: `content_faithfulness::tests::production_alphabets_satisfy_content_faithfulness`
/// - witness: `content_faithfulness::tests::adversarial_alphabets_are_explicitly_exempt`
/// - witness: `content_faithfulness::tests::same_position_dependence_separates_multiplicity`
/// - witness: `content_faithfulness::tests::mutations_are_rejected_by_the_fixture`
pub trait CellAlphabet: Copy + Default + Eq + Ord + core::hash::Hash + core::fmt::Debug
{
    /// The **command-pattern term type** — the tree a cell's two faces are
    /// and the engines rewrite. Equality and hashing are structural content
    /// identity (see the module-level trust warning).
    type Cmd: Clone + Eq + core::hash::Hash + core::fmt::Debug;
    /// The **metavariable** type — a hole in a pattern.
    type Var: Clone + Eq + core::hash::Hash + core::fmt::Debug;
    /// The **hole identity** — how a metavariable is keyed across a cell's
    /// two faces (the sequent alphabet keys by *name*: a name worn at two
    /// polarities is one hole, the dinaturality case).
    type Hole: Clone + Eq + Ord + core::hash::Hash + core::fmt::Debug;
    /// The **position** type — a path addressing a command subterm (the seam
    /// an overlap roots at or a step fires at).
    type Pos: Clone + Eq + core::hash::Hash + core::fmt::Debug;
    /// The **ordered substitution map** — deterministic (a `BTreeMap`-backed
    /// carrier; hash-order iteration is denied workspace-wide).
    type Subst: Clone + Default + Eq + core::hash::Hash + core::fmt::Debug;
    /// How a cell's orientation was fixed (the alphabet's tag vocabulary).
    type Orientation: Copy + Eq + core::hash::Hash + core::fmt::Debug;
    /// Where a cell came from (the alphabet's tag vocabulary).
    type Provenance: Copy + Eq + core::hash::Hash + core::fmt::Debug;
    /// The **derived per-cell metadata** (variance/linearity/invertibility in
    /// the sequent alphabet), computed live by [`CellAlphabet::derive_meta`].
    type Meta: Clone + Eq + core::hash::Hash + core::fmt::Debug;

    /// **One-sided matching** — bind `pattern`'s metavariables so the
    /// instantiated pattern equals the ground `target`, extending `subst`
    /// (cell application; the sequent alphabet also checks cut polarity, K2).
    ///
    /// # Contract
    /// - requires: `target` is metavariable-free.
    /// - ensures: a positive decision instantiates `pattern` to `target` under
    ///   `subst`; a negative decision may leave `subst` partially extended.
    /// - panics: none.
    fn match_cmd(
        pattern: &Self::Cmd,
        target: &Self::Cmd,
        subst: &mut Self::Subst,
    ) -> SubstitutionDecision;

    /// **Two-sided unification** — a most general unifier of `lhs` and `rhs`,
    /// extending `subst` (overlap superposition).
    ///
    /// # Contract
    /// - requires: the two sides' metavariables are kept apart (the enumerator
    ///   renames one cell with [`CellAlphabet::rename_apart`] first).
    /// - ensures: a positive decision makes `subst` a unifier — both sides
    ///   instantiate to the same term; a negative decision (a clash or an
    ///   occurs-check failure) may leave `subst` partially extended.
    /// - panics: none.
    fn unify_cmd(
        lhs: &Self::Cmd,
        rhs: &Self::Cmd,
        subst: &mut Self::Subst,
    ) -> SubstitutionDecision;

    /// Apply a substitution to a term (instantiating every bound
    /// metavariable).
    ///
    /// # Contract
    /// - ensures: every bound metavariable leaf is replaced by its binding;
    ///   unbound metavariables are left in place.
    /// - panics: none.
    fn apply_subst(
        subst: &Self::Subst,
        cmd: &Self::Cmd,
    ) -> Self::Cmd;

    /// The term's metavariables, in left-to-right order **with repeats** (the
    /// metadata derivation counts linearity by occurrences).
    ///
    /// # Contract
    /// - ensures: one entry per metavariable occurrence, in left-to-right
    ///   order.
    /// - panics: none.
    fn metavariables(cmd: &Self::Cmd) -> Vec<Self::Var>;

    /// Every **command position** of the term — the seams a cell can fire at
    /// and the composition enumerator unifies at, in a deterministic
    /// outer-to-inner order.
    ///
    /// # Contract
    /// - ensures: each returned position addresses a command subterm
    ///   ([`CellAlphabet::subterm_cmd_at`] returns `Some` at it).
    /// - panics: none.
    fn command_positions(cmd: &Self::Cmd) -> Vec<Self::Pos>;

    /// The root position (the seam of a confluence overlap).
    ///
    /// # Contract
    /// - ensures: [`CellAlphabet::subterm_cmd_at`] at the root is the whole
    ///   term.
    /// - panics: none.
    fn root_position() -> Self::Pos;

    /// The position addressing the child-index `path`, read from the root
    /// outward.
    ///
    /// This is [`CellAlphabet::root_position`] extended by one step per index,
    /// and it is the inverse of the reading [`CellAlphabet::position_order`]
    /// takes: an alphabet whose positions are child-index paths (which every
    /// alphabet's are, by the [`CellAlphabet::Pos`] contract) builds it from
    /// the path directly.
    ///
    /// It exists because a position can arrive from **outside** the store: a
    /// circuit block's elaborated redex occurrences address the block's derived
    /// boundary by argument-index path
    /// (`gandr_theory_levitation::RedexOccurrence`), and an application site
    /// that reads those paths into positions is reading the record rather than
    /// fabricating positions of its own
    /// (`gandr_theory_computads::instantiate::instantiate_two_redex_rule`).
    ///
    /// # Contract
    /// - requires: `path` reads the child indices from the root outward.
    /// - ensures: the empty path is [`CellAlphabet::root_position`]; two
    ///   positions built here relate under [`CellAlphabet::position_order`]
    ///   exactly as their two paths relate under [`path_order`]; the position
    ///   addresses a subterm ([`CellAlphabet::subterm_cmd_at`] is `Some`) iff
    ///   the term has one at that path, and a path off the term is a position
    ///   that addresses nothing rather than a panic.
    /// - provides: the seam an external record's positions enter the engines
    ///   through.
    /// - panics: none.
    fn position_at_path(path: &[PositionStep]) -> Self::Pos;

    /// How two positions relate — whether one addresses a subtree containing
    /// the other, or the two address **disjoint** subtrees.
    ///
    /// An alphabet whose positions are child-index paths (which every
    /// alphabet's are, by the [`CellAlphabet::Pos`] contract) answers this with
    /// [`path_order`].
    ///
    /// # Contract
    /// - ensures: [`PositionOrder::Incomparable`] exactly when neither position
    ///   is a prefix of the other, in which case the subterms they address are
    ///   disjoint and neither survives inside the other; the relation is
    ///   symmetric up to swapping [`PositionOrder::Encloses`] and
    ///   [`PositionOrder::EnclosedBy`].
    /// - provides: the first conjunct of the shift guard
    ///   (`gandr_theory_deep_inference::shift::derive_shift_equivalence`).
    /// - panics: none.
    fn position_order(
        left: &Self::Pos,
        right: &Self::Pos,
    ) -> PositionOrder;

    /// The **convexity discharge** `store` certifies — the warrant the shift
    /// guard's third conjunct is skipped under, or the refusal to skip it.
    ///
    /// This is a claim about the alphabet's grammar rather than a computation
    /// over the store: an alphabet whose expressible left-hand sides are all
    /// strongly connected, over targets that are all acyclic, forces every
    /// match convex, so no rewrite can destroy a property no match is able to
    /// fail. The store is supplied because a later alphabet — one whose
    /// left-hand sides are *not* all strongly connected — must certify the
    /// property of the cells actually present rather than of the grammar.
    ///
    /// # Contract
    /// - requires: an implementor returns
    ///   [`ConvexityDischarge::LeftConnectedOverAcyclicTarget`] only when every
    ///   left-hand side in `store` is strongly connected and every target the
    ///   engines rewrite is acyclic; otherwise
    ///   [`ConvexityDischarge::ReCheckRequired`].
    /// - ensures: the value is stable for a given store, so a witness carrying
    ///   it names one warrant rather than a sampled one.
    /// - provides: the third conjunct of the shift guard, as a datum instead of
    ///   a per-query reachability sweep.
    /// - panics: none.
    fn convexity_discharge(store: &CellStore<Self>) -> ConvexityDischarge;

    /// The command subterm at `pos`, if the path addresses one.
    ///
    /// # Contract
    /// - ensures: `Some(subterm)` when `pos` addresses a command inside `cmd`;
    ///   `None` when the path leaves the tree or addresses a non-command
    ///   subterm.
    /// - panics: none.
    fn subterm_cmd_at(
        cmd: &Self::Cmd,
        pos: &Self::Pos,
    ) -> Option<Self::Cmd>;

    /// Splice `replacement` into `cmd` at `pos`, returning the rebuilt term.
    ///
    /// # Contract
    /// - requires: `pos` addresses an existing command subterm.
    /// - ensures: `Some(rebuilt)` with the subterm at `pos` replaced; `None`
    ///   when the path leaves the tree or the replacement does not fit the
    ///   slot.
    /// - panics: none.
    fn splice_cmd_at(
        cmd: &Self::Cmd,
        pos: &Self::Pos,
        replacement: Self::Cmd,
    ) -> Option<Self::Cmd>;

    /// The **reduction order** — a well-founded orientation completion uses
    /// to turn a divergent critical pair into a rule (larger on the left).
    ///
    /// # Contract
    /// - ensures: [`core::cmp::Ordering::Equal`] only for pairs the order
    ///   cannot orient (an honest obstruction, never a guessed orientation).
    /// - panics: none.
    fn reduction_cmp(
        lhs: &Self::Cmd,
        rhs: &Self::Cmd,
    ) -> core::cmp::Ordering;

    /// Rename `renamed`'s metavariables disjoint from `anchor`'s (the
    /// deterministic apartness renaming overlap enumeration performs on the
    /// right cell of a pair before unifying).
    ///
    /// # Contract
    /// - ensures: the pair `(lhs, rhs)` of `renamed` with every metavariable
    ///   replaced by a fresh one absent from both `anchor` faces and from the
    ///   other renamed variables, preserving the pattern shapes (so the
    ///   original right leg is recoverable by zipping occurrences); an
    ///   already-disjoint cell is returned unchanged.
    /// - panics: none.
    fn rename_apart(
        anchor: (&Self::Cmd, &Self::Cmd),
        renamed: (&Self::Cmd, &Self::Cmd),
    ) -> (Self::Cmd, Self::Cmd);

    /// **Skolemize** a term — replace each metavariable with a fresh,
    /// reserved constant so replay can ground-rewrite the schematic peak.
    ///
    /// # Contract
    /// - ensures: the result is metavariable-free; the mapping is
    ///   name-deterministic, so shared metavariables skolemize identically
    ///   across a tracelet's peak and join, and the constants are irreducible
    ///   under the alphabet's own cells.
    /// - panics: none.
    fn skolemize(cmd: &Self::Cmd) -> Self::Cmd;

    /// The hole identity of a metavariable (the composition gate keys flow by
    /// it).
    ///
    /// # Contract
    /// - ensures: metavariables denoting the same hole across a cell's two
    ///   faces map to equal holes.
    /// - panics: none.
    fn hole_of(var: &Self::Var) -> Self::Hole;

    /// Whether `provenance` marks an **invertible joinability certificate**
    /// (a completion-emitted cell) as opposed to an oriented optimization
    /// cell — the flag the two-mode composition lane branches on.
    ///
    /// # Contract
    /// - ensures: positive exactly for the provenance
    ///   [`CellAlphabet::derived_provenance`] produces.
    /// - panics: none.
    fn completion_certificate(provenance: &Self::Provenance) -> CellInvertibility;

    /// Derive the per-cell metadata **live** from the two faces (never a
    /// stage-0 constant).
    ///
    /// # Contract
    /// - ensures: the metadata a freshly built cell carries; `invertible` is
    ///   carried through verbatim.
    /// - panics: none.
    fn derive_meta(
        lhs: &Self::Cmd,
        rhs: &Self::Cmd,
        invertible: CellInvertibility,
    ) -> Self::Meta;

    /// The `(variable, flow role)` endpoints `meta` carries for `hole` — the
    /// composition gate's per-cell read of one seam hole.
    ///
    /// # Contract
    /// - ensures: one endpoint per metadata entry whose hole identity equals
    ///   `hole`; empty when the hole does not occur.
    /// - panics: none.
    fn hole_flow(
        meta: &Self::Meta,
        hole: &Self::Hole,
    ) -> Vec<(Self::Var, SeamRole)>;

    /// The **firing discipline** — whether a cell of `provenance` may fire at
    /// `target` (the sequent alphabet's η-polarity gate: an η cell fires only
    /// at a cut of its required polarity; every other cell fires freely).
    ///
    /// # Contract
    /// - ensures: a negative permission means the cell must not fire at
    ///   `target`, however well its left-hand side matches.
    /// - panics: none.
    fn may_fire(
        provenance: &Self::Provenance,
        target: &Self::Cmd,
    ) -> FiringPermission;

    /// The orientation tag stamped on cells completion and fusion derive.
    ///
    /// # Contract
    /// - ensures: the orientation
    ///   `gandr_theory_coherent_resolutions::completion` and
    ///   `gandr_theory_coherent_resolutions::tracelet::derive_fused` stamp
    ///   derived cells with.
    /// - panics: none.
    fn derived_orientation() -> Self::Orientation;

    /// The provenance tag stamped on cells completion and fusion derive (the
    /// tag [`CellAlphabet::completion_certificate`] reads).
    ///
    /// # Contract
    /// - ensures: the provenance
    ///   `gandr_theory_coherent_resolutions::completion` and
    ///   `gandr_theory_coherent_resolutions::tracelet::derive_fused` stamp
    ///   derived cells with.
    /// - panics: none.
    fn derived_provenance() -> Self::Provenance;
}
