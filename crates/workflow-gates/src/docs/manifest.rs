//! Typed documentation manifest loading and BLAKE3 drift verification.
//!
//! This module retains documentation-manifest drift behavior: it rejects
//! vacuous or malformed manifests, verifies registered Markdown bytes in
//! process with BLAKE3,
//! reports unregistered Markdown files in sorted order, and computes stale
//! downstream context with an iterative reverse-edge walk.

extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
#[cfg(test)]
use core::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use yaml_rust2::yaml::Hash;
use yaml_rust2::yaml::Yaml;
use yaml_rust2::yaml::YamlLoader;

use crate::Finding;
use crate::GateError;
use crate::GateResult;
use crate::support;

crate::semantic_str!(pub struct SourceText);
crate::semantic_str!(pub struct KeyNameText);
crate::semantic_str!(pub struct DetailText);
crate::semantic_str!(pub struct RootText);
crate::semantic_str!(pub struct ActualText);
crate::semantic_str!(pub struct NeedleText);
crate::semantic_str!(pub struct ExpectedText);
crate::semantic_str!(pub struct NodesYamlText);
crate::semantic_str!(pub struct RelText);
crate::semantic_str!(pub struct TextText);
crate::semantic_str!(pub struct RawText);
crate::semantic_bytes!(pub struct BytesBytes);
crate::semantic_str!(pub struct NameText);
crate::semantic_str!(pub struct AsStrText);
crate::semantic_str!(pub struct B3Text);
crate::semantic_str!(pub struct ToText);
crate::semantic_str!(pub struct TargetPathText);
crate::semantic_str!(pub struct TargetFragmentText);

/// Parsed target path and fragment parts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetSplit<'target>
{
    /// Path before the target fragment marker.
    path: TargetPathText<'target>,
    /// Fragment after the target marker.
    fragment: TargetFragmentText<'target>,
}

impl<'target> TargetSplit<'target>
{
    /// Return the target path before the fragment marker.
    #[inline]
    #[must_use]
    pub fn path(&self) -> TargetPathText<'target>
    {
        self.path
    }

    /// Return the target fragment after the marker.
    #[inline]
    #[must_use]
    pub fn fragment(&self) -> TargetFragmentText<'target>
    {
        self.fragment
    }
}

crate::semantic_optional_str!(pub struct OptionalYamlStringText);
crate::semantic_copy!(pub struct ManifestPathIsCorpusRelativeFlag(bool));
crate::semantic_copy!(pub struct PathExistsFlag(bool));
crate::semantic_copy!(pub struct TextFlag(bool));
crate::semantic_str!(pub struct LabelText);

/// Default repository-relative Gandr documentation manifest path.
pub const DEFAULT_MANIFEST_PATH: &str = "docs/gandr/MANIFEST.yml";

/// Required manifest hash algorithm name.
pub const REQUIRED_HASH_ALGORITHM: &str = "blake3";

/// Loaded documentation manifest and its manifest-relative path context.
///
/// # Contract
/// - ensures: contains a nonempty, path-unique node list parsed from one YAML
///   document whose `registry.hash` is `blake3`.
/// - provides: the shared path model consumed by drift and reference-integrity
///   gates.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — clean, empty-node, wrong-hash, duplicate-path,
///   and malformed-edge manifests are separated by exact loader outcomes.
/// - witness: `manifest::tests::clean_manifest_has_no_drift_findings`
/// - witness: `manifest::tests::empty_manifest_nodes_fail_loudly`
#[derive(Clone)]
pub struct ManifestContext
{
    /// Path of the manifest that was loaded.
    manifest_path: PathBuf,
    /// Directory used as the root for manifest-relative node paths.
    corpus_dir: PathBuf,
    /// Nonempty registered documentation nodes, in manifest order.
    nodes: Vec<ManifestNode>,
}

impl ManifestContext
{
    /// Load and validate a documentation manifest from `manifest_path`.
    ///
    /// # Contract
    /// - requires: `manifest_path` names the YAML manifest to inspect.
    /// - ensures: returns a typed context only for a single-document YAML
    ///   manifest with `registry.hash = blake3`, at least one node, unique
    ///   nonempty node paths, nonempty node hashes, and well-formed edges.
    /// - provides: one manifest loader shared by drift and reference gates.
    /// - fails: returns [`GateError`] when the manifest is absent, unreadable,
    ///   unparseable, vacuous, uses the wrong hash algorithm, or has malformed
    ///   node or edge records.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns I/O errors from manifest reads and operational errors for YAML
    /// parse or shape violations.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — absent, unparseable, empty-node,
    ///   wrong-hash, and duplicate-path fixtures each force a distinct loader
    ///   branch before any downstream gate can silently pass.
    /// - witness: `manifest::tests::clean_manifest_has_no_drift_findings`
    /// - witness: `manifest::tests::empty_manifest_nodes_fail_loudly`
    #[inline]
    pub fn load(manifest_path: &Path) -> Result<Self, GateError>
    {
        if !path_exists(manifest_path).map(|value| value.into().0)? {
            return Err(GateError::operational(format!(
                "manifest not found: {}",
                manifest_path.display()
            )));
        }

        let manifest_source = support::read_utf8(manifest_path)?;
        let document = manifest_document(manifest_path, &manifest_source)?;
        let manifest = yaml_hash(
            manifest_path,
            &document,
            "manifest root must be a mapping with hash and nodes",
        )?;
        let algorithm = yaml_mapping_value(manifest, "hash")
            .and_then(|value| yaml_string(value).into().0)
            .unwrap_or("");
        if algorithm != REQUIRED_HASH_ALGORITHM {
            return Err(GateError::operational(format!(
                "unsupported manifest hash algorithm: {algorithm} (expected blake3)"
            )));
        }

        let nodes_yaml = yaml_mapping_value(manifest, "nodes")
            .ok_or_else(|| empty_nodes_error(manifest_path))?;
        let node_values = yaml_array(manifest_path, nodes_yaml, "manifest nodes must be a list")?;
        if node_values.is_empty() {
            return Err(empty_nodes_error(manifest_path));
        }

        let corpus_dir = manifest_parent(manifest_path);
        let mut nodes = Vec::new();
        let mut seen_paths = BTreeSet::new();
        for node_yaml in node_values {
            let node = parse_node(manifest_path, node_yaml)?;
            ensure_manifest_path_stays_in_scope(&corpus_dir, node.path())?;
            if !seen_paths.insert(String::from(node.path.as_str().into().0)) {
                return Err(GateError::operational(
                    "corpus node paths must be unique (the heading index keys by manifest-relative path)",
                ));
            }
            nodes.push(node);
        }

        Ok(Self {
            manifest_path: manifest_path.to_path_buf(),
            corpus_dir,
            nodes,
        })
    }

    /// Return the loaded manifest path.
    #[inline]
    #[must_use]
    pub fn manifest_path(&self) -> &Path
    {
        return &self.manifest_path;
    }

    /// Return the directory against which manifest node paths are resolved.
    #[inline]
    #[must_use]
    pub fn corpus_dir(&self) -> &Path
    {
        return &self.corpus_dir;
    }

    /// Return registered manifest nodes in manifest order.
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> &[ManifestNode]
    {
        return &self.nodes;
    }

    /// Resolve a manifest-relative node path against the corpus directory.
    ///
    /// # Contract
    /// - requires: `path` is a manifest node path from this context.
    /// - ensures: returns the filesystem path obtained by joining `path` to the
    ///   manifest directory without canonicalizing or normalizing traversal.
    /// - provides: the script-compatible path model shared by documentation
    ///   gates.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn resolve_path(
        &self,
        path: &ManifestPath,
    ) -> PathBuf
    {
        return path.join_to(self.corpus_dir());
    }

    /// Return registered node paths as a sorted set of manifest strings.
    ///
    /// # Contract
    /// - ensures: each registered path appears exactly once in lexicographic
    ///   order.
    /// - provides: deterministic membership testing for unregistered Markdown
    ///   detection.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn registered_path_set(&self) -> BTreeSet<String>
    {
        let mut paths = BTreeSet::new();
        for node in self.nodes() {
            paths.insert(String::from(node.path().as_str().into().0));
        }
        return paths;
    }
}

/// Manifest-relative document path validated to be nonempty.
///
/// # Contract
/// - ensures: the inner string is nonempty and is not canonicalized.
/// - provides: a typed boundary for paths stored in `MANIFEST.yml`.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — empty and nonempty path entries distinguish the
///   only validation branch before downstream path joins occur.
/// - witness: `manifest::tests::empty_manifest_nodes_fail_loudly`
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ManifestPath
{
    /// Raw manifest path text.
    raw: String,
}

impl ManifestPath
{
    /// Build a manifest path from a YAML string field.
    ///
    /// # Contract
    /// - ensures: returns a path wrapper preserving `raw` exactly when it is
    ///   nonempty.
    /// - provides: shared validation for node paths.
    /// - fails: returns a gate error for empty strings.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational error when `raw` is empty.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — an empty path and an ordinary path separate
    ///   the only validation branch.
    /// - witness: `manifest::tests::empty_manifest_nodes_fail_loudly`
    #[inline]
    pub fn new<'semantic, Raw>(raw: Raw) -> Result<Self, GateError>
    where
        Raw: Into<RawText<'semantic>>,
    {
        let raw = raw.into().0;
        if raw.is_empty() {
            return Err(GateError::operational("Node.path must be non-empty"));
        }
        if !manifest_path_is_corpus_relative(raw).into().0 {
            return Err(GateError::operational(
                "Node.path must be relative manifest text without absolute or `.` components",
            ));
        }
        Ok(Self {
            raw: String::from(raw),
        })
    }

    /// Return the manifest path text.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> impl Into<AsStrText<'_>>
    {
        return &self.raw;
    }

    /// Join the manifest path to `root` without canonicalization.
    ///
    /// # Contract
    /// - requires: `root` is the manifest directory for this path.
    /// - ensures: returns `root.join(self.as_str().into().0)` and preserves
    ///   traversal semantics exactly as the retained gate did.
    /// - provides: one shared path join for documentation gates.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn join_to(
        &self,
        root: &Path,
    ) -> PathBuf
    {
        return root.join(self.as_str().into().0);
    }
}

/// Registered documentation node from the manifest.
///
/// # Contract
/// - ensures: `path` and `b3` are nonempty and `edges` contains only nonempty
///   relation/target pairs.
/// - provides: shared manifest node data for drift and reference checks.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — clean, missing-file, drifted-file, and
///   malformed-edge fixtures distinguish every consumed field.
/// - witness: `manifest::tests::clean_manifest_has_no_drift_findings`
/// - witness: `manifest::tests::drift_missing_and_unregistered_docs_are_reported`
#[derive(Clone)]
pub struct ManifestNode
{
    /// Manifest-relative path of this documentation node.
    path: ManifestPath,
    /// Expected BLAKE3 hex digest from the manifest.
    b3: String,
    /// Manifest provenance edges owned by this node.
    edges: Vec<ManifestEdge>,
}

impl ManifestNode
{
    /// Return the node path.
    #[inline]
    #[must_use]
    pub fn path(&self) -> &ManifestPath
    {
        return &self.path;
    }

    /// Return the expected BLAKE3 digest string.
    #[inline]
    #[must_use]
    pub fn b3(&self) -> impl Into<B3Text<'_>>
    {
        return &self.b3;
    }

    /// Return manifest edges in manifest order.
    #[inline]
    #[must_use]
    pub fn edges(&self) -> &[ManifestEdge]
    {
        return &self.edges;
    }
}

/// Registered manifest edge with nonempty relation and target fields.
///
/// # Contract
/// - ensures: `rel` and `to` are nonempty and retain manifest text exactly.
/// - provides: a typed edge record shared by drift reverse traversal and anchor
///   integrity checks.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — nonempty, empty-relation, and empty-target edge
///   records distinguish all loader branches.
/// - witness: `references::tests::dangling_edge_adr_and_section_refs_are_reported`
#[derive(Clone, Eq, PartialEq)]
pub struct ManifestEdge
{
    /// Nonempty manifest relation label.
    rel: String,
    /// Nonempty manifest target path, optionally with a `#fragment` suffix.
    to: String,
}

impl ManifestEdge
{
    /// Return the edge relation label.
    #[inline]
    #[must_use]
    pub fn rel(&self) -> impl Into<RelText<'_>>
    {
        return &self.rel;
    }

    /// Return the raw edge target.
    #[inline]
    #[must_use]
    pub fn to(&self) -> impl Into<ToText<'_>>
    {
        return &self.to;
    }

    /// Return the target path and optional fragment split at the first `#`.
    ///
    /// # Contract
    /// - ensures: returns `None` when `to` contains no fragment marker,
    ///   otherwise returns the prefix before the first `#` and the suffix after
    ///   it.
    /// - provides: shared anchor parsing for drift and reference checks.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn split_target(&self) -> Option<TargetSplit<'_>>
    {
        let (path, fragment) = self.to().into().0.split_once('#')?;
        return Some(TargetSplit {
            path: TargetPathText(path),
            fragment: TargetFragmentText(fragment),
        });
    }

    /// Return the target path with any `#fragment` stripped.
    #[inline]
    #[must_use]
    pub fn target_path(&self) -> impl Into<TargetPathText<'_>>
    {
        return match self.split_target() {
            | Some(target) => target.path.0,
            | None => self.to().into().0,
        };
    }
}

/// Verify manifest hashes, missing files, and unregistered Markdown documents.
///
/// # Contract
/// - requires: `manifest_path` points at a Gandr documentation manifest.
/// - ensures: returns semantic findings in deterministic order: drift/missing
///   nodes in manifest order, followed by unregistered Markdown paths sorted
///   lexicographically.
/// - provides: in-process BLAKE3 verification and sorted orphan detection.
/// - fails: returns [`GateError`] for manifest, filesystem, UTF-8, or traversal
///   failures that would make the gate vacuous or incomplete.
/// - panics: none.
/// - intension: downstream stale-context edges are computed by an iterative
///   reverse-edge worklist, never input-scaled recursion.
///
/// # Errors
/// Returns loader errors from [`ManifestContext::load`], file I/O errors from
/// registered document reads, and traversal errors from support file walking.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — clean, drift, missing, unregistered, and
///   downstream-edge fixtures separate all observable report branches.
/// - witness: `manifest::tests::clean_manifest_has_no_drift_findings`
/// - witness: `manifest::tests::drift_missing_and_unregistered_docs_are_reported`
#[inline]
pub fn run_manifest_drift(manifest_path: &Path) -> GateResult
{
    let context = ManifestContext::load(manifest_path)?;
    let verified_nodes = verify_nodes(&context)?;
    let reverse_edges = reverse_edges(&context);
    let mut findings = Vec::new();

    for verification in &verified_nodes {
        if verification.status != VerifyStatus::Ok {
            findings.push(verification_finding(verification, &reverse_edges)?);
        }
    }

    for unregistered_path in unregistered_markdown(&context)? {
        findings.push(Finding::new(
            "docs-manifest-unregistered",
            "docs",
            unregistered_path,
            "UNREGISTERED",
            "corpus doc has no manifest entry; register it with at least one edge",
        ));
    }

    Ok(findings)
}

/// Parse one YAML manifest document.
fn manifest_document<'semantic, Source>(
    manifest_path: &Path,
    source: Source,
) -> Result<Yaml, GateError>
where
    Source: Into<SourceText<'semantic>>,
{
    let source = source.into().0;
    let mut documents = YamlLoader::load_from_str(source).map_err(|error| {
        GateError::operational(format!(
            "manifest present but unparseable: {}: {error}",
            manifest_path.display()
        ))
    })?;
    if documents.len() != 1_usize {
        return Err(GateError::operational(format!(
            "manifest YAML must contain exactly one document, found {}: {}",
            documents.len(),
            manifest_path.display()
        )));
    }
    return documents.pop().ok_or_else(|| {
        GateError::operational(format!(
            "manifest YAML must contain exactly one document, found 0: {}",
            manifest_path.display()
        ))
    });
}

/// Return a YAML string slice.
fn yaml_string(value: &Yaml) -> impl Into<OptionalYamlStringText<'_>>
{
    match *value {
        | Yaml::String(ref text) => Some(text.as_str()),
        | _ => None,
    }
}

/// Build the vacuous-node operational error.
fn empty_nodes_error(manifest_path: &Path) -> GateError
{
    GateError::operational(format!(
        "manifest present but defines no registered nodes: {} — a corpus manifest must list at least one node (corrupt manifest or wrong file?)",
        manifest_path.display()
    ))
}

/// Return the manifest parent directory or `.` for a bare filename.
fn manifest_parent(manifest_path: &Path) -> PathBuf
{
    return manifest_path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
}

/// Return whether a manifest path is relative manifest text.
///
/// # Contract
/// - ensures: accepts normal relative components and parent hops that the
///   manifest-scope validator can bound later.
/// - provides: a parser-side guard against absolute and current-directory
///   entries before filesystem reads occur.
/// - panics: none.
fn manifest_path_is_corpus_relative<'semantic, Raw>(
    raw: Raw
) -> impl Into<ManifestPathIsCorpusRelativeFlag>
where
    Raw: Into<RawText<'semantic>>,
{
    let raw = raw.into().0;
    let path = Path::new(raw);
    return !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::ParentDir));
}

/// Reject registered paths that escape scope or cross symlink components.
///
/// # Contract
/// - requires: `path` has already passed relative manifest-text validation.
/// - ensures: normalizes `..` components lexically against the manifest scope,
///   inspects existing path components without following a final symlink, and
///   leaves missing files for drift reporting.
/// - provides: traversal and symlink escape rejection while preserving sibling
///   ADR nodes and missing-node findings.
/// - fails: returns an I/O error for metadata failures and an operational error
///   for symlink components or scope escapes.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Io`] when metadata cannot be inspected, and
/// [`GateError::Operational`] when a path escapes scope or crosses a symlink.
fn ensure_manifest_path_stays_in_scope(
    corpus_dir: &Path,
    path: &ManifestPath,
) -> Result<(), GateError>
{
    let scope_root = manifest_scope_root(corpus_dir);
    let mut candidate = corpus_dir.to_path_buf();
    for component in Path::new(path.as_str().into().0).components() {
        match component {
            | Component::Normal(part) => {
                candidate.push(part);
                match fs::symlink_metadata(&candidate) {
                    | Ok(metadata) if metadata.file_type().is_symlink() => {
                        return Err(GateError::operational(format!(
                            "Node.path must not cross symlink component: {}",
                            path.as_str().into().0
                        )));
                    },
                    | Ok(_metadata) => {},
                    | Err(source) if source.kind() == std::io::ErrorKind::NotFound => {},
                    | Err(source) => {
                        return Err(GateError::Io {
                            path: candidate,
                            source,
                        });
                    },
                }
            },
            | Component::ParentDir => {
                if candidate == scope_root || !candidate.pop() {
                    return Err(GateError::operational(format!(
                        "Node.path escapes documentation manifest scope: {}",
                        path.as_str().into().0
                    )));
                }
            },
            | Component::CurDir | Component::RootDir | Component::Prefix(_) => {
                return Err(GateError::operational(
                    "Node.path must be relative manifest text without absolute or `.` components",
                ));
            },
        }
    }
    Ok(())
}

/// Return the documentation scope root allowed for manifest node paths.
///
/// # Contract
/// - ensures: paths may name files under the manifest directory or its direct
///   parent, matching the retained Gandr/ADR corpus layout.
/// - provides: the lexical boundary used to reject true traversal escapes while
///   preserving registered sibling ADR documents.
/// - panics: none.
fn manifest_scope_root(corpus_dir: &Path) -> PathBuf
{
    return corpus_dir
        .parent()
        .map_or_else(|| corpus_dir.to_path_buf(), Path::to_path_buf);
}

/// Parse one manifest node mapping.
fn parse_node(
    manifest_path: &Path,
    node_yaml: &Yaml,
) -> Result<ManifestNode, GateError>
{
    let mapping = yaml_hash(manifest_path, node_yaml, "manifest node must be a mapping")?;
    let path = yaml_mapping_value(mapping, "path")
        .and_then(|value| yaml_string(value).into().0)
        .ok_or_else(|| GateError::operational("Node.path must be non-empty"))?;
    let b3 = yaml_mapping_value(mapping, "b3")
        .and_then(|value| yaml_string(value).into().0)
        .ok_or_else(|| GateError::operational("Node.b3 must be non-empty"))?;
    if b3.is_empty() {
        return Err(GateError::operational("Node.b3 must be non-empty"));
    }

    let edges = match yaml_mapping_value(mapping, "edges") {
        | Some(edges_yaml) => parse_edges(manifest_path, edges_yaml)?,
        | None => Vec::new(),
    };

    Ok(ManifestNode {
        path: ManifestPath::new(path)?,
        b3: String::from(b3),
        edges,
    })
}

/// Return a string-keyed mapping value without allocating a temporary key.
fn yaml_mapping_value<'semantic, 'yaml, KeyName>(
    mapping: &'yaml Hash,
    key_name: KeyName,
) -> Option<&'yaml Yaml>
where
    KeyName: Into<KeyNameText<'semantic>>,
{
    let key_name = key_name.into().0;
    for (key, value) in mapping {
        if let Yaml::String(ref candidate) = *key
            && candidate == key_name
        {
            return Some(value);
        }
    }
    return None;
}

/// Parse a manifest edge list.
fn parse_edges(
    manifest_path: &Path,
    edges_yaml: &Yaml,
) -> Result<Vec<ManifestEdge>, GateError>
{
    let values = yaml_array(
        manifest_path,
        edges_yaml,
        "manifest node edges must be a list",
    )?;
    let mut edges = Vec::new();
    for edge_yaml in values {
        edges.push(parse_edge(manifest_path, edge_yaml)?);
    }
    return Ok(edges);
}

/// Return a YAML array or a stable manifest-shape error.
fn yaml_array<'semantic, 'yaml, Detail>(
    manifest_path: &Path,
    value: &'yaml Yaml,
    detail: Detail,
) -> Result<&'yaml Vec<Yaml>, GateError>
where
    Detail: Into<DetailText<'semantic>>,
{
    let detail = detail.into().0;
    match *value {
        | Yaml::Array(ref values) => Ok(values),
        | _ => Err(GateError::operational(format!(
            "manifest shape error: {}: {detail}",
            manifest_path.display()
        ))),
    }
}

/// Parse one manifest edge mapping.
fn parse_edge(
    manifest_path: &Path,
    edge_yaml: &Yaml,
) -> Result<ManifestEdge, GateError>
{
    let mapping = yaml_hash(manifest_path, edge_yaml, "manifest edge must be a mapping")?;
    let rel = yaml_mapping_value(mapping, "rel")
        .and_then(|value| yaml_string(value).into().0)
        .ok_or_else(|| GateError::operational("ManifestEdge.rel must be non-empty"))?;
    let to = yaml_mapping_value(mapping, "to")
        .and_then(|value| yaml_string(value).into().0)
        .ok_or_else(|| GateError::operational("ManifestEdge.to must be non-empty"))?;
    if rel.is_empty() {
        return Err(GateError::operational("ManifestEdge.rel must be non-empty"));
    }
    if to.is_empty() {
        return Err(GateError::operational("ManifestEdge.to must be non-empty"));
    }
    Ok(ManifestEdge {
        rel: String::from(rel),
        to: String::from(to),
    })
}

/// Return a YAML mapping or a stable manifest-shape error.
fn yaml_hash<'semantic, 'yaml, Detail>(
    manifest_path: &Path,
    value: &'yaml Yaml,
    detail: Detail,
) -> Result<&'yaml Hash, GateError>
where
    Detail: Into<DetailText<'semantic>>,
{
    let detail = detail.into().0;
    match *value {
        | Yaml::Hash(ref mapping) => Ok(mapping),
        | _ => Err(GateError::operational(format!(
            "manifest shape error: {}: {detail}",
            manifest_path.display()
        ))),
    }
}

/// Verify every node in manifest order.
fn verify_nodes(context: &ManifestContext) -> Result<Vec<VerifyResult>, GateError>
{
    let mut verified_nodes = Vec::new();
    for node in context.nodes() {
        verified_nodes.push(verify_node(context, node)?);
    }
    return Ok(verified_nodes);
}

/// Verify one node by reading its UTF-8 Markdown text and hashing its bytes.
fn verify_node(
    context: &ManifestContext,
    node: &ManifestNode,
) -> Result<VerifyResult, GateError>
{
    let file_path = context.resolve_path(node.path());
    if !path_exists(&file_path).map(|value| value.into().0)? {
        return Ok(VerifyResult {
            path: String::from(node.path().as_str().into().0),
            status: VerifyStatus::Missing,
            expected: String::from(node.b3().into().0),
            actual: None,
        });
    }

    let source = support::read_utf8(&file_path)?;
    let actual = blake3_hex(source.as_bytes());
    let status = if actual == node.b3().into().0 {
        VerifyStatus::Ok
    }
    else {
        VerifyStatus::Drift
    };
    Ok(VerifyResult {
        path: String::from(node.path().as_str().into().0),
        status,
        expected: String::from(node.b3().into().0),
        actual: Some(actual),
    })
}

/// Return whether `path` currently exists, mapping metadata failures.
///
/// # Contract
/// - ensures: returns `Ok(false)` for absent paths and `Ok(true)` for existing
///   paths.
/// - provides: the absent-versus-unreadable split needed by docs gates.
/// - fails: returns an I/O error for metadata failures other than absence.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Io`] when the host cannot determine path existence.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — absent paths and metadata errors reach distinct
///   downstream branches in drift and reference tests.
/// - witness: `manifest::tests::drift_missing_and_unregistered_docs_are_reported`
#[inline]
pub fn path_exists(path: &Path) -> Result<impl Into<PathExistsFlag>, GateError>
{
    crate::support::HOST_FILESYSTEM
        .try_exists(path)
        .map(bool::from)
}

/// Return a lowercase BLAKE3 hex digest for `bytes`.
fn blake3_hex<'semantic, Bytes>(bytes: Bytes) -> String
where
    Bytes: Into<BytesBytes<'semantic>>,
{
    let bytes = bytes.into().0;
    let digest = blake3::hash(bytes);
    return format!("{}", digest.to_hex());
}

/// Flatten manifest edges and deduplicate them while preserving first-seen
/// order.
fn reverse_edges(context: &ManifestContext) -> Vec<ReverseEdge>
{
    let mut edges = Vec::new();
    for node in context.nodes() {
        for manifest_edge in node.edges() {
            let candidate = ReverseEdge {
                from: String::from(node.path().as_str().into().0),
                rel: String::from(manifest_edge.rel().into().0),
                to: String::from(manifest_edge.target_path().into().0),
            };
            push_unique_edge(&mut edges, candidate);
        }
    }
    return edges;
}

/// Append `candidate` only when the edge is not already present.
fn push_unique_edge(
    edges: &mut Vec<ReverseEdge>,
    candidate: ReverseEdge,
)
{
    if edges.iter().all(|edge| edge != &candidate) {
        edges.push(candidate);
    }
}

/// Build one semantic finding for a drifted or missing node.
fn verification_finding(
    verification: &VerifyResult,
    edges: &[ReverseEdge],
) -> Result<Finding, GateError>
{
    let status_label = verification.status.label().into().0;
    let actual = verification.actual.as_deref().unwrap_or("<file missing>");
    let downstream = downstream_edges(&verification.path, edges)?;
    let detail = verification_detail(verification, actual, &downstream);
    Ok(Finding::new(
        "docs-manifest-drift",
        "docs",
        String::from(verification.path.as_str()),
        status_label,
        detail,
    ))
}

/// Walk reverse edges from `root` without recursion.
fn downstream_edges<'semantic, Root>(
    root: Root,
    edges: &[ReverseEdge],
) -> Result<Vec<ReverseEdge>, GateError>
where
    Root: Into<RootText<'semantic>>,
{
    let root = root.into().0;
    let mut reached = vec![String::from(root)];
    let mut frontier = vec![String::from(root)];
    let mut frontier_index = 0_usize;
    let mut collected = Vec::new();

    while frontier_index < frontier.len() {
        let current = match frontier.get(frontier_index) {
            | Some(value) => String::from(value.as_str()),
            | None => break,
        };
        for edge in edges {
            if edge.to.as_str() == current.as_str() && !contains_text(&reached, &edge.from).into().0
            {
                reached.push(String::from(edge.from.as_str()));
                frontier.push(String::from(edge.from.as_str()));
                collected.push(edge.clone());
            }
        }
        frontier_index = frontier_index
            .checked_add(1_usize)
            .ok_or_else(|| GateError::operational("downstream edge traversal index overflowed"))?;
    }

    Ok(collected)
}

/// Render one drift/missing detail string.
fn verification_detail<'semantic, Actual>(
    verification: &VerifyResult,
    actual: Actual,
    downstream: &[ReverseEdge],
) -> String
where
    Actual: Into<ActualText<'semantic>>,
{
    let actual = actual.into().0;
    let mut detail = format!(
        "expected b3 {}; actual b3 {}",
        verification.expected, actual
    );
    if downstream.is_empty() {
        detail.push_str("; downstream: none (no registered reverse edges)");
    }
    else {
        detail.push_str("; downstream (stale reasoning context):");
        for edge in downstream {
            detail.push(' ');
            detail.push_str(&edge.from);
            detail.push_str(" -[");
            detail.push_str(&edge.rel);
            detail.push_str("]-> ");
            detail.push_str(&edge.to);
            detail.push(';');
        }
    }
    return detail;
}

/// Return whether `values` contains `needle`.
fn contains_text<'semantic, Needle>(
    values: &[String],
    needle: Needle,
) -> impl Into<TextFlag>
where
    Needle: Into<NeedleText<'semantic>>,
{
    let needle = needle.into().0;
    return values.iter().any(|value| value == needle);
}

/// Return sorted Markdown files under the corpus directory that are not nodes.
fn unregistered_markdown(context: &ManifestContext) -> Result<Vec<String>, GateError>
{
    let registered_paths = context.registered_path_set();
    let markdown_files = support::walk_files(context.corpus_dir(), OsStr::new("md"))?;
    let mut unregistered = Vec::new();
    for markdown_file in markdown_files {
        let relative_path = manifest_relative_string(context.corpus_dir(), &markdown_file);
        if !registered_paths.contains(&relative_path) {
            unregistered.push(relative_path);
        }
    }
    unregistered.sort();
    unregistered.dedup();
    return Ok(unregistered);
}

/// Convert a file path to a manifest-relative display string.
fn manifest_relative_string(
    corpus_dir: &Path,
    path: &Path,
) -> String
{
    let relative_path = match path.strip_prefix(corpus_dir) {
        | Ok(stripped_path) => stripped_path,
        | Err(_error) => path,
    };
    return format!("{}", relative_path.display());
}

/// Compare findings by the emitted diagnostic field order.
#[cfg(test)]
fn finding_order(
    left: &Finding,
    right: &Finding,
) -> Ordering
{
    left.kind
        .cmp(&right.kind)
        .then_with(|| left.package.cmp(&right.package))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.declaration.cmp(&right.declaration))
        .then_with(|| left.detail.cmp(&right.detail))
}

/// Per-node hash verification status.
#[derive(Clone, Copy, Eq, PartialEq)]
enum VerifyStatus
{
    /// File exists and matches the manifest digest.
    Ok,
    /// File is absent.
    Missing,
    /// File exists but its BLAKE3 digest differs from the manifest digest.
    Drift,
}

/// Result of verifying one manifest node.
struct VerifyResult
{
    /// Manifest-relative path being verified.
    path: String,
    /// Verification status.
    status: VerifyStatus,
    /// Expected manifest digest.
    expected: String,
    /// Recomputed digest, or `None` when the file is missing.
    actual: Option<String>,
}

/// Flattened manifest edge used for reverse traversal.
#[derive(Clone, Eq, PartialEq)]
struct ReverseEdge
{
    /// Owning node path.
    from: String,
    /// Edge relation.
    rel: String,
    /// Target node path with any fragment stripped.
    to: String,
}

impl VerifyStatus
{
    /// Return the legacy uppercase status label.
    fn label(self) -> impl Into<LabelText<'static>>
    {
        match self {
            | Self::Ok => "OK",
            | Self::Missing => "MISSING",
            | Self::Drift => "DRIFT",
        }
    }
}

#[cfg(test)]
mod tests
{
    //! Unit witnesses for documentation manifest drift behavior.

    use alloc::string::String;
    use alloc::vec::Vec;
    use core::error::Error;
    use std::path::Path;
    use std::path::PathBuf;

    use super::*;

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
    }

    /// Return findings sorted by the stable field order.
    fn sorted_findings(mut findings: Vec<Finding>) -> Vec<Finding>
    {
        findings.sort_by(finding_order);
        return findings;
    }

    /// Assert that a manifest loader failure contains `expected`.
    fn assert_manifest_error_contains<'semantic, Expected>(
        result: Result<ManifestContext, GateError>,
        expected: Expected,
    ) where
        Expected: Into<ExpectedText<'semantic>>,
    {
        let expected = expected.into().0;
        let Err(error) = result
        else {
            panic!("manifest fixture unexpectedly loaded successfully");
        };
        let text = error.to_string();
        assert!(
            text.contains(expected),
            "manifest error `{text}` did not contain `{expected}`"
        );
    }

    /// A clean manifest reports no drift, missing files, or unregistered docs.
    #[test]
    fn clean_manifest_has_no_drift_findings() -> TestResult
    {
        let fixture = fixture("clean")?;
        let hash = write_doc(&fixture.corpus, "intro.md", "# Intro\n\nBody\n")?;
        write_manifest(
            &fixture.manifest,
            &format!(
                "  - path: intro.md\n    b3: {hash}\n    edges:\n      - rel: cites\n        to: intro.md#intro\n"
            ),
        )?;

        let findings = run_manifest_drift(&fixture.manifest)?;

        assert!(
            findings.is_empty(),
            "clean manifest should not emit drift findings"
        );
        assert!(
            fixture.root.exists(),
            "fixture root should remain available for failure inspection"
        );
        Ok(())
    }

    /// Drift, missing files, and unregistered docs all become stable findings.
    #[test]
    fn drift_missing_and_unregistered_docs_are_reported() -> TestResult
    {
        let fixture = fixture("dirty")?;
        let clean_hash = write_doc(&fixture.corpus, "changed.md", "# Changed\n")?;
        let _unregistered_hash = write_doc(&fixture.corpus, "orphan.md", "# Orphan\n")?;
        write_manifest(
            &fixture.manifest,
            &format!(
                "  - path: changed.md\n    b3: \"0000000000000000000000000000000000000000000000000000000000000000\"\n    edges:\n      - rel: derives\n        to: missing.md#target\n  - path: missing.md\n    b3: {clean_hash}\n    edges:\n      - rel: uses\n        to: changed.md\n"
            ),
        )?;

        let findings = sorted_findings(run_manifest_drift(&fixture.manifest)?);
        let drift = findings.iter().any(|finding| {
            finding.kind == "docs-manifest-drift"
                && finding.path == "changed.md"
                && finding.declaration == "DRIFT"
        });
        let missing = findings.iter().any(|finding| {
            finding.kind == "docs-manifest-drift"
                && finding.path == "missing.md"
                && finding.declaration == "MISSING"
        });
        let unregistered = findings.iter().any(|finding| {
            finding.kind == "docs-manifest-unregistered" && finding.path == "orphan.md"
        });

        assert!(drift, "changed node should report digest drift");
        assert!(
            missing,
            "absent registered node should report missing status"
        );
        assert!(
            unregistered,
            "Markdown file outside the manifest should be reported"
        );
        Ok(())
    }

    /// Empty manifest node lists fail before the gate can pass vacuously.
    #[test]
    fn empty_manifest_nodes_fail_loudly() -> TestResult
    {
        let fixture = fixture("empty")?;
        crate::support::HOST_FILESYSTEM
            .write(&fixture.manifest, "version: 1\nhash: blake3\nnodes: []\n")?;

        let error = ManifestContext::load(&fixture.manifest)
            .err()
            .ok_or_else(|| {
                GateError::operational("empty manifest unexpectedly loaded successfully")
            })?;

        assert!(
            error.to_string().contains("defines no registered nodes"),
            "empty node lists should keep the historical vacuity failure"
        );
        Ok(())
    }

    /// Build a clean fixture directory for `name`.
    fn fixture<'semantic, Name>(name: Name) -> Result<Fixture, Box<dyn Error>>
    where
        Name: Into<NameText<'semantic>>,
    {
        let name = name.into().0;
        let root = std::env::temp_dir().join(format!(
            "gandr-workflow-gates-docs-manifest-{}-{name}",
            std::process::id()
        ));
        crate::support::HOST_FILESYSTEM.remove_dir_if_exists(&root)?;
        let corpus = root.join("docs/gandr");
        crate::support::HOST_FILESYSTEM.create_dir_all(&corpus)?;
        let manifest = corpus.join("MANIFEST.yml");
        Ok(Fixture {
            root,
            manifest,
            corpus,
        })
    }

    /// Malformed or absent manifests fail closed with stable diagnostics.
    #[test]
    fn malformed_manifest_inputs_fail_closed() -> TestResult
    {
        let absent = fixture("absent")?;
        assert_manifest_error_contains(
            ManifestContext::load(&absent.manifest),
            "manifest not found",
        );

        let wrong_hash = fixture("wrong-hash")?;
        crate::support::HOST_FILESYSTEM.write(
            &wrong_hash.manifest,
            "version: 1\nhash: sha256\nnodes:\n  - path: intro.md\n    b3: abc\n",
        )?;
        assert_manifest_error_contains(
            ManifestContext::load(&wrong_hash.manifest),
            "unsupported manifest hash algorithm",
        );

        let malformed_edge = fixture("malformed-edge")?;
        write_manifest(
            &malformed_edge.manifest,
            "  - path: intro.md\n    b3: abc\n    edges:\n      - just-a-string\n",
        )?;
        assert_manifest_error_contains(
            ManifestContext::load(&malformed_edge.manifest),
            "manifest edge must be a mapping",
        );
        Ok(())
    }

    /// Loader shape failures cover malformed YAML, duplicates, and bad fields.
    #[test]
    fn manifest_loader_rejects_malformed_duplicate_missing_and_traversal_entries() -> TestResult
    {
        let unparseable = fixture("unparseable")?;
        crate::support::HOST_FILESYSTEM.write(&unparseable.manifest, "version: [")?;
        assert_manifest_error_contains(
            ManifestContext::load(&unparseable.manifest),
            "manifest present but unparseable",
        );

        let no_documents = fixture("no-documents")?;
        crate::support::HOST_FILESYSTEM.write(&no_documents.manifest, "")?;
        assert_manifest_error_contains(
            ManifestContext::load(&no_documents.manifest),
            "must contain exactly one document, found 0",
        );

        let many_documents = fixture("many-documents")?;
        crate::support::HOST_FILESYSTEM.write(
            &many_documents.manifest,
            "hash: blake3\nnodes: []\n---\nhash: blake3\n",
        )?;
        assert_manifest_error_contains(
            ManifestContext::load(&many_documents.manifest),
            "must contain exactly one document, found 2",
        );

        let non_list = fixture("non-list")?;
        crate::support::HOST_FILESYSTEM.write(
            &non_list.manifest,
            "version: 1\nhash: blake3\nnodes: nope\n",
        )?;
        assert_manifest_error_contains(
            ManifestContext::load(&non_list.manifest),
            "manifest nodes must be a list",
        );

        let duplicate = fixture("duplicate")?;
        write_manifest(
            &duplicate.manifest,
            "  - path: intro.md\n    b3: abc\n  - path: intro.md\n    b3: def\n",
        )?;
        assert_manifest_error_contains(
            ManifestContext::load(&duplicate.manifest),
            "corpus node paths must be unique",
        );

        let traversal = fixture("traversal")?;
        write_manifest(
            &traversal.manifest,
            "  - path: ../../escape.md\n    b3: abc\n",
        )?;
        assert_manifest_error_contains(
            ManifestContext::load(&traversal.manifest),
            "escapes documentation manifest scope",
        );

        let missing_fields = fixture("missing-fields")?;
        write_manifest(&missing_fields.manifest, "  - path: ''\n    b3: abc\n")?;
        assert_manifest_error_contains(
            ManifestContext::load(&missing_fields.manifest),
            "Node.path must be non-empty",
        );

        let missing_b3 = fixture("missing-b3")?;
        write_manifest(&missing_b3.manifest, "  - path: intro.md\n    b3: ''\n")?;
        assert_manifest_error_contains(
            ManifestContext::load(&missing_b3.manifest),
            "Node.b3 must be non-empty",
        );

        let non_list_edges = fixture("non-list-edges")?;
        write_manifest(
            &non_list_edges.manifest,
            "  - path: intro.md\n    b3: abc\n    edges: nope\n",
        )?;
        assert_manifest_error_contains(
            ManifestContext::load(&non_list_edges.manifest),
            "manifest node edges must be a list",
        );

        let empty_edge_rel = fixture("empty-edge-rel")?;
        write_manifest(
            &empty_edge_rel.manifest,
            "  - path: intro.md\n    b3: abc\n    edges:\n      - rel: ''\n        to: other.md\n",
        )?;
        assert_manifest_error_contains(
            ManifestContext::load(&empty_edge_rel.manifest),
            "ManifestEdge.rel must be non-empty",
        );

        let empty_edge_to = fixture("empty-edge-to")?;
        write_manifest(
            &empty_edge_to.manifest,
            "  - path: intro.md\n    b3: abc\n    edges:\n      - rel: cites\n        to: ''\n",
        )?;
        assert_manifest_error_contains(
            ManifestContext::load(&empty_edge_to.manifest),
            "ManifestEdge.to must be non-empty",
        );
        Ok(())
    }

    /// Write a manifest with raw node YAML.
    fn write_manifest<'semantic, NodesYaml>(
        manifest: &Path,
        nodes_yaml: NodesYaml,
    ) -> Result<(), Box<dyn Error>>
    where
        NodesYaml: Into<NodesYamlText<'semantic>>,
    {
        let nodes_yaml = nodes_yaml.into().0;
        crate::support::HOST_FILESYSTEM.write(
            manifest,
            format!("version: 1\nhash: blake3\nnodes:\n{nodes_yaml}"),
        )?;
        Ok(())
    }

    /// Drift findings keep manifest order before sorted unregistered files.
    #[test]
    fn manifest_drift_finding_order_and_digest_details_are_deterministic() -> TestResult
    {
        let fixture = fixture("order")?;
        let missing_hash = write_doc(&fixture.corpus, "seed.md", "# Seed\n")?;
        crate::support::HOST_FILESYSTEM.remove_file(fixture.corpus.join("seed.md"))?;
        let _z_hash = write_doc(&fixture.corpus, "z.md", "# Z\n")?;
        let _a_hash = write_doc(&fixture.corpus, "a.md", "# A\n")?;
        let _orphan_b = write_doc(&fixture.corpus, "b-orphan.md", "# B\n")?;
        let _orphan_a = write_doc(&fixture.corpus, "a-orphan.md", "# A orphan\n")?;
        write_manifest(
            &fixture.manifest,
            &format!(
                "  - path: z.md\n    b3: \"0000000000000000000000000000000000000000000000000000000000000000\"\n    edges:\n      - rel: feeds\n        to: missing.md\n  - path: missing.md\n    b3: {missing_hash}\n    edges: []\n  - path: a.md\n    b3: \"1111111111111111111111111111111111111111111111111111111111111111\"\n    edges: []\n"
            ),
        )?;

        let findings = run_manifest_drift(&fixture.manifest)?;
        let projection = findings
            .iter()
            .map(|finding| format!("{}:{}", finding.path, finding.declaration))
            .collect::<Vec<_>>();

        assert_eq!(
            vec![
                String::from("z.md:DRIFT"),
                String::from("missing.md:MISSING"),
                String::from("a.md:DRIFT"),
                String::from("a-orphan.md:UNREGISTERED"),
                String::from("b-orphan.md:UNREGISTERED"),
            ],
            projection
        );
        assert!(
            findings.iter().any(|finding| finding.path == "z.md"
                && finding.detail.contains("expected b3")
                && finding.detail.contains("actual b3")),
            "digest mismatch detail should include both expected and actual hashes"
        );
        Ok(())
    }

    /// Symlinked node path components are rejected before reads escape corpus.
    #[cfg(unix)]
    #[test]
    fn manifest_loader_rejects_symlinked_node_components() -> TestResult
    {
        let fixture = fixture("symlink")?;
        crate::support::HOST_FILESYSTEM.create_dir_all(fixture.corpus.join("real"))?;
        let hash = write_doc(&fixture.corpus, "real/doc.md", "# Real\n")?;
        crate::support::HOST_FILESYSTEM
            .symlink(fixture.corpus.join("real"), fixture.corpus.join("link"))?;
        write_manifest(
            &fixture.manifest,
            &format!("  - path: link/doc.md\n    b3: {hash}\n"),
        )?;

        assert_manifest_error_contains(
            ManifestContext::load(&fixture.manifest),
            "must not cross symlink component",
        );
        Ok(())
    }

    /// Write a corpus document and return its BLAKE3 hash.
    fn write_doc<'semantic, Rel, Text>(
        corpus: &Path,
        rel: Rel,
        text: Text,
    ) -> Result<String, Box<dyn Error>>
    where
        Rel: Into<RelText<'semantic>>,
        Text: Into<TextText<'semantic>>,
    {
        let text = text.into().0;
        let rel = rel.into().0;
        let doc_path = corpus.join(rel);
        let Some(parent) = doc_path.parent()
        else {
            return Err(Box::new(std::io::Error::other(
                "fixture path has no parent",
            )));
        };
        crate::support::HOST_FILESYSTEM.create_dir_all(parent)?;
        crate::support::HOST_FILESYSTEM.write(&doc_path, text)?;
        Ok(blake3_hex(text.as_bytes()))
    }
}
