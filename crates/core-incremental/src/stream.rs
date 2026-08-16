//! Ordered, resumable synthesis events over validated checkpoints.

use alloc::vec::Vec;

use crate::boundary::AdoptionDecision;
use crate::checkpoint::ItemTyping;
use crate::checkpoint::Resume;

/// How one branch of a match stands against the scrutinee it was checked
/// against — the three-valued verdict a scrutinee containing holes forces.
///
/// A two-valued verdict cannot be given here without lying. A scrutinee that is
/// (or contains) an unfilled hole makes some branches undecidable, and both
/// two-valued answers are wrong for them: calling such a branch refuted hides a
/// match the finished program will take, and calling it satisfied claims one it
/// may not. [`BranchStatus::Possibly`] is the honest third answer, and it is
/// what keeps checking from blocking on an unfinished program.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BranchStatus
{
    /// The branch matches the scrutinee, whatever any hole is later filled
    /// with.
    Satisfied,
    /// The branch may or may not match: some hole decides it.
    Possibly,
    /// The branch cannot match, whatever any hole is later filled with.
    Refuted,
}

/// The per-branch verdicts of one match expression, in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchLiveness
{
    /// Source-order index of the match within its item.
    pub match_index: usize,
    /// One status per branch, in source order.
    pub branches: Vec<BranchStatus>,
}

/// One item's match liveness, addressed by the item's source-order index in the
/// program the resume validated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemLiveness
{
    /// Source-order index of the item within the program.
    pub index: usize,
    /// The item's matches, in source order.
    pub matches: Vec<MatchLiveness>,
}

/// One observable unit emitted by the synthesis stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SynthesisEvent
{
    /// Marks the beginning of a stream for a fixed item count.
    Started
    {
        /// Number of items in the stream.
        item_count: usize,
    },
    /// Carries one item typing and whether its checkpoint was reused.
    Item
    {
        /// Source-order index.
        index: usize,
        /// Validated typing result.
        typing: ItemTyping,
        /// Whether the prior checkpoint was adopted.
        adopted: AdoptionDecision,
    },
    /// Carries one match's per-branch liveness, immediately after the
    /// [`SynthesisEvent::Item`] of the item that owns it.
    ///
    /// This is the stream's window onto a **stuck** match: one whose scrutinee
    /// holds a hole, so no branch can be committed to. Its branches are
    /// published rather than its result, because there is no result yet — and a
    /// consumer that must show what the program is waiting on needs exactly the
    /// per-branch verdicts.
    Match
    {
        /// Source-order index of the owning item.
        index: usize,
        /// Source-order index of the match within that item.
        match_index: usize,
        /// One status per branch, in source order.
        branches: Vec<BranchStatus>,
    },
    /// Marks successful completion after all items were emitted.
    Completed,
}

/// A deterministic stream that owns no input or output accumulation.
#[derive(Clone, Debug)]
pub struct SynthesisStream
{
    /// Ordered synthesis events.
    events: Vec<SynthesisEvent>,
    /// Next event cursor.
    cursor: usize,
}

impl SynthesisStream
{
    /// Creates a stream from one validated resume result.
    #[must_use]
    #[inline]
    pub fn from_resume(resume: &Resume) -> Self
    {
        Self::from_resume_with_liveness(resume, &[])
    }

    /// Creates a stream from one validated resume result, interleaving each
    /// item's match liveness after that item's own event.
    ///
    /// Liveness is addressed by item index rather than positionally, so a
    /// caller that analyzed only part of the program — the items it just
    /// lowered, say — passes only those entries and the rest of the stream is
    /// unchanged.
    ///
    /// # Contract
    /// - requires: each [`ItemLiveness::index`] is an item index of `resume`;
    ///   entries for other indices are ignored.
    /// - ensures: emits `Started`, then per item in source order its `Item`
    ///   event followed by that item's `Match` events in match order, then
    ///   `Completed`, so equal inputs produce equal event sequences — the
    ///   stream reads its inputs and computes nothing of its own.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — liveness rides beside the item events without
    ///   disturbing them, and the sequence is a function of the inputs rather
    ///   than of the path that produced them — so a run that adopted its
    ///   predecessors and a run that typed them from scratch publish the same
    ///   events.
    /// - witness: `stream::tests::liveness_rides_beside_the_item_it_addresses`
    /// - witness: `stream::tests::absent_liveness_leaves_the_stream_unchanged`
    #[must_use]
    #[inline]
    pub fn from_resume_with_liveness(
        resume: &Resume,
        liveness: &[ItemLiveness],
    ) -> Self
    {
        let item_count = resume.typings().len();
        let mut events = Vec::with_capacity(item_count.saturating_add(2));
        events.push(SynthesisEvent::Started { item_count });
        for (index, (typing, adopted)) in resume.typings().zip(resume.adopted()).enumerate() {
            events.push(SynthesisEvent::Item {
                index,
                typing: typing.clone(),
                adopted,
            });
            for item in liveness.iter().filter(|item| item.index == index) {
                for one_match in &item.matches {
                    events.push(SynthesisEvent::Match {
                        index,
                        match_index: one_match.match_index,
                        branches: one_match.branches.clone(),
                    });
                }
            }
        }
        events.push(SynthesisEvent::Completed);
        Self { events, cursor: 0 }
    }
}

impl Iterator for SynthesisStream
{
    type Item = SynthesisEvent;

    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        let event = self.events.get(self.cursor).cloned();
        if event.is_some() {
            self.cursor = self.cursor.saturating_add(1);
        }
        event
    }
}

#[cfg(test)]
mod tests
{
    use alloc::string::String;

    use gandr_core_checker::syntax::Term;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::types::Ty;
    use gandr_core_checker::types::ValueType;

    use super::*;
    use crate::checkpoint::Checkpoints;
    use crate::checkpoint::ItemCheckpoint;
    use crate::footprint::footprint_of;

    /// A two-item resume whose items are trivially typed, so the events under
    /// test are the ones this module builds rather than anything the checker
    /// decided.
    fn two_item_resume() -> Resume
    {
        let items = (0 .. 2)
            .map(|index| {
                let term = Term::Value(Value::int(index));
                ItemCheckpoint {
                    name: Some(String::from("item")),
                    ascription: None,
                    footprint: footprint_of(&term),
                    term,
                    typing: ItemTyping::Expression {
                        ty: Ty::Value(ValueType::integer()),
                    },
                }
            })
            .collect();
        Resume::from_checkpoints(Checkpoints { items })
    }

    /// One match's liveness is emitted immediately after the item that owns it,
    /// and nowhere else — the addressing a consumer relies on to attribute a
    /// branch status to a source item.
    #[test]
    fn liveness_rides_beside_the_item_it_addresses()
    {
        let stream =
            SynthesisStream::from_resume_with_liveness(&two_item_resume(), &[ItemLiveness {
                index: 1,
                matches: alloc::vec![MatchLiveness {
                    match_index: 0,
                    branches: alloc::vec![BranchStatus::Refuted, BranchStatus::Possibly],
                }],
            }]);
        let events: Vec<SynthesisEvent> = stream.collect();
        let positions: Vec<usize> = events
            .iter()
            .enumerate()
            .filter_map(|(position, event)| match *event {
                | SynthesisEvent::Match { index, .. } => Some((position, index)),
                | _ => None,
            })
            .map(|(position, _)| position)
            .collect();
        assert_eq!(
            alloc::vec![3_usize],
            positions,
            "the match event follows item 1 (Started, Item 0, Item 1, Match); events: {events:?}"
        );
        assert!(
            events.contains(&SynthesisEvent::Match {
                index: 1,
                match_index: 0,
                branches: alloc::vec![BranchStatus::Refuted, BranchStatus::Possibly],
            }),
            "the branches are published verbatim; events: {events:?}"
        );
    }

    /// A resume with no liveness produces exactly the stream it produced before
    /// liveness existed, so an analysis that has nothing to say costs the
    /// consumer nothing to read.
    #[test]
    fn absent_liveness_leaves_the_stream_unchanged()
    {
        let resume = two_item_resume();
        let plain: Vec<SynthesisEvent> = SynthesisStream::from_resume(&resume).collect();
        let empty: Vec<SynthesisEvent> =
            SynthesisStream::from_resume_with_liveness(&resume, &[]).collect();
        assert_eq!(plain, empty, "no liveness, no extra events");
        assert!(
            !plain
                .iter()
                .any(|event| matches!(*event, SynthesisEvent::Match { .. })),
            "and nothing match-shaped appears; events: {plain:?}"
        );
    }
}
