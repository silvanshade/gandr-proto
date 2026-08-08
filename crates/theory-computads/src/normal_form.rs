//! The **tracelet normal form** — a canonical form on certificate data, and a
//! decidable **sound under-approximation** of replay-equality.
//!
//! Research reference: N. Behr & J. Kock, "Tracelet Hopf Algebras and
//! Decomposition Spaces (Extended Abstract)", EPTCS 372, 323–337, DOI
//! `10.4204/EPTCS.372.23` — register key `behr-kock-2021-tracelet-hopf` in
//! `docs/gandr/spec/bibliography.yml`. The shift quotient of the tracelet
//! algebra is free symmetric monoidal on primitives — **statement-level locator
//! pending**: the register key resolves and the work is the right one, but the
//! numbered statement this sentence paraphrases is not yet cited here, so the
//! attribution is to the paper and not to a checked lemma. It is in any case
//! the statement this module reads *semantically*: a derivation factors
//! uniquely into content-addressed primitives carrying integer multiplicities,
//! scheduled canonically. Nothing
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
//!   A [`CellStore`] deduplicating on structural equality makes that refusal
//!   unreachable over both shipped alphabets — one content, one identifier —
//!   but it does not make the arm dead: an alphabet with a field the digest
//!   cannot see gives two identifiers one address without any 128-bit collision
//!   at all, which is how the arm is witnessed.
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
//! Neither refusal is reachable over the alphabets this workspace ships, and
//! that is a fact about those alphabets rather than evidence that the check is
//! redundant: both compute `position_order` from the position path and both
//! splice locally, so their canonical schedules always fire and always land on
//! the recorded join. Both arms are nonetheless *witnessed*, over adversarial
//! alphabets built for it in the crate's integration suite — one that reports a
//! nesting pair as incomparable, and one whose splice disturbs a position it
//! was not asked about. The second is the one worth remembering: it satisfies
//! every conjunct of the guard honestly, because the guard reads positions and
//! cell contents and **locality of the term algebra is neither**. That premise
//! is [`CellAlphabet::splice_cmd_at`]'s own `- ensures:`, it is taken on trust
//! here, and this replay is what catches its violation.
//!
//! **Observable at [`normalize`], and only there.** The certificate-level
//! entry point [`tracelets_nf_equal`] collapses *every* obstruction to a
//! negative answer, kill signals included, because its return type has nowhere
//! to put one. That is the safe direction for the equality and the wrong
//! direction for the signal, so a consumer that wants the signal calls
//! [`normalize`] and reads the [`NormalFormObstruction`] — a fast path built on
//! [`tracelets_nf_equal`] alone would silently degrade a kill signal into a
//! cache miss.
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
/// A value **[`normalize`] returns** is a replay receipt as well as a canonical
/// form: it exists because `normalize` ran the derivation and confirmed both
/// the recorded path and the canonical schedule reach the recorded join.
///
/// That is a property of `normalize`'s outputs, **not an invariant of this
/// type**: the fields are public and the struct is `Clone`, so a value can be
/// assembled — or edited after the fact — without any replay having happened.
/// The receipt property is therefore carried as [`nf_equal`]'s `- requires:`
/// clause, which a caller must discharge by provenance. Anything that treats a
/// [`TraceletNf`] it did not obtain from [`normalize`] as evidence is trusting
/// its own bookkeeping, not this module's.
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
///   [`normalize`] refuses a collision outright. The digest is stable for **one
///   build of one target** and no further: [`core::hash::Hasher`]'s integer
///   writers encode native-endian and `usize` at the target's width, so the
///   same `(cell content, position)` pair digests differently on targets that
///   differ in either. Within a process every consumer sees one address per
///   content, which is all the factorization's ordering and [`nf_equal`] need;
///   a [`TraceletNf`] is correspondingly a *comparable* value, never a portable
///   identifier, and nothing may persist or transport one as though it were.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the decision surface is "same content, same
///   address; different content, different address", separated by a cell pair
///   differing only in its right-hand side, a position pair differing only in
///   one step, and an equal pair rebuilt independently. The intension's choice
///   of *which* deterministic digest is deliberately unwitnessed: no declared
///   projection exposes it, so retuning it is a noninterfering change.
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
///   path is replayed and must reach its own join). **All six** `- fails:`
///   modes own a decision surface separated by a fixture that triggers only it.
///   Three are reached over a shipped alphabet: a fabricated identifier, a step
///   at a position carrying no redex, and a retargeted join.
/// - hypothesis: the other three are reached only over an **adversarial
///   alphabet**, because each fires exactly when an implementor of
///   [`CellAlphabet`] gives an answer the two shipped alphabets cannot give,
///   and the fix for that gap is an input rather than an assertion. The two
///   kill-signal modes fire when the independence relation licenses a
///   transposition the semantics does not have: an alphabet reporting a nesting
///   pair as incomparable reaches
///   [`NormalFormObstruction::ShiftedScheduleDoesNotFire`], and an alphabet
///   whose splice is non-local — the one defect no conjunct of the guard can
///   see, since the guard reads positions and cell contents and neither records
///   locality — reaches
///   [`NormalFormObstruction::ShiftedScheduleMissesTheJoin`].
///   [`NormalFormObstruction::ContentAddressCollision`] needs two occurrences
///   sharing an address under differing [`PrimCert`]s, which a [`CellStore`]
///   deduplicating on structural equality withholds *only while every field of
///   a cell is inside the digest's reach*: an alphabet whose orientation tag
///   hashes to nothing gives one address to two identifiers, legally, since
///   [`core::hash::Hash`] never promises injectivity and this function's `-
///   intension:` says as much. A schedule permutation over a shipped alphabet
///   exercises the *success* path of the canonicalization and never its
///   refusal, so none of these three is reachable by adding a fixture over the
///   sequent or toy alphabet.
/// - witness: `normal_form::tests::a_recorded_derivation_normalizes_to_a_replay_receipt`
/// - witness: `normal_form::tests::an_unknown_cell_identifier_is_refused`
/// - witness: `normal_form::tests::a_step_that_does_not_fire_is_refused`
/// - witness: `normal_form::tests::a_path_that_misses_its_join_is_refused`
/// - witness: `normal_form::tests::a_unit_step_is_eliminated`
/// - witness: `normal_form::tests::the_canonical_path_replays_to_the_recorded_join`
/// - witness: `normal_form::tests::the_shift_quotient_is_empty_over_the_sequent_alphabet`
/// - witness: `normal_form::tests::a_derivation_from_a_different_peak_is_nf_distinct`
/// - witness: `normal_form::tests::an_alphabet_that_calls_nesting_incomparable_trips_the_kill_signal`
/// - witness: `normal_form::tests::a_non_local_term_algebra_trips_the_kill_signal_at_the_join`
/// - witness: `normal_form::tests::two_primitives_sharing_a_content_address_are_refused_rather_than_merged`
/// - witness: `normal_form::tests::a_withheld_convexity_warrant_empties_the_shift_quotient`
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
///   as uninformative by a pair that is replay-equal and NF-distinct. The
///   residue is *which* fields carry the decision: the boundary is separated by
///   an erasing rule that carries two peaks to one join under one schedule, so
///   dropping the peak comparison becomes unsound rather than merely imprecise.
///   Dropping the join, the factorization, or the convexity warrant instead is
///   **not** separable and is not claimed to be: against one store the peak and
///   the schedule already determine the first two, and the third is one value
///   per store because [`CellAlphabet::convexity_discharge`] is keyed by the
///   store alone — so all three comparisons are redundant belt-and-braces and
///   their removal is an equivalent mutation.
/// - witness: `normal_form::tests::one_derivation_is_nf_equal_to_itself`
/// - witness: `normal_form::tests::replay_equal_derivations_may_be_nf_distinct`
/// - witness: `normal_form::tests::a_derivation_from_a_different_peak_is_nf_distinct`
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
///   keeps the normal forms. **Every [`NormalFormObstruction`] any of the four
///   normalizations raises is collapsed to a negative** — the two kill-signal
///   variants included, because a [`NormalFormEquality`] has nowhere to put
///   one. Collapsing toward the negative is what keeps the equality safe, and
///   it is what makes this the wrong entry point for observing the kill signal:
///   a consumer that must not lose it calls [`normalize`] and reads the
///   obstruction.
///
/// # Adequacy
/// - hypothesis: L2 agreement — the oracle is external
///   ([`crate::tracelet::replay_equivalent`]) and the differential asserts the
///   implication over generated derivation pairs rather than over hand-picked
///   ones. The L3 residue is the two negative directions the generator cannot
///   reach, because it only ever generates certificates that normalize and
///   agree: the *direction* an obstruction collapses in, separated by a
///   certificate whose recorded step does not fire, and the conjunction over
///   both legs, separated by a pair that agrees on `path_a` and not on
///   `path_b`.
/// - witness: `normal_form::tests::nf_equal_certificates_are_replay_equivalent`
/// - witness: `normal_form::tests::every_nf_equal_pair_is_replay_equivalent`
/// - witness: `normal_form::tests::a_certificate_that_does_not_replay_is_not_certified`
/// - witness: `normal_form::tests::a_tracelet_pair_agreeing_only_on_its_first_leg_is_not_certified`
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
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the unit test `next == current` is the whole
///   decision surface, separated by a reflexive cell that fires and moves
///   nothing beside a cell that moves the term, on both alphabets; the two `-
///   fails:` modes are separated by a fabricated identifier and a position
///   carrying no redex.
/// - witness: `normal_form::tests::a_unit_step_is_eliminated`
/// - witness: `normal_form::tests::an_unknown_cell_identifier_is_refused`
/// - witness: `normal_form::tests::a_step_that_does_not_fire_is_refused`
/// - witness: `normal_form::tests::a_unit_step_is_eliminated_over_the_toy_alphabet`
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
///
/// # Adequacy
/// - hypothesis: L3 pointwise, and the decision surface is reachable **only
///   over an adversarial alphabet**. On an alphabet whose `position_order` is
///   honest the canonical schedule always fires, so no fixture over a shipped
///   alphabet reaches [`NormalFormObstruction::ShiftedScheduleDoesNotFire`] and
///   this whole call could be deleted without any such test noticing. The
///   witnesses below supply that alphabet, and they assert the exact variant
///   rather than a refusal: swapping this arm for
///   [`NormalFormObstruction::StepDoesNotFire`] — degrading the kill signal to
///   an ordinary bad-certificate refusal, which is the confusion the two
///   functions are kept apart to prevent — fails them.
/// - witness: `normal_form::tests::an_alphabet_that_calls_nesting_incomparable_trips_the_kill_signal`
/// - witness: `normal_form::tests::a_non_local_term_algebra_trips_the_kill_signal_at_the_join`
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
///
/// # Adequacy
/// - hypothesis: L3 pointwise, and it needs **three** layers to bite. The
///   recurrence has two residues — "maximum over every earlier dependence"
///   versus the nearest or the first one, and the `1 + ` on the prior depth —
///   and a two-layer derivation separates neither, because with one dependence
///   apiece every variant agrees. The separating fixture is a step whose
///   nearest and first earlier dependences both sit at depth zero while its
///   deepest sits at depth one, arranged so the collapsed layering emits a
///   schedule that cannot fire. The tie-break arm is separated by a layer with
///   two occupants, whose declared ascending address order is observable in
///   [`TraceletNf::schedule`].
/// - hypothesis: the recurrence's remaining residue is that a depth taken from
///   the earlier step's *position index* instead of its depth is still a valid
///   topological layering — it increases strictly along every dependence edge,
///   so it fires and reaches the join — and therefore survives every fixture
///   whose dependence order is a single chain. It is separated not by
///   inspecting a depth but by **shift-invariance across two recorded orders**:
///   over two interleaved chains (`x` depending on `a`, `y` on `b`, with `a`,
///   `b` independent and `x`, `y` independent) the index-based variant gives
///   the two recorded orders of one trace class two different schedules, while
///   the depth recurrence gives them one. So the flattened schedule *can* carry
///   this residue after all, and what stays unobservable is narrower than the
///   whole class: only a depth assignment that flattens to the same sequence
///   for **every** recorded order. [`CausalDepth`] is still computed and never
///   surfaced — the schedule flattens the layering — so a layer-grouped
///   schedule remains the better projection for a consumer that wants the
///   parallel batches, but it is not what this residue needed.
/// - hypothesis: two mutations of the loop are **equivalent (semantic)** and
///   are excluded here rather than tested. Sorting with
///   [`slice::sort_unstable_by`] instead of [`slice::sort_by`] changes nothing
///   observable, because the key is total by the `- ensures:` argument above
///   and a total key leaves stability nothing to decide. Extending the inner
///   range to include the step itself also changes nothing: a step is always
///   dependent on itself (its position order with itself is
///   [`crate::alphabet::PositionOrder::Same`], never `Incomparable`) and its
///   own depth is not yet recorded, so the self-comparison contributes exactly
///   `1` and the recurrence's unique solution shifts from `d` to `d + 1`
///   uniformly — an order-preserving relabelling of the layers.
/// - witness: `normal_form::tests::a_layered_derivation_keeps_its_dependent_step_last`
/// - witness: `normal_form::tests::a_three_layer_derivation_orders_each_layer_by_content_address`
/// - witness: `normal_form::tests::two_interleaved_dependence_chains_layer_by_depth_and_not_by_position`
/// - witness: `normal_form::tests::a_shuffled_independent_schedule_has_one_normal_form`
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
/// The relation is **symmetric**, which the causal layering depends on and
/// which is a fact about [`check_shift_guard`] rather than an assumption here:
/// its first conjunct answers [`crate::alphabet::PositionOrder::Incomparable`]
/// for a pair exactly when it does for the swapped pair, its second asks the
/// overlap enumerator in *both* ordered directions, and its third does not read
/// the pair at all. Were it asymmetric, a licensed transposition could change
/// which pairs count as dependent and the layering's depths would stop being a
/// property of the derivation.
///
/// # Contract
/// - ensures: a positive answer exactly when the guard's three conjuncts hold
///   of the pair under `convexity`; the answer does not depend on which of the
///   two steps is passed as `left`.
/// - provides: the independence relation the causal layering quotients by.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the relation is not restated here, so its
///   conjuncts are witnessed at [`check_shift_guard`]'s own suite; what this
///   wrapper adds is the direction of the collapse, separated by an overlapping
///   pair at incomparable positions whose two orders demonstrably reach one
///   term and which the quotient still refuses. The third conjunct's
///   contribution reaches this wrapper only over an alphabet that withholds the
///   warrant, which is a missing input rather than a missing assertion: both
///   shipped alphabets discharge convexity for every store, so nothing else can
///   tell whether [`normalize`] asks the alphabet, layers under the answer it
///   got, and records the warrant it used. Symmetry is **not** separated by any
///   fixture and is not claimed to be: swapping the arguments is an equivalent
///   mutation, because the guard is symmetric.
/// - hypothesis: the relation's soundness is conditional on a premise no
///   conjunct checks and this wrapper cannot see — that the alphabet's term
///   algebra is **local**, so a rewrite at one position leaves every
///   incomparable position alone ([`CellAlphabet::splice_cmd_at`]'s own `-
///   ensures:`). Two applications can satisfy all three conjuncts honestly and
///   still fail to commute if that clause is broken, and the only thing
///   standing between such an alphabet and a wrong identification is
///   [`normalize`]'s replay of its own canonical schedule. That is the sharpest
///   statement of why the replay is not redundant, and it has a witness.
/// - witness: `normal_form::tests::an_overlapping_pair_keeps_its_recorded_order`
/// - witness: `normal_form::tests::a_layered_derivation_keeps_its_dependent_step_last`
/// - witness: `normal_form::tests::a_withheld_convexity_warrant_empties_the_shift_quotient`
/// - witness: `normal_form::tests::a_non_local_term_algebra_trips_the_kill_signal_at_the_join`
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
