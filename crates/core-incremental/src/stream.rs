//! Ordered, resumable synthesis events over validated checkpoints.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use crate::boundary::AdoptionDecision;
use crate::boundary::LivenessEmpty;
use crate::boundary::MatchOrdinal;
use crate::boundary::SourceItemOrdinal;
use crate::boundary::SubmissionOrdinal;
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

/// Where one analyzed match came from, as its producer identifies it.
///
/// **No core position appears here, and that is the point.** Liveness is a fact
/// about a match the programmer wrote, and a source item may lower to any
/// number of core items — so addressing a match by the position of "its" core
/// item is addressing it by something that is not a function of it. The three
/// components are the producer's own coordinates: which submission the match
/// was written in, which source item of that submission owns it, and which
/// match of that source item it is.
///
/// The stream never interprets them. Their only properties here are that they
/// identify a match uniquely and order it totally, which is what makes
/// [`Liveness`] a keyed map rather than a list to be searched.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MatchOrigin
{
    /// Which submission the match was written in.
    pub submission: SubmissionOrdinal,
    /// Which source item of that submission owns the match.
    pub source_item: SourceItemOrdinal,
    /// Which match of that source item this is, in source order.
    pub match_index: MatchOrdinal,
}

/// The match liveness a stream publishes, addressed by [`MatchOrigin`].
///
/// Two things live here, and they are different answers to the same question.
/// A match whose origin is a key of [`Liveness::matches`] has per-branch
/// verdicts. A match belonging to a submission named by
/// [`Liveness::unretained`] has none, and cannot: its producer no longer holds
/// the source records the verdicts are computed from. The second case is
/// **published rather than omitted**, because a consumer that reads silence as
/// "nothing to report" would read a session resumed from a core artifact as a
/// program whose earlier submissions held no matches at all.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Liveness
{
    /// One entry per analyzed match, keyed by origin.
    matches: BTreeMap<MatchOrigin, Vec<BranchStatus>>,
    /// The submissions whose source records were not retained.
    unretained: BTreeSet<SubmissionOrdinal>,
}

impl Liveness
{
    /// An empty liveness map — nothing analyzed, nothing missing.
    ///
    /// # Contract
    /// - ensures: carries no match and names no unretained submission.
    /// - panics: none.
    #[must_use]
    #[inline]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Records one match's per-branch verdicts at its origin.
    ///
    /// # Contract
    /// - ensures: the map holds exactly one entry for `origin` afterwards;
    ///   returns the entry this call displaced, so a producer that would have
    ///   published a match twice can be caught doing it rather than silently
    ///   publishing the second reading.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — one-entry-per-origin is the invariant a positional
    ///   scheme cannot state; keying is what enforces it, and the displaced
    ///   return is what makes a violation observable.
    /// - witness: `stream::tests::an_origin_carries_exactly_one_liveness_entry`
    #[inline]
    pub fn insert(
        &mut self,
        origin: MatchOrigin,
        branches: Vec<BranchStatus>,
    ) -> Option<Vec<BranchStatus>>
    {
        self.matches.insert(origin, branches)
    }

    /// Names a submission whose source records were not retained.
    ///
    /// # Contract
    /// - ensures: the submission is named by a [`SynthesisEvent`] of its own
    ///   rather than left silent.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the difference between "no matches to report" and
    ///   "the records are gone" is the whole content of this method.
    /// - witness: `stream::tests::an_unretained_submission_is_published_not_omitted`
    #[inline]
    pub fn mark_unretained(
        &mut self,
        submission: SubmissionOrdinal,
    )
    {
        let _present = self.unretained.insert(submission);
    }

    /// Whether anything at all is published — no matches and no unretained
    /// submissions.
    ///
    /// # Contract
    /// - ensures: `true` exactly when the stream gains no liveness events.
    /// - panics: none.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> LivenessEmpty
    {
        LivenessEmpty::from(self.matches.is_empty() && self.unretained.is_empty())
    }

    /// Every recorded match, ascending by origin.
    fn entries(&self) -> impl Iterator<Item = (MatchOrigin, &[BranchStatus])>
    {
        self.matches
            .iter()
            .map(|(origin, branches)| (*origin, branches.as_slice()))
    }

    /// Every submission this map says anything about, in ascending order.
    fn submissions(&self) -> BTreeSet<SubmissionOrdinal>
    {
        let mut submissions = self.unretained.clone();
        submissions.extend(self.matches.keys().map(|origin| origin.submission));
        submissions
    }
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
    /// Carries one match's per-branch liveness, addressed by the source origin
    /// the producer identifies the match with.
    ///
    /// **Every recorded match publishes one of these, decided or not.** A match
    /// whose scrutinee settles all its branches is here beside one whose
    /// scrutinee holds a hole; the event's presence says a match was analyzed,
    /// and nothing more than that. What distinguishes them is inside: a
    /// [`BranchStatus::Possibly`] among the branches is what marks a branch
    /// some hole still decides, so a consumer looking for what the program
    /// is waiting on reads the verdicts rather than counting events.
    ///
    /// Branches are published rather than a match result because there is no
    /// result to publish: this analysis decides how each branch stands against
    /// the scrutinee, never which branch fires.
    Match
    {
        /// Which match this is, in its producer's own coordinates.
        origin: MatchOrigin,
        /// One status per branch, in source order.
        branches: Vec<BranchStatus>,
    },
    /// Names a submission whose source match records the producer no longer
    /// holds, so no liveness can be computed for it.
    ///
    /// A session resumed from a persisted core artifact is the case this
    /// exists for: the artifact carries checkpoints, and checkpoints carry
    /// lowered terms rather than the patterns the programmer wrote. The
    /// verdicts are therefore unrecoverable, and **saying so is the contract**
    /// — a consumer that saw nothing would read the resumed session as a
    /// program whose earlier matches had all become decided.
    SourceNotRetained
    {
        /// The submission whose source records are gone.
        submission: SubmissionOrdinal,
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
        Self::from_resume_with_liveness(resume, &Liveness::new())
    }

    /// Creates a stream from one validated resume result and the match liveness
    /// its producer computed, published after the item events in origin order.
    ///
    /// **Liveness follows the items rather than interleaving with them, and the
    /// separation is deliberate.** An item event is addressed by core position;
    /// a match event is addressed by source origin. One source item may lower
    /// to several core items, so there is no position a match belongs "beside"
    /// — and a stream that placed it beside one anyway would be asserting an
    /// attribution nothing supports. A consumer resolves an origin to a source
    /// span through the producer's own analysis result, which carries the
    /// range.
    ///
    /// # Contract
    /// - requires: nothing of `liveness` beyond what its own type enforces —
    ///   the keying makes a duplicate origin unrepresentable, and an origin
    ///   naming no item of `resume` is published unchanged rather than dropped,
    ///   because the stream is not the authority on which matches exist.
    /// - ensures: emits `Started`, then every `Item` event in source order,
    ///   then per submission in ascending order either its `SourceNotRetained`
    ///   event or its `Match` events in origin order, then `Completed`. Equal
    ///   inputs therefore produce equal event sequences — the stream reads its
    ///   inputs and computes nothing of its own.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — liveness follows the item events without disturbing
    ///   them, each origin appears exactly once, an unretained submission is
    ///   named rather than omitted, and the sequence is a function of the
    ///   inputs rather than of the path that produced them — so a run that
    ///   adopted its predecessors and a run that typed them from scratch
    ///   publish the same events.
    /// - witness: `stream::tests::liveness_follows_the_items_in_origin_order`
    /// - witness: `stream::tests::an_origin_carries_exactly_one_liveness_entry`
    /// - witness: `stream::tests::an_unretained_submission_is_published_not_omitted`
    /// - witness: `stream::tests::absent_liveness_leaves_the_stream_unchanged`
    #[must_use]
    #[inline]
    pub fn from_resume_with_liveness(
        resume: &Resume,
        liveness: &Liveness,
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
        }
        for submission in liveness.submissions() {
            if liveness.unretained.contains(&submission) {
                events.push(SynthesisEvent::SourceNotRetained { submission });
                continue;
            }
            for (origin, branches) in liveness
                .entries()
                .filter(|&(origin, _)| origin.submission == submission)
            {
                events.push(SynthesisEvent::Match {
                    origin,
                    branches: branches.to_vec(),
                });
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

    /// One origin, in this fixture's coordinates.
    fn origin(
        submission: SubmissionOrdinal,
        source_item: SourceItemOrdinal,
        match_index: MatchOrdinal,
    ) -> MatchOrigin
    {
        MatchOrigin {
            submission,
            source_item,
            match_index,
        }
    }

    /// Liveness follows every item event, in ascending origin order, and the
    /// verdicts arrive verbatim.
    ///
    /// The ordering is the whole assertion. A match is addressed by where it
    /// was written, never by a core position, so its event cannot be placed
    /// "beside" an item — and a consumer relying on adjacency would be relying
    /// on an attribution that fails the moment one source item lowers to two
    /// core items.
    #[test]
    fn liveness_follows_the_items_in_origin_order()
    {
        let mut liveness = Liveness::new();
        let _displaced = liveness.insert(
            origin(0_usize.into(), 1_usize.into(), 0_usize.into()),
            alloc::vec![BranchStatus::Refuted, BranchStatus::Possibly],
        );
        let _displaced = liveness.insert(
            origin(0_usize.into(), 0_usize.into(), 1_usize.into()),
            alloc::vec![BranchStatus::Satisfied],
        );
        let events: Vec<SynthesisEvent> =
            SynthesisStream::from_resume_with_liveness(&two_item_resume(), &liveness).collect();
        let origins: Vec<MatchOrigin> = events
            .iter()
            .filter_map(|event| match *event {
                | SynthesisEvent::Match { origin, .. } => Some(origin),
                | _ => None,
            })
            .collect();
        assert_eq!(
            alloc::vec![
                origin(0_usize.into(), 0_usize.into(), 1_usize.into()),
                origin(0_usize.into(), 1_usize.into(), 0_usize.into())
            ],
            origins,
            "ascending by origin, whatever order the producer recorded them in; events: \
             {events:?}"
        );
        let Some(first_match) = events
            .iter()
            .position(|event| matches!(*event, SynthesisEvent::Match { .. }))
        else {
            panic!("expected the liveness events on the stream, got {events:?}");
        };
        assert_eq!(
            2,
            events
                .iter()
                .take(first_match)
                .filter(|event| matches!(**event, SynthesisEvent::Item { .. }))
                .count(),
            "both item events precede the liveness; events: {events:?}"
        );
        assert!(
            events.contains(&SynthesisEvent::Match {
                origin: origin(0_usize.into(), 1_usize.into(), 0_usize.into()),
                branches: alloc::vec![BranchStatus::Refuted, BranchStatus::Possibly],
            }),
            "the branches are published verbatim; events: {events:?}"
        );
    }

    /// One origin carries one entry: recording it twice displaces the first
    /// reading rather than publishing both.
    ///
    /// This is the invariant a positional scheme could not state. A source item
    /// lowering to several core items would, under item-indexed addressing,
    /// give one match several homes; keying by origin makes that
    /// unrepresentable, and the displaced return makes a producer's double
    /// publication visible instead of silent.
    #[test]
    fn an_origin_carries_exactly_one_liveness_entry()
    {
        let mut liveness = Liveness::new();
        assert_eq!(
            None,
            liveness.insert(
                origin(0_usize.into(), 0_usize.into(), 0_usize.into()),
                alloc::vec![BranchStatus::Possibly]
            ),
            "the first recording displaces nothing"
        );
        assert_eq!(
            Some(alloc::vec![BranchStatus::Possibly]),
            liveness.insert(
                origin(0_usize.into(), 0_usize.into(), 0_usize.into()),
                alloc::vec![BranchStatus::Refuted]
            ),
            "the second is reported as a displacement, not silently accepted beside the first"
        );
        let events: Vec<SynthesisEvent> =
            SynthesisStream::from_resume_with_liveness(&two_item_resume(), &liveness).collect();
        assert_eq!(
            1,
            events
                .iter()
                .filter(|event| matches!(**event, SynthesisEvent::Match { .. }))
                .count(),
            "and the stream carries one event for that origin; events: {events:?}"
        );
    }

    /// A submission whose source records were not retained is named on the
    /// stream, and carries no match events of its own.
    ///
    /// Silence would be the wrong answer here: it reads as "this submission had
    /// no matches", which is a claim about the program rather than about what
    /// the producer still holds.
    #[test]
    fn an_unretained_submission_is_published_not_omitted()
    {
        let mut liveness = Liveness::new();
        liveness.mark_unretained(SubmissionOrdinal::from(0_usize));
        let _displaced = liveness.insert(
            origin(1_usize.into(), 0_usize.into(), 0_usize.into()),
            alloc::vec![BranchStatus::Possibly],
        );
        let events: Vec<SynthesisEvent> =
            SynthesisStream::from_resume_with_liveness(&two_item_resume(), &liveness).collect();
        assert!(
            events.contains(&SynthesisEvent::SourceNotRetained {
                submission: SubmissionOrdinal::from(0_usize),
            }),
            "submission 0 is named as unretained; events: {events:?}"
        );
        let origins: Vec<MatchOrigin> = events
            .iter()
            .filter_map(|event| match *event {
                | SynthesisEvent::Match { origin, .. } => Some(origin),
                | _ => None,
            })
            .collect();
        assert_eq!(
            alloc::vec![origin(1_usize.into(), 0_usize.into(), 0_usize.into())],
            origins,
            "and only the retained submission publishes verdicts; events: {events:?}"
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
            SynthesisStream::from_resume_with_liveness(&resume, &Liveness::new()).collect();
        assert_eq!(plain, empty, "no liveness, no extra events");
        assert!(
            bool::from(Liveness::new().is_empty()),
            "and the empty map says so of itself"
        );
        assert!(
            !plain.iter().any(|event| matches!(
                *event,
                SynthesisEvent::Match { .. } | SynthesisEvent::SourceNotRetained { .. }
            )),
            "and nothing liveness-shaped appears; events: {plain:?}"
        );
    }
}
