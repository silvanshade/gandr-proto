//! Corpus-level validation for the prose document classes.
//!
//! Parsing (see [`crate::doc::parse`]) enforces per-file structure, banner
//! placement, and status presence; this module enforces the class schema and
//! the cross-reference invariants: document-id uniqueness, the banner
//! obligation each class carries, label define-once within a document, and
//! label, cite, and coined-anchor resolution. Any violation is a
//! [`crate::Diagnostic`]; a run with diagnostics fails.
//!
//! The class boundary is deliberately minimal. All three classes share the
//! block substrate; they differ only in their banner obligation and in whether
//! they may coin cross-referenced labels — the `R1`/`HZ-1`/`O1` anchor device
//! is a research-record affordance, so a workflow or crate-status document that
//! coins one is a schema violation.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Source path attached to diagnostics for one document.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct DocumentPath<'path>(&'path str);

impl<'path> From<&'path str> for DocumentPath<'path>
{
    #[inline]
    fn from(path: &'path str) -> Self
    {
        Self(path)
    }
}

impl fmt::Display for DocumentPath<'_>
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(self.0)
    }
}

/// Whether a document class admits coined labels.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct LabelAdmission(bool);

impl From<LabelAdmission> for bool
{
    #[inline]
    fn from(admission: LabelAdmission) -> Self
    {
        admission.0
    }
}

use crate::Diagnostic;
use crate::doc::model::Banner;
use crate::doc::model::DocBlock;
use crate::doc::model::DocClass;
use crate::doc::model::DocInline;
use crate::doc::model::DocRecord;

/// A validated (or to-be-validated) prose-document corpus: the parsed records
/// plus the resolvable cite-key set from the references file.
#[derive(Clone, Debug, Default)]
pub struct DocCorpus
{
    /// Parsed documents, in discovery order.
    pub records: Vec<DocRecord>,
    /// Cite keys declared in the hayagriva references file.
    pub reference_keys: BTreeSet<String>,
}

impl DocCorpus
{
    /// Build a corpus from parsed records and reference keys.
    #[inline]
    #[must_use]
    pub fn new(
        records: Vec<DocRecord>,
        reference_keys: BTreeSet<String>,
    ) -> Self
    {
        Self {
            records,
            reference_keys,
        }
    }
}

/// Whether a class may coin cross-referenced labels (the anchor device).
///
/// The recommendation/hazard/question anchor (`R1`, `HZ-1`, `O1`) is a
/// research-record affordance; the other classes carry no such device.
#[inline]
#[must_use]
const fn admits_labels(class: DocClass) -> LabelAdmission
{
    LabelAdmission(matches!(class, DocClass::ResearchRecord))
}

/// Validate a prose-document corpus, returning every violation as a diagnostic.
///
/// The returned vector is sorted, so the report is deterministic. An empty
/// result means the corpus conforms.
#[inline]
#[must_use]
pub fn validate_doc_corpus(corpus: &DocCorpus) -> Vec<Diagnostic>
{
    let mut diagnostics = Vec::new();
    let mut ids: BTreeMap<String, String> = BTreeMap::new();
    for record in &corpus.records {
        let location = format!(
            "{}:<{}#{}>",
            record.source_path,
            record.class.as_ref(),
            record.id,
        );
        if let Some(first) = ids.get(&record.id) {
            diagnostics.push(Diagnostic::new(
                "duplicate-id".into(),
                location,
                format!("id '{}' already declared at {first}", record.id),
            ));
        }
        else {
            ids.insert(record.id.clone(), location);
        }
        validate_record(record, &corpus.reference_keys, &mut diagnostics);
    }
    diagnostics.sort();
    diagnostics
}

/// Validate one document against its class schema and cross-reference rules.
fn validate_record(
    record: &DocRecord,
    reference_keys: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
)
{
    let path: DocumentPath<'_> = record.source_path.as_str().into();
    check_banner(record.class, &record.banner, path, diagnostics);

    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    let mut refs: Vec<(String, String)> = Vec::new();
    let mut cites: Vec<(String, String)> = Vec::new();

    if let Some(ref read_when) = record.banner.read_when {
        collect_inlines(
            read_when,
            record.class,
            path,
            &mut labels,
            &mut refs,
            &mut cites,
            diagnostics,
        );
    }
    for note in &record.banner.notes {
        collect_inlines(
            note,
            record.class,
            path,
            &mut labels,
            &mut refs,
            &mut cites,
            diagnostics,
        );
    }
    for block in &record.blocks {
        collect_block_inlines(
            block,
            record.class,
            path,
            &mut labels,
            &mut refs,
            &mut cites,
            diagnostics,
        );
    }

    for entry in &refs {
        let (key, location) = (&entry.0, &entry.1);
        if !labels.contains_key(key) {
            diagnostics.push(Diagnostic::new(
                "unresolved-label".into(),
                location.clone(),
                format!("label '{key}' is referenced but never coined in this document"),
            ));
        }
    }
    for entry in &cites {
        let (key, location) = (&entry.0, &entry.1);
        if !reference_keys.contains(key) {
            diagnostics.push(Diagnostic::new(
                "unresolved-cite".into(),
                location.clone(),
                format!("cite key '{key}' is not present in the references file"),
            ));
        }
    }
}

/// Apply the class banner obligation.
fn check_banner(
    class: DocClass,
    banner: &Banner,
    path: DocumentPath<'_>,
    diagnostics: &mut Vec<Diagnostic>,
)
{
    if bool::from(banner.is_empty()) {
        diagnostics.push(Diagnostic::new(
            "empty-banner".into(),
            format!("{path}:<banner>"),
            "a prose document banner must carry a read-when line or a note".to_owned(),
        ));
    }
    if class == DocClass::WorkflowDoc && banner.read_when.is_none() {
        diagnostics.push(Diagnostic::new(
            "missing-read-when".into(),
            format!("{path}:<banner>"),
            "a workflow document banner must carry a <read-when> line".to_owned(),
        ));
    }
}

/// Collect the inline references reachable from one block and its descendants.
///
/// # Contract
///
/// - requires: `block` is a finite, structurally well-formed block tree.
/// - ensures: records every nested label definition, label reference, and cite
///   exactly once in document order, retaining the first definition of a key.
/// - provides: updated label, reference, cite, and diagnostic state.
/// - panics: none.
/// - intension: [`DocBlock::walk`] supplies depth-first document order without
///   input-scaled native-stack recursion.
///
/// # Adequacy
///
/// - hypothesis: L3 pointwise — a duplicate nested label exposes missing,
///   repeated, or reordered traversal through the exact diagnostic and the
///   retained first-definition location.
/// - witness: `tests::nested_sections_validate_in_order`.
fn collect_block_inlines(
    block: &DocBlock,
    class: DocClass,
    path: DocumentPath<'_>,
    labels: &mut BTreeMap<String, String>,
    refs: &mut Vec<(String, String)>,
    cites: &mut Vec<(String, String)>,
    diagnostics: &mut Vec<Diagnostic>,
)
{
    for block in block.walk() {
        match *block {
            | DocBlock::Prose(ref inlines) => {
                collect_inlines(inlines, class, path, labels, refs, cites, diagnostics);
            },
            | DocBlock::List(ref list) => {
                for item in &list.items {
                    collect_inlines(&item.body, class, path, labels, refs, cites, diagnostics);
                }
            },
            | DocBlock::Table(ref table) => {
                for cell in &table.header {
                    collect_inlines(cell.as_ref(), class, path, labels, refs, cites, diagnostics);
                }
                for row in &table.rows {
                    for cell in row.as_ref() {
                        collect_inlines(
                            cell.as_ref(),
                            class,
                            path,
                            labels,
                            refs,
                            cites,
                            diagnostics,
                        );
                    }
                }
            },
            | DocBlock::Section(_) | DocBlock::Code(_) => {},
        }
    }
}

/// Collect the label, reference, and cite inlines of one inline sequence.
fn collect_inlines(
    inlines: &[DocInline],
    class: DocClass,
    path: DocumentPath<'_>,
    labels: &mut BTreeMap<String, String>,
    refs: &mut Vec<(String, String)>,
    cites: &mut Vec<(String, String)>,
    diagnostics: &mut Vec<Diagnostic>,
)
{
    for inline in inlines {
        match *inline {
            | DocInline::Text(_) | DocInline::InlineCode(_) => {},
            | DocInline::Label(ref label) => {
                let location = format!("{path}:<label key='{}'>", label.key);
                if !bool::from(admits_labels(class)) {
                    diagnostics.push(Diagnostic::new(
                        "disallowed-inline".into(),
                        location.clone(),
                        format!(
                            "<label> is only permitted in a research-record, not a {}",
                            class.as_ref()
                        ),
                    ));
                }
                if let Some(first) = labels.get(&label.key) {
                    diagnostics.push(Diagnostic::new(
                        "duplicate-label".into(),
                        location,
                        format!(
                            "label '{}' already coined at {first} (define-once)",
                            label.key
                        ),
                    ));
                }
                else {
                    labels.insert(label.key.clone(), location);
                }
            },
            | DocInline::Ref(ref label_ref) => {
                if !bool::from(admits_labels(class)) {
                    diagnostics.push(Diagnostic::new(
                        "disallowed-inline".into(),
                        format!("{path}:<ref key='{}'>", label_ref.as_ref()),
                        format!(
                            "<ref> is only permitted in a research-record, not a {}",
                            class.as_ref()
                        ),
                    ));
                }
                refs.push((
                    label_ref.as_ref().to_owned(),
                    format!("{path}:<ref key='{}'>", label_ref.as_ref()),
                ));
            },
            | DocInline::Cite(ref cite) => {
                cites.push((
                    cite.as_ref().to_owned(),
                    format!("{path}:<cite key='{}'>", cite.as_ref()),
                ));
            },
        }
    }
}

#[cfg(test)]
mod tests
{
    //! Validator witnesses: duplicate label, unresolved label, unresolved cite,
    //! disallowed label outside a research-record, and nested define-once
    //! order.

    use alloc::collections::BTreeSet;
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::DocCorpus;
    use super::validate_doc_corpus;
    use crate::Diagnostic;
    use crate::doc::model::Banner;
    use crate::doc::model::DocBlock;
    use crate::doc::model::DocClass;
    use crate::doc::model::DocInline;
    use crate::doc::model::DocItem;
    use crate::doc::model::DocList;
    use crate::doc::model::DocRecord;
    use crate::doc::model::DocSection;
    use crate::doc::model::Label;
    use crate::doc::model::LabelRef;
    use crate::model::CiteKey;
    use crate::model::Status;

    /// Document id carried by a validation fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct FixtureDocumentId<'id>(&'id str);

    impl<'id> From<&'id str> for FixtureDocumentId<'id>
    {
        #[inline]
        fn from(id: &'id str) -> Self
        {
            Self(id)
        }
    }

    /// Coined-label key carried by a validation fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct FixtureLabelKey<'key>(&'key str);

    impl<'key> From<&'key str> for FixtureLabelKey<'key>
    {
        #[inline]
        fn from(key: &'key str) -> Self
        {
            Self(key)
        }
    }

    /// Render diagnostics as a single Display line for assert context.
    fn summary(diagnostics: &[Diagnostic]) -> String
    {
        diagnostics
            .iter()
            .map(alloc::string::ToString::to_string)
            .collect::<Vec<String>>()
            .join("; ")
    }

    /// Build a minimal research-record with a non-empty banner and the blocks.
    fn research(
        id: FixtureDocumentId<'_>,
        blocks: Vec<DocBlock>,
    ) -> DocRecord
    {
        DocRecord {
            class: DocClass::ResearchRecord,
            id: id.0.to_owned(),
            title: "T".to_owned(),
            status: Status::DesignPass,
            crate_scope: None,
            banner: Banner {
                read_when: None,
                notes: alloc::vec![alloc::vec![DocInline::Text("scope".to_owned())]],
            },
            blocks,
            source_path: format!("mem:{}.xml", id.0),
        }
    }

    /// A single-item list whose one item carries the given inlines.
    fn list_of(inlines: Vec<DocInline>) -> DocBlock
    {
        DocBlock::List(DocList {
            ordered: false,
            items: alloc::vec![DocItem {
                lead: None,
                body: inlines,
            }],
        })
    }

    /// A prose block of the given inlines.
    fn prose(inlines: Vec<DocInline>) -> DocBlock
    {
        DocBlock::Prose(inlines)
    }

    /// A coined label with a fixed display text.
    fn label(key: FixtureLabelKey<'_>) -> DocInline
    {
        DocInline::Label(Label {
            key: key.0.to_owned(),
            text: key.0.to_owned(),
        })
    }

    /// A label defined twice within one document violates define-once.
    #[test]
    fn duplicate_label_is_flagged()
    {
        let corpus = DocCorpus::new(
            alloc::vec![research("r".into(), alloc::vec![
                prose(alloc::vec![label("R1".into())]),
                prose(alloc::vec![label("R1".into())]),
            ])],
            BTreeSet::new(),
        );
        let diagnostics = validate_doc_corpus(&corpus);
        assert!(
            diagnostics.iter().any(|d| d.code == "duplicate-label"),
            "expected a duplicate-label diagnostic, got {}",
            summary(&diagnostics),
        );
    }

    /// A label reference with no matching definition is unresolved.
    #[test]
    fn unresolved_label_reference_is_flagged()
    {
        let corpus = DocCorpus::new(
            alloc::vec![research("r".into(), alloc::vec![list_of(alloc::vec![
                DocInline::Ref(LabelRef::from("ghost")),
            ])])],
            BTreeSet::new(),
        );
        let diagnostics = validate_doc_corpus(&corpus);
        assert!(
            diagnostics.iter().any(|d| d.code == "unresolved-label"),
            "expected an unresolved-label diagnostic, got {}",
            summary(&diagnostics),
        );
    }

    /// A cite key absent from the references file is unresolved; a present one
    /// passes.
    #[test]
    fn cite_resolution_tracks_the_reference_keys()
    {
        let corpus = DocCorpus::new(
            alloc::vec![research("r".into(), alloc::vec![prose(alloc::vec![
                DocInline::Cite(CiteKey::from("A-1a")),
            ])])],
            BTreeSet::new(),
        );
        let diagnostics = validate_doc_corpus(&corpus);
        assert!(
            diagnostics.iter().any(|d| d.code == "unresolved-cite"),
            "expected an unresolved-cite diagnostic with no reference keys, got {}",
            summary(&diagnostics),
        );

        let mut keys = BTreeSet::new();
        keys.insert("A-1a".to_owned());
        let resolvable = DocCorpus::new(
            alloc::vec![research("r".into(), alloc::vec![prose(alloc::vec![
                DocInline::Cite(CiteKey::from("A-1a")),
            ])])],
            keys,
        );
        let clean = validate_doc_corpus(&resolvable);
        assert!(
            !clean.iter().any(|d| d.code == "unresolved-cite"),
            "expected no unresolved-cite diagnostic once the key resolves, got {}",
            summary(&clean),
        );
    }

    /// A coined label outside a research-record is a schema violation.
    #[test]
    fn label_outside_research_record_is_flagged()
    {
        let workflow = DocRecord {
            class: DocClass::WorkflowDoc,
            id: "w".to_owned(),
            title: "T".to_owned(),
            status: Status::Built,
            crate_scope: None,
            banner: Banner {
                read_when: Some(alloc::vec![DocInline::Text("editing".to_owned())]),
                notes: Vec::new(),
            },
            blocks: alloc::vec![prose(alloc::vec![label("R1".into())])],
            source_path: "mem:w.xml".to_owned(),
        };
        let corpus = DocCorpus::new(alloc::vec![workflow], BTreeSet::new());
        let diagnostics = validate_doc_corpus(&corpus);
        assert!(
            diagnostics.iter().any(|d| d.code == "disallowed-inline"),
            "expected a disallowed-inline diagnostic, got {}",
            summary(&diagnostics),
        );
    }

    /// A workflow document without a read-when line is flagged.
    #[test]
    fn workflow_without_read_when_is_flagged()
    {
        let workflow = DocRecord {
            class: DocClass::WorkflowDoc,
            id: "w".to_owned(),
            title: "T".to_owned(),
            status: Status::Built,
            crate_scope: None,
            banner: Banner {
                read_when: None,
                notes: alloc::vec![alloc::vec![DocInline::Text("note".to_owned())]],
            },
            blocks: Vec::new(),
            source_path: "mem:w.xml".to_owned(),
        };
        let corpus = DocCorpus::new(alloc::vec![workflow], BTreeSet::new());
        let diagnostics = validate_doc_corpus(&corpus);
        assert!(
            diagnostics.iter().any(|d| d.code == "missing-read-when"),
            "expected a missing-read-when diagnostic, got {}",
            summary(&diagnostics),
        );
    }

    /// Nested-section define-once retains the first label in document order.
    #[test]
    fn nested_sections_validate_in_order()
    {
        let inner = DocBlock::Section(DocSection {
            id: None,
            date: None,
            title: "Inner".to_owned(),
            blocks: alloc::vec![prose(alloc::vec![label("R1".into())])],
        });
        let outer = DocBlock::Section(DocSection {
            id: None,
            date: None,
            title: "Outer".to_owned(),
            blocks: alloc::vec![prose(alloc::vec![label("R1".into())]), inner],
        });
        let corpus = DocCorpus::new(
            alloc::vec![research("r".into(), alloc::vec![outer])],
            BTreeSet::new(),
        );
        let diagnostics = validate_doc_corpus(&corpus);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.code == "duplicate-label")
                .count(),
            1,
            "exactly one nested duplicate label, got {}",
            summary(&diagnostics),
        );
    }
}
