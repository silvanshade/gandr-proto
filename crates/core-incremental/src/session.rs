//! One-call submission, checkpoint persistence, invalidation, and synthesis.

use crate::checkpoint::Resume;
use crate::checkpoint::checkpoint_program;
use crate::checkpoint::resume;
use crate::persistence::BackendArtifact;
use crate::persistence::CheckpointObserver;
use crate::persistence::CheckpointStore;
use crate::persistence::CheckpointStoreError;
use crate::persistence::persist;
use crate::region::Program;
use crate::stream::SynthesisStream;

/// Stateful incremental checking session over ordered lowered programs.
pub struct IncrementalSession<S>
{
    store: S,
    backend: BackendArtifact,
    last_resume: Option<Resume>,
}

impl<S> IncrementalSession<S>
{
    /// Creates a session with no submitted revision.
    #[must_use]
    pub fn new(
        store: S,
        backend: BackendArtifact,
    ) -> Self
    {
        Self {
            store,
            backend,
            last_resume: None,
        }
    }

    /// Submits a lowered revision, reusing the previous validated checkpoints.
    pub fn submit<O>(
        &mut self,
        program: Program,
        observer: &mut O,
    ) -> Result<Resume, CheckpointStoreError>
    where
        S: CheckpointStore,
        O: CheckpointObserver,
    {
        let resume = match self.last_resume.as_ref() {
            | Some(previous) => resume(previous.checkpoints(), &program),
            | None => Resume::from_checkpoints(checkpoint_program(&program)),
        };
        persist(
            &mut self.store,
            &program,
            self.backend,
            resume.checkpoints().clone(),
            observer,
        )?;
        self.last_resume = Some(resume.clone());
        Ok(resume)
    }

    /// Streams the most recently submitted validated result in item order.
    #[must_use]
    pub fn stream(&self) -> Option<SynthesisStream>
    {
        self.last_resume.as_ref().map(SynthesisStream::from_resume)
    }

    /// Returns the backing store after the session is finished.
    #[must_use]
    pub fn into_store(self) -> S
    {
        self.store
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::persistence::MemoryCheckpointStore;

    #[derive(Default)]
    struct Observer;

    impl CheckpointObserver for Observer
    {
    }

    #[test]
    fn submit_persists_and_streams_in_order()
    {
        let mut session = IncrementalSession::new(
            MemoryCheckpointStore::default(),
            BackendArtifact::from_bytes(b"backend"),
        );
        let mut observer = Observer;
        let first = session
            .submit(Program::default(), &mut observer)
            .expect("submit");
        assert_eq!(usize::from(first.adopted_count()), 0);
        let events: alloc::vec::Vec<_> = session.stream().expect("stream").collect();
        assert_eq!(events, alloc::vec![
            crate::stream::SynthesisEvent::Started { item_count: 0 },
            crate::stream::SynthesisEvent::Completed,
        ]);
    }
}
