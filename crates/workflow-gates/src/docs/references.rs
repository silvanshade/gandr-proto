//! Markdown reference-integrity checks for registered documentation nodes.
//!
//! This module retains documentation-reference behavior: manifest edge anchors
//! must resolve, in-body `ADR-N` references must have split ADR record files,
//! and in-body `§N(.M)` references must name a section number defined by some
//! registered corpus document.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use crate::Finding;
use crate::GateError;
use crate::GateResult;
use crate::docs::manifest::ManifestContext;
use crate::docs::manifest::path_exists;
use crate::support;

crate::semantic_str!(pub struct TargetRelText);
crate::semantic_str!(pub struct SlugText);
crate::semantic_str!(pub struct LineText);
crate::semantic_copy!(pub struct MinHashesCount(usize));
crate::semantic_copy!(pub struct MaxHashesCount(usize));
crate::semantic_str!(pub struct CandidateText);
crate::semantic_str!(pub struct NeedleText);
crate::semantic_str!(pub struct KindText);
crate::semantic_str!(pub struct NodesYamlText);
crate::semantic_str!(pub struct RelText);
crate::semantic_str!(pub struct TextText);
crate::semantic_str!(pub struct FragmentText);
crate::semantic_str!(pub struct PathText);
crate::semantic_str!(pub struct NameText);
crate::semantic_copy!(pub struct TargetHasSlugFlag(bool));
crate::semantic_copy!(pub struct DirectChildFlag(bool));
crate::semantic_optional_str!(pub struct OptionalHeadingTextText);
crate::semantic_copy!(pub struct TextFlag(bool));

/// Verify manifest edge anchors and Markdown ADR/section references.
///
/// # Contract
/// - requires: `manifest_path` points at the Gandr documentation manifest.
/// - ensures: returns deterministic dangling-reference findings in manifest
///   node order and source line order.
/// - provides: anchor, ADR, and section-number reference integrity for the
///   registered documentation corpus.
/// - fails: returns [`GateError`] for manifest, filesystem, UTF-8, or traversal
///   failures that would make the gate vacuous or incomplete.
/// - panics: none.
/// - intension: registered files are scanned linearly; section numbers are
///   first-seen unique values matching the retained gate's traversal order.
///
/// # Errors
/// Returns loader errors from [`ManifestContext::load`], I/O errors for present
/// but unreadable registered files, and traversal errors while listing ADR
/// records.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — edge-anchor, ADR reference, section-reference,
///   clean-reference, and missing-body branches are separated by exact fixture
///   manifests and Markdown bodies.
/// - witness: `references::tests::dangling_edge_adr_and_section_refs_are_reported`
#[inline]
pub fn run_reference_integrity(manifest_path: &Path) -> GateResult
{
    let context = ManifestContext::load(manifest_path)?;
    let documents = document_infos(&context)?;
    let sections = all_sections(&documents);
    let adrs = adr_numbers(context.corpus_dir())?;
    let mut findings = Vec::new();

    collect_edge_anchor_findings(&context, &documents, &adrs, &mut findings);
    collect_body_reference_findings(&documents, &sections, &adrs, &mut findings)?;

    Ok(findings)
}

/// Read and summarize every registered document.
fn document_infos(context: &ManifestContext) -> Result<Vec<DocumentInfo>, GateError>
{
    let mut documents = Vec::new();
    for node in context.nodes() {
        let file_path = context.resolve_path(node.path());
        let lines = read_doc_lines(&file_path)?;
        let mut sections = Vec::new();
        let mut slugs = Vec::new();
        for line in &lines {
            if let Some(section) = section_heading(line) {
                sections.push(section);
            }
            if let Some(slug) = heading_slug(line) {
                slugs.push(slug);
            }
        }
        documents.push(DocumentInfo {
            rel: String::from(node.path().as_str().into().0),
            basename: basename_or_path(node.path().as_str().into().0),
            lines,
            sections,
            slugs,
        });
    }
    return Ok(documents);
}

/// Read a registered corpus file's lines with the legacy absent/unreadable
/// split.
fn read_doc_lines(path: &Path) -> Result<Vec<String>, GateError>
{
    if !path_exists(path).map(|value| value.into().0)? {
        return Ok(Vec::new());
    }
    let source = support::read_utf8(path)?;
    let mut lines = Vec::new();
    for line in source.lines() {
        lines.push(String::from(line));
    }
    return Ok(lines);
}

/// Return unique section numbers in first-seen order.
fn all_sections(documents: &[DocumentInfo]) -> Vec<String>
{
    let mut sections = Vec::new();
    for document in documents {
        for section in &document.sections {
            push_unique_text(&mut sections, section);
        }
    }
    return sections;
}

/// Collect edge-anchor findings from manifest edges.
fn collect_edge_anchor_findings(
    context: &ManifestContext,
    documents: &[DocumentInfo],
    adrs: &[String],
    findings: &mut Vec<Finding>,
)
{
    for node in context.nodes() {
        for edge in node.edges() {
            let Some(target) = edge.split_target()
            else {
                continue;
            };
            let target_rel = target.path();
            let fragment = target.fragment();
            if let Some(adr_number) = anchored_adr_fragment(fragment.as_ref()) {
                if !contains_text(adrs, &adr_number).into().0 {
                    findings.push(ref_finding(RefFailure {
                        kind: "edge-anchor",
                        at: String::from(node.path().as_str().into().0),
                        reference: String::from(edge.to().into().0),
                        why: format!("ADR-{adr_number} has no docs/adr/ record file"),
                    }));
                }
            }
            else if !target_has_slug(documents, target_rel.as_ref(), fragment.as_ref())
                .into()
                .0
            {
                findings.push(ref_finding(RefFailure {
                    kind: "edge-anchor",
                    at: String::from(node.path().as_str().into().0),
                    reference: String::from(edge.to().into().0),
                    why: format!(
                        "no heading in {} with slug `{}`",
                        target_rel.as_ref(),
                        fragment.as_ref()
                    ),
                }));
            }
        }
    }
}

/// Return an ADR number when a fragment is exactly `adr-N`.
fn anchored_adr_fragment<'semantic>(fragment: impl Into<FragmentText<'semantic>>)
-> Option<String>
{
    let fragment = fragment.into().0;
    let suffix = fragment.strip_prefix("adr-")?;
    let digits = leading_ascii_digits(suffix);
    if digits.is_empty() || digits != suffix {
        return None;
    }
    return Some(digits);
}

/// Return whether `documents` contains `slug` for `target_rel`.
fn target_has_slug<'semantic>(
    documents: &[DocumentInfo],
    target_rel: impl Into<TargetRelText<'semantic>>,
    slug: impl Into<SlugText<'semantic>>,
) -> impl Into<TargetHasSlugFlag>
{
    let slug = slug.into().0;
    let target_rel = target_rel.into().0;
    for document in documents {
        if document.rel.as_str() == target_rel {
            return contains_text(&document.slugs, slug).into().0;
        }
    }
    return false;
}

/// Return all ADR numbers from direct `docs/adr/*.md` record filenames.
fn adr_numbers(corpus_dir: &Path) -> Result<Vec<String>, GateError>
{
    let Some(docs_dir) = corpus_dir.parent()
    else {
        return Ok(Vec::new());
    };
    let adr_dir = docs_dir.join("adr");
    if !path_exists(&adr_dir).map(|value| value.into().0)? {
        return Ok(Vec::new());
    }
    let files = support::walk_files(&adr_dir, OsStr::new("md"))?;
    let mut adrs = Vec::new();
    for file in files {
        if !is_direct_child(&adr_dir, &file).into().0 {
            continue;
        }
        if let Some(number) = adr_number_from_path(&file) {
            push_unique_text(&mut adrs, &number);
        }
    }
    return Ok(adrs);
}

/// Collect in-body ADR and section reference findings.
fn collect_body_reference_findings(
    documents: &[DocumentInfo],
    sections: &[String],
    adrs: &[String],
    findings: &mut Vec<Finding>,
) -> Result<(), GateError>
{
    for document in documents {
        for (line_index, line) in document.lines.iter().enumerate() {
            let line_number = line_index
                .checked_add(1_usize)
                .ok_or_else(|| GateError::operational("documentation line number overflowed"))?;
            let at = format!("{}:{line_number}", document.basename);
            for adr in adr_references(line) {
                if !contains_text(adrs, &adr).into().0 {
                    findings.push(ref_finding(RefFailure {
                        kind: "adr-ref",
                        at: String::from(at.as_str()),
                        reference: format!("ADR-{adr}"),
                        why: String::from("no matching docs/adr/NNNN-*.md record"),
                    }));
                }
            }
            for section in section_references(line) {
                if !contains_text(sections, &section).into().0 {
                    findings.push(ref_finding(RefFailure {
                        kind: "section-ref",
                        at: String::from(at.as_str()),
                        reference: format!("§{section}"),
                        why: String::from("section number defined in no corpus document"),
                    }));
                }
            }
        }
    }
    Ok(())
}

/// Return whether `path` is directly inside `dir`.
fn is_direct_child(
    dir: &Path,
    path: &Path,
) -> impl Into<DirectChildFlag>
{
    return path.parent().is_some_and(|parent| parent == dir);
}

/// Parse an ADR number from an ADR record filename.
fn adr_number_from_path(path: &Path) -> Option<String>
{
    let file_name = path.file_name()?.to_string_lossy();
    let mut saw_digit = false;
    let mut significant = String::new();
    for character in file_name.chars() {
        if character == '-' {
            if !saw_digit {
                return None;
            }
            if significant.is_empty() {
                return Some(String::from("0"));
            }
            return Some(significant);
        }
        if !character.is_ascii_digit() {
            return None;
        }
        saw_digit = true;
        if character != '0' || !significant.is_empty() {
            significant.push(character);
        }
    }
    return None;
}

/// Parse a numbered Markdown heading section from `line`.
fn section_heading<'semantic>(line: impl Into<LineText<'semantic>>) -> Option<String>
{
    let line = line.into().0;
    let text = heading_text(line, 2_usize, 6_usize).into().0?;
    return leading_section_number(text);
}

/// Parse a GitHub-style heading slug from `line`.
fn heading_slug<'semantic>(line: impl Into<LineText<'semantic>>) -> Option<String>
{
    let line = line.into().0;
    let text = heading_text(line, 1_usize, 6_usize).into().0?;
    let lowercase = text.to_lowercase();
    let mut stripped = String::new();
    for character in lowercase.chars() {
        if character.is_alphanumeric()
            || character == '_'
            || character == '-'
            || character.is_whitespace()
        {
            stripped.push(character);
        }
    }
    return Some(collapse_slug_whitespace(stripped.trim()));
}

/// Return heading text after `#` markers when marker count is in range.
fn heading_text<'semantic>(
    line: impl Into<LineText<'semantic>>,
    min_hashes: impl Into<MinHashesCount>,
    max_hashes: impl Into<MaxHashesCount>,
) -> impl Into<OptionalHeadingTextText<'semantic>>
{
    let min_hashes = min_hashes.into().0;
    let max_hashes = max_hashes.into().0;
    let line = line.into().0;
    let mut remaining = line;
    let mut hashes = 0_usize;
    while let Some(after_hash) = remaining.strip_prefix('#') {
        hashes = hashes.saturating_add(1_usize);
        remaining = after_hash;
    }
    if hashes < min_hashes || hashes > max_hashes {
        return None;
    }
    let mut characters = remaining.chars();
    let first = characters.next()?;
    if !first.is_whitespace() {
        return None;
    }
    return Some(characters.as_str());
}

/// Collapse whitespace runs to single hyphens for a slug body.
fn collapse_slug_whitespace<'semantic>(text: impl Into<TextText<'semantic>>) -> String
{
    let text = text.into().0;
    let mut slug = String::new();
    let mut previous_whitespace = false;
    for character in text.chars() {
        if character.is_whitespace() {
            if !slug.is_empty() && !previous_whitespace {
                slug.push('-');
            }
            previous_whitespace = true;
        }
        else {
            slug.push(character);
            previous_whitespace = false;
        }
    }
    return slug;
}

/// Return all in-body `ADR-N` references on `line`.
fn adr_references<'semantic>(line: impl Into<LineText<'semantic>>) -> Vec<String>
{
    let line = line.into().0;
    let mut references = Vec::new();
    for suffix in line.split("ADR-").skip(1_usize) {
        let digits = leading_ascii_digits(suffix);
        if !digits.is_empty() {
            references.push(digits);
        }
    }
    return references;
}

/// Convert a dangling-reference record into a stable finding row.
fn ref_finding(failure: RefFailure) -> Finding
{
    Finding::new(
        "docs-reference-dangling",
        "docs",
        failure.at,
        failure.kind,
        format!("{} — {}", failure.reference, failure.why),
    )
}

/// Return leading ASCII digits from `text`.
fn leading_ascii_digits<'semantic>(text: impl Into<TextText<'semantic>>) -> String
{
    let text = text.into().0;
    let mut digits = String::new();
    for character in text.chars() {
        if !character.is_ascii_digit() {
            break;
        }
        digits.push(character);
    }
    return digits;
}

/// Return a leading section number matching the regex prefix `N(.M)*`.
fn leading_section_number<'semantic>(text: impl Into<TextText<'semantic>>) -> Option<String>
{
    let text = text.into().0;
    let mut number = String::new();
    let mut saw_digit = false;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character.is_ascii_digit() {
            number.push(character);
            saw_digit = true;
        }
        else if character == '.' && saw_digit {
            let next_is_digit = characters.peek().is_some_and(char::is_ascii_digit);
            if !next_is_digit {
                break;
            }
            number.push(character);
        }
        else {
            break;
        }
    }
    if !saw_digit {
        return None;
    }
    return Some(number);
}

/// Push `candidate` when it is not already present.
fn push_unique_text<'semantic>(
    values: &mut Vec<String>,
    candidate: impl Into<CandidateText<'semantic>>,
)
{
    let candidate = candidate.into().0;
    if !contains_text(values, candidate).into().0 {
        values.push(String::from(candidate));
    }
}

/// Return whether `values` contains `needle`.
fn contains_text<'semantic>(
    values: &[String],
    needle: impl Into<NeedleText<'semantic>>,
) -> impl Into<TextFlag>
{
    let needle = needle.into().0;
    return values.iter().any(|value| value == needle);
}

/// Return all in-body `§N(.M)` references on `line`.
fn section_references<'semantic>(line: impl Into<LineText<'semantic>>) -> Vec<String>
{
    let line = line.into().0;
    let mut references = Vec::new();
    for suffix in line.split('§').skip(1_usize) {
        if let Some(number) = leading_section_number(suffix) {
            references.push(number);
        }
    }
    return references;
}

/// Return the final path component or the input path text when absent.
fn basename_or_path<'semantic>(path: impl Into<PathText<'semantic>>) -> String
{
    let path = path.into().0;
    let path_buf = PathBuf::from(path);
    return path_buf.file_name().map_or_else(
        || String::from(path),
        |file_name| format!("{}", file_name.to_string_lossy()),
    );
}

/// Parsed information for one registered documentation file.
struct DocumentInfo
{
    /// Manifest-relative path.
    rel: String,
    /// Basename used by legacy line diagnostics.
    basename: String,
    /// Raw Markdown lines, empty when the file is absent.
    lines: Vec<String>,
    /// Numbered section definitions found in this file.
    sections: Vec<String>,
    /// GitHub-style heading slugs found in this file.
    slugs: Vec<String>,
}

/// Shape of one dangling reference before conversion to [`Finding`].
struct RefFailure
{
    /// Reference failure class.
    kind: &'static str,
    /// Source location or manifest node path.
    at: String,
    /// Reference text that failed to resolve.
    reference: String,
    /// Stable reason phrase.
    why: String,
}

#[cfg(test)]
mod tests
{
    //! Unit witnesses for documentation reference integrity.

    use alloc::format;
    use alloc::string::String;
    use core::error::Error;
    use std::path::Path;
    use std::path::PathBuf;

    use super::*;
    use crate::docs::manifest::REQUIRED_HASH_ALGORITHM;

    /// Test result used by filesystem fixture checks.
    type TestResult = Result<(), Box<dyn Error>>;

    /// A writable documentation fixture rooted in a temp directory.
    struct Fixture
    {
        /// Temp root path.
        root: PathBuf,
        /// Manifest path under the temp root.
        manifest: PathBuf,
        /// Corpus directory under the temp root.
        corpus: PathBuf,
        /// ADR directory under the temp root.
        adr: PathBuf,
    }

    /// Write a manifest with raw node YAML.
    fn write_manifest<'semantic>(
        manifest: &Path,
        nodes_yaml: impl Into<NodesYamlText<'semantic>>,
    ) -> Result<(), Box<dyn Error>>
    {
        let nodes_yaml = nodes_yaml.into().0;
        crate::support::HOST_FILESYSTEM.write(
            manifest,
            format!("version: 1\nhash: {REQUIRED_HASH_ALGORITHM}\nnodes:\n{nodes_yaml}"),
        )?;
        Ok(())
    }

    /// Dangling edge anchors, ADRs, and sections are all reported together.
    #[test]
    fn dangling_edge_adr_and_section_refs_are_reported() -> TestResult
    {
        let fixture = fixture("dangling")?;
        let source = "# Root\n\nSee ADR-999 and §7.2.\n";
        let root_hash = write_doc(&fixture.corpus, "root.md", source)?;
        let target_hash = write_doc(&fixture.corpus, "target.md", "# Different Heading\n")?;
        write_manifest(
            &fixture.manifest,
            &format!(
                "  - path: root.md\n    b3: {root_hash}\n    edges:\n      - rel: cites\n        to: target.md#missing-heading\n  - path: target.md\n    b3: {target_hash}\n    edges: []\n"
            ),
        )?;

        let findings = run_reference_integrity(&fixture.manifest)?;
        let edge = findings.iter().any(|finding| {
            finding.kind == "docs-reference-dangling"
                && finding.declaration == "edge-anchor"
                && finding.detail.contains("missing-heading")
        });
        let adr = findings.iter().any(|finding| {
            finding.kind == "docs-reference-dangling"
                && finding.declaration == "adr-ref"
                && finding.detail.contains("ADR-999")
        });
        let section = findings.iter().any(|finding| {
            finding.kind == "docs-reference-dangling"
                && finding.declaration == "section-ref"
                && finding.detail.contains("§7.2")
        });

        assert!(edge, "dangling manifest edge anchor should be reported");
        assert!(adr, "dangling ADR references should be reported");
        assert!(section, "dangling section references should be reported");
        assert!(
            fixture.root.exists(),
            "fixture root should remain available for failure inspection"
        );
        Ok(())
    }

    /// ADR filenames normalize zero padding before reference matching.
    #[test]
    fn adr_filename_numbers_resolve_references() -> TestResult
    {
        let fixture = fixture("adr")?;
        crate::support::HOST_FILESYSTEM.write(fixture.adr.join("0007-record.md"), "# ADR-7\n")?;
        let source = "# Root\n\nSee ADR-7.\n## 1 Section\nSee §1.\n";
        let root_hash = write_doc(&fixture.corpus, "root.md", source)?;
        write_manifest(
            &fixture.manifest,
            &format!(
                "  - path: root.md\n    b3: {root_hash}\n    edges:\n      - rel: self\n        to: root.md#root\n"
            ),
        )?;

        let findings = run_reference_integrity(&fixture.manifest)?;

        assert!(
            findings.is_empty(),
            "existing ADR records, slugs, and section numbers should resolve"
        );
        Ok(())
    }

    /// Write a corpus document and return its BLAKE3 hash.
    fn write_doc<'semantic>(
        corpus: &Path,
        rel: impl Into<RelText<'semantic>>,
        text: impl Into<TextText<'semantic>>,
    ) -> Result<String, Box<dyn Error>>
    {
        let text = text.into().0;
        let rel = rel.into().0;
        let path = corpus.join(rel);
        let Some(parent) = path.parent()
        else {
            return Err(Box::new(std::io::Error::other(
                "fixture path has no parent",
            )));
        };
        crate::support::HOST_FILESYSTEM.create_dir_all(parent)?;
        crate::support::HOST_FILESYSTEM.write(&path, text)?;
        Ok(format!("{}", blake3::hash(text.as_bytes()).to_hex()))
    }

    /// Reference parsers cover ADR, section, slug, and filename boundaries.
    #[test]
    fn reference_parsers_cover_boundary_filenames_and_sections()
    {
        assert_eq!(
            Some(String::from("hello-rust-gates")),
            heading_slug("# Hello,   Rust & Gates!")
        );
        assert_eq!(None, heading_slug("#No separating space"));
        assert_eq!(None, heading_slug("####### Too deep"));
        assert_eq!(
            Some(String::from("12.3")),
            section_heading("## 12.3 Nested Contract")
        );
        assert_eq!(None, section_heading("# 1 Top-level title"));
        assert_eq!(
            vec![String::from("007"), String::from("8")],
            adr_references("ADR-007, ADR-x, ADR-8.")
        );
        assert_eq!(
            vec![String::from("1"), String::from("2.03"), String::from("4")],
            section_references("see §1, §2.03, §bad, and §4.")
        );
        assert_eq!(
            Some(String::from("0")),
            adr_number_from_path(Path::new("0000-zero.md"))
        );
        assert_eq!(
            Some(String::from("7")),
            adr_number_from_path(Path::new("0007-record.md"))
        );
        assert_eq!(None, adr_number_from_path(Path::new("-missing-prefix.md")));
        assert_eq!(None, adr_number_from_path(Path::new("abc-record.md")));
        assert_eq!(None, adr_number_from_path(Path::new("0007")));
    }

    /// Missing bodies, ADR edge fragments, and nested ADR files are stable.
    #[test]
    fn missing_docs_and_adr_edge_fragments_are_reported_in_order() -> TestResult
    {
        let fixture = fixture("missing-and-adr-fragment")?;
        crate::support::HOST_FILESYSTEM.remove_dir_all(&fixture.adr)?;
        assert!(
            adr_numbers(&fixture.corpus)?.is_empty(),
            "absent ADR directories should be treated as an empty registry"
        );
        crate::support::HOST_FILESYSTEM.create_dir_all(fixture.adr.join("nested"))?;
        crate::support::HOST_FILESYSTEM
            .write(fixture.adr.join("nested/0041-record.md"), "# Ignored\n")?;
        let source = "# Root\n\nSee ADR-41 and §2.3.\n## 2 Existing\n";
        let root_hash = write_doc(&fixture.corpus, "root.md", source)?;
        write_manifest(
            &fixture.manifest,
            &format!(
                "  - path: root.md\n    b3: {root_hash}\n    edges:\n      - rel: cites\n        to: root.md#adr-42\n  - path: missing.md\n    b3: {root_hash}\n    edges: []\n"
            ),
        )?;

        let findings = run_reference_integrity(&fixture.manifest)?;
        let projection = findings
            .iter()
            .map(|finding| format!("{}:{}", finding.declaration, finding.detail))
            .collect::<Vec<_>>();

        assert_eq!(
            vec![
                String::from("edge-anchor:root.md#adr-42 — ADR-42 has no docs/adr/ record file"),
                String::from("adr-ref:ADR-41 — no matching docs/adr/NNNN-*.md record"),
                String::from("section-ref:§2.3 — section number defined in no corpus document"),
            ],
            projection
        );
        Ok(())
    }

    /// Build a clean fixture directory for `name`.
    fn fixture<'semantic>(name: impl Into<NameText<'semantic>>) -> Result<Fixture, Box<dyn Error>>
    {
        let name = name.into().0;
        let root = std::env::temp_dir().join(format!(
            "gandr-workflow-gates-docs-refs-{}-{name}",
            std::process::id()
        ));
        crate::support::HOST_FILESYSTEM.remove_dir_if_exists(&root)?;
        let corpus = root.join("docs/gandr");
        let adr = root.join("docs/adr");
        crate::support::HOST_FILESYSTEM.create_dir_all(&corpus)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&adr)?;
        let manifest = corpus.join("MANIFEST.yml");
        Ok(Fixture {
            root,
            manifest,
            corpus,
            adr,
        })
    }
}
