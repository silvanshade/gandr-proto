//! Failure-atomic publication for mutation campaign reports.

use alloc::format;
use alloc::vec::Vec;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::GateError;

crate::semantic_copy!(pub struct MovedPreviousFlag(bool));
crate::semantic_copy!(pub struct FinalFailedFlag(bool));
crate::semantic_copy!(pub struct RollbackFailedFlag(bool));
crate::semantic_str!(pub struct ValueText);
crate::semantic_str!(pub struct NameText);
crate::semantic_copy!(pub struct ExistsFlag(bool));
crate::semantic_copy!(pub struct PreserveCurrentReportFlag(bool));
crate::semantic_copy!(pub struct PathExistsFlag(bool));

/// Directory name for the published mutation report.
const CURRENT_REPORT_DIR: &str = "mutants.out";
/// Directory name used while staging the next mutation report.
const STAGED_REPORT_DIR: &str = "mutants.out.next";
/// Directory name used to preserve the previous report during final rename.
const PREVIOUS_REPORT_DIR: &str = "mutants.out.previous";

/// Filesystem paths used by the report publication protocol.
///
/// # Contract
/// - requires: `workspace_root` names the directory that owns the mutation
///   report siblings.
/// - ensures: every report path is a direct child of `workspace_root`.
/// - provides: same-directory paths so rename operations stay on one
///   filesystem.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the sibling-name contract is killed by publication
///   tests that observe `mutants.out`, `mutants.out.next`, and
///   `mutants.out.previous` after success and rollback paths.
/// - witness: `mutants::report::tests::successful_publication_replaces_current_report`
/// - witness: `mutants::report::tests::simulated_final_rename_failure_restores_prior_report`
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) struct ReportPaths
{
    /// Destination report directory.
    current: PathBuf,
    /// Staged next report directory.
    staging: PathBuf,
    /// Previous-report backup directory.
    previous: PathBuf,
}

impl ReportPaths
{
    /// Build the three report sibling paths under `workspace_root`.
    #[inline]
    #[must_use]
    pub(super) fn new(workspace_root: &Path) -> Self
    {
        Self {
            current: workspace_root.join(CURRENT_REPORT_DIR),
            staging: workspace_root.join(STAGED_REPORT_DIR),
            previous: workspace_root.join(PREVIOUS_REPORT_DIR),
        }
    }

    /// Return the published report directory.
    #[inline]
    #[must_use]
    pub(super) fn current(&self) -> &Path
    {
        return &self.current;
    }

    /// Return the staged report directory.
    #[inline]
    #[must_use]
    pub(super) fn staging(&self) -> &Path
    {
        return &self.staging;
    }

    /// Return the previous-report backup directory.
    #[inline]
    #[must_use]
    pub(super) fn previous(&self) -> &Path
    {
        return &self.previous;
    }
}

/// Side-effect surface needed by the report publication protocol.
pub(super) trait ReportFileSystem
{
    /// Return whether `path` exists.
    fn exists(
        &mut self,
        path: &Path,
    ) -> impl Into<ExistsFlag>;

    /// Copy `source` recursively into a newly staged report directory.
    ///
    /// # Errors
    /// Returns a typed error when the source cannot be traversed, a destination
    /// directory cannot be created, a regular file cannot be copied, or the
    /// report tree contains an unsupported file type.
    fn copy_dir(
        &mut self,
        source: &Path,
        destination: &Path,
    ) -> Result<(), GateError>;

    /// Rename `source` to `destination`.
    ///
    /// # Errors
    /// Returns the typed rename error when the filesystem rejects the move.
    fn rename(
        &mut self,
        source: &Path,
        destination: &Path,
    ) -> Result<(), GateError>;

    /// Remove `path` and every child below it.
    ///
    /// # Errors
    /// Returns the typed remove error when cleanup fails.
    fn remove_dir_all(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>;
}

/// Host filesystem adapter for report publication.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub(super) struct StdReportFileSystem;

impl ReportFileSystem for StdReportFileSystem
{
    #[inline]
    fn exists(
        &mut self,
        path: &Path,
    ) -> impl Into<ExistsFlag>
    {
        match crate::support::HOST_FILESYSTEM.try_exists(path) {
            | Ok(exists) => return bool::from(exists),
            | Err(_source) => return false,
        }
    }

    #[inline]
    fn copy_dir(
        &mut self,
        source: &Path,
        destination: &Path,
    ) -> Result<(), GateError>
    {
        copy_report_dir(source, destination)
    }

    #[inline]
    fn rename(
        &mut self,
        source: &Path,
        destination: &Path,
    ) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM.rename(source, destination)
    }

    #[inline]
    fn remove_dir_all(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM.remove_dir_all(path)
    }
}

/// Publish `report` into `workspace_root/mutants.out` atomically.
///
/// # Contract
/// - requires: `report` is a complete report directory and `workspace_root` is
///   the directory that should contain the durable report siblings.
/// - ensures: on success, `mutants.out` contains the new report and
///   `mutants.out.next` / `mutants.out.previous` are absent.
/// - provides: staged-copy, previous-report backup, final rename, and rollback
///   semantics matching the historical Nushell driver.
/// - fails: returns [`GateError`] on stale rollback state, staging failure,
///   previous-report preservation failure, final rename failure, rollback
///   failure, or stale-backup cleanup failure.
/// - panics: none.
///
/// # Errors
/// Returns an operational error for protocol failures and wraps filesystem
/// failures in the stable error detail where they occur.
///
/// # Adequacy
/// - hypothesis: L3 only — the success path and the simulated final-rename
///   failure distinguish every state transition: stage, preserve, finalize,
///   rollback, and cleanup.
/// - witness: `mutants::report::tests::successful_publication_replaces_current_report`
/// - witness: `mutants::report::tests::simulated_final_rename_failure_restores_prior_report`
#[inline]
pub(super) fn publish_report(
    report: &Path,
    workspace_root: &Path,
) -> Result<(), GateError>
{
    let mut filesystem = StdReportFileSystem;
    publish_report_with_filesystem(&mut filesystem, report, workspace_root)
}

/// Publish `report` with an injected filesystem adapter.
///
/// # Contract
/// - requires: `filesystem` implements the same rename/copy/remove semantics as
///   the host filesystem or a faithful test double.
/// - ensures: uses only the [`ReportFileSystem`] methods for side effects.
/// - provides: deterministic rollback behavior that can be tested without
///   invoking the VM or shell commands.
/// - fails: returns the same failures as [`publish_report`].
/// - panics: none.
///
/// # Errors
/// Returns stale-state, staging, preservation, final-rename, rollback, or
/// cleanup failures as [`GateError`] values.
///
/// # Adequacy
/// - hypothesis: L3 only — an adapter that fails exactly one final rename kills
///   mutants that publish by overwriting the current report before rollback is
///   possible.
/// - witness: `mutants::report::tests::simulated_final_rename_failure_restores_prior_report`
#[inline]
pub(super) fn publish_report_with_filesystem<FileSystem>(
    filesystem: &mut FileSystem,
    report: &Path,
    workspace_root: &Path,
) -> Result<(), GateError>
where
    FileSystem: ReportFileSystem,
{
    let paths = ReportPaths::new(workspace_root);
    restore_or_reject_previous(filesystem, &paths)?;
    reject_staged_report(filesystem, &paths)?;
    stage_report(filesystem, report, workspace_root, &paths)?;

    let moved_previous = preserve_current_report(filesystem, &paths).map(|value| value.into().0)?;
    let finalized = filesystem.rename(paths.staging(), paths.current());
    match finalized {
        | Ok(()) => cleanup_previous_after_success(filesystem, &paths),
        | Err(final_error) => {
            rollback_final_rename(filesystem, &paths, moved_previous, &final_error)
        },
    }
}

/// Restore an orphan backup or reject ambiguous rollback state.
fn restore_or_reject_previous<FileSystem>(
    filesystem: &mut FileSystem,
    paths: &ReportPaths,
) -> Result<(), GateError>
where
    FileSystem: ReportFileSystem,
{
    if !filesystem.exists(paths.previous()).into().0 {
        return Ok(());
    }
    if filesystem.exists(paths.current()).into().0 {
        return Err(GateError::operational(format!(
            "mutants-vm: prior report rollback remains at {}; current report was left untouched",
            paths.previous().display()
        )));
    }
    filesystem
        .rename(paths.previous(), paths.current())
        .map_err(|source| {
            GateError::operational(format!(
                "mutants-vm: cannot restore prior mutation report: {}",
                source
            ))
        })
}

/// Reject an interrupted staged report before touching the current report.
fn reject_staged_report<FileSystem>(
    filesystem: &mut FileSystem,
    paths: &ReportPaths,
) -> Result<(), GateError>
where
    FileSystem: ReportFileSystem,
{
    if filesystem.exists(paths.staging()).into().0 {
        return Err(GateError::operational(format!(
            "mutants-vm: interrupted staged report remains at {}; current report was left untouched",
            paths.staging().display()
        )));
    }
    Ok(())
}

/// Copy a completed report into the staged sibling directory.
fn stage_report<FileSystem>(
    filesystem: &mut FileSystem,
    report: &Path,
    workspace_root: &Path,
    paths: &ReportPaths,
) -> Result<(), GateError>
where
    FileSystem: ReportFileSystem,
{
    filesystem
        .copy_dir(report, paths.staging())
        .map_err(|source| {
            GateError::operational(format!(
                "mutants-vm: cannot stage mutation report: {}; new report remains at {}",
                source,
                workspace_root.display()
            ))
        })
}

/// Move the current report aside before the staged report is finalized.
fn preserve_current_report<FileSystem>(
    filesystem: &mut FileSystem,
    paths: &ReportPaths,
) -> Result<impl Into<PreserveCurrentReportFlag>, GateError>
where
    FileSystem: ReportFileSystem,
{
    if !filesystem.exists(paths.current()).into().0 {
        return Ok(false);
    }
    filesystem
        .rename(paths.current(), paths.previous())
        .map_err(|source| {
            GateError::operational(format!(
                "mutants-vm: cannot preserve prior mutation report: {}; staged report remains at {}",
                source,
                paths.staging().display()
            ))
        })?;
    Ok(true)
}

/// Remove the previous-report backup after the new report is durable.
fn cleanup_previous_after_success<FileSystem>(
    filesystem: &mut FileSystem,
    paths: &ReportPaths,
) -> Result<(), GateError>
where
    FileSystem: ReportFileSystem,
{
    if !filesystem.exists(paths.previous()).into().0 {
        return Ok(());
    }
    filesystem
        .remove_dir_all(paths.previous())
        .map_err(|source| {
            GateError::operational(format!(
                "mutants-vm: new report is published but prior backup cleanup failed: {}",
                source
            ))
        })
}

/// Restore the previous report when the final staged-to-current rename fails.
fn rollback_final_rename<FileSystem, MovedPrevious>(
    filesystem: &mut FileSystem,
    paths: &ReportPaths,
    moved_previous: MovedPrevious,
    final_error: &GateError,
) -> Result<(), GateError>
where
    FileSystem: ReportFileSystem,
    MovedPrevious: Into<MovedPreviousFlag>,
{
    let moved_previous = moved_previous.into().0;
    if !moved_previous {
        return Err(GateError::operational(format!(
            "mutants-vm: final report rename failed: {}; new report remains at {}",
            final_error,
            paths.staging().display()
        )));
    }

    let restored = filesystem.rename(paths.previous(), paths.current());
    match restored {
        | Ok(()) => Err(GateError::operational(format!(
            "mutants-vm: final report rename failed: {}; prior report restored and new report remains at {}",
            final_error,
            paths.staging().display()
        ))),
        | Err(restore_error) => Err(GateError::operational(format!(
            "mutants-vm: final report rename failed: {}; rollback also failed: {}; reports remain at {} and {}",
            final_error,
            restore_error,
            paths.staging().display(),
            paths.previous().display()
        ))),
    }
}

/// Copy one report directory using an explicit worklist.
fn copy_report_dir(
    source: &Path,
    destination: &Path,
) -> Result<(), GateError>
{
    crate::support::HOST_FILESYSTEM.create_dir_all(destination)?;

    let mut pending = Vec::new();
    pending.push((source.to_path_buf(), destination.to_path_buf()));
    while let Some((source_dir, destination_dir)) = pending.pop() {
        let entries = fs::read_dir(&source_dir).map_err(|io_source| GateError::Io {
            path: source_dir.clone(),
            source: io_source,
        })?;
        for entry_result in entries {
            let entry = entry_result.map_err(|io_source| GateError::Io {
                path: source_dir.clone(),
                source: io_source,
            })?;
            let entry_source = entry.path();
            let entry_destination = destination_dir.join(entry.file_name());
            let metadata = entry.metadata().map_err(|io_source| GateError::Io {
                path: entry_source.clone(),
                source: io_source,
            })?;
            let file_type = metadata.file_type();
            if file_type.is_dir() {
                crate::support::HOST_FILESYSTEM.create_dir_all(&entry_destination)?;
                pending.push((entry_source, entry_destination));
            }
            else if file_type.is_symlink() {
                return Err(GateError::operational(format!(
                    "mutants-vm: report contains unsupported symlink at {}",
                    entry_source.display()
                )));
            }
            else {
                crate::support::HOST_FILESYSTEM.copy(entry_source, entry_destination)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests
{
    //! Behavioral tests for failure-atomic report publication.

    use alloc::format;
    use alloc::string::String;
    use core::error::Error;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    use super::ExistsFlag;
    use super::NameText;
    use super::PathExistsFlag;
    use super::ReportFileSystem;
    use super::ReportPaths;
    use super::StdReportFileSystem;
    use super::publish_report_with_filesystem;
    use crate::GateError;

    /// Result type used by publication unit tests.
    type TestResult = Result<(), Box<dyn Error>>;

    /// Filesystem adapter that injects one final rename failure.
    struct FinalRenameFailureFileSystem
    {
        /// Real filesystem adapter used for every non-injected operation.
        inner: StdReportFileSystem,
        /// Source path whose rename must fail once.
        failing_source: PathBuf,
        /// Destination path whose rename must fail once.
        failing_destination: PathBuf,
        /// Whether the synthetic failure has already been consumed.
        failed_once: bool,
    }

    impl FinalRenameFailureFileSystem
    {
        /// Build an adapter that fails the staged-to-current rename once.
        fn new(paths: &ReportPaths) -> Self
        {
            Self {
                inner: StdReportFileSystem,
                failing_source: paths.staging().to_path_buf(),
                failing_destination: paths.current().to_path_buf(),
                failed_once: false,
            }
        }
    }

    impl ReportFileSystem for FinalRenameFailureFileSystem
    {
        fn exists(
            &mut self,
            path: &Path,
        ) -> impl Into<ExistsFlag>
        {
            self.inner.exists(path)
        }

        fn copy_dir(
            &mut self,
            source: &Path,
            destination: &Path,
        ) -> Result<(), GateError>
        {
            self.inner.copy_dir(source, destination)
        }

        fn rename(
            &mut self,
            source: &Path,
            destination: &Path,
        ) -> Result<(), GateError>
        {
            if !self.failed_once
                && source == self.failing_source.as_path()
                && destination == self.failing_destination.as_path()
            {
                self.failed_once = true;
                return Err(GateError::operational(
                    "injected final report rename failure",
                ));
            }
            self.inner.rename(source, destination)
        }

        fn remove_dir_all(
            &mut self,
            path: &Path,
        ) -> Result<(), GateError>
        {
            self.inner.remove_dir_all(path)
        }
    }

    /// Filesystem adapter that fails stale-backup cleanup after publication.
    struct CleanupPreviousFailureFileSystem
    {
        /// Real filesystem adapter used for every non-injected operation.
        inner: StdReportFileSystem,
        /// Previous-report path whose cleanup must fail.
        failing_path: PathBuf,
    }

    impl CleanupPreviousFailureFileSystem
    {
        /// Build an adapter that fails previous-report removal.
        fn new(paths: &ReportPaths) -> Self
        {
            Self {
                inner: StdReportFileSystem,
                failing_path: paths.previous().to_path_buf(),
            }
        }
    }

    impl ReportFileSystem for CleanupPreviousFailureFileSystem
    {
        fn exists(
            &mut self,
            path: &Path,
        ) -> impl Into<ExistsFlag>
        {
            self.inner.exists(path)
        }

        fn copy_dir(
            &mut self,
            source: &Path,
            destination: &Path,
        ) -> Result<(), GateError>
        {
            self.inner.copy_dir(source, destination)
        }

        fn rename(
            &mut self,
            source: &Path,
            destination: &Path,
        ) -> Result<(), GateError>
        {
            self.inner.rename(source, destination)
        }

        fn remove_dir_all(
            &mut self,
            path: &Path,
        ) -> Result<(), GateError>
        {
            if path == self.failing_path.as_path() {
                return Err(GateError::operational("injected previous cleanup failure"));
            }
            self.inner.remove_dir_all(path)
        }
    }

    /// Filesystem adapter that fails the final rename and the rollback rename.
    struct RollbackFailureFileSystem
    {
        /// Real filesystem adapter used for non-injected operations.
        inner: StdReportFileSystem,
        /// Staged report path.
        staging: PathBuf,
        /// Current report path.
        current: PathBuf,
        /// Previous report path.
        previous: PathBuf,
        /// Whether the final rename failure was consumed.
        final_failed: bool,
        /// Whether the rollback rename failure was consumed.
        rollback_failed: bool,
    }

    impl RollbackFailureFileSystem
    {
        /// Build an adapter that fails both finalization and rollback once.
        fn new(paths: &ReportPaths) -> Self
        {
            Self {
                inner: StdReportFileSystem,
                staging: paths.staging().to_path_buf(),
                current: paths.current().to_path_buf(),
                previous: paths.previous().to_path_buf(),
                final_failed: false,
                rollback_failed: false,
            }
        }
    }

    impl ReportFileSystem for RollbackFailureFileSystem
    {
        fn exists(
            &mut self,
            path: &Path,
        ) -> impl Into<ExistsFlag>
        {
            self.inner.exists(path)
        }

        fn copy_dir(
            &mut self,
            source: &Path,
            destination: &Path,
        ) -> Result<(), GateError>
        {
            self.inner.copy_dir(source, destination)
        }

        fn rename(
            &mut self,
            source: &Path,
            destination: &Path,
        ) -> Result<(), GateError>
        {
            if !self.final_failed
                && source == self.staging.as_path()
                && destination == self.current.as_path()
            {
                self.final_failed = true;
                return Err(GateError::operational(
                    "injected final report rename failure",
                ));
            }
            if !self.rollback_failed
                && source == self.previous.as_path()
                && destination == self.current.as_path()
            {
                self.rollback_failed = true;
                return Err(GateError::operational("injected rollback failure"));
            }
            self.inner.rename(source, destination)
        }

        fn remove_dir_all(
            &mut self,
            path: &Path,
        ) -> Result<(), GateError>
        {
            self.inner.remove_dir_all(path)
        }
    }

    /// Read a marker file from a report directory.
    fn read_marker(report_dir: &Path) -> Result<String, GateError>
    {
        crate::support::HOST_FILESYSTEM.read_to_string(report_dir.join("marker"))
    }

    /// Successful publication moves the staged report into `mutants.out`.
    #[test]
    fn successful_publication_replaces_current_report() -> TestResult
    {
        let root = unique_root("success")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let paths = ReportPaths::new(&root);
        let incoming = root.join("incoming-report");
        write_marker(paths.current(), "old report")?;
        write_marker(&incoming, "new report")?;

        let mut filesystem = StdReportFileSystem;
        publish_report_with_filesystem(&mut filesystem, &incoming, &root)?;

        assert_eq!(
            "new report",
            read_marker(paths.current())?,
            "current report should contain the staged report after publication"
        );
        assert!(
            !path_exists(paths.staging()).map(|value| value.into().0)?,
            "staged report should be removed by successful final rename"
        );
        assert!(
            !path_exists(paths.previous()).map(|value| value.into().0)?,
            "previous backup should be removed after successful publication"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// Remove a test directory when it exists.
    fn cleanup_root(root: &Path) -> Result<(), GateError>
    {
        if !path_exists(root).map(|value| value.into().0)? {
            return Ok(());
        }
        crate::support::HOST_FILESYSTEM.remove_dir_all(root)
    }

    /// A failed final rename restores the prior report and leaves the new
    /// report staged.
    #[test]
    fn simulated_final_rename_failure_restores_prior_report() -> TestResult
    {
        let root = unique_root("rollback")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let paths = ReportPaths::new(&root);
        let incoming = root.join("incoming-report");
        write_marker(paths.current(), "old report")?;
        write_marker(&incoming, "new report")?;

        let mut filesystem = FinalRenameFailureFileSystem::new(&paths);
        let failure = publish_report_with_filesystem(&mut filesystem, &incoming, &root)
            .expect_err("injected final rename failure should make publication fail");
        assert!(
            failure
                .to_string()
                .contains("prior report restored and new report remains"),
            "rollback diagnostic should say that the prior report was restored"
        );
        assert_eq!(
            "old report",
            read_marker(paths.current())?,
            "current report should be restored after final rename failure"
        );
        assert_eq!(
            "new report",
            read_marker(paths.staging())?,
            "new report should remain staged for later inspection"
        );
        assert!(
            !path_exists(paths.previous()).map(|value| value.into().0)?,
            "previous backup should be consumed by successful rollback"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// Return whether a path exists.
    fn path_exists(path: &Path) -> Result<impl Into<PathExistsFlag>, GateError>
    {
        crate::support::HOST_FILESYSTEM
            .try_exists(path)
            .map(bool::from)
    }

    /// An orphan previous report is restored before normal publication starts.
    #[test]
    fn orphan_previous_report_is_restored_before_publication() -> TestResult
    {
        let root = unique_root("orphan-previous")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let paths = ReportPaths::new(&root);
        let incoming = root.join("incoming-report");
        write_marker(paths.previous(), "old report")?;
        write_marker(&incoming, "new report")?;

        let mut filesystem = StdReportFileSystem;
        publish_report_with_filesystem(&mut filesystem, &incoming, &root)?;

        assert_eq!(
            "new report",
            read_marker(paths.current())?,
            "publication should replace the restored previous report with the incoming report"
        );
        assert!(
            !path_exists(paths.previous()).map(|value| value.into().0)?,
            "restored previous report should be cleaned after successful publication"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// A stale staged report is rejected before the current report is touched.
    #[test]
    fn interrupted_staged_report_is_rejected_without_touching_current() -> TestResult
    {
        let root = unique_root("staged-reject")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let paths = ReportPaths::new(&root);
        let incoming = root.join("incoming-report");
        write_marker(paths.current(), "old report")?;
        write_marker(paths.staging(), "interrupted report")?;
        write_marker(&incoming, "new report")?;

        let mut filesystem = StdReportFileSystem;
        let failure = publish_report_with_filesystem(&mut filesystem, &incoming, &root)
            .expect_err("stale staged report should reject publication");

        assert!(
            failure.to_string().contains("interrupted staged report"),
            "stale staging diagnostic should name the interrupted report"
        );
        assert_eq!(
            "old report",
            read_marker(paths.current())?,
            "current report should be left untouched when staging is stale"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// Ambiguous current-plus-previous rollback state is rejected.
    #[test]
    fn ambiguous_previous_and_current_reports_are_rejected() -> TestResult
    {
        let root = unique_root("ambiguous-previous")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let paths = ReportPaths::new(&root);
        let incoming = root.join("incoming-report");
        write_marker(paths.current(), "old report")?;
        write_marker(paths.previous(), "older report")?;
        write_marker(&incoming, "new report")?;

        let mut filesystem = StdReportFileSystem;
        let failure = publish_report_with_filesystem(&mut filesystem, &incoming, &root)
            .expect_err("ambiguous previous/current state should fail closed");

        assert!(
            failure
                .to_string()
                .contains("prior report rollback remains"),
            "ambiguous rollback diagnostic should preserve the prior-report location"
        );
        assert_eq!(
            "old report",
            read_marker(paths.current())?,
            "current report should remain untouched"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// Nested report directories are copied through the explicit worklist.
    #[test]
    fn nested_report_directories_are_preserved() -> TestResult
    {
        let root = unique_root("nested-copy")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let paths = ReportPaths::new(&root);
        let incoming = root.join("incoming-report");
        write_marker(&incoming.join("nested"), "nested report")?;

        let mut filesystem = StdReportFileSystem;
        publish_report_with_filesystem(&mut filesystem, &incoming, &root)?;

        assert_eq!(
            "nested report",
            read_marker(&paths.current().join("nested"))?,
            "nested report entries should be copied into the published report"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// Cleanup failures after a successful final rename are reported.
    #[test]
    fn previous_cleanup_failure_is_reported_after_publication() -> TestResult
    {
        let root = unique_root("cleanup-failure")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let paths = ReportPaths::new(&root);
        let incoming = root.join("incoming-report");
        write_marker(paths.current(), "old report")?;
        write_marker(&incoming, "new report")?;

        let mut filesystem = CleanupPreviousFailureFileSystem::new(&paths);
        let failure = publish_report_with_filesystem(&mut filesystem, &incoming, &root)
            .expect_err("previous cleanup failure should be reported after publication");

        assert!(
            failure.to_string().contains("prior backup cleanup failed"),
            "cleanup failure should keep the publication-success diagnostic"
        );
        assert_eq!(
            "new report",
            read_marker(paths.current())?,
            "new report should still be current when only cleanup failed"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// Failed rollback after a failed final rename reports both failures.
    #[test]
    fn rollback_failure_reports_staging_and_previous_locations() -> TestResult
    {
        let root = unique_root("rollback-failure")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let paths = ReportPaths::new(&root);
        let incoming = root.join("incoming-report");
        write_marker(paths.current(), "old report")?;
        write_marker(&incoming, "new report")?;

        let mut filesystem = RollbackFailureFileSystem::new(&paths);
        let failure = publish_report_with_filesystem(&mut filesystem, &incoming, &root)
            .expect_err("rollback failure should be reported after final rename failure");

        assert!(
            failure.to_string().contains("rollback also failed"),
            "diagnostic should preserve both finalization and rollback failures"
        );
        assert_eq!(
            "old report",
            read_marker(paths.previous())?,
            "prior report should remain in previous when rollback fails"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// Report symlinks are rejected rather than copied into the published tree.
    #[cfg(unix)]
    #[test]
    fn symlink_report_entries_are_rejected() -> TestResult
    {
        let root = unique_root("symlink-reject")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let incoming = root.join("incoming-report");
        crate::support::HOST_FILESYSTEM.create_dir_all(&incoming)?;
        crate::support::HOST_FILESYSTEM.symlink("target", incoming.join("link"))?;

        let mut filesystem = StdReportFileSystem;
        let failure = publish_report_with_filesystem(&mut filesystem, &incoming, &root)
            .expect_err("symlink report entries should be rejected");

        assert!(
            failure.to_string().contains("unsupported symlink"),
            "symlink rejection should name the unsupported report entry"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// Return a unique temporary test directory path.
    fn unique_root<'semantic, Name>(name: Name) -> Result<PathBuf, GateError>
    where
        Name: Into<NameText<'semantic>>,
    {
        let name = name.into().0;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|source| GateError::operational(format!("test clock failed: {source}")))?;
        Ok(std::env::temp_dir().join(format!(
            "gandr-workflow-gates-report-{name}-{}-{}",
            std::process::id(),
            timestamp.as_nanos()
        )))
    }

    /// A final rename failure without a prior current report leaves the staged
    /// report for inspection and cannot roll back.
    #[test]
    fn final_rename_failure_without_current_leaves_staging() -> TestResult
    {
        let root = unique_root("final-no-current")?;
        cleanup_root(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        let paths = ReportPaths::new(&root);
        let incoming = root.join("incoming-report");
        write_marker(&incoming, "new report")?;

        let mut filesystem = FinalRenameFailureFileSystem::new(&paths);
        let failure = publish_report_with_filesystem(&mut filesystem, &incoming, &root)
            .expect_err("final rename failure without current report should fail");

        assert!(
            failure.to_string().contains("final report rename failed"),
            "failure should preserve the final rename diagnostic"
        );
        assert_eq!(
            "new report",
            read_marker(paths.staging())?,
            "new report should remain staged when no previous report can be restored"
        );
        assert!(
            !path_exists(paths.current()).map(|value| value.into().0)?,
            "current report should remain absent when final rename fails without prior current"
        );
        cleanup_root(&root)?;
        Ok(())
    }

    /// Write a marker file into a report directory.
    fn write_marker<'semantic, Value>(
        report_dir: &Path,
        value: Value,
    ) -> Result<(), GateError>
    where
        Value: Into<super::ValueText<'semantic>>,
    {
        let value = value.into().0;
        crate::support::HOST_FILESYSTEM.create_dir_all(report_dir)?;
        crate::support::HOST_FILESYSTEM.write(report_dir.join("marker"), value)
    }
}
