//! The engine-confinement check.
//!
//! # Why this is a test and not a promise
//!
//! The assessment's whole standing rests on the engine being confined to this
//! crate: nothing in the shipping tree may acquire a dependency on it while a
//! question about adopting it is still open. A rule stated only in prose is a
//! rule that holds until someone adds one line to one manifest, and nothing
//! reports it.
//!
//! So the rule is a scan the crate's own suite runs. It is the same shape the
//! workspace already uses to keep its graph facade honest: the boundary is
//! checked mechanically, against the manifests as they actually are, rather
//! than trusted to review.
//!
//! The scan is deliberately textual. A dependency-resolution query would answer
//! a subtly different question — what the built graph contains — and a manifest
//! naming the engine is a problem before anything is built.

use std::path::Path;
use std::path::PathBuf;

use crate::boundary::ManifestText;
use crate::boundary::MemberName;

/// A failure to inspect the workspace's manifests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManifestScanError
{
    /// The workspace root could not be located above this crate.
    RootNotFound,
    /// A manifest or directory could not be read.
    Unreadable(PathBuf),
}

/// The name of the crate the engine is confined to.
const CONFINED_TO: &str = "core-incremental-assessment";

/// The manifest forms that name a dependency in this workspace's style.
const ENGINE_MARKERS: [&str; 3] = ["salsa.workspace", "salsa =", "dependencies.salsa]"];

/// The workspace root, located from this crate's own manifest directory.
///
/// # Contract
/// - ensures: returns the directory holding the workspace manifest that
///   declares this crate.
/// - fails: returns [`ManifestScanError::RootNotFound`] when no ancestor holds
///   a `crates` directory beside a `Cargo.toml`.
/// - panics: none.
///
/// # Errors
///
/// Returns [`ManifestScanError::RootNotFound`] when no ancestor holds a
/// `crates` directory beside a `Cargo.toml`.
#[inline]
pub fn workspace_root() -> Result<PathBuf, ManifestScanError>
{
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest_dir.ancestors() {
        let manifest = ancestor.join("Cargo.toml");
        let crates = ancestor.join("crates");
        if manifest.is_file() && crates.is_dir() {
            return Ok(ancestor.to_path_buf());
        }
    }
    Err(ManifestScanError::RootNotFound)
}

/// Every member crate whose manifest names the engine.
///
/// # Contract
/// - ensures: returns the directory name of each `crates/<name>/Cargo.toml`
///   that names the engine, sorted, with no entry for the workspace manifest
///   itself.
/// - fails: returns [`ManifestScanError::Unreadable`] naming the path that
///   could not be read.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: the scan must find a real occurrence and must not find a
///   coincidental one; a scan matching any substring would fire on an unrelated
///   crate whose name merely begins the same way, and a scan matching nothing
///   would pass an actual leak.
/// - witness: `confinement::the_engine_reaches_exactly_one_member_manifest`
/// - witness: `confinement::the_scan_recognizes_this_crate_own_declaration`
///
/// # Errors
///
/// Returns [`ManifestScanError::Unreadable`] naming the path that could not
/// be read.
#[inline]
pub fn members_naming_the_engine() -> Result<Vec<MemberName>, ManifestScanError>
{
    let root = workspace_root()?;
    let crates = root.join("crates");
    let entries = std::fs::read_dir(&crates)
        .map_err(|_error| ManifestScanError::Unreadable(crates.clone()))?;
    let mut found: Vec<MemberName> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_error| ManifestScanError::Unreadable(crates.clone()))?;
        let directory = entry.path();
        let manifest = directory.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&manifest)
            .map_err(|_error| ManifestScanError::Unreadable(manifest.clone()))?;
        if bool::from(names_the_engine(ManifestText::from(text.as_str()))) {
            let Some(name) = directory.file_name().and_then(std::ffi::OsStr::to_str)
            else {
                continue;
            };
            found.push(MemberName::from(name.to_owned()));
        }
    }
    found.sort();
    Ok(found)
}

/// The crate name the engine is permitted to reach.
///
/// # Contract
/// - ensures: returns the directory name of this crate.
/// - panics: none.
#[inline]
#[must_use]
pub fn confined_to() -> MemberName
{
    MemberName::from(CONFINED_TO.to_owned())
}

/// Whether one manifest's text declares the engine as a dependency.
///
/// # Contract
/// - ensures: reports a declaration only for one of this workspace's manifest
///   forms, so a crate whose own name merely starts with the same letters does
///   not match.
/// - panics: none.
#[inline]
#[must_use]
pub fn names_the_engine(manifest: ManifestText<'_>) -> ManifestVerdict
{
    let found = manifest.lines().any(|line| {
        let line = line.trim();
        if line.starts_with('#') {
            return false;
        }
        ENGINE_MARKERS.iter().any(|marker| line.contains(marker))
    });
    ManifestVerdict::from(found)
}

/// Whether a manifest declares the engine.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestVerdict(bool);

impl From<bool> for ManifestVerdict
{
    #[inline]
    fn from(found: bool) -> Self
    {
        Self(found)
    }
}

impl From<ManifestVerdict> for bool
{
    #[inline]
    fn from(verdict: ManifestVerdict) -> Self
    {
        verdict.0
    }
}
