//! Corpus discovery and the `check`, `build`, and `fmt` orchestration.
//!
//! Discovery globs `*.xml` under the spec directory in sorted order; `check`
//! runs the full parse-validate pass; `build` renders the no-JavaScript `HTML`
//! only when the corpus conforms; `format_paths` applies canonical formatting.

use alloc::string::String;
use alloc::vec::Vec;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use crate::Diagnostic;
use crate::DocError;
use crate::bibliography;
use crate::bibliography::Bibliography;
use crate::doc::model::DocRecord;
use crate::doc::parse::parse_doc_document;
use crate::doc::validate::DocCorpus;
use crate::doc::validate::validate_doc_corpus;
use crate::format;
use crate::model::Document;
use crate::parse;
use crate::render;
use crate::render::RenderContext;
use crate::typst_leaf;
use crate::validate;
use crate::validate::Corpus;

/// Outcome of a `check` run over the corpus.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CheckReport
{
    /// Number of components parsed into the model.
    pub component_count: usize,
    /// Every diagnostic (sorted); empty means the corpus conforms.
    pub diagnostics: Vec<Diagnostic>,
}

/// Outcome of a `build` run over the corpus.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct BuildReport
{
    /// The check outcome; a build only writes pages when it is clean.
    pub check: CheckReport,
    /// Paths of the pages written.
    pub pages: Vec<PathBuf>,
    /// Leaf-compilation notes (for example, an unavailable typst tool).
    pub notes: Vec<String>,
}

/// Discover the component files under a spec directory, sorted.
///
/// # Errors
/// Returns [`DocError::Io`] when the directory cannot be read.
#[inline]
pub fn discover_component_files(spec_dir: &Path) -> Result<Vec<PathBuf>, DocError>
{
    let entries = std::fs::read_dir(spec_dir).map_err(|source| DocError::Io {
        path: spec_dir.to_path_buf(),
        source,
    })?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| DocError::Io {
            path: spec_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(OsStr::to_str) == Some("xml") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Parse every component and validate the corpus, returning the typed corpus
/// and sorted diagnostics.
///
/// # Errors
/// Returns [`DocError`] on a filesystem, `XML`, or `YAML` failure.
fn assemble(
    spec_dir: &Path,
    refs_path: &Path,
) -> Result<(Corpus, Vec<Diagnostic>), DocError>
{
    let mut diagnostics = Vec::new();
    let mut documents: Vec<Document> = Vec::new();
    for file in discover_component_files(spec_dir)? {
        let text = std::fs::read_to_string(&file).map_err(|source| DocError::Io {
            path: file.clone(),
            source,
        })?;
        let parsed = parse::parse_document(&file, &text)?;
        diagnostics.extend(parsed.diagnostics);
        if let Some(document) = parsed.document {
            documents.push(document);
        }
    }
    let references = bibliography::load(refs_path)?;
    let corpus = Corpus::new(documents, references);
    diagnostics.extend(validate::validate_corpus(&corpus));
    diagnostics.sort();
    Ok((corpus, diagnostics))
}

/// Parse and validate the whole corpus.
///
/// # Errors
/// Returns [`DocError`] on a filesystem, `XML`, or `YAML` failure.
#[inline]
pub fn check(
    spec_dir: &Path,
    refs_path: &Path,
) -> Result<CheckReport, DocError>
{
    let (corpus, diagnostics) = assemble(spec_dir, refs_path)?;
    Ok(CheckReport {
        component_count: corpus.documents.len(),
        diagnostics,
    })
}

/// Build the no-JavaScript `HTML` corpus into an output directory.
///
/// The build refuses to write pages when the corpus does not conform; the
/// returned report then carries the diagnostics and no pages.
///
/// # Errors
/// Returns [`DocError`] on a filesystem, `XML`, or `YAML` failure.
#[inline]
pub fn build(
    spec_dir: &Path,
    refs_path: &Path,
    out_dir: &Path,
) -> Result<BuildReport, DocError>
{
    let (corpus, diagnostics) = assemble(spec_dir, refs_path)?;
    let check = CheckReport {
        component_count: corpus.documents.len(),
        diagnostics,
    };
    if !check.diagnostics.is_empty() {
        return Ok(BuildReport {
            check,
            pages: Vec::new(),
            notes: Vec::new(),
        });
    }
    let (pages, notes) = write_pages(&corpus.documents, &corpus.references, out_dir)?;
    Ok(BuildReport {
        check,
        pages,
        notes,
    })
}

/// Render and write every page, returning the written paths and any notes.
fn write_pages(
    documents: &[Document],
    references: &Bibliography,
    out_dir: &Path,
) -> Result<(Vec<PathBuf>, Vec<String>), DocError>
{
    let cache_dir = typst_leaf::default_cache_dir(out_dir);
    std::fs::create_dir_all(&cache_dir).map_err(|source| DocError::Io {
        path: cache_dir.clone(),
        source,
    })?;
    let terms = render::term_map(documents);
    let anchors = render::anchor_map(documents);
    let ctx = RenderContext::new(&cache_dir, &terms, &anchors, references);
    let mut notes = Vec::new();
    let mut pages = Vec::new();
    let index_path = out_dir.join("index.html");
    write_file(&index_path, &render::render_index(documents))?;
    pages.push(index_path);
    for document in documents {
        let html = render::render_document(document, &ctx, &mut notes)?;
        let page_path = out_dir.join(format!("{}.html", document.id));
        write_file(&page_path, &html)?;
        pages.push(page_path);
    }
    Ok((pages, notes))
}

/// Write a file, creating parent directories as needed.
fn write_file(
    path: &Path,
    contents: &str,
) -> Result<(), DocError>
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| DocError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, contents).map_err(|source| DocError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Apply canonical formatting to the given files, returning the changed ones.
///
/// # Errors
/// Returns [`DocError`] on a filesystem or `XML` failure.
#[inline]
pub fn format_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, DocError>
{
    let mut changed = Vec::new();
    for path in paths {
        if format::format_file(path)? {
            changed.push(path.clone());
        }
    }
    Ok(changed)
}

/// Outcome of a `check-docs` run over the prose document classes.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
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
        let parsed = parse_doc_document(&file, &text)?;
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
