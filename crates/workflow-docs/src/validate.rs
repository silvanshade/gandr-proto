//! Corpus-level validation: identifier uniqueness, define-once, and term,
//! cite, and provenance resolution.
//!
//! Parsing (see [`crate::parse`]) enforces per-file structure and status
//! presence; this module enforces the cross-file invariants. Any violation is a
//! [`Diagnostic`]; a run with diagnostics fails.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use std::path::Path;

use yaml_rust2::YamlLoader;

use crate::Diagnostic;
use crate::DocError;
use crate::model::Block;
use crate::model::Document;
use crate::model::Inline;

/// A validated (or to-be-validated) corpus: the parsed components plus the
/// resolvable cite-key set from the references file.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Corpus
{
    /// Parsed components, in discovery order.
    pub documents: Vec<Document>,
    /// Cite keys declared in the hayagriva references file.
    pub reference_keys: BTreeSet<String>,
}

impl Corpus
{
    /// Build a corpus from parsed documents and reference keys.
    #[inline]
    #[must_use]
    pub fn new(
        documents: Vec<Document>,
        reference_keys: BTreeSet<String>,
    ) -> Self
    {
        Self {
            documents,
            reference_keys,
        }
    }
}

/// Read the top-level cite keys from a hayagriva references file.
///
/// # Errors
/// Returns [`DocError::Io`] when the file cannot be read and [`DocError::Yaml`]
/// when it is not well-formed.
#[inline]
pub fn load_reference_keys(path: &Path) -> Result<BTreeSet<String>, DocError>
{
    let text = std::fs::read_to_string(path).map_err(|source| DocError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let documents = YamlLoader::load_from_str(&text).map_err(|error| DocError::Yaml {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    let mut keys = BTreeSet::new();
    for document in &documents {
        if let Some(mapping) = document.as_hash() {
            for entry in mapping.keys() {
                if let Some(key) = entry.as_str() {
                    keys.insert(key.to_owned());
                }
            }
        }
    }
    Ok(keys)
}

/// Validate a corpus, returning every violation as a diagnostic.
///
/// The returned vector is sorted, so the report is deterministic. An empty
/// result means the corpus conforms.
#[inline]
#[must_use]
pub fn validate_corpus(corpus: &Corpus) -> Vec<Diagnostic>
{
    let mut collector = Collector::new();
    for document in &corpus.documents {
        collector.component_ids.insert(document.id.clone());
    }
    for document in &corpus.documents {
        collector.visit_document(document);
    }
    collector.finish(&corpus.reference_keys);
    let mut diagnostics = collector.diagnostics;
    diagnostics.sort();
    diagnostics
}

/// Mutable accumulator threaded through the corpus traversal.
struct Collector
{
    /// Every declared identifier mapped to its first location.
    ids: BTreeMap<String, String>,
    /// Every component identifier, for provenance resolution.
    component_ids: BTreeSet<String>,
    /// Every defined term key mapped to its first location.
    defined_terms: BTreeMap<String, String>,
    /// Each term reference as a key and location.
    term_refs: Vec<(String, String)>,
    /// Each cite reference as a key and location.
    cites: Vec<(String, String)>,
    /// Each provenance edge as a target identifier and location.
    provenance: Vec<(String, String)>,
    /// Accumulated diagnostics.
    diagnostics: Vec<Diagnostic>,
}

impl Collector
{
    /// Build an empty collector.
    fn new() -> Self
    {
        Self {
            ids: BTreeMap::new(),
            component_ids: BTreeSet::new(),
            defined_terms: BTreeMap::new(),
            term_refs: Vec::new(),
            cites: Vec::new(),
            provenance: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Record an identifier, flagging a duplicate against its first location.
    fn record_id(
        &mut self,
        id: &str,
        location: &str,
    )
    {
        if let Some(first) = self.ids.get(id) {
            self.diagnostics.push(Diagnostic::new(
                "duplicate-id",
                location.to_owned(),
                format!("id '{id}' already declared at {first}"),
            ));
        }
        else {
            self.ids.insert(id.to_owned(), location.to_owned());
        }
    }

    /// Visit a component and its blocks.
    fn visit_document(
        &mut self,
        document: &Document,
    )
    {
        let component_location = format!("{}:<component#{}>", document.source_path, document.id);
        self.record_id(&document.id, &component_location);
        for edge in document.grounds.iter().chain(document.derives.iter()) {
            self.provenance
                .push((edge.clone(), component_location.clone()));
        }
        for block in &document.blocks {
            self.visit_block(block, &document.source_path);
        }
    }

    /// Visit one block, recursing into nested blocks and inline content.
    fn visit_block(
        &mut self,
        block: &Block,
        path: &str,
    )
    {
        match *block {
            | Block::Section(ref section) => {
                if let Some(ref id) = section.id {
                    self.record_id(id, &format!("{path}:<section#{id}>"));
                }
                for nested in &section.blocks {
                    self.visit_block(nested, path);
                }
            },
            | Block::Example(ref example) => {
                for nested in &example.blocks {
                    self.visit_block(nested, path);
                }
            },
            | Block::Prose(ref inlines) => {
                for inline in inlines {
                    self.visit_inline(inline, path);
                }
            },
            | Block::Definition(ref definition) => {
                if let Some(ref id) = definition.id {
                    self.record_id(id, &format!("{path}:<definition#{id}>"));
                }
                for inline in &definition.body {
                    self.visit_inline(inline, path);
                }
            },
            | Block::Rule(ref rule) => {
                if let Some(ref id) = rule.id {
                    self.record_id(id, &format!("{path}:<rule#{id}>"));
                }
            },
            | Block::Diagram(ref diagram) => {
                self.record_id(&diagram.id, &format!("{path}:<diagram#{}>", diagram.id));
                for cite in &diagram.cites {
                    self.cites
                        .push((cite.key.clone(), format!("{path}:<diagram#{}>", diagram.id)));
                }
            },
            | Block::References(ref keys) => {
                for cite in keys {
                    self.cites
                        .push((cite.key.clone(), format!("{path}:<references>")));
                }
            },
            | Block::Judgements(_) | Block::Grammar(_) | Block::Code(_) => {},
        }
    }

    /// Visit one inline element.
    fn visit_inline(
        &mut self,
        inline: &Inline,
        path: &str,
    )
    {
        match *inline {
            | Inline::TermDef(ref term_def) => {
                let location = format!("{path}:<term-def key='{}'>", term_def.key);
                if let Some(first) = self.defined_terms.get(&term_def.key) {
                    self.diagnostics.push(Diagnostic::new(
                        "duplicate-term",
                        location,
                        format!(
                            "term '{}' already defined at {first} (define-once)",
                            term_def.key
                        ),
                    ));
                }
                else {
                    self.defined_terms.insert(term_def.key.clone(), location);
                }
            },
            | Inline::TermRef(ref term_ref) => {
                self.term_refs.push((
                    term_ref.key.clone(),
                    format!("{path}:<term key='{}'>", term_ref.key),
                ));
            },
            | Inline::Cite(ref cite) => {
                self.cites.push((
                    cite.key.clone(),
                    format!("{path}:<cite key='{}'>", cite.key),
                ));
            },
            | Inline::Text(_) | Inline::Math(_) => {},
        }
    }

    /// Resolve deferred references and produce the remaining diagnostics.
    fn finish(
        &mut self,
        reference_keys: &BTreeSet<String>,
    )
    {
        for entry in &self.term_refs {
            let (key, location) = (&entry.0, &entry.1);
            if !self.defined_terms.contains_key(key) {
                self.diagnostics.push(Diagnostic::new(
                    "unresolved-term",
                    location.clone(),
                    format!("term '{key}' is referenced but never defined"),
                ));
            }
        }
        for entry in &self.cites {
            let (key, location) = (&entry.0, &entry.1);
            if !reference_keys.contains(key) {
                self.diagnostics.push(Diagnostic::new(
                    "unresolved-cite",
                    location.clone(),
                    format!("cite key '{key}' is not present in the references file"),
                ));
            }
        }
        for entry in &self.provenance {
            let (target, location) = (&entry.0, &entry.1);
            if !self.component_ids.contains(target) {
                self.diagnostics.push(Diagnostic::new(
                    "unresolved-provenance",
                    location.clone(),
                    format!("provenance edge '{target}' does not resolve to a component"),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests
{
    //! Validator witnesses: duplicate identifier, unresolved term, unresolved
    //! cite, and missing status.

    use alloc::collections::BTreeSet;
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::Corpus;
    use super::validate_corpus;
    use crate::Diagnostic;
    use crate::model::Block;
    use crate::model::CiteKey;
    use crate::model::Document;
    use crate::model::Inline;
    use crate::model::Status;
    use crate::model::TermDef;
    use crate::model::TermRef;
    use crate::parse::parse_document;

    /// Render diagnostics as a single Display line for assert context.
    fn summary(diagnostics: &[Diagnostic]) -> String
    {
        diagnostics
            .iter()
            .map(alloc::string::ToString::to_string)
            .collect::<Vec<String>>()
            .join("; ")
    }

    /// Build a minimal component with the given blocks.
    fn component(
        id: &str,
        blocks: Vec<Block>,
    ) -> Document
    {
        Document {
            id: id.to_owned(),
            spec_version: "1".to_owned(),
            title: "T".to_owned(),
            status: Status::Partial,
            grounds: Vec::new(),
            derives: Vec::new(),
            blocks,
            source_path: format!("mem:{id}.xml"),
        }
    }

    /// Two components sharing an identifier are a duplicate-id violation.
    #[test]
    fn duplicate_component_id_is_flagged()
    {
        let corpus = Corpus::new(
            alloc::vec![component("dup", Vec::new()), component("dup", Vec::new())],
            BTreeSet::new(),
        );
        let diagnostics = validate_corpus(&corpus);
        assert!(
            diagnostics.iter().any(|d| d.code == "duplicate-id"),
            "expected a duplicate-id diagnostic, got {}",
            summary(&diagnostics),
        );
    }

    /// A term reference with no matching definition is unresolved.
    #[test]
    fn unresolved_term_reference_is_flagged()
    {
        let prose = Block::Prose(alloc::vec![Inline::TermRef(TermRef {
            key: "ghost".to_owned(),
        })]);
        let corpus = Corpus::new(
            alloc::vec![component("c", alloc::vec![prose])],
            BTreeSet::new(),
        );
        let diagnostics = validate_corpus(&corpus);
        assert!(
            diagnostics.iter().any(|d| d.code == "unresolved-term"),
            "expected an unresolved-term diagnostic, got {}",
            summary(&diagnostics),
        );
    }

    /// A term defined twice violates define-once.
    #[test]
    fn duplicate_term_definition_is_flagged()
    {
        let define = |key: &str| {
            Block::Prose(alloc::vec![Inline::TermDef(TermDef {
                key: key.to_owned(),
                text: "x".to_owned(),
            })])
        };
        let corpus = Corpus::new(
            alloc::vec![component("c", alloc::vec![define("t"), define("t")])],
            BTreeSet::new(),
        );
        let diagnostics = validate_corpus(&corpus);
        assert!(
            diagnostics.iter().any(|d| d.code == "duplicate-term"),
            "expected a duplicate-term diagnostic, got {}",
            summary(&diagnostics),
        );
    }

    /// A cite key absent from the references file is unresolved.
    #[test]
    fn unresolved_cite_key_is_flagged()
    {
        let prose = Block::Prose(alloc::vec![Inline::Cite(CiteKey {
            key: "Z-99".to_owned(),
        })]);
        let corpus = Corpus::new(
            alloc::vec![component("c", alloc::vec![prose])],
            BTreeSet::new(),
        );
        let diagnostics = validate_corpus(&corpus);
        assert!(
            diagnostics.iter().any(|d| d.code == "unresolved-cite"),
            "expected an unresolved-cite diagnostic, got {}",
            summary(&diagnostics),
        );
    }

    /// A resolvable cite key produces no cite diagnostic.
    #[test]
    fn resolvable_cite_key_passes()
    {
        let prose = Block::Prose(alloc::vec![Inline::Cite(CiteKey {
            key: "A-1a".to_owned(),
        })]);
        let mut keys = BTreeSet::new();
        keys.insert("A-1a".to_owned());
        let corpus = Corpus::new(alloc::vec![component("c", alloc::vec![prose])], keys);
        let diagnostics = validate_corpus(&corpus);
        assert!(
            !diagnostics.iter().any(|d| d.code == "unresolved-cite"),
            "expected no unresolved-cite diagnostic, got {}",
            summary(&diagnostics),
        );
    }

    /// A component element without a status attribute is a parse-level
    /// violation.
    #[test]
    fn missing_status_is_flagged() -> Result<(), crate::DocError>
    {
        let xml = r#"<component id="c" spec-version="1" title="T"></component>"#;
        let parsed = parse_document(std::path::Path::new("mem:c.xml"), xml)?;
        assert!(
            parsed.document.is_none(),
            "a component missing status must not yield a document",
        );
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|d| d.code == "missing-attribute" && d.message.contains("status")),
            "expected a missing status diagnostic, got {}",
            summary(&parsed.diagnostics),
        );
        Ok(())
    }
}
