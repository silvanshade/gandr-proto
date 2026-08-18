//! Live source-corpus loader for the sequent/kernel integration gates.
//!
//! The front end is available again at rung F4, so the semantic sweeps consume
//! the surface-corpus model and pathological examples through
//! [`gandr_surface_engine::lower::lower_source_total`]. Corpus provenance is
//! derived directly from each checked-in `.gandr` source; no pre-lowered
//! representation is consulted. F4 staging O6 keeps the FFI-capability and
//! regex fixtures' semantic anchors frozen until those optional features land.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use gandr_core_term::syntax::Term;
use gandr_surface_engine::lower::lower_source_total;

/// Sources whose pre-feature semantic records remain frozen at F4 staging O6.
const FEATURE_FROZEN_SOURCES: [&str; 2] = [
    "model/22-ffi-effect-and-capability.gandr",
    "model/28-regex-and-path-builtins.gandr",
];
/// One named top-level source-corpus tree.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CorpusTree(&'static str);

impl CorpusTree
{
    /// The representative model corpus.
    pub const MODEL: Self = Self("model");

    /// The pathological stress corpus.
    pub const PATHOLOGICAL: Self = Self("pathological");
}

impl core::fmt::Display for CorpusTree
{
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(self.0)
    }
}

/// One live-lowered corpus source and its source-byte snapshot.
pub struct Fixture
{
    /// Corpus-relative source path (`model/foo.gandr`).
    pub source: String,
    /// Path of the checked-in `.gandr` source bytes.
    pub source_path: PathBuf,
    /// Path of the corresponding checked-in `.outcome` snapshot.
    pub snapshot_path: PathBuf,
    /// Live terms produced by total lowering, in source order.
    pub items: Vec<Term>,
    /// Whether F4 staging O6 freezes this fixture's semantic record.
    ///
    /// The live terms still execute; only comparison against the immutable
    /// `.outcome` and partition rows is deferred until the feature lands.
    pub snapshot_is_feature_frozen: bool,
}

/// Lowers every source under one corpus tree (`model` or `pathological`).
///
/// # Contract
///
/// - requires: `tree` names a directory under the surface corpus's `examples/`
///   tree.
/// - ensures: returns one fixture per `.gandr` source, sorted by relative path,
///   with items produced only by live total lowering.
/// - provides: the single corpus-input seam shared by the differential,
///   totality, partition, and export gates.
/// - panics: unreadable/malformed checked-in test data or lowering
///   infrastructure failure.
pub fn read_tree(tree: CorpusTree) -> Vec<Fixture>
{
    let source_root = surface_corpus_root().join(tree.0);
    let mut source_paths = gandr_files(&source_root);
    source_paths.sort();
    let mut fixtures = Vec::with_capacity(source_paths.len());
    for source_path in source_paths {
        let relative = source_path
            .strip_prefix(surface_corpus_root())
            .unwrap_or_else(|error| {
                panic!(
                    "corpus source `{}` must be under `{}`: {error}",
                    source_path.display(),
                    surface_corpus_root().display()
                )
            });
        let source = relative.to_string_lossy().replace('\\', "/");
        let text = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", source_path.display()));
        let lowered = lower_source_total(text.as_str().into())
            .unwrap_or_else(|error| panic!("{source} must lower totally: {error}"));
        let snapshot_path = fixture_root().join(relative).with_extension("outcome");
        assert!(
            snapshot_path.is_file(),
            "{source}: missing outcome snapshot `{}`",
            snapshot_path.display()
        );
        fixtures.push(Fixture {
            snapshot_is_feature_frozen: FEATURE_FROZEN_SOURCES.contains(&source.as_str()),
            source,
            source_path,
            snapshot_path,
            items: lowered.items.into_iter().map(|item| item.term).collect(),
        });
    }
    fixtures
}

/// A BLAKE3 fixture-provenance digest over one or more corpus trees.
///
/// Each record is framed by the repository-relative source path, a NUL byte,
/// the lowercase BLAKE3 digest of the exact `.gandr` source bytes, and a LF.
/// Source bytes are the authority for all corpus manifests.
pub fn corpus_fixtures_b3sum(trees: &[CorpusTree]) -> String
{
    let mut hasher = blake3::Hasher::new();
    for &tree in trees {
        for fixture in read_tree(tree) {
            let bytes = fs::read(&fixture.source_path).unwrap_or_else(|error| {
                panic!(
                    "cannot read corpus source `{}`: {error}",
                    fixture.source_path.display()
                )
            });
            update_fixture_digest(
                &mut hasher,
                SourcePath(&fixture.source),
                SourceBytes(&bytes),
            );
        }
    }
    hasher.finalize().to_hex().to_string()
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct SourcePath<'source>(&'source str);

#[derive(Clone, Copy)]
#[repr(transparent)]
struct SourceBytes<'bytes>(&'bytes [u8]);

fn update_fixture_digest(
    hasher: &mut blake3::Hasher,
    source: SourcePath<'_>,
    bytes: SourceBytes<'_>,
)
{
    hasher.update(source.0.as_bytes());
    hasher.update(&[0x00]);
    hasher.update(blake3::hash(bytes.0).to_hex().as_bytes());
    hasher.update(&[0x0a]);
}

/// Collects every `.gandr` file under `dir` with an explicit directory
/// worklist.
fn gandr_files(dir: &Path) -> Vec<PathBuf>
{
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current).unwrap_or_else(|error| {
            panic!(
                "cannot read corpus directory `{}`: {error}",
                current.display()
            )
        });
        for entry in entries {
            let entry = entry.unwrap_or_else(|error| panic!("cannot read corpus entry: {error}"));
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            }
            else if path
                .extension()
                .is_some_and(|extension| extension == "gandr")
            {
                files.push(path);
            }
        }
    }
    files
}

/// Root of the ported executable source corpus.
fn surface_corpus_root() -> PathBuf
{
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../surface-corpus/examples"
    ))
}
/// Root of the checked-in corpus outcome snapshots.
fn fixture_root() -> PathBuf
{
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/corpus"
    ))
}

#[cfg(test)]
mod tests
{
    use super::SourceBytes;
    use super::SourcePath;
    use super::update_fixture_digest;

    #[test]
    fn source_byte_changes_change_the_framed_digest()
    {
        let mut first = blake3::Hasher::new();
        update_fixture_digest(
            &mut first,
            SourcePath("model/example.gandr"),
            SourceBytes(b"def value = 1;"),
        );
        let mut second = blake3::Hasher::new();
        update_fixture_digest(
            &mut second,
            SourcePath("model/example.gandr"),
            SourceBytes(b"def value = 2;"),
        );
        assert_ne!(first.finalize(), second.finalize());
    }
}
