//! The **check-memo vocabulary**: a statically dispatched seam letting a
//! checker skip a question it has already answered in this process, with the
//! storage and the policy owned outside the checker.
//!
//! # Why the vocabulary is dependency-free and generic
//!
//! This crate names no term, type, or identifier of its own. The support a
//! checker keys on and the outcome it caches are both **type parameters**,
//! supplied by the consumer, exactly as the conversion-decision vocabulary
//! carries a consumer-owned identifier rather than a term. Two things follow.
//! The certified kernel can depend on this crate without a cycle, since this
//! crate cannot mention the kernel's types. And the table's bytes, its
//! eviction, and its lifetime live here rather than inside the checker — so
//! what the checker holds is a seam, not a cache.
//!
//! # What a memo hit claims, and what it does not
//!
//! A hit claims exactly this: **this process already computed this answer for
//! this support**. It does not claim the answer is right, that the support was
//! well formed, or that anything was validated. A memo is sound only when its
//! consumer's support is the *whole* input to the computation it indexes — if
//! two calls with equal supports could differ, the memo is a defect and no
//! property of this crate can rescue it. The consumer owns that argument.
//!
//! Nothing here persists, and nothing here is a wire format. A memo is a
//! process-local accelerator whose whole justification is that recomputing
//! would produce the same answer.
//!
//! # The static-dispatch discipline
//!
//! [`CheckMemo`] carries an associated [`MemoActivity`] constant so a consumer
//! can branch on liveness at **compile** time. Instantiated at [`NullMemo`],
//! every memo interaction — the support construction included, when the
//! consumer guards it — is a constant-false branch that monomorphization
//! removes, so the unmemoized path keeps the code it had before the seam
//! existed. That is what makes the memoized and unmemoized paths comparable:
//! the differential's fresh side is not a re-implementation, it is the same
//! function at a different type parameter.

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;

/// Whether a [`CheckMemo`] implementation ever answers.
///
/// Read at compile time through [`CheckMemo::ACTIVITY`], so a consumer's
/// memo-handling branch costs nothing when the memo is inactive. A consumer
/// matches on it directly — `matches!(M::ACTIVITY, MemoActivity::Active)` —
/// rather than through a predicate, so the guard stays a constant the
/// optimizer folds away and no boolean crosses an interface.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoActivity
{
    /// The memo may answer, so the consumer builds supports and consults it.
    Active,
    /// The memo never answers, so the consumer may skip the whole interaction.
    Inactive,
}

/// A process-local store of already-computed outcomes, keyed by support.
///
/// # Contract
/// - requires: the consumer's `Support` is the **complete** input to the
///   computation the outcome indexes — equal supports must imply equal
///   outcomes, or the memo changes answers.
/// - ensures: [`CheckMemo::recall`] returns only what [`CheckMemo::remember`]
///   was given for an equal support.
/// - provides: the checker's skip-a-repeated-question seam.
/// - fails: never — a miss is `None`, not an error.
/// - panics: none.
pub trait CheckMemo<Support, Outcome>
{
    /// Whether this implementation ever answers, known at compile time.
    const ACTIVITY: MemoActivity;

    /// The outcome already recorded for `support`, if any.
    fn recall(
        &self,
        support: &Support,
    ) -> Option<Outcome>;

    /// Record `outcome` as the answer for `support`, replacing any prior one.
    fn remember(
        &mut self,
        support: Support,
        outcome: Outcome,
    );
}

/// The memo that never answers: the unmemoized path, as a type parameter.
///
/// Zero-sized, and every method is a constant, so a consumer instantiated here
/// compiles to the code it would have had with no memo at all. This is the
/// differential's fresh side — the same checker, not a second one.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NullMemo;

impl<Support, Outcome> CheckMemo<Support, Outcome> for NullMemo
{
    const ACTIVITY: MemoActivity = MemoActivity::Inactive;

    #[inline]
    fn recall(
        &self,
        _support: &Support,
    ) -> Option<Outcome>
    {
        None
    }

    #[inline]
    fn remember(
        &mut self,
        _support: Support,
        _outcome: Outcome,
    )
    {
    }
}

/// How many supports a [`VerdictMemo`] holds.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MemoEntryCount(usize);

impl From<usize> for MemoEntryCount
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<MemoEntryCount> for usize
{
    #[inline]
    fn from(value: MemoEntryCount) -> Self
    {
        value.0
    }
}

/// An ordered in-memory memo: the storage half, owned here so the checker
/// holds none of it.
///
/// Ordered rather than hashed on purpose. The key is a consumer-owned support
/// whose ordering the consumer already defines, no hasher enters the kernel's
/// dependency wall, and iteration order is deterministic — which is what makes
/// a measurement over a memo re-derivable rather than run-dependent.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VerdictMemo<Support, Outcome>
{
    /// The recorded answers, ordered by support.
    entries: BTreeMap<Support, Outcome>,
}

impl<Support, Outcome> VerdictMemo<Support, Outcome>
where
    Support: Ord,
{
    /// An empty memo.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// How many supports are recorded.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: the number of distinct supports [`CheckMemo::remember`] has
    ///   been called with.
    /// - provides: the measurement surface — distinct answered questions, as
    ///   against total questions asked.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn entry_count(&self) -> MemoEntryCount
    {
        MemoEntryCount::from(self.entries.len())
    }

    /// Every recorded support, in support order.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: each support [`CheckMemo::remember`] recorded, once, in the
    ///   ordering the support type defines.
    /// - provides: the measurement surface a consumer needs to count entries
    ///   **per kind** — an aggregate count cannot tell a live memo from one
    ///   whose second half never fired.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    pub fn supports(&self) -> impl Iterator<Item = &Support>
    {
        self.entries.keys()
    }
}

impl<Support, Outcome> CheckMemo<Support, Outcome> for VerdictMemo<Support, Outcome>
where
    Support: Ord,
    Outcome: Clone,
{
    const ACTIVITY: MemoActivity = MemoActivity::Active;

    #[inline]
    fn recall(
        &self,
        support: &Support,
    ) -> Option<Outcome>
    {
        self.entries.get(support).cloned()
    }

    #[inline]
    fn remember(
        &mut self,
        support: Support,
        outcome: Outcome,
    )
    {
        let _replaced = self.entries.insert(support, outcome);
    }
}

#[cfg(test)]
mod tests
{
    use super::CheckMemo;
    use super::MemoActivity;
    use super::MemoEntryCount;
    use super::NullMemo;
    use super::VerdictMemo;

    /// A support standing in for a consumer's own key type.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct Support(u8);

    /// An outcome standing in for a consumer's own verdict type.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Outcome(u8);

    #[test]
    fn the_null_memo_never_answers()
    {
        let mut memo = NullMemo;
        CheckMemo::remember(&mut memo, Support(1), Outcome(9));
        assert_eq!(
            None,
            CheckMemo::<Support, Outcome>::recall(&memo, &Support(1)),
            "the null memo forgets what it was told, which is what makes it the fresh side"
        );
        assert_eq!(
            MemoActivity::Inactive,
            <NullMemo as CheckMemo<Support, Outcome>>::ACTIVITY,
            "the null memo declares itself inactive so the consumer's branch is constant"
        );
        assert!(
            matches!(
                <VerdictMemo<Support, Outcome> as CheckMemo<Support, Outcome>>::ACTIVITY,
                MemoActivity::Active
            ),
            "and the storing memo declares itself active, so the two are distinguishable"
        );
    }

    #[test]
    fn a_verdict_memo_returns_what_it_was_told()
    {
        let mut memo: VerdictMemo<Support, Outcome> = VerdictMemo::new();
        assert_eq!(
            MemoEntryCount::from(0),
            memo.entry_count(),
            "a fresh memo holds nothing"
        );
        memo.remember(Support(1), Outcome(9));
        memo.remember(Support(2), Outcome(8));
        assert_eq!(
            Some(Outcome(9)),
            memo.recall(&Support(1)),
            "the first answer"
        );
        assert_eq!(
            Some(Outcome(8)),
            memo.recall(&Support(2)),
            "the second answer"
        );
        assert_eq!(None, memo.recall(&Support(3)), "an unasked support misses");
        assert_eq!(
            MemoEntryCount::from(2),
            memo.entry_count(),
            "two distinct supports were answered"
        );
    }

    #[test]
    fn remembering_a_support_twice_replaces_rather_than_accumulates()
    {
        let mut memo: VerdictMemo<Support, Outcome> = VerdictMemo::new();
        memo.remember(Support(1), Outcome(9));
        memo.remember(Support(1), Outcome(7));
        assert_eq!(
            Some(Outcome(7)),
            memo.recall(&Support(1)),
            "the later answer stands, so a consumer cannot grow two answers for one support"
        );
        assert_eq!(
            MemoEntryCount::from(1),
            memo.entry_count(),
            "one support, one entry"
        );
    }
}
