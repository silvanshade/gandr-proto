//! Ordered, resumable synthesis events over validated checkpoints.

use alloc::vec::Vec;

use crate::boundary::AdoptionDecision;
use crate::checkpoint::ItemTyping;
use crate::checkpoint::Resume;

/// One observable unit emitted by the synthesis stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SynthesisEvent
{
    /// Marks the beginning of a stream for a fixed item count.
    Started
    {
        item_count: usize
    },
    /// Carries one item typing and whether its checkpoint was reused.
    Item
    {
        index: usize,
        typing: ItemTyping,
        adopted: AdoptionDecision,
    },
    /// Marks successful completion after all items were emitted.
    Completed,
}

/// A deterministic stream that owns no input or output accumulation.
#[derive(Clone, Debug)]
pub struct SynthesisStream
{
    events: Vec<SynthesisEvent>,
    cursor: usize,
}

impl SynthesisStream
{
    /// Creates a stream from one validated resume result.
    #[must_use]
    pub fn from_resume(resume: &Resume) -> Self
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
        events.push(SynthesisEvent::Completed);
        Self { events, cursor: 0 }
    }
}

impl Iterator for SynthesisStream
{
    type Item = SynthesisEvent;

    fn next(&mut self) -> Option<Self::Item>
    {
        let event = self.events.get(self.cursor).cloned();
        if event.is_some() {
            self.cursor = self.cursor.saturating_add(1);
        }
        event
    }
}
