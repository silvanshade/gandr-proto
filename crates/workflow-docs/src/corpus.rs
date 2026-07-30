//! Document-class discovery and the `check-docs`/`fmt` orchestration.
//!
//! Discovery globs `*.xml` under the prose-document roots in sorted order;
//! `check_docs` runs the full parse-validate pass over the classes;
//! `format_paths` applies canonical formatting.

use alloc::vec::Vec;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use crate::Diagnostic;
use crate::DocError;
use crate::bibliography;
use crate::doc::model::DocRecord;
use crate::doc::parse::parse_doc_document;
use crate::doc::validate::DocCorpus;
use crate::doc::validate::validate_doc_corpus;
use crate::format;

/// Apply canonical formatting to the given files, returning the changed ones.
///
/// # Errors
/// Returns [`DocError`] on a filesystem or `XML` failure.
#[inline]
pub fn format_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, DocError>
{
    let mut changed = Vec::new();
    for path in paths {
        if bool::from(format::format_file(path)?) {
            changed.push(path.clone());
        }
    }
    Ok(changed)
}

/// Outcome of a `check-docs` run over the prose document classes.
#[derive(Clone, Debug, Default)]
pub struct DocCheckReport
{
    /// Number of prose documents parsed into the model.
    pub record_count: usize,
    /// Every diagnostic (sorted); empty means the corpus conforms.
    pub diagnostics: Vec<Diagnostic>,
}

/// The directories that hold prose-document `XML`, relative to a workspace
/// root.
///
/// The research and workflow roots are fixed; the crate-status root is every
/// `crates/*/docs` directory that exists. Nonexistent roots are skipped, so a
/// fresh tree with no migrated `XML` yet is not an error.
///
/// # Errors
/// Returns [`DocError::Io`] when the `crates` directory cannot be read.
#[inline]
pub fn doc_class_dirs(workspace_root: &Path) -> Result<Vec<PathBuf>, DocError>
{
    let mut dirs = alloc::vec![
        workspace_root.join("docs/research"),
        workspace_root.join("docs/workflow"),
    ];
    let crates_root = workspace_root.join("crates");
    if crates_root.is_dir() {
        let entries = std::fs::read_dir(&crates_root).map_err(|source| DocError::Io {
            path: crates_root.clone(),
            source,
        })?;
        let mut crate_docs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| DocError::Io {
                path: crates_root.clone(),
                source,
            })?;
            let docs_dir = entry.path().join("docs");
            if docs_dir.is_dir() {
                crate_docs.push(docs_dir);
            }
        }
        crate_docs.sort();
        dirs.extend(crate_docs);
    }
    Ok(dirs)
}

/// Discover the prose-document files under the given directories, sorted.
///
/// Nonexistent directories are skipped; only `*.xml` files are returned.
///
/// # Errors
/// Returns [`DocError::Io`] when a directory that exists cannot be read.
#[inline]
pub fn discover_doc_files(dirs: &[PathBuf]) -> Result<Vec<PathBuf>, DocError>
{
    let mut files = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let entries = std::fs::read_dir(dir).map_err(|source| DocError::Io {
            path: dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| DocError::Io {
                path: dir.clone(),
                source,
            })?;
            let path = entry.path();
            if path.extension().and_then(OsStr::to_str) == Some("xml") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Parse and validate every prose document under a workspace root.
///
/// # Errors
/// Returns [`DocError`] on a filesystem, `XML`, or `YAML` failure.
#[inline]
pub fn check_docs(
    workspace_root: &Path,
    refs_path: &Path,
) -> Result<DocCheckReport, DocError>
{
    let dirs = doc_class_dirs(workspace_root)?;
    let files = discover_doc_files(&dirs)?;
    let mut diagnostics = Vec::new();
    let mut records: Vec<DocRecord> = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).map_err(|source| DocError::Io {
            path: file.clone(),
            source,
        })?;
        let parsed = parse_doc_document(&file, text.as_str().into())?;
        diagnostics.extend(parsed.diagnostics);
        if let Some(record) = parsed.document {
            records.push(record);
        }
    }
    let references = bibliography::load(refs_path)?;
    let corpus = DocCorpus::new(records, references.key_set());
    diagnostics.extend(validate_doc_corpus(&corpus));
    diagnostics.sort();
    Ok(DocCheckReport {
        record_count: corpus.records.len(),
        diagnostics,
    })
}
