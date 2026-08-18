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
    /// Persistence backend for complete checkpoint sets.
    store: S,
    /// Backend artifact identity required for reuse.
    backend: BackendArtifact,
    /// Most recently validated resume result.
    last_resume: Option<Resume>,
}

impl<S> IncrementalSession<S>
{
    /// Creates a session with no submitted revision.
    #[must_use]
    #[inline]
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
    /// Reopens a session from a previously validated resume.
    #[must_use]
    #[inline]
    pub fn reopen(
        store: S,
        backend: BackendArtifact,
        resume: Resume,
    ) -> Self
    {
        Self {
            store,
            backend,
            last_resume: Some(resume),
        }
    }

    /// Submits a lowered revision, reusing the previous validated checkpoints.
    ///
    /// # Errors
    ///
    /// Returns the persistence error when storing the resulting checkpoints
    /// fails.
    #[inline]
    pub fn submit<O>(
        &mut self,
        program: &Program,
        observer: &mut O,
    ) -> Result<Resume, CheckpointStoreError>
    where
        S: CheckpointStore,
        O: CheckpointObserver,
    {
        let resume = match self.last_resume.as_ref() {
            | Some(previous) => resume(previous.checkpoints(), program),
            | None => Resume::from_checkpoints(checkpoint_program(program)),
        };
        persist(
            &mut self.store,
            program,
            self.backend,
            resume.checkpoints().clone(),
            observer,
        )?;
        self.last_resume = Some(resume.clone());
        Ok(resume)
    }

    /// Streams the most recently submitted validated result in item order.
    #[must_use]
    #[inline]
    pub fn stream(&self) -> Option<SynthesisStream>
    {
        self.last_resume.as_ref().map(SynthesisStream::from_resume)
    }

    /// Returns the backing store after the session is finished.
    #[must_use]
    #[inline]
    pub fn into_store(self) -> S
    {
        self.store
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Term;
    use gandr_core_term::syntax::Value;

    use super::*;
    use crate::persistence::FileCheckpointStore;
    use crate::persistence::MemoryCheckpointStore;
    use crate::persistence::address_of;
    use crate::persistence::restore;
    use crate::region::Item;

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
            .submit(&Program::default(), &mut observer)
            .expect("submit");
        assert_eq!(usize::from(first.adopted_count()), 0);
        let events: alloc::vec::Vec<_> = session.stream().expect("stream").collect();
        assert_eq!(events, alloc::vec![
            crate::stream::SynthesisEvent::Started { item_count: 0 },
            crate::stream::SynthesisEvent::Completed,
        ]);
    }
    #[test]
    fn separately_reopened_file_session_resumes_supported_checkpoint()
    {
        let root = std::env::temp_dir().join(format!("gandr-session-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&root));
        let backend = BackendArtifact::from_bytes(b"backend");
        let program = Program::new(alloc::vec![Item::new(
            None,
            None,
            Term::Comp(Comp::ret(Value::int(1))),
        )]);
        let mut observer = Observer;

        let mut first = IncrementalSession::new(FileCheckpointStore::open(&root).unwrap(), backend);
        let _submitted = first.submit(&program, &mut observer).unwrap();
        drop(first.into_store());

        let address = address_of(&program).unwrap();
        let mut loader = FileCheckpointStore::open(&root).unwrap();
        let checkpoints = restore(&mut loader, &program, address, backend, &mut observer)
            .unwrap()
            .unwrap();
        drop(loader);

        let resume = Resume::from_checkpoints(checkpoints);
        let mut reopened =
            IncrementalSession::reopen(FileCheckpointStore::open(&root).unwrap(), backend, resume);
        let resumed = reopened.submit(&program, &mut observer).unwrap();
        assert_eq!(usize::from(resumed.adopted_count()), 1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
