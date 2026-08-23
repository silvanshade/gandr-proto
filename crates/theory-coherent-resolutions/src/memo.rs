//! **In-process replay memoization** — per-step outcome reuse keyed by the
//! step's full support.
//!
//! A replay step does exactly one thing that costs anything: it fires one
//! resolved cell at one position on one input term
//! ([`rewrite_at`]). That call is a pure function of those three
//! values — it reads no store, carries no state across calls, and starts from a
//! fresh substitution — so its outcome may be reused whenever all three recur.
//! [`ReplayMemo`] is that reuse, and [`StepSupport`] is the support it keys on.
//!
//! # Why the key is resolved content, not a [`CellId`]
//!
//! A [`CellId`] is an insertion-order index into one store, so it is stable
//! only within one construction history ([`CellStore`]'s own contract). Keying
//! a memo by the identifier would make the memo agree with replay only for as
//! long as that assignment held, and a store permutation — which rebinds an
//! identifier to different content — would produce a **wrong hit**: the memo
//! would answer for content the engine would never have fired.
//!
//! Keying by the resolved cell's structural content removes the failure mode
//! rather than guarding against it. Append-only store growth preserves every
//! key, so unaffected steps reuse across appends; a permutation resolves each
//! identifier to different content, so every permuted step is a **wholesale
//! miss** and the engine answers afresh.
//!
//! # The memo carries no authority
//!
//! It is in-process only and never persists. A memo hit is a claim that this
//! process already ran this exact step, not a claim that the step is valid; the
//! moment a recorded outcome outlives the process that computed it, nothing
//! connects it to the engine that would have produced it. Replay stays
//! "replayed, not trusted", and the memo is an optimization strictly inside one
//! run.
//!
//! # Consulting the memo is opt-in
//!
//! [`crate::tracelet::Tracelet::replay`] is unchanged and consults nothing.
//! [`crate::tracelet::Tracelet::replay_memoized`] is the memoizing companion,
//! and a caller that wants reuse threads a memo through it explicitly.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::hash::Hash as _;
use core::hash::Hasher as _;

use gandr_theory_cell_complexes::alphabet::CellAlphabet;
use gandr_theory_cell_complexes::boundary::StepExecutionCount;
use gandr_theory_cell_complexes::boundary::StepReuseCount;
use gandr_theory_cell_complexes::cell::Cell;
use gandr_theory_cell_complexes::cell::CellId;
use gandr_theory_cell_complexes::cell::CellStore;
use gandr_theory_cell_complexes::sequent::SequentAlphabet;

use crate::rewrite::rewrite_at;

/// The **full support** of one replay step: everything the step's outcome is a
/// function of.
///
/// The three fields are exactly the arguments of
/// [`rewrite_at`], which is the whole of a replay step's work.
/// Two steps sharing a support have the same outcome, whatever store, whatever
/// certificate, and whichever path they occur on.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StepSupport<A: CellAlphabet = SequentAlphabet>
{
    /// The **resolved** cell's structural content — the cell itself, not the
    /// identifier the certificate recorded it under.
    pub cell: Cell<A>,
    /// The position the step fires at.
    pub at: A::Pos,
    /// The input term the step fires on.
    pub input: A::Cmd,
}

/// What one replay step did.
///
/// The refusal is memoized alongside the firing: a step that does not fire
/// costs the same match attempt as one that does, so recording only the
/// successes would leave the expensive half of the work unmemoized.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum StepOutcome<A: CellAlphabet = SequentAlphabet>
{
    /// The cell fired, rewriting the input into this whole term.
    Fired(A::Cmd),
    /// The cell did not fire — no redex at the position, or the alphabet's
    /// firing discipline refused it.
    Refused,
}

/// Whether [`ReplayMemo::poison`] found the support it was asked to overwrite.
///
/// The seam exists for the poisoned-memo mutation check, which is what proves
/// the incremental-equals-batch differential has teeth: a differential that
/// cannot tell a corrupted memo from an honest one is not testing the memo.
///
/// Corrupting a memo is not an operation the crate offers its callers, so the
/// seam exists only under `cfg(test)` and is exercised by the in-crate replay
/// differential rather than reachable from outside.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum MemoPoisonOutcome
{
    /// The support was memoized and its outcome now reads as supplied.
    Poisoned,
    /// The support was not memoized, so nothing was overwritten.
    NotMemoized,
}

/// A deterministic 64-bit digest of a [`StepSupport`], used to bucket the memo.
///
/// The digest never decides a hit on its own: a bucket holds the full supports
/// that landed in it, and a lookup compares them structurally. A collision
/// therefore costs a comparison and degrades to a miss; it can never produce a
/// wrong answer.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct StepSupportDigest(u64);

/// The supports that share one digest, each with the outcome recorded for it.
///
/// A bucket is normally a single entry; it holds more only when two distinct
/// supports digest alike, which the structural comparison then separates.
type MemoBucket<A> = Vec<(StepSupport<A>, StepOutcome<A>)>;

/// An **in-process replay memo**: step outcomes indexed by their full support.
///
/// The map is keyed by digest and bucketed by full support, because the
/// alphabet's associated types are bounded [`Eq`] and [`core::hash::Hash`] but
/// not [`Ord`], so the supports themselves cannot order a tree map. Bucket
/// iteration is over an ordered map of digests, so a memo's behavior does not
/// depend on hash iteration order.
///
/// A memo is bound to no store and no certificate: it may be threaded across
/// replays of different tracelets against different stores, and a support that
/// recurs is reused wherever it recurs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReplayMemo<A: CellAlphabet = SequentAlphabet>
{
    /// The memoized outcomes, bucketed by support digest.
    entries: BTreeMap<StepSupportDigest, MemoBucket<A>>,
    /// How many steps were executed against the engine.
    executed: StepExecutionCount,
    /// How many steps were answered from the memo.
    reused: StepReuseCount,
}

impl<A: CellAlphabet> ReplayMemo<A>
{
    /// An empty memo, having executed and reused nothing.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// The number of steps this memo executed against the rewriting engine.
    ///
    /// Together with [`ReplayMemo::steps_reused`] this is the reuse
    /// measurement: the two counts partition every step consultation the memo
    /// answered, so their sum is the work a non-memoized replay would have
    /// done.
    #[inline]
    #[must_use]
    pub fn steps_executed(&self) -> StepExecutionCount
    {
        self.executed
    }

    /// The number of steps this memo answered without consulting the engine.
    #[inline]
    #[must_use]
    pub fn steps_reused(&self) -> StepReuseCount
    {
        self.reused
    }

    /// The memoized supports and their outcomes, in digest order.
    ///
    /// # Contract
    /// - ensures: one entry per distinct memoized support, in an order fixed by
    ///   the support digests rather than by insertion or by hash iteration.
    /// - panics: none.
    #[inline]
    pub fn entries(&self) -> impl Iterator<Item = (&StepSupport<A>, &StepOutcome<A>)>
    {
        self.entries
            .values()
            .flat_map(|bucket| bucket.iter().map(|entry| (&entry.0, &entry.1)))
    }

    /// Answer one replay step, from the memo where its support recurs and from
    /// the engine otherwise.
    ///
    /// # Contract
    /// - ensures: the returned outcome equals `rewrite_at(cell, input, at)`
    ///   read as [`StepOutcome`], for every call, whether answered from the
    ///   memo or from the engine — the memo changes what is computed, never
    ///   what is answered.
    /// - ensures: a call whose support was not memoized executes the step,
    ///   records it, and advances [`ReplayMemo::steps_executed`]; a call whose
    ///   support was memoized advances [`ReplayMemo::steps_reused`] and
    ///   executes nothing.
    /// - provides: the reuse the memoized replay path is built out of.
    /// - panics: none.
    /// - intension: a lookup hashes the support and compares the full supports
    ///   in the digest's bucket; only a miss clones the support into the memo,
    ///   so a hit allocates nothing beyond the returned outcome.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — a repeated support and a fresh support
    ///   separate the reuse and execution paths, and the memoized and fresh
    ///   replay verdicts agree over the whole composition corpus while a
    ///   poisoned entry makes them disagree.
    /// - witness: `tracelet::tests::a_poisoned_memo_entry_makes_the_memoized_verdict_disagree`
    /// - witness: `tracelet::tests::a_poisoned_refusal_licenses_a_step_the_engine_refuses`
    /// - witness: `replay_memo::tests::replaying_one_tracelet_twice_reuses_every_step`
    /// - witness: `replay_memo::tests::the_composition_corpus_agrees_with_and_without_the_memo`
    #[inline]
    pub fn resolve(
        &mut self,
        cell: &Cell<A>,
        at: &A::Pos,
        input: &A::Cmd,
    ) -> StepOutcome<A>
    {
        let digest = digest_of(cell, at, input);
        let memoized = self.entries.get(&digest).and_then(|bucket| {
            bucket
                .iter()
                .find(|entry| entry.0.cell == *cell && entry.0.at == *at && entry.0.input == *input)
                .map(|entry| entry.1.clone())
        });
        if let Some(outcome) = memoized {
            self.reused = StepReuseCount::from(usize::from(self.reused).saturating_add(1));
            return outcome;
        }
        let outcome = match rewrite_at(cell, input, at) {
            | Some(result) => StepOutcome::Fired(result),
            | None => StepOutcome::Refused,
        };
        self.executed = StepExecutionCount::from(usize::from(self.executed).saturating_add(1));
        let support = StepSupport {
            cell: cell.clone(),
            at: at.clone(),
            input: input.clone(),
        };
        self.entries
            .entry(digest)
            .or_default()
            .push((support, outcome.clone()));
        outcome
    }

    /// Overwrite the outcome recorded for `support` — the poisoned-memo seam.
    ///
    /// # Contract
    /// - ensures: [`MemoPoisonOutcome::Poisoned`] with `support`'s recorded
    ///   outcome replaced by `replacement` when the support is memoized;
    ///   [`MemoPoisonOutcome::NotMemoized`] leaving the memo unchanged
    ///   otherwise.
    /// - provides: the corruption the differential must catch, so that the
    ///   differential's agreement is evidence rather than a tautology.
    /// - panics: none.
    #[cfg(test)]
    #[inline]
    pub(crate) fn poison(
        &mut self,
        support: &StepSupport<A>,
        replacement: StepOutcome<A>,
    ) -> MemoPoisonOutcome
    {
        let digest = digest_of(&support.cell, &support.at, &support.input);
        let Some(bucket) = self.entries.get_mut(&digest)
        else {
            return MemoPoisonOutcome::NotMemoized;
        };
        let Some(entry) = bucket.iter_mut().find(|entry| entry.0 == *support)
        else {
            return MemoPoisonOutcome::NotMemoized;
        };
        entry.1 = replacement;
        MemoPoisonOutcome::Poisoned
    }
}

/// Resolve one recorded step's cell and answer it through `memo`.
///
/// A step naming a cell the store does not hold has **no resolved content**, so
/// there is nothing to key a memo entry on; it is reported unresolved and left
/// to the caller, exactly as the non-memoized path reports it stuck.
///
/// # Contract
/// - ensures: `Some(outcome)` agreeing with `rewrite_at` on the resolved cell
///   when `store` holds `cell`; `None` when it does not.
/// - panics: none.
#[inline]
pub(crate) fn resolve_step<A>(
    store: &CellStore<A>,
    memo: &mut ReplayMemo<A>,
    cell: CellId,
    at: &A::Pos,
    input: &A::Cmd,
) -> Option<StepOutcome<A>>
where
    A: CellAlphabet,
{
    let resolved = store.get(cell)?;
    Some(memo.resolve(resolved, at, input))
}

/// The deterministic digest of one step support.
///
/// # Contract
/// - ensures: equal supports digest equally; the value depends on no randomized
///   seed, so a memo's bucket order is the same across runs.
/// - panics: none.
#[inline]
fn digest_of<A>(
    cell: &Cell<A>,
    at: &A::Pos,
    input: &A::Cmd,
) -> StepSupportDigest
where
    A: CellAlphabet,
{
    let mut hasher = SupportHasher::new();
    cell.hash(&mut hasher);
    at.hash(&mut hasher);
    input.hash(&mut hasher);
    StepSupportDigest(hasher.finish())
}

/// A deterministic FNV-1a hasher, so support digests are stable across runs —
/// unlike the standard library's randomized default hasher, whose per-process
/// seed would make the memo's bucket order vary run to run.
#[repr(transparent)]
struct SupportHasher
{
    /// The running 64-bit FNV-1a state.
    state: u64,
}

impl SupportHasher
{
    /// The FNV-1a 64-bit offset basis.
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    /// The FNV-1a 64-bit prime.
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    /// A fresh hasher seeded at the FNV offset basis.
    #[inline]
    fn new() -> Self
    {
        Self {
            state: Self::OFFSET_BASIS,
        }
    }
}

impl core::hash::Hasher for SupportHasher
{
    #[inline]
    fn finish(&self) -> u64
    {
        self.state
    }

    #[inline]
    fn write(
        &mut self,
        bytes: &[u8],
    )
    {
        let mut state = self.state;
        for &byte in bytes {
            state ^= u64::from(byte);
            state = state.wrapping_mul(Self::PRIME);
        }
        self.state = state;
    }
}
