//! Live source-corpus loader for the sequent/kernel integration gates.
//!
//! The front end is available again at rung F4, so the semantic sweeps consume
//! `surface-corpus/examples/{model,pathological}` through
//! [`gandr_surface_engine::lower::lower_source_total`] instead of decoding the
//! B1 pre-lowered s-expression captures. The checked-in `.sexp` files remain as
//! byte/provenance anchors for the three invariant manifests, but never supply
//! executable terms. F4 staging O6 keeps the FFI-capability and regex fixtures'
//! semantic anchors frozen until those optional features land.
#![cfg_attr(
    test,
    allow(
        clippy::panic,
        reason = "checked-in corpus fixtures are trusted test data; malformed data must fail loudly"
    )
)]

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use gandr_core_checker::syntax::Term;
use gandr_surface_engine::lower::lower_source_total;

/// Sources whose pre-feature semantic records remain frozen at F4 staging O6.
const FEATURE_FROZEN_SOURCES: [&str; 2] = [
    "model/22-ffi-effect-and-capability.gandr",
    "model/28-regex-and-path-builtins.gandr",
];

/// One live-lowered corpus source and its checked-in B1 byte anchor.
pub struct Fixture
{
    /// Corpus-relative source path (`model/foo.gandr`).
    pub source: String,
    /// Path of the corresponding checked-in `.sexp` byte anchor.
    pub path: PathBuf,
    /// Live terms produced by total lowering, in source order.
    pub items: Vec<Term>,
    /// Whether F4 staging O6 freezes this fixture's pre-feature semantic
    /// record.
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
///   tree and every source has one B1 `.sexp` anchor.
/// - ensures: returns one fixture per `.gandr` source, sorted by relative path,
///   with items produced only by live total lowering.
/// - provides: the single corpus-input seam shared by the differential,
///   totality, partition, and export gates.
/// - panics: unreadable/malformed checked-in test data or lowering
///   infrastructure failure.
pub fn read_tree(tree: &str) -> Vec<Fixture>
{
    let source_root = surface_corpus_root().join(tree);
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
        let path = fixture_root().join(relative).with_extension("sexp");
        assert!(
            path.is_file(),
            "{source}: missing byte anchor `{}`",
            path.display()
        );
        fixtures.push(Fixture {
            snapshot_is_feature_frozen: FEATURE_FROZEN_SOURCES.contains(&source.as_str()),
            source,
            path,
            items: lowered.items.into_iter().map(|item| item.term).collect(),
        });
    }
    fixtures
}

/// A BLAKE3 fixture-provenance digest over one or more corpus trees.
///
/// The fold is intentionally byte-for-byte identical to the retired B1 reader:
/// path and file bytes remain the authority for the partition/export manifests
/// even though execution now comes from live lowering.
pub fn corpus_fixtures_b3sum(trees: &[&str]) -> String
{
    let mut hasher = blake3::Hasher::new();
    for tree in trees {
        for fixture in read_tree(tree) {
            let bytes = fs::read(&fixture.path).unwrap_or_else(|error| {
                panic!("cannot read fixture `{}`: {error}", fixture.path.display())
            });
            hasher.update(fixture.source.as_bytes());
            hasher.update(&[0x00]);
            hasher.update(blake3::hash(&bytes).to_hex().as_bytes());
            hasher.update(&[0x0a]);
        }
    }
    hasher.finalize().to_hex().to_string()
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../surface-corpus/examples")
}

/// Root of the immutable B1 `.sexp`/`.outcome` anchors.
fn fixture_root() -> PathBuf
{
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/corpus")
}
