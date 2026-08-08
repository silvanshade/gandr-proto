//! The **tracelet normal form** — a canonical form on certificate data, and a
//! decidable **sound under-approximation** of replay-equality.
//!
//! Research reference: N. Behr & J. Kock, "Tracelet Hopf Algebras and
//! Decomposition Spaces (Extended Abstract)", EPTCS 372, 323–337, DOI
//! `10.4204/EPTCS.372.23` — register key `behr-kock-2021-tracelet-hopf` in
//! `docs/gandr/spec/bibliography.yml`. The shift quotient of the tracelet
//! algebra is free symmetric monoidal on primitives — the statement this module
//! reads *semantically*: a derivation factors uniquely into content-addressed
//! primitives carrying integer multiplicities, scheduled canonically. Nothing
//! algebraic ships — no vector space, no formal sum, no antipode, and no
//! word-problem procedure. The Hopf structure is metatheory and specification
//! currency; what is built here is the canonical form on certificate data that
//! currency licenses.
//!
//! # The one contract that must not be misread
//!
//! **NF-equal implies replay-equal. The converse is never claimed, and is
//! false.**
//!
//! [`nf_equal`] is a *sufficient* condition for two derivations to be the same
//! transformation, never a necessary one. Two derivations that reach one
//! boundary by genuinely different primitive factorizations are replay-equal
//! and NF-distinct, and that is the intended behavior rather than a gap to be
//! closed: [`crate::tracelet::replay_equivalent`] remains the semantic oracle
//! and certificate identity is unchanged by this module. The witness that the
//! asymmetry is real, not a disclaimer, is
//! [`crate::tracelet::derive_fused`]: it produces one boundary whose `path_a`
//! is a two-step derivation and whose `path_b` is a single fused step — the
//! same transformation, two normal forms.
//!
//! Read the direction off the types rather than off the prose: nothing here
//! returns "not equal"; [`nf_equal`] answers a [`NormalFormEquality`] whose
//! **positive** value is the only load-bearing one, and every path that cannot
//! confirm the identification answers negatively or refuses with a
//! [`NormalFormObstruction`].
//!
//! # Why the soundness argument is local
//!
//! [`normalize`] does not reason about a recorded path — it **runs** it, under
//! the same skolemization discipline replay uses ([`CellAlphabet::skolemize`],
//! ADR-69), and refuses anything it cannot confirm. A returned
//! [`TraceletNf`] is therefore already a replay receipt: the recorded path
//! fired step by step from the skolemized peak and landed on the skolemized
//! join, and the normal form records the boundary it did that across. So
//! `nf_equal(a, b)` gives peak equality, join equality, and "each replays" —
//! which is exactly [`crate::tracelet::replay_equivalent`], with no appeal to
//! a chain of commutations.
//!
//! The primitive factorization and the schedule are what make the relation a
//! *normal form* rather than a boundary record: they make NF-equality strictly
//! **finer** than boundary equality, and finer is still sound. What they buy is
//! that the answer is precomputable — one replay per certificate, then map
//! comparison forever after — which is the whole point of a fast path.
//!
//! # The three quotients, and where each one is decided
//!
//! `equiv_N` is the reflexive-symmetric-transitive closure of
//! `equiv_A ∪ equiv_T ∪ equiv_S`:
//!
//! - **`equiv_A`, content addressing.** Two occurrences of one primitive are
//!   one primitive with multiplicity two. The address ([`PrimId`]) is a digest
//!   over the **resolved cell's content** and the position — never the store
//!   index — computed by [`prim_address`]. It is an index and an ordering
//!   device, and **nowhere the identity witness**: a digest collision inside
//!   one normal form is detected at construction and refused
//!   ([`NormalFormObstruction::ContentAddressCollision`]), and across two
//!   normal forms the map values carry the [`PrimCert`]s, which [`nf_equal`]
//!   compares. So a collision costs a refusal, never an unsound identification.
//! - **`equiv_T`, unit elimination.** A step that fires and leaves the term
//!   unchanged contributes nothing and is dropped. Dropping it cannot move the
//!   endpoint, because it did not move the term.
//! - **`equiv_S`, the shift quotient.** Adjacent applications at incomparable
//!   positions whose cells have trivial overlap commute. This module does not
//!   restate that guard — it **asks** it
//!   ([`crate::shift::derive_shift_equivalence`]'s own three conjuncts, through
//!   the crate's single [`crate::shift`] guard), and reads any refusal as
//!   dependence, which is the conservative direction.
//!
//! # The canonical schedule, and why it is shift-invariant
//!
//! The schedule is the **causal layering**: each surviving occurrence takes the
//! depth `1 + max` over the earlier occurrences it depends on (zero when it
//! depends on none), and the occurrences are then stably sorted by
//! `(depth, address)`. A licensed transposition swaps two occurrences with no
//! dependence edge between them, so it changes no edge at all and therefore no
//! depth — which is why two derivations related by licensed transpositions sort
//! to the *same* sequence. Two occurrences cannot tie: the same primitive at
//! the same position is always dependent on itself, so a repeat sits at a
//! strictly greater depth.
//!
//! # And the canonicalization is checked, not trusted
//!
//! The canonical schedule is **replayed** from the peak before the normal form
//! is returned. A schedule that does not fire, or that fires and reaches
//! something other than the recorded join, is refused
//! ([`NormalFormObstruction::ShiftedScheduleDoesNotFire`],
//! [`NormalFormObstruction::ShiftedScheduleMissesTheJoin`]).
//!
//! **Either refusal is this lane's kill signal.** A shift-equivalent,
//! replay-divergent schedule is a soundness defect in position or overlap
//! bookkeeping, and it is not a case to work around: it means the independence
//! relation licensed a commutation the semantics does not have. Raising it here
//! rather than only in the test suite is deliberate — the failure mode must be
//! observable in the engine, not only under a generator.
//!
//! # Cost, and what this rung deliberately does not build
//!
//! Building a normal form costs one replay of the recorded path, one replay of
//! the canonical schedule, and a **quadratic** number of independence questions
//! in the surviving path length. That quadratic is the measured falsifier the
//! cheapness argument can lose to, not a prediction
//! (`shift::tests::an_adjacent_transposition_schedule_asks_a_quadratic_number_of_questions`
//! in the crate's integration suite). The overlap conjunct is cell-pair-keyed
//! and cacheable, and **that cache is not built here**: it is the
//! overlap-support cache rung, and adding a private copy of it now would put a
//! second index in the crate for the same relation.
//!
//! [`CellAlphabet::skolemize`]: crate::alphabet::CellAlphabet::skolemize

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::btree_map::Entry;
use alloc::vec::Vec;

use crate::alphabet::CellAlphabet;
use crate::alphabet::ConvexityDischarge;
use crate::boundary::CausalDepth;
use crate::boundary::NormalFormEquality;
use crate::boundary::PrimMultiplicity;
use crate::boundary::StepIndependence;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::rewrite::CellApp;
use crate::rewrite::rewrite_at;
use crate::sequent::SequentAlphabet;
use crate::shift::check_shift_guard;
use crate::tracelet::Tracelet;

/// The FNV-1a offset basis of the 128-bit primitive content digest.
const CONTENT_BASIS: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;

/// The FNV-1a prime of the 128-bit primitive content digest.
const CONTENT_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// The domain separator mixed in before a primitive's content, so a digest
/// taken here cannot collide with one taken over the same bytes for another
/// purpose.
const PRIMITIVE_DOMAIN: &[u8] = b"gandr.tracelet.primitive.v1";

/// A **content address** of a primitive certificate — a 128-bit digest over the
/// resolved cell's content and the position it fires at.
///
/// It orders the primitive factorization and breaks the canonical schedule's
/// ties. It is **not** an identity witness: see the module's `equiv_A` note for
/// how a collision is refused rather than believed.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PrimId(u128);

/// A **primitive certificate** — one indecomposable factor of a normalized
/// derivation.
///
/// Representationally a recorded step; nominally distinct from one, because a
/// [`CellApp`] is a step *of a path* (which may be a unit, and whose place in
/// the path is the recorded one) while a [`PrimCert`] is a *factor of a normal
/// form* (never a unit, and placed by the canonical schedule).
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PrimCert<A: CellAlphabet = SequentAlphabet>(pub CellApp<A>);

impl<A: CellAlphabet> PrimCert<A>
{
    /// The recorded step this primitive applies.
    #[inline]
    #[must_use]
    pub const fn step(&self) -> &CellApp<A>
    {
        &self.0
    }
}

/// The **tracelet normal form** of one recorded derivation — its boundary, its
/// graded primitive factorization, and its canonical schedule.
///
/// A value of this type is a replay receipt as well as a canonical form: it
/// exists only because [`normalize`] ran the derivation and confirmed both the
/// recorded path and the canonical schedule reach the recorded join.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceletNf<A: CellAlphabet = SequentAlphabet>
{
    /// The term the derivation starts from, as recorded (schematic; replay
    /// skolemizes it).
    pub peak: A::Cmd,
    /// The term the derivation reaches, as recorded (schematic).
    pub joins_at: A::Cmd,
    /// The warrant the shift quotient's convexity conjunct was decided under,
    /// carried rather than recomputed — the normal form says which
    /// independence relation it was taken with respect to.
    pub convexity: ConvexityDischarge,
    /// The **unique primitive factorization**: each primitive once, under its
    /// content address, with the number of times it occurs.
    pub primitives: BTreeMap<PrimId, (PrimCert<A>, PrimMultiplicity)>,
    /// The **canonical schedule** — the causal layering, flattened, as content
    /// addresses. Its length is the sum of the multiplicities.
    pub schedule: Vec<PrimId>,
}

impl<A: CellAlphabet> TraceletNf<A>
{
    /// The canonical schedule as a runnable path.
    ///
    /// # Contract
    /// - ensures: `Some(path)` whose steps are the [`TraceletNf::schedule`]'s
    ///   addresses resolved through [`TraceletNf::primitives`], in schedule
    ///   order; `None` only for a normal form whose schedule names an address
    ///   its factorization does not hold, which [`normalize`] cannot produce.
    /// - provides: the compressed certificate's decompression — the path whose
    ///   replay [`normalize`] already confirmed.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the decompressed path is validated against
    ///   the input rather than a predicted answer: it is replayed and must
    ///   reach the normal form's own join.
    /// - witness: `normal_form::tests::the_canonical_path_replays_to_the_recorded_join`
    #[inline]
    #[must_use]
    pub fn canonical_path(&self) -> Option<Vec<CellApp<A>>>
    {
        let mut path = Vec::with_capacity(self.schedule.len());
        for address in &self.schedule {
            let graded = self.primitives.get(address)?;
            path.push(graded.0.0.clone());
        }
        Some(path)
    }
}

/// Why a recorded derivation was **refused** a normal form.
///
/// Refusal is data, never a panic and never a silent identity. The last two
/// variants are this lane's **kill signal** and are documented at their
/// declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalFormObstruction<A: CellAlphabet = SequentAlphabet>
{
    /// A recorded step names a cell the store does not hold.
    UnknownCell
    {
        /// The identifier that resolved to nothing.
        cell: CellId,
    },
    /// A recorded step does not fire at its recorded position, so the path is
    /// not a derivation at all and there is nothing to normalize.
    StepDoesNotFire
    {
        /// The step that failed to fire.
        step: Box<CellApp<A>>,
    },
    /// The recorded path fires throughout and lands somewhere other than the
    /// recorded join — the certificate does not replay, so it has no normal
    /// form.
    PathMissesTheJoin
    {
        /// The term the recorded path actually reached.
        reached: Box<A::Cmd>,
    },
    /// Two distinct primitives share a content address inside one normal form.
    ///
    /// A 128-bit digest collision, refused rather than believed: identity never
    /// rests on the address, so the honest outcome is to decline the normal
    /// form.
    ContentAddressCollision
    {
        /// The address both primitives hashed to.
        address: PrimId,
        /// The primitive already recorded under it.
        held: Box<PrimCert<A>>,
        /// The primitive that collided with it.
        offered: Box<PrimCert<A>>,
    },
    /// **Kill signal.** The canonical schedule contains a step that does not
    /// fire, although the recorded path did.
    ///
    /// The independence relation licensed a transposition the semantics does
    /// not have; that is a soundness defect in position or overlap
    /// bookkeeping and is never a case to work around.
    ShiftedScheduleDoesNotFire
    {
        /// The canonical step that failed to fire.
        step: Box<CellApp<A>>,
    },
    /// **Kill signal.** The canonical schedule fires throughout and reaches a
    /// term other than the one the recorded path reached.
    ///
    /// Same defect as [`NormalFormObstruction::ShiftedScheduleDoesNotFire`],
    /// caught one step later: a shift-equivalent, replay-divergent pair.
    ShiftedScheduleMissesTheJoin
    {
        /// The term the canonical schedule reached.
        reached: Box<A::Cmd>,
    },
}

/// The **content address** of a primitive — a digest over the cell's content
/// and the position, never over the store index.
///
/// # Contract
/// - requires: `cell` is the cell a [`PrimCert`]'s [`CellId`] resolves to in
///   the store the certificate is read against.
/// - ensures: equal `(cell content, position)` pairs give equal addresses, for
///   any alphabet, deterministically and without session state — the
///   [`CellAlphabet`] contract pins [`core::hash::Hash`] to structural content
///   identity, which is what makes this an address rather than a fingerprint.
/// - provides: the total order a [`BTreeMap`]-keyed factorization needs, which
///   neither `A::Pos` nor `A::Cmd` supplies.
/// - panics: none.
/// - intension: FNV-1a over 128 bits, domain-separated. Distinct inputs may in
///   principle collide; no caller treats an address as proof of identity, and
///   [`normalize`] refuses a collision outright.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the decision surface is "same content, same
///   address; different content, different address", separated by a cell pair
///   differing only in its right-hand side, a position pair differing only in
///   one step, and an equal pair rebuilt independently.
/// - witness: `normal_form::tests::the_content_address_is_taken_over_content`
#[inline]
#[must_use]
pub fn prim_address<A>(
    cell: &Cell<A>,
    at: &A::Pos,
) -> PrimId
where
    A: CellAlphabet,
{
    let mut hasher = ContentHasher::new();
    core::hash::Hasher::write(&mut hasher, PRIMITIVE_DOMAIN);
    core::hash::Hash::hash(cell, &mut hasher);
    core::hash::Hash::hash(at, &mut hasher);
    hasher.digest()
}

/// **Normalize** a recorded derivation to its tracelet normal form, or refuse
/// it.
///
/// The three quotients are applied in the order the module documents: the
/// recorded path is run and its unit steps dropped (`equiv_T`), the survivors
/// are content-addressed and graded (`equiv_A`), and the canonical schedule is
/// computed by causal layering under the shift guard's independence relation
/// (`equiv_S`) and then **replayed** before the normal form is returned.
///
/// # Contract
/// - requires: `path` is the derivation recorded from `peak` to `joins_at`
///   against `store` — the same triple [`crate::tracelet::Tracelet::replay`]
///   would be given.
/// - ensures: `Ok(nf)` only when the recorded path fires step by step from the
///   skolemized `peak` and reaches the skolemized `joins_at`, **and** the
///   canonical schedule does the same. The returned normal form records that
///   boundary verbatim, so possessing one is possessing a replay receipt; its
///   factorization is graded by occurrence count and its schedule is the causal
///   layering flattened.
/// - provides: the decidable **sound under-approximation** of replay-equality
///   the module opens with — `Ok` on both sides plus [`nf_equal`] implies
///   [`crate::tracelet::replay_equivalent`]. The converse does not hold and is
///   not claimed.
/// - fails: [`NormalFormObstruction`] — a stale cell id, a recorded step that
///   does not fire, a recorded path that misses the join, a content-address
///   collision, or (the kill signal) a canonical schedule that does not fire or
///   misses the join.
/// - panics: none.
/// - intension: one replay of the recorded path, one replay of the canonical
///   schedule, and a number of independence questions quadratic in the
///   surviving path length; the questions go through the crate's single shift
///   guard and are not cached here.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the normal form is a certificate validated
///   against the input rather than against a predicted answer (its canonical
///   path is replayed and must reach its own join), and each `- fails:` mode
///   owns a decision surface separated by a fixture that triggers only it: a
///   fabricated identifier, a step at a position carrying no redex, a
///   retargeted join, and — over an alphabet with a nonempty shift extension —
///   a schedule permutation.
/// - witness: `normal_form::tests::a_recorded_derivation_normalizes_to_a_replay_receipt`
/// - witness: `normal_form::tests::an_unknown_cell_identifier_is_refused`
/// - witness: `normal_form::tests::a_step_that_does_not_fire_is_refused`
/// - witness: `normal_form::tests::a_path_that_misses_its_join_is_refused`
/// - witness: `normal_form::tests::a_unit_step_is_eliminated`
/// - witness: `normal_form::tests::the_canonical_path_replays_to_the_recorded_join`
/// - witness: `normal_form::tests::the_shift_quotient_is_empty_over_the_sequent_alphabet`
#[inline]
pub fn normalize<A>(
    store: &CellStore<A>,
    peak: &A::Cmd,
    joins_at: &A::Cmd,
    path: &[CellApp<A>],
) -> Result<TraceletNf<A>, NormalFormObstruction<A>>
where
    A: CellAlphabet,
{
    let start = A::skolemize(peak);
    let target = A::skolemize(joins_at);
    let survivors = run_recording(store, &start, path)?;
    let reached = survivors.reached;
    if reached != target {
        return Err(NormalFormObstruction::PathMissesTheJoin {
            reached: Box::new(reached),
        });
    }
    let convexity = A::convexity_discharge(store);
    let occurrences = layer_causally(store, survivors.steps, convexity);
    let mut primitives: BTreeMap<PrimId, (PrimCert<A>, PrimMultiplicity)> = BTreeMap::new();
    let mut schedule = Vec::with_capacity(occurrences.len());
    let mut canonical = Vec::with_capacity(occurrences.len());
    for occurrence in occurrences {
        let cert = PrimCert(occurrence.step.clone());
        match primitives.entry(occurrence.address) {
            | Entry::Vacant(slot) => {
                slot.insert((cert, PrimMultiplicity::from(1_u32)));
            },
            | Entry::Occupied(mut slot) => {
                let graded = slot.get_mut();
                if graded.0 != cert {
                    return Err(NormalFormObstruction::ContentAddressCollision {
                        address: occurrence.address,
                        held: Box::new(graded.0.clone()),
                        offered: Box::new(cert),
                    });
                }
                graded.1 = PrimMultiplicity::from(u32::from(graded.1).saturating_add(1_u32));
            },
        }
        schedule.push(occurrence.address);
        canonical.push(occurrence.step);
    }
    let shifted = run_schedule(store, &start, &canonical)?;
    if shifted != target {
        return Err(NormalFormObstruction::ShiftedScheduleMissesTheJoin {
            reached: Box::new(shifted),
        });
    }
    Ok(TraceletNf {
        peak: peak.clone(),
        joins_at: joins_at.clone(),
        convexity,
        primitives,
        schedule,
    })
}

/// Whether two normal forms are the **same normal form**.
///
/// # Contract
/// - requires: both normal forms were produced by [`normalize`] against the
///   **same** store, because a [`PrimCert`] names its cell by store identifier;
///   comparing normal forms taken against different stores compares handles
///   whose meanings need not agree.
/// - ensures: positive iff the two agree on peak, join, convexity warrant,
///   graded factorization, and canonical schedule.
/// - provides: **the sound direction only** — a positive answer means the two
///   derivations are the same transformation, so
///   [`crate::tracelet::replay_equivalent`] holds of any two tracelets carrying
///   these boundaries and paths. A negative answer means **nothing**: two
///   replay-equal derivations with different primitive factorizations are
///   NF-distinct by design.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the positive direction is validated against the
///   input by replaying both derivations, and the negative direction is pinned
///   as uninformative by a pair that is replay-equal and NF-distinct.
/// - witness: `normal_form::tests::one_derivation_is_nf_equal_to_itself`
/// - witness: `normal_form::tests::replay_equal_derivations_may_be_nf_distinct`
#[inline]
#[must_use]
pub fn nf_equal<A>(
    left: &TraceletNf<A>,
    right: &TraceletNf<A>,
) -> NormalFormEquality
where
    A: CellAlphabet,
{
    NormalFormEquality::from(left == right)
}

/// The **certificate-level fast path**: whether two tracelets are certified
/// equal by their normal forms.
///
/// This is the shape a `cells_equal` fast path consumes. It is deliberately
/// *not* wired into any equality decision by this module: adopting it is the
/// certificate-adoption rung's business, and the guard plus tractability
/// witness that adoption owes is the engine-metatheory contract's.
///
/// # Contract
/// - requires: both tracelets are read against `store`.
/// - ensures: positive iff all four recorded paths normalize against `store`
///   and the two tracelets' normal forms agree path-for-path — which forces
///   equal peaks, equal joins, and successful replay of both tracelets, and so
///   forces [`crate::tracelet::replay_equivalent`].
/// - provides: the sound direction only. A negative answer is not a claim that
///   the two certificates differ; it is the absence of a cheap proof that they
///   agree, and the caller falls back to the replay oracle.
/// - panics: none.
/// - intension: it normalizes rather than consulting a cache, so it pays two
///   replays per path; a consumer that compares repeatedly normalizes once and
///   keeps the normal forms.
///
/// # Adequacy
/// - hypothesis: L2 agreement — the oracle is external
///   ([`crate::tracelet::replay_equivalent`]) and the differential asserts the
///   implication over generated derivation pairs rather than over hand-picked
///   ones.
/// - witness: `normal_form::tests::nf_equal_certificates_are_replay_equivalent`
/// - witness: `normal_form::tests::every_nf_equal_pair_is_replay_equivalent`
#[inline]
#[must_use]
pub fn tracelets_nf_equal<A>(
    store: &CellStore<A>,
    left: &Tracelet<A>,
    right: &Tracelet<A>,
) -> NormalFormEquality
where
    A: CellAlphabet,
{
    let Ok(left_a) = normalize(store, &left.overlap.peak, &left.joins_at, &left.path_a)
    else {
        return NormalFormEquality::from(false);
    };
    let Ok(left_b) = normalize(store, &left.overlap.peak, &left.joins_at, &left.path_b)
    else {
        return NormalFormEquality::from(false);
    };
    let Ok(right_a) = normalize(store, &right.overlap.peak, &right.joins_at, &right.path_a)
    else {
        return NormalFormEquality::from(false);
    };
    let Ok(right_b) = normalize(store, &right.overlap.peak, &right.joins_at, &right.path_b)
    else {
        return NormalFormEquality::from(false);
    };
    NormalFormEquality::from(
        bool::from(nf_equal(&left_a, &right_a)) && bool::from(nf_equal(&left_b, &right_b)),
    )
}

/// One surviving step of a run, with its content address.
#[derive(Clone, Debug)]
struct RunStep<A: CellAlphabet>
{
    /// The step, as recorded.
    step: CellApp<A>,
    /// The content address of the primitive it applies.
    address: PrimId,
}

/// The outcome of running a recorded path: the surviving steps and the term
/// reached.
#[derive(Clone, Debug)]
struct RunSurvivors<A: CellAlphabet>
{
    /// The steps that moved the term, in recorded order (unit steps dropped).
    steps: Vec<RunStep<A>>,
    /// The term the whole recorded path reached.
    reached: A::Cmd,
}

/// One surviving step placed in the causal layering.
#[derive(Clone, Debug)]
struct Occurrence<A: CellAlphabet>
{
    /// The step, as recorded.
    step: CellApp<A>,
    /// The content address of the primitive it applies.
    address: PrimId,
    /// Its layer in the dependence order.
    depth: CausalDepth,
}

/// Run `path` from `start`, dropping the steps that leave the term unchanged.
///
/// This is `equiv_T`, unit elimination, decided by observation rather than by a
/// syntactic test: a step is a unit exactly when firing it is a no-op, which is
/// why dropping it cannot move the endpoint.
///
/// # Contract
/// - ensures: `Ok(survivors)` when every recorded step's cell resolves and
///   fires at its recorded position; `survivors.reached` is the term the whole
///   path reached and `survivors.steps` are the steps that changed it.
/// - fails: [`NormalFormObstruction::UnknownCell`] for a stale identifier and
///   [`NormalFormObstruction::StepDoesNotFire`] for a position carrying no
///   redex.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
#[inline]
fn run_recording<A>(
    store: &CellStore<A>,
    start: &A::Cmd,
    path: &[CellApp<A>],
) -> Result<RunSurvivors<A>, NormalFormObstruction<A>>
where
    A: CellAlphabet,
{
    let mut current = start.clone();
    let mut steps = Vec::with_capacity(path.len());
    for step in path {
        let Some(cell) = store.get(step.cell)
        else {
            return Err(NormalFormObstruction::UnknownCell { cell: step.cell });
        };
        let Some(next) = rewrite_at(cell, &current, &step.at)
        else {
            return Err(NormalFormObstruction::StepDoesNotFire {
                step: Box::new(step.clone()),
            });
        };
        if next == current {
            continue;
        }
        steps.push(RunStep {
            step: step.clone(),
            address: prim_address(cell, &step.at),
        });
        current = next;
    }
    Ok(RunSurvivors {
        steps,
        reached: current,
    })
}

/// Run a canonical schedule from `start`, refusing with the kill-signal
/// variant.
///
/// # Contract
/// - ensures: `Ok(term)` when every canonical step fires in order.
/// - fails: [`NormalFormObstruction::UnknownCell`] for a stale identifier and
///   [`NormalFormObstruction::ShiftedScheduleDoesNotFire`] — the kill signal —
///   for a canonical step that carries no redex although the recorded path
///   fired.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
#[inline]
fn run_schedule<A>(
    store: &CellStore<A>,
    start: &A::Cmd,
    schedule: &[CellApp<A>],
) -> Result<A::Cmd, NormalFormObstruction<A>>
where
    A: CellAlphabet,
{
    let mut current = start.clone();
    for step in schedule {
        let Some(cell) = store.get(step.cell)
        else {
            return Err(NormalFormObstruction::UnknownCell { cell: step.cell });
        };
        let Some(next) = rewrite_at(cell, &current, &step.at)
        else {
            return Err(NormalFormObstruction::ShiftedScheduleDoesNotFire {
                step: Box::new(step.clone()),
            });
        };
        current = next;
    }
    Ok(current)
}

/// Place the surviving steps in the causal layering and sort them canonically.
///
/// # Contract
/// - ensures: each step takes the depth `1 + max` over the earlier steps it
///   depends on (zero when it depends on none), and the result is stably sorted
///   by `(depth, address)`. Because a repeat of one primitive always depends on
///   its earlier occurrence, no two entries can tie on both keys, so the order
///   is total and the result is a canonical representative of the shift class.
/// - provides: `equiv_S`, decided through the crate's single independence
///   relation rather than a second copy of it.
/// - panics: none.
/// - intension: quadratic in the number of surviving steps, one independence
///   question per ordered pair.
#[inline]
fn layer_causally<A>(
    store: &CellStore<A>,
    steps: Vec<RunStep<A>>,
    convexity: ConvexityDischarge,
) -> Vec<Occurrence<A>>
where
    A: CellAlphabet,
{
    let mut depths: Vec<CausalDepth> = Vec::with_capacity(steps.len());
    for (index, current) in steps.iter().enumerate() {
        let mut depth = 0_usize;
        for (earlier, prior) in steps.iter().enumerate().take(index) {
            if bool::from(step_independence(store, prior, current, convexity)) {
                continue;
            }
            let prior_depth = depths.get(earlier).copied().unwrap_or_default();
            depth = depth.max(usize::from(prior_depth).saturating_add(1_usize));
        }
        depths.push(CausalDepth::from(depth));
    }
    let mut occurrences: Vec<Occurrence<A>> = steps
        .into_iter()
        .zip(depths)
        .map(|(entry, depth)| Occurrence {
            step: entry.step,
            address: entry.address,
            depth,
        })
        .collect();
    occurrences.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.address.cmp(&right.address))
    });
    occurrences
}

/// Whether two recorded steps are licensed to commute.
///
/// The question is delegated to the crate's single shift guard
/// ([`check_shift_guard`]) rather than restated, and **any** refusal — a
/// comparable position, a genuine overlap, an undischarged convexity conjunct,
/// or an unresolvable identifier — is read as dependence. That direction is the
/// conservative one: refusing to commute keeps the recorded order, which is
/// always a valid derivation.
///
/// # Contract
/// - ensures: a positive answer exactly when the guard's three conjuncts hold
///   of the pair under `convexity`.
/// - provides: the independence relation the causal layering quotients by.
/// - panics: none.
#[inline]
fn step_independence<A>(
    store: &CellStore<A>,
    left: &RunStep<A>,
    right: &RunStep<A>,
    convexity: ConvexityDischarge,
) -> StepIndependence
where
    A: CellAlphabet,
{
    StepIndependence::from(check_shift_guard(store, &left.step, &right.step, convexity).is_ok())
}

/// A deterministic 128-bit FNV-1a digest over [`core::hash::Hash`] writes.
///
/// It exists because the crate needs a **content address** and has no hashing
/// dependency: [`core::hash::Hash`] is already pinned to structural content
/// identity by the [`CellAlphabet`] contract, so streaming it through a
/// fixed-seed hasher turns that guarantee into an orderable key.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
struct ContentHasher
{
    /// The accumulated digest state.
    state: u128,
}

impl ContentHasher
{
    /// A fresh digest state at the FNV-1a offset basis.
    const fn new() -> Self
    {
        Self {
            state: CONTENT_BASIS,
        }
    }

    /// The digest accumulated so far.
    const fn digest(&self) -> PrimId
    {
        PrimId(self.state)
    }
}

impl core::hash::Hasher for ContentHasher
{
    #[inline]
    fn finish(&self) -> u64
    {
        let folded = self.state ^ (self.state >> 64_u32);
        u64::try_from(folded & u128::from(u64::MAX)).unwrap_or_default()
    }

    #[inline]
    fn write(
        &mut self,
        bytes: &[u8],
    )
    {
        for byte in bytes {
            self.state ^= u128::from(*byte);
            self.state = self.state.wrapping_mul(CONTENT_PRIME);
        }
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;

    use super::*;
    use crate::cell::Cell;
    use crate::overlap::OverlapKind;
    use crate::overlap::enumerate_overlaps;
    use crate::pattern::CmdPat;
    use crate::pattern::ConsPat;
    use crate::pattern::Pos;
    use crate::pattern::ProdPat;
    use crate::pattern::Sym;
    use crate::sequent::CellProvenance;
    use crate::sequent::Orientation;
    use crate::sequent::frame_defining_cell;
    use crate::tracelet::derive_fused;
    use crate::tracelet::replay_equivalent;

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

    /// A reflexive cell `⟨Zero | ⊤⟩ ~> ⟨Zero | ⊤⟩` — the unit step `equiv_T`
    /// eliminates.
    fn reflexive_cell() -> Cell
    {
        let face = CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top);
        Cell::new(
            face.clone(),
            face,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// The fused-cell store and the tracelet `derive_fused` certifies.
    fn fused_fixture() -> (CellStore, Tracelet)
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let composition = enumerate_overlaps(&store)
            .into_iter()
            .find(|candidate| {
                candidate.kind == OverlapKind::Composition
                    && candidate.left == frame
                    && candidate.right == add
            })
            .expect("the composition overlap exists");
        let (_fused, tracelet) =
            derive_fused(&composition, &mut store).expect("the fused cell is derived");
        (store, tracelet)
    }

    #[test]
    fn a_recorded_derivation_normalizes_to_a_replay_receipt()
    {
        let (store, tracelet) = fused_fixture();
        let normal = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_a,
        )
        .expect("the two-step derivation replays, so it normalizes");
        assert_eq!(
            tracelet.overlap.peak, normal.peak,
            "the normal form records the boundary it was verified across"
        );
        assert_eq!(tracelet.joins_at, normal.joins_at, "join included");
        assert_eq!(
            2,
            normal.schedule.len(),
            "the two-step path factors into two primitives"
        );
        assert_eq!(
            2,
            normal.primitives.len(),
            "and the two are distinct primitives, so each is graded once"
        );
        for graded in normal.primitives.values() {
            assert_eq!(
                PrimMultiplicity::from(1_u32),
                graded.1,
                "each primitive occurs once"
            );
        }
    }

    #[test]
    fn the_canonical_path_replays_to_the_recorded_join()
    {
        let (store, tracelet) = fused_fixture();
        let normal = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_a,
        )
        .expect("the two-step derivation normalizes");
        let path = normal
            .canonical_path()
            .expect("every scheduled address is in the factorization");
        let replayed = Tracelet {
            overlap: tracelet.overlap,
            path_a: path.clone(),
            path_b: path,
            joins_at: tracelet.joins_at,
        };
        assert!(
            bool::from(replayed.replay(&store)),
            "the decompressed canonical schedule is a derivation of the same boundary"
        );
    }

    #[test]
    fn one_derivation_is_nf_equal_to_itself()
    {
        let (store, tracelet) = fused_fixture();
        let normal = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_a,
        )
        .expect("the two-step derivation normalizes");
        assert!(
            bool::from(nf_equal(&normal, &normal)),
            "a normal form is its own normal form"
        );
        assert!(
            bool::from(tracelets_nf_equal(&store, &tracelet, &tracelet)),
            "and the certificate-level fast path agrees"
        );
    }

    #[test]
    fn replay_equal_derivations_may_be_nf_distinct()
    {
        // THE ASYMMETRY, exhibited rather than disclaimed. `derive_fused` gives
        // one boundary with a two-step `path_a` and a one-step `path_b`: the
        // same transformation by the replay oracle, two different primitive
        // factorizations, so NF-distinct. A negative from `nf_equal` therefore
        // means nothing at all about the certificates.
        let (store, tracelet) = fused_fixture();
        let two_step = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_a,
        )
        .expect("the two-step derivation normalizes");
        let fused = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_b,
        )
        .expect("the single fused step normalizes");
        assert!(
            !bool::from(nf_equal(&two_step, &fused)),
            "the two factorizations differ, so the normal forms do"
        );
        let as_two_step = Tracelet {
            overlap: tracelet.overlap.clone(),
            path_a: tracelet.path_a.clone(),
            path_b: tracelet.path_a.clone(),
            joins_at: tracelet.joins_at.clone(),
        };
        assert!(
            bool::from(replay_equivalent(&tracelet, &as_two_step, &store)),
            "and the replay oracle calls them the same transformation all the same"
        );
    }

    #[test]
    fn nf_equal_certificates_are_replay_equivalent()
    {
        let (store, tracelet) = fused_fixture();
        let twin = tracelet.clone();
        assert!(
            bool::from(tracelets_nf_equal(&store, &tracelet, &twin)),
            "the fast path certifies the pair"
        );
        assert!(
            bool::from(replay_equivalent(&tracelet, &twin, &store)),
            "and the replay oracle confirms it — the implication the fast path owes"
        );
    }

    #[test]
    fn a_unit_step_is_eliminated()
    {
        let mut store = CellStore::new();
        let reflexive = store.insert(reflexive_cell());
        let face = CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top);
        let path = alloc::vec![
            CellApp {
                cell: reflexive,
                at: Pos::root(),
            },
            CellApp {
                cell: reflexive,
                at: Pos::root(),
            },
        ];
        let normal =
            normalize(&store, &face, &face, &path).expect("a reflexive path reaches its own peak");
        assert!(
            normal.schedule.is_empty(),
            "both steps left the term unchanged, so unit elimination drops both"
        );
        assert!(
            normal.primitives.is_empty(),
            "and the factorization is the empty product"
        );
    }

    #[test]
    fn an_unknown_cell_identifier_is_refused()
    {
        let (store, tracelet) = fused_fixture();
        let missing = CellId(97);
        let path = alloc::vec![CellApp {
            cell: missing,
            at: Pos::root(),
        }];
        let refusal = normalize(&store, &tracelet.overlap.peak, &tracelet.joins_at, &path)
            .expect_err("an unresolvable identifier is refused");
        assert_eq!(
            NormalFormObstruction::UnknownCell { cell: missing },
            refusal,
            "the refusal names the identifier that resolved to nothing"
        );
    }

    #[test]
    fn a_step_that_does_not_fire_is_refused()
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let peak = CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top);
        let step = CellApp {
            cell: frame,
            at: Pos::root(),
        };
        let refusal = normalize(&store, &peak, &peak, core::slice::from_ref(&step))
            .expect_err("the frame cell has no redex at this peak");
        assert_eq!(
            NormalFormObstruction::StepDoesNotFire {
                step: Box::new(step)
            },
            refusal,
            "the refusal carries the step that did not fire"
        );
    }

    #[test]
    fn a_path_that_misses_its_join_is_refused()
    {
        let (store, tracelet) = fused_fixture();
        let elsewhere = CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top);
        let refusal = normalize(&store, &tracelet.overlap.peak, &elsewhere, &tracelet.path_a)
            .expect_err("the recorded path does not reach this join");
        let NormalFormObstruction::PathMissesTheJoin { reached } = refusal
        else {
            panic!("the join check is what refuses this path: {refusal:?}");
        };
        assert_eq!(
            SequentAlphabet::skolemize(&tracelet.joins_at),
            *reached,
            "the refusal carries the term the recorded path actually reached"
        );
    }

    #[test]
    fn the_shift_quotient_is_empty_over_the_sequent_alphabet()
    {
        // A sequent command pattern is one cut whose children are a producer
        // and a consumer, so the only command position is the root and two
        // applications can never be incomparable. The normal form's canonical
        // schedule is therefore the recorded order here, and the shift quotient
        // has to be exercised on an alphabet whose terms nest commands
        // (`tests/normal_form.rs`). This is a property of the alphabet, not a
        // defect of the quotient.
        let (store, tracelet) = fused_fixture();
        let normal = normalize(
            &store,
            &tracelet.overlap.peak,
            &tracelet.joins_at,
            &tracelet.path_a,
        )
        .expect("the two-step derivation normalizes");
        let recorded: Vec<PrimId> = tracelet
            .path_a
            .iter()
            .map(|step| {
                let cell = store.get(step.cell).expect("the step names a stored cell");
                prim_address(cell, &step.at)
            })
            .collect();
        assert_eq!(
            recorded, normal.schedule,
            "no transposition is licensed, so the canonical schedule is the recorded one"
        );
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            normal.convexity,
            "and the normal form carries the warrant it was taken under"
        );
    }

    #[test]
    fn the_content_address_is_taken_over_content()
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let frame_cell = store.get(frame).expect("the frame cell is stored");
        let add_cell = store.get(add).expect("add-S is stored");
        let root = Pos::root();
        let child = Pos::from_indices(alloc::vec![0_usize]);
        assert_eq!(
            prim_address(frame_cell, &root),
            prim_address(frame_cell, &root),
            "the address is a function of the content"
        );
        assert_ne!(
            prim_address(frame_cell, &root),
            prim_address(add_cell, &root),
            "cells with different content take different addresses"
        );
        assert_ne!(
            prim_address(frame_cell, &root),
            prim_address(frame_cell, &child),
            "and so do the same cell at different positions"
        );
        // The address is over content, not over the store index: an
        // independently built store hands the same cell a different id and the
        // same address.
        let mut other = CellStore::new();
        let _padding = other.insert(add_s());
        let elsewhere = other.insert(frame_defining_cell(&Sym::new("Succ")));
        assert_ne!(
            frame, elsewhere,
            "the two stores number the cell differently"
        );
        let elsewhere_cell = other.get(elsewhere).expect("the frame cell is stored");
        assert_eq!(
            prim_address(frame_cell, &root),
            prim_address(elsewhere_cell, &root),
            "and the content address is the same all the same"
        );
    }
}
