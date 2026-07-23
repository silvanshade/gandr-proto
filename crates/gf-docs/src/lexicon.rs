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
use gandr_workflow_grammatical_framework::sexp::Sexp;
use gandr_workflow_grammatical_framework::sexp::unquote;

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
                gf_str(text)
            ));
        }
        for (constant, key) in &self.cites {
            out.push_str(&format!("    {constant} = {{ key = {} }} ;\n", gf_str(key)));
        }
        for (constant, (id, title)) in &self.anchors {
            out.push_str(&format!(
                "    {constant} = {{ id = {} ; title = {} }} ;\n",
                gf_str(id),
                gf_str(title)
            ));
        }
        out.push_str("}\n");
        out
    }
}

/// Generate the corpus-wide lexicon from the `.gfd` corpus (the production
/// path) and `refs.yml`.
///
/// The `.gfd` trees carry every text-destined string already `HTML`-escaped
/// (the translation boundary), so values insert as-is; the transition-era
/// `XML` collector ([`generate_xml`]) escapes at insert to agree exactly.
///
/// # Errors
/// [`GfDocsError::Parse`] when a `.gfd` file fails the reader or departs from
/// the expected constructor shapes; [`GfDocsError::Model`] when the
/// bibliography fails to load; [`GfDocsError::Translation`] on any id/term
/// collision.
pub fn generate(
    corpus_dir: &Path,
    refs_path: &Path,
) -> Result<Lexicon, GfDocsError>
{
    let mut lexicon = Lexicon::default();
    let mut files = std::fs::read_dir(corpus_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "gfd"))
        .collect::<Vec<_>>();
    files.sort();
    for path in files {
        let text = std::fs::read_to_string(&path)?;
        let tree = gandr_workflow_grammatical_framework::sexp::parse(&text)?;
        collect_gfd(&tree, &mut lexicon)?;
    }
    let bibliography =
        bibliography::load(refs_path).map_err(|e| GfDocsError::Model(e.to_string()))?;
    for key in bibliography.key_set() {
        lexicon.insert_cite(&key)?;
    }
    Ok(lexicon)
}

/// Generate the corpus-wide lexicon from the legacy `XML` components (the
/// transition-era bootstrap: the migration invariant asserts it agrees with
/// [`generate`] exactly while the `XML` still exists).
///
/// # Errors
/// [`GfDocsError::Model`] when a component fails the legacy parse or the
/// bibliography fails to load; [`GfDocsError::Translation`] on any id/term
/// collision or a declared-id-less section, rule, or definition.
pub fn generate_xml(
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
    lexicon.insert_anchor(&document.id, &escape_text(&document.title))?;
    let mut pending: Vec<&Block> = document.blocks.iter().collect();
    while let Some(block) = pending.pop() {
        match *block {
            | Block::Section(ref section) => {
                let id = declared_id(section.id.as_deref(), "section")?;
                lexicon.insert_anchor(id, &escape_text(&section.title))?;
                pending.extend(section.blocks.iter());
            },
            | Block::Example(ref example) => pending.extend(example.blocks.iter()),
            | Block::Rule(ref rule) => {
                lexicon.insert_anchor(
                    declared_id(rule.id.as_deref(), "rule")?,
                    &escape_text(&rule.name),
                )?;
            },
            | Block::Definition(ref definition) => {
                lexicon.insert_anchor(
                    declared_id(definition.id.as_deref(), "definition")?,
                    &escape_text(&format!("{} (definition)", definition.term)),
                )?;
                lexicon.insert_term(&definition.term, &escape_text(&definition.term))?;
                harvest_inlines(&definition.body, lexicon)?;
            },
            | Block::Diagram(ref diagram) => {
                lexicon.insert_anchor(&diagram.id, &escape_text(&diagram.caption))?;
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
            lexicon.insert_term(&definition.key, &escape_text(&definition.text))?;
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

// ── the `.gfd` collector (the production path) ──────────────────────────────

/// Collect every lexicon record one `.gfd` component tree declares.
fn collect_gfd(
    tree: &Sexp,
    lexicon: &mut Lexicon,
) -> Result<(), GfDocsError>
{
    let args = expect_app(tree, "MkComponent")?;
    let [anchor, title, _status, _grounds, _derives, sections, _refs] = args
    else {
        return Err(GfDocsError::Parse("MkComponent arity is not seven".into()));
    };
    lexicon.insert_anchor(&anchor_id(anchor)?, &quoted(title)?)?;
    walk_list(sections, "Section", &mut |section| {
        collect_section(section, lexicon)
    })
}

/// Collect one section's records (its anchor, then its blocks).
fn collect_section(
    section: &Sexp,
    lexicon: &mut Lexicon,
) -> Result<(), GfDocsError>
{
    let args = expect_app(section, "MkSection")?;
    let [anchor, title, _status, blocks] = args
    else {
        return Err(GfDocsError::Parse("MkSection arity is not four".into()));
    };
    lexicon.insert_anchor(&anchor_id(anchor)?, &quoted(title)?)?;
    walk_list(blocks, "Block", &mut |block| collect_block(block, lexicon))
}

/// Dispatch one block to its collector.
fn collect_block(
    block: &Sexp,
    lexicon: &mut Lexicon,
) -> Result<(), GfDocsError>
{
    let Sexp::App { head, args } = block
    else {
        return Ok(());
    };
    match head.as_str() {
        | "NestedSection" => {
            let [section] = args.as_slice()
            else {
                return Err(GfDocsError::Parse("NestedSection arity is not one".into()));
            };
            collect_section(section, lexicon)
        },
        | "DefinitionBlock" => {
            let [anchor, term, body] = args.as_slice()
            else {
                return Err(GfDocsError::Parse(
                    "DefinitionBlock arity is not three".into(),
                ));
            };
            let key = unmangle(atom_of(term)?, "term_")?;
            lexicon.insert_anchor(
                &anchor_id(anchor)?,
                &escape_text(&format!("{key} (definition)")),
            )?;
            lexicon.insert_term(&key, &escape_text(&key))?;
            harvest_inline_terms(body, lexicon)
        },
        | "RuleBlock" => {
            let [anchor, name, _premises, _conclusion] = args.as_slice()
            else {
                return Err(GfDocsError::Parse("RuleBlock arity is not four".into()));
            };
            lexicon.insert_anchor(&anchor_id(anchor)?, &quoted(name)?)
        },
        | "DiagramBlock" => {
            let [anchor, caption, _cite, _source] = args.as_slice()
            else {
                return Err(GfDocsError::Parse("DiagramBlock arity is not four".into()));
            };
            lexicon.insert_anchor(&anchor_id(anchor)?, &quoted(caption)?)
        },
        | "ProseBlock" => {
            let [inlines] = args.as_slice()
            else {
                return Err(GfDocsError::Parse("ProseBlock arity is not one".into()));
            };
            harvest_inline_terms(inlines, lexicon)
        },
        | "ExampleBlock" => {
            let [_title, blocks] = args.as_slice()
            else {
                return Err(GfDocsError::Parse("ExampleBlock arity is not two".into()));
            };
            walk_list(blocks, "Block", &mut |inner| collect_block(inner, lexicon))
        },
        | "RegisterBlock" | "PlainRegisterBlock" => {
            let [_order, items] = args.as_slice()
            else {
                return Err(GfDocsError::Parse("register arity is not two".into()));
            };
            walk_list(items, "Item", &mut |item| collect_item(item, lexicon))
        },
        | "InventoryBlock" | "StagingPlanBlock" | "DecisionTableBlock" => {
            let [_caption, header, rows] = args.as_slice()
            else {
                return Err(GfDocsError::Parse("table arity is not three".into()));
            };
            collect_row(header, lexicon)?;
            walk_list(rows, "Row", &mut |row| collect_row(row, lexicon))
        },
        | _ => Ok(()),
    }
}

/// Collect one list item's inline terms.
fn collect_item(
    item: &Sexp,
    lexicon: &mut Lexicon,
) -> Result<(), GfDocsError>
{
    let Sexp::App { head, args } = item
    else {
        return Ok(());
    };
    let body = match (head.as_str(), args.as_slice()) {
        | ("MkItem", [_lead, body]) => body,
        | ("MkPlainItem", [body]) => body,
        | _ => return Ok(()),
    };
    harvest_inline_terms(body, lexicon)
}

/// Collect one table row's inline terms.
fn collect_row(
    row: &Sexp,
    lexicon: &mut Lexicon,
) -> Result<(), GfDocsError>
{
    let Sexp::App { head, args } = row
    else {
        return Ok(());
    };
    if !matches!(head.as_str(), "MkHeaderRow" | "MkBodyRow") {
        return Ok(());
    }
    let [cells] = args.as_slice()
    else {
        return Err(GfDocsError::Parse("row arity is not one".into()));
    };
    walk_list(cells, "Cell", &mut |cell| {
        let args = expect_app(cell, "MkCell")?;
        let [content] = args
        else {
            return Err(GfDocsError::Parse("MkCell arity is not one".into()));
        };
        harvest_inline_terms(content, lexicon)
    })
}

/// Harvest the inline term definitions one inline list carries.
fn harvest_inline_terms(
    inlines: &Sexp,
    lexicon: &mut Lexicon,
) -> Result<(), GfDocsError>
{
    walk_list_any(inlines, &mut |inline| {
        if let Sexp::App { head, args } = inline {
            match head.as_str() {
                | "TermDef" => {
                    let [term, display] = args.as_slice()
                    else {
                        return Err(GfDocsError::Parse("TermDef arity is not two".into()));
                    };
                    lexicon.insert_term(&unmangle(atom_of(term)?, "term_")?, &quoted(display)?)?;
                },
                | "Bold" | "Italic" => {
                    let [inner] = args.as_slice()
                    else {
                        return Err(GfDocsError::Parse("emphasis arity is not one".into()));
                    };
                    harvest_inline_terms(inner, lexicon)?;
                },
                | _ => {},
            }
        }
        Ok(())
    })
}

/// Walk a `Cons`/`Base` list of the named category, applying `visit` to each
/// element.
fn walk_list(
    list: &Sexp,
    category: &str,
    visit: &mut dyn FnMut(&Sexp) -> Result<(), GfDocsError>,
) -> Result<(), GfDocsError>
{
    let mut cursor = list;
    loop {
        match cursor {
            | Sexp::Atom(atom) if *atom == format!("Base{category}") => return Ok(()),
            | Sexp::App { head, args } if *head == format!("Cons{category}") => {
                let [element, tail] = args.as_slice()
                else {
                    return Err(GfDocsError::Parse(format!(
                        "Cons{category} arity is not two"
                    )));
                };
                visit(element)?;
                cursor = tail;
            },
            | _ => return Err(GfDocsError::Parse(format!("malformed [{category}] list"))),
        }
    }
}

/// Walk any `Cons`/`Base` inline list (the glue constructor included).
fn walk_list_any(
    list: &Sexp,
    visit: &mut dyn FnMut(&Sexp) -> Result<(), GfDocsError>,
) -> Result<(), GfDocsError>
{
    let mut cursor = list;
    loop {
        match cursor {
            | Sexp::Atom(atom) if atom.starts_with("Base") => return Ok(()),
            | Sexp::App { head, args } if head.starts_with("Cons") => {
                let [element, tail] = args.as_slice()
                else {
                    return Err(GfDocsError::Parse(format!("{head} arity is not two")));
                };
                visit(element)?;
                cursor = tail;
            },
            | _ => return Err(GfDocsError::Parse("malformed list expression".into())),
        }
    }
}

/// The arguments of an application with the expected head constructor.
fn expect_app<'tree>(
    sexp: &'tree Sexp,
    head: &str,
) -> Result<&'tree [Sexp], GfDocsError>
{
    match sexp {
        | Sexp::App { head: actual, args } if actual == head => Ok(args),
        | _ => Err(GfDocsError::Parse(format!("expected a {head} application"))),
    }
}

/// The text of an atom expression.
fn atom_of(sexp: &Sexp) -> Result<&str, GfDocsError>
{
    match sexp {
        | Sexp::Atom(text) => Ok(text),
        | _ => Err(GfDocsError::Parse("expected an atom".into())),
    }
}

/// The unquoted text of a string-literal atom expression.
fn quoted(sexp: &Sexp) -> Result<String, GfDocsError>
{
    unquote(atom_of(sexp)?).ok_or_else(|| GfDocsError::Parse("expected a string literal".into()))
}

/// The id behind an anchor constant.
fn anchor_id(sexp: &Sexp) -> Result<String, GfDocsError>
{
    unmangle(atom_of(sexp)?, "anchor_")
}

/// Reverse the corpus key-to-constant mangle (`prefix` stripped, `_` back to
/// `-`): exact under the lexicon's own collision detector (no corpus key
/// contains an underscore).
fn unmangle(
    constant: &str,
    prefix: &str,
) -> Result<String, GfDocsError>
{
    constant
        .strip_prefix(prefix)
        .map(|rest| rest.replace('_', "-"))
        .ok_or_else(|| GfDocsError::Parse(format!("'{constant}' lacks the '{prefix}' prefix")))
}
