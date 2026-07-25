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

/// Borrowed source for one math leaf.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct MathSource<'source>(&'source str);

impl<'source> From<&'source str> for MathSource<'source>
{
    #[inline]
    fn from(source: &'source str) -> Self
    {
        Self(source)
    }
}

/// Borrowed premise text for one inference-rule leaf.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RulePremises<'source>(&'source str);

impl<'source> From<&'source str> for RulePremises<'source>
{
    #[inline]
    fn from(source: &'source str) -> Self
    {
        Self(source)
    }
}

/// Borrowed conclusion text for one inference-rule leaf.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct RuleConclusion<'source>(&'source str);

impl<'source> From<&'source str> for RuleConclusion<'source>
{
    #[inline]
    fn from(source: &'source str) -> Self
    {
        Self(source)
    }
}

/// Borrowed source for one diagram leaf.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct DiagramSource<'source>(&'source str);

impl<'source> From<&'source str> for DiagramSource<'source>
{
    #[inline]
    fn from(source: &'source str) -> Self
    {
        Self(source)
    }
}

/// Complete generated Typst document source.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct TypstDocument<'source>(&'source str);

impl<'source> From<&'source str> for TypstDocument<'source>
{
    #[inline]
    fn from(source: &'source str) -> Self
    {
        Self(source)
    }
}

/// Whether the Typst process completed successfully.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TypstSucceeded(bool);

impl From<TypstSucceeded> for bool
{
    #[inline]
    fn from(succeeded: TypstSucceeded) -> Self
    {
        succeeded.0
    }
}

/// Compile an inline or display math leaf to inline `SVG`.
///
/// # Errors
/// Returns [`DocError::Io`] when the cache directory or a temporary file cannot
/// be written.
#[inline]
pub fn compile_math(
    source: MathSource<'_>,
    cache_dir: &Path,
) -> Result<Leaf, DocError>
{
    let document = format!("{PRELUDE}\n$ {} $\n", source.0);
    compile_typst(document.as_str().into(), cache_dir)
}

/// Compile an inference rule (premises over a conclusion) to inline `SVG`.
///
/// # Errors
/// Returns [`DocError::Io`] when the cache directory or a temporary file cannot
/// be written.
#[inline]
pub fn compile_rule(
    premises: RulePremises<'_>,
    conclusion: RuleConclusion<'_>,
    cache_dir: &Path,
) -> Result<Leaf, DocError>
{
    let document = format!("{PRELUDE}\n$ frac({}, {}) $\n", premises.0, conclusion.0);
    compile_typst(document.as_str().into(), cache_dir)
}

/// Compile a diagram leaf (fletcher source) to inline `SVG`.
///
/// # Errors
/// Returns [`DocError::Io`] when the cache directory or a temporary file cannot
/// be written.
#[inline]
pub fn compile_diagram(
    source: DiagramSource<'_>,
    cache_dir: &Path,
) -> Result<Leaf, DocError>
{
    let document = format!("{DIAGRAM_HEADER}\n{}\n", source.0);
    compile_typst(document.as_str().into(), cache_dir)
}

/// Compile a full typst document to inline `SVG`, using the content-hash cache.
fn compile_typst(
    document: TypstDocument<'_>,
    cache_dir: &Path,
) -> Result<Leaf, DocError>
{
    std::fs::create_dir_all(cache_dir).map_err(|source| DocError::Io {
        path: cache_dir.to_path_buf(),
        source,
    })?;
    let digest = blake3::hash(document.0.as_bytes()).to_hex();
    let stem = digest.as_str();
    let input = cache_dir.join(format!("{stem}.typ"));
    let output = cache_dir.join(format!("{stem}.svg"));
    if output.exists() {
        return read_svg(&output);
    }
    std::fs::write(&input, document.0).map_err(|source| DocError::Io {
        path: input.clone(),
        source,
    })?;
    match run_typst(cache_dir, &input, &output) {
        | Ok(succeeded) if bool::from(succeeded) => read_svg(&output),
        | Ok(_) => Ok(Leaf::Missing(String::from(
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
) -> Result<TypstSucceeded, String>
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
        | Ok(result) => Ok(TypstSucceeded(result.status.success())),
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
