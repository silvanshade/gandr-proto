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

/// Parse every component and validate the corpus, returning the documents and
/// the sorted diagnostics.
///
/// # Errors
/// Returns [`DocError`] on a filesystem, `XML`, or `YAML` failure.
fn assemble(
    spec_dir: &Path,
    refs_path: &Path,
) -> Result<(Vec<Document>, Vec<Diagnostic>), DocError>
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
    let reference_keys = validate::load_reference_keys(refs_path)?;
    let corpus = Corpus::new(documents, reference_keys);
    diagnostics.extend(validate::validate_corpus(&corpus));
    diagnostics.sort();
    Ok((corpus.documents, diagnostics))
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
    let (documents, diagnostics) = assemble(spec_dir, refs_path)?;
    Ok(CheckReport {
        component_count: documents.len(),
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
    let (documents, diagnostics) = assemble(spec_dir, refs_path)?;
    let check = CheckReport {
        component_count: documents.len(),
        diagnostics,
    };
    if !check.diagnostics.is_empty() {
        return Ok(BuildReport {
            check,
            pages: Vec::new(),
            notes: Vec::new(),
        });
    }
    let (pages, notes) = write_pages(&documents, out_dir)?;
    Ok(BuildReport {
        check,
        pages,
        notes,
    })
}

/// Render and write every page, returning the written paths and any notes.
fn write_pages(
    documents: &[Document],
    out_dir: &Path,
) -> Result<(Vec<PathBuf>, Vec<String>), DocError>
{
    let cache_dir = typst_leaf::default_cache_dir(out_dir);
    std::fs::create_dir_all(&cache_dir).map_err(|source| DocError::Io {
        path: cache_dir.clone(),
        source,
    })?;
    let terms = render::term_map(documents);
    let ctx = RenderContext::new(&cache_dir, &terms);
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
