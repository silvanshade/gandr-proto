//! Math and diagram leaf compilation to `SVG` via the pinned typst tool.
//!
//! Integration choice (deviation, decision `gandr-fcw.8`): the tool shells out
//! to the pinned `typst` command-line binary rather than linking
//! `typst-as-library`. Two reasons: the pinned `typst` `0.15` build has no
//! `MathML` `HTML` export (that needs a feature the pin lacks), so math renders
//! to per-equation `SVG` (the blessed fallback); and shelling out keeps the
//! regex family out of the default cargo graph entirely, satisfying the
//! forbidden-default-graph gate without feature-gating a linked library.
//!
//! Each leaf is content-addressed by the `blake3` hash of its full typst
//! source, so a compiled `SVG` is cached under `cache_dir` and reused.

use alloc::string::String;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::DocError;

/// The single house semantic-macro prelude prepended to every math leaf.
///
/// Kept intentionally small for the skeleton; it carries the notation the early
/// spot-check exercises (`mu`-tilde is builtin, cut brackets, an inference
/// helper).
const PRELUDE: &str = r#"#set page(width: auto, height: auto, margin: 2pt)
#set text(size: 11pt)
// House notation macros (semantic prelude).
#let cut(x) = $lr(⟦ #x ⟧)$
#let sem(x) = $lr(⟦ #x ⟧)$
"#;

/// Import header for diagram leaves (fletcher commutative diagrams).
const DIAGRAM_HEADER: &str = r#"#set page(width: auto, height: auto, margin: 4pt)
#import "@preview/fletcher:0.5.8" as fletcher: diagram, node, edge
"#;

/// Outcome of compiling one leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Leaf
{
    /// Compilation succeeded; the payload is inline `SVG` markup.
    Svg(String),
    /// Compilation was skipped or failed; the payload explains why.
    Missing(String),
}

/// Compile an inline or display math leaf to inline `SVG`.
///
/// # Errors
/// Returns [`DocError::Io`] when the cache directory or a temporary file cannot
/// be written.
#[inline]
pub fn compile_math(
    source: &str,
    cache_dir: &Path,
) -> Result<Leaf, DocError>
{
    let document = format!("{PRELUDE}\n$ {source} $\n");
    compile_typst(&document, cache_dir)
}

/// Compile an inference rule (premises over a conclusion) to inline `SVG`.
///
/// # Errors
/// Returns [`DocError::Io`] when the cache directory or a temporary file cannot
/// be written.
#[inline]
pub fn compile_rule(
    premises: &str,
    conclusion: &str,
    cache_dir: &Path,
) -> Result<Leaf, DocError>
{
    let document = format!("{PRELUDE}\n$ frac({premises}, {conclusion}) $\n");
    compile_typst(&document, cache_dir)
}

/// Compile a diagram leaf (fletcher source) to inline `SVG`.
///
/// # Errors
/// Returns [`DocError::Io`] when the cache directory or a temporary file cannot
/// be written.
#[inline]
pub fn compile_diagram(
    source: &str,
    cache_dir: &Path,
) -> Result<Leaf, DocError>
{
    let document = format!("{DIAGRAM_HEADER}\n{source}\n");
    compile_typst(&document, cache_dir)
}

/// Compile a full typst document to inline `SVG`, using the content-hash cache.
fn compile_typst(
    document: &str,
    cache_dir: &Path,
) -> Result<Leaf, DocError>
{
    std::fs::create_dir_all(cache_dir).map_err(|source| DocError::Io {
        path: cache_dir.to_path_buf(),
        source,
    })?;
    let digest = blake3::hash(document.as_bytes()).to_hex();
    let stem = digest.as_str();
    let input = cache_dir.join(format!("{stem}.typ"));
    let output = cache_dir.join(format!("{stem}.svg"));
    if output.exists() {
        return read_svg(&output);
    }
    std::fs::write(&input, document).map_err(|source| DocError::Io {
        path: input.clone(),
        source,
    })?;
    match run_typst(cache_dir, &input, &output) {
        | Ok(true) => read_svg(&output),
        | Ok(false) => Ok(Leaf::Missing(String::from(
            "typst compile failed (see stderr); leaf rendered as a source placeholder",
        ))),
        | Err(reason) => Ok(Leaf::Missing(reason)),
    }
}

/// Run the pinned typst tool, returning whether it succeeded.
fn run_typst(
    root: &Path,
    input: &Path,
    output: &Path,
) -> Result<bool, String>
{
    let invocation = Command::new("typst")
        .arg("compile")
        .arg("--root")
        .arg(root)
        .arg("-f")
        .arg("svg")
        .arg(input)
        .arg(output)
        .output();
    match invocation {
        | Ok(result) => Ok(result.status.success()),
        | Err(error) => Err(format!(
            "typst tool unavailable ({error}); leaf rendered as a source placeholder"
        )),
    }
}

/// Read compiled `SVG` and strip any leading `XML` declaration for inlining.
fn read_svg(output: &Path) -> Result<Leaf, DocError>
{
    let raw = std::fs::read_to_string(output).map_err(|source| DocError::Io {
        path: output.to_path_buf(),
        source,
    })?;
    let inline = raw
        .find("<svg")
        .and_then(|start| raw.get(start ..))
        .unwrap_or(&raw);
    Ok(Leaf::Svg(inline.trim().to_owned()))
}

/// Default cache directory for compiled leaves under a build output directory.
#[inline]
#[must_use]
pub fn default_cache_dir(build_dir: &Path) -> PathBuf
{
    build_dir.join("assets")
}
