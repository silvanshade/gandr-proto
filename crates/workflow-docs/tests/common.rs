//! Shared integration-test support: the repo-root locator every suite uses.

use std::path::PathBuf;

/// The repo root (the crate manifest dir's grandparent).
pub(crate) fn repo_root() -> PathBuf
{
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
