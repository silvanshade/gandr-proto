//! Lexicon generation (proposal §3.4): the corpus-wide scan that emits the
//! `GF` lexicon modules (`GandrDocsLex.gf` and `GandrDocsLexHtml.gf`).
//!
//! The generator is also the corpus's duplicate detector: every id of every
//! kind lands in one anchor namespace and every term key in one term
//! namespace, and generation fails on any collision — including two distinct
//! keys that mangle to the same `GF` constant. Output is deterministic
//! (records sorted by constant name) so the modules are committed derived
//! files (the `refs.yml` pattern: regenerate, never hand-edit).

use std::collections::BTreeMap;
use std::path::Path;

use gandr_workflow_docs::bibliography;
use gandr_workflow_docs::corpus::discover_component_files;
use gandr_workflow_docs::model::Block;
use gandr_workflow_docs::model::Document;
use gandr_workflow_docs::model::Inline;
use gandr_workflow_docs::parse::parse_document;

use crate::error::GfDocsError;
use crate::migrate::anchor_const;
use crate::migrate::cite_const;
use crate::migrate::escape_text;
use crate::migrate::gf_str;
use crate::migrate::term_const;

/// The collected lexicon: three namespaces of constant records, each keyed by
/// `GF` constant name for deterministic (sorted) emission.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Lexicon
{
    /// Term records: constant → (key, display text).
    terms: BTreeMap<String, (String, String)>,
    /// Cite records: constant → key.
    cites: BTreeMap<String, String>,
    /// Anchor records: constant → (id, display title).
    anchors: BTreeMap<String, (String, String)>,
}

impl Lexicon
{
    /// Insert one term record, deduplicating a `term-def`/definition-term
    /// pair that agrees on the display text.
    ///
    /// # Errors
    /// [`GfDocsError::Translation`] on a display-text conflict for one key or
    /// a constant collision between distinct keys.
    fn insert_term(
        &mut self,
        key: &str,
        text: &str,
    ) -> Result<(), GfDocsError>
    {
        let constant = term_const(key);
        match self.terms.get(&constant) {
            | Some((existing_key, existing_text)) if existing_key == key => {
                if existing_text != text {
                    return Err(GfDocsError::Translation(format!(
                        "term '{key}' declares conflicting display texts: '{existing_text}' vs '{text}'"
                    )));
                }
            },
            | Some((existing_key, _)) => {
                return Err(GfDocsError::Translation(format!(
                    "term keys '{existing_key}' and '{key}' mangle to the same constant '{constant}'"
                )));
            },
            | None => {
                self.terms
                    .insert(constant, (key.to_owned(), text.to_owned()));
            },
        }
        Ok(())
    }

    /// Insert one cite record.
    ///
    /// # Errors
    /// [`GfDocsError::Translation`] on a constant collision between distinct
    /// keys.
    fn insert_cite(
        &mut self,
        key: &str,
    ) -> Result<(), GfDocsError>
    {
        let constant = cite_const(key);
        if let Some(existing_key) = self.cites.get(&constant)
            && existing_key != key
        {
            return Err(GfDocsError::Translation(format!(
                "cite keys '{existing_key}' and '{key}' mangle to the same constant '{constant}'"
            )));
        }
        self.cites.insert(constant, key.to_owned());
        Ok(())
    }

    /// Insert one anchor record.
    ///
    /// # Errors
    /// [`GfDocsError::Translation`] on a duplicate id or a constant collision
    /// between distinct ids (the single-namespace detector).
    fn insert_anchor(
        &mut self,
        id: &str,
        title: &str,
    ) -> Result<(), GfDocsError>
    {
        let constant = anchor_const(id);
        match self.anchors.get(&constant) {
            | Some((existing_id, _)) if existing_id == id => {
                return Err(GfDocsError::Translation(format!(
                    "anchor id '{id}' is declared twice in the corpus"
                )));
            },
            | Some((existing_id, _)) => {
                return Err(GfDocsError::Translation(format!(
                    "anchor ids '{existing_id}' and '{id}' mangle to the same constant '{constant}'"
                )));
            },
            | None => {
                self.anchors
                    .insert(constant, (id.to_owned(), title.to_owned()));
            },
        }
        Ok(())
    }

    /// Render the abstract lexicon module (`GandrDocsLex.gf`).
    #[must_use]
    pub fn render_abstract(&self) -> String
    {
        let mut out = String::from("abstract GandrDocsLex = GandrDocs ** {\n  fun\n");
        out.push_str("    -- terms (generated from the corpus term registry)\n");
        for constant in self.terms.keys() {
            out.push_str(&format!("    {constant} : Term ;\n"));
        }
        out.push_str("    -- cite keys (generated from refs.yml)\n");
        for constant in self.cites.keys() {
            out.push_str(&format!("    {constant} : CiteKey ;\n"));
        }
        out.push_str("    -- anchors (generated from the corpus id namespace)\n");
        for constant in self.anchors.keys() {
            out.push_str(&format!("    {constant} : Anchor ;\n"));
        }
        out.push_str("}\n");
        out
    }

    /// Render the concrete lexicon module (`GandrDocsLexHtml.gf`).
    #[must_use]
    pub fn render_concrete(&self) -> String
    {
        let mut out =
            String::from("concrete GandrDocsLexHtml of GandrDocsLex = GandrDocsHtml ** {\n  lin\n");
        for (constant, (key, text)) in &self.terms {
            out.push_str(&format!(
                "    {constant} = {{ key = {} ; text = {} }} ;\n",
                gf_str(key),
                gf_str(&escape_text(text))
            ));
        }
        for (constant, key) in &self.cites {
            out.push_str(&format!("    {constant} = {{ key = {} }} ;\n", gf_str(key)));
        }
        for (constant, (id, title)) in &self.anchors {
            out.push_str(&format!(
                "    {constant} = {{ id = {} ; title = {} }} ;\n",
                gf_str(id),
                gf_str(&escape_text(title))
            ));
        }
        out.push_str("}\n");
        out
    }
}

/// Generate the corpus-wide lexicon from the spec components and `refs.yml`.
///
/// # Errors
/// [`GfDocsError::Model`] when a component fails the legacy parse or the
/// bibliography fails to load; [`GfDocsError::Translation`] on any id/term
/// collision or a declared-id-less section, rule, or definition.
pub fn generate(
    spec_dir: &Path,
    refs_path: &Path,
) -> Result<Lexicon, GfDocsError>
{
    let mut lexicon = Lexicon::default();
    for path in discover_component_files(spec_dir).map_err(|e| GfDocsError::Model(e.to_string()))? {
        let xml = std::fs::read_to_string(&path)?;
        let parsed = parse_document(&path, &xml).map_err(|e| GfDocsError::Model(e.to_string()))?;
        let document = parsed.document.ok_or_else(|| {
            GfDocsError::Model(format!("{}: no document produced", path.display()))
        })?;
        collect(&document, &mut lexicon)?;
    }
    let bibliography =
        bibliography::load(refs_path).map_err(|e| GfDocsError::Model(e.to_string()))?;
    for key in bibliography.key_set() {
        lexicon.insert_cite(&key)?;
    }
    Ok(lexicon)
}

/// Collect every lexicon record one component declares.
fn collect(
    document: &Document,
    lexicon: &mut Lexicon,
) -> Result<(), GfDocsError>
{
    lexicon.insert_anchor(&document.id, &document.title)?;
    let mut pending: Vec<&Block> = document.blocks.iter().collect();
    while let Some(block) = pending.pop() {
        match *block {
            | Block::Section(ref section) => {
                let id = declared_id(section.id.as_deref(), "section")?;
                lexicon.insert_anchor(id, &section.title)?;
                pending.extend(section.blocks.iter());
            },
            | Block::Example(ref example) => pending.extend(example.blocks.iter()),
            | Block::Rule(ref rule) => {
                lexicon.insert_anchor(declared_id(rule.id.as_deref(), "rule")?, &rule.name)?;
            },
            | Block::Definition(ref definition) => {
                lexicon.insert_anchor(
                    declared_id(definition.id.as_deref(), "definition")?,
                    &format!("{} (definition)", definition.term),
                )?;
                lexicon.insert_term(&definition.term, &definition.term)?;
                harvest_inlines(&definition.body, lexicon)?;
            },
            | Block::Diagram(ref diagram) => {
                lexicon.insert_anchor(&diagram.id, &diagram.caption)?;
            },
            | Block::Prose(ref inlines) => harvest_inlines(inlines, lexicon)?,
            | Block::List(ref list) => {
                for item in &list.items {
                    harvest_inlines(&item.body, lexicon)?;
                }
            },
            | Block::Table(ref table) => {
                for cell in table
                    .header
                    .iter()
                    .chain(table.rows.iter().flat_map(|row| row.cells.iter()))
                {
                    harvest_inlines(&cell.content, lexicon)?;
                }
            },
            | _ => {},
        }
    }
    Ok(())
}

/// Harvest the inline term definitions one inline sequence carries.
fn harvest_inlines(
    inlines: &[Inline],
    lexicon: &mut Lexicon,
) -> Result<(), GfDocsError>
{
    for inline in inlines {
        if let Inline::TermDef(ref definition) = *inline {
            lexicon.insert_term(&definition.key, &definition.text)?;
        }
    }
    Ok(())
}

/// Unwrap an optional declared id, naming its owner kind on absence.
fn declared_id<'a>(
    id: Option<&'a str>,
    kind: &str,
) -> Result<&'a str, GfDocsError>
{
    id.ok_or_else(|| GfDocsError::Translation(format!("{kind} without id")))
}
