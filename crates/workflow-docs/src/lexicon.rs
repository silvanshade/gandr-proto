//! Lexicon generation (proposal §3.4): the corpus-wide scan that emits the
//! `GF` lexicon modules (`GandrDocsLex.gf` and `GandrDocsLexHtml.gf`).
//!
//! The generator is also the corpus's duplicate detector: every id of every
//! kind lands in one anchor namespace and every term key in one term
//! namespace, and generation fails on any collision — including two distinct
//! keys that mangle to the same `GF` constant. Output is deterministic
//! (records sorted by constant name) so the modules are committed derived
//! files (the `refs.yml` pattern: regenerate, never hand-edit).

use alloc::collections::BTreeMap;
use core::fmt::Write as _;
use std::path::Path;

use gandr_workflow_grammatical_framework::rt::ExprText;
use gandr_workflow_grammatical_framework::rt::GfRuntime;
use gandr_workflow_grammatical_framework::sexp::Sexp;
use gandr_workflow_grammatical_framework::sexp::unquote;

use crate::bibliography;
use crate::error::GfDocsError;

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
    /// The term records (constant → key/display text), for the
    /// application-grammar lane's domain-lexicon generation.
    #[inline]
    #[must_use]
    pub fn term_records(&self) -> &BTreeMap<String, (String, String)>
    {
        &self.terms
    }
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
            | Some(&(ref existing_key, ref existing_text)) if existing_key == key => {
                if existing_text != text {
                    return Err(GfDocsError::Translation(format!(
                        "term '{key}' declares conflicting display texts: '{existing_text}' vs '{text}'"
                    )));
                }
            },
            | Some(&(ref existing_key, _)) => {
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
            | Some(&(ref existing_id, _)) if existing_id == id => {
                return Err(GfDocsError::Translation(format!(
                    "anchor id '{id}' is declared twice in the corpus"
                )));
            },
            | Some(&(ref existing_id, _)) => {
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
    #[inline]
    #[must_use]
    pub fn render_abstract(&self) -> String
    {
        let mut out = String::from("abstract GandrDocsLex = GandrDocs ** {\n  fun\n");
        out.push_str("    -- terms (generated from the corpus term registry)\n");
        for constant in self.terms.keys() {
            let _res = writeln!(out, "    {constant} : Term ;");
        }
        out.push_str("    -- cite keys (generated from refs.yml)\n");
        for constant in self.cites.keys() {
            let _res = writeln!(out, "    {constant} : CiteKey ;");
        }
        out.push_str("    -- anchors (generated from the corpus id namespace)\n");
        for constant in self.anchors.keys() {
            let _res = writeln!(out, "    {constant} : Anchor ;");
        }
        out.push_str("}\n");
        out
    }

    /// Render the concrete lexicon module (`GandrDocsLexHtml.gf`).
    #[inline]
    #[must_use]
    pub fn render_concrete(&self) -> String
    {
        let mut out =
            String::from("concrete GandrDocsLexHtml of GandrDocsLex = GandrDocsHtml ** {\n  lin\n");
        for (constant, entry) in &self.terms {
            let (ref key, ref text) = *entry;
            let _res = writeln!(
                out,
                "    {constant} = {{ key = {} ; text = {} }} ;",
                gf_str(key),
                gf_str(text)
            );
        }
        for (constant, key) in &self.cites {
            let _res = writeln!(out, "    {constant} = {{ key = {} }} ;", gf_str(key));
        }
        for (constant, entry) in &self.anchors {
            let (ref id, ref title) = *entry;
            let _res = writeln!(
                out,
                "    {constant} = {{ id = {} ; title = {} }} ;",
                gf_str(id),
                gf_str(title)
            );
        }
        out.push_str("}\n");
        out
    }
}

/// Generate the corpus-wide lexicon from the `.gfd` corpus (the production
/// path) and `refs.yml`.
///
/// The `.gfd` trees are read by the `GF` runtime (the bindings-first doctrine:
/// no house reader shadows `readExpr`); they carry every text-destined string
/// already `HTML`-escaped (the translation boundary), so values insert as-is.
///
/// # Errors
/// [`GfDocsError::Parse`] when a `.gfd` file fails the runtime's reader or
/// departs from the expected constructor shapes; [`GfDocsError::Model`] when
/// the bibliography fails to load; [`GfDocsError::Translation`] on any id/term
/// collision.
#[inline]
pub fn generate<R>(
    runtime: &R,
    corpus_dir: &Path,
    refs_path: &Path,
) -> Result<Lexicon, GfDocsError>
where
    R: GfRuntime + ?Sized,
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
        let tree = runtime.read_tree(&ExprText::new(text))?;
        collect_gfd(&tree, &mut lexicon)?;
    }
    let bibliography =
        bibliography::load(refs_path).map_err(|e| GfDocsError::Model(e.to_string()))?;
    for key in bibliography.key_set() {
        lexicon.insert_cite(&key)?;
    }
    Ok(lexicon)
}

// ── the `.gfd` collector (the production path) ──────────────────────────────

/// Collect every lexicon record one `.gfd` component tree declares.
fn collect_gfd(
    tree: &Sexp,
    lexicon: &mut Lexicon,
) -> Result<(), GfDocsError>
{
    let args = expect_app(tree, "MkComponent")?;
    let [
        ref anchor,
        ref title,
        ref _status,
        ref _grounds,
        ref _derives,
        ref sections,
        ref _refs,
    ] = *args
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
    let [ref anchor, ref title, ref _status, ref blocks] = *args
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
    let Sexp::App { ref head, ref args } = *block
    else {
        return Ok(());
    };
    match head.as_str() {
        | "NestedSection" => {
            let [ref section] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("NestedSection arity is not one".into()));
            };
            collect_section(section, lexicon)
        },
        | "DefinitionBlock" => {
            let [ref anchor, ref term, ref body] = *args.as_slice()
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
            let [ref anchor, ref name, ref _premises, ref _conclusion] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("RuleBlock arity is not four".into()));
            };
            lexicon.insert_anchor(&anchor_id(anchor)?, &quoted(name)?)
        },
        | "DiagramBlock" => {
            let [ref anchor, ref caption, ref _cite, ref _source] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("DiagramBlock arity is not four".into()));
            };
            lexicon.insert_anchor(&anchor_id(anchor)?, &quoted(caption)?)
        },
        | "ProseBlock" => {
            let [ref inlines] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("ProseBlock arity is not one".into()));
            };
            harvest_inline_terms(inlines, lexicon)
        },
        | "ExampleBlock" => {
            let [ref _title, ref blocks] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("ExampleBlock arity is not two".into()));
            };
            walk_list(blocks, "Block", &mut |inner| collect_block(inner, lexicon))
        },
        | "RegisterBlock" | "PlainRegisterBlock" => {
            let [ref _order, ref items] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("register arity is not two".into()));
            };
            walk_list(items, "Item", &mut |item| collect_item(item, lexicon))
        },
        | "InventoryBlock" | "StagingPlanBlock" | "DecisionTableBlock" => {
            let [ref _caption, ref header, ref rows] = *args.as_slice()
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
    let Sexp::App { ref head, ref args } = *item
    else {
        return Ok(());
    };
    let body = match head.as_str() {
        | "MkItem" => {
            let [_, ref body] = *args.as_slice()
            else {
                return Ok(());
            };
            body
        },
        | "MkPlainItem" => {
            let [ref body] = *args.as_slice()
            else {
                return Ok(());
            };
            body
        },
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
    let Sexp::App { ref head, ref args } = *row
    else {
        return Ok(());
    };
    if !matches!(head.as_str(), "MkHeaderRow" | "MkBodyRow") {
        return Ok(());
    }
    let [ref cells] = *args.as_slice()
    else {
        return Err(GfDocsError::Parse("row arity is not one".into()));
    };
    walk_list(cells, "Cell", &mut |cell| {
        let args = expect_app(cell, "MkCell")?;
        let [ref content] = *args
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
        if let Sexp::App { ref head, ref args } = *inline {
            match head.as_str() {
                | "TermDef" => {
                    let [ref term, ref display] = *args.as_slice()
                    else {
                        return Err(GfDocsError::Parse("TermDef arity is not two".into()));
                    };
                    lexicon.insert_term(&unmangle(atom_of(term)?, "term_")?, &quoted(display)?)?;
                },
                | "Bold" | "Italic" => {
                    let [ref inner] = *args.as_slice()
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
            | &Sexp::Atom(ref atom) if *atom == format!("Base{category}") => return Ok(()),
            | &Sexp::App { ref head, ref args } if *head == format!("Cons{category}") => {
                let [ref element, ref tail] = *args.as_slice()
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
            | &Sexp::Atom(ref atom) if atom.starts_with("Base") => return Ok(()),
            | &Sexp::App { ref head, ref args } if head.starts_with("Cons") => {
                let [ref element, ref tail] = *args.as_slice()
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
        | &Sexp::App {
            head: ref actual,
            ref args,
        } if actual == head => Ok(args),
        | _ => Err(GfDocsError::Parse(format!("expected a {head} application"))),
    }
}

/// The text of an atom expression.
fn atom_of(sexp: &Sexp) -> Result<&str, GfDocsError>
{
    match sexp {
        | &Sexp::Atom(ref text) => Ok(text),
        | _ => Err(GfDocsError::Parse("expected an atom".into())),
    }
}

/// The unquoted text of a string-literal atom expression.
fn quoted(sexp: &Sexp) -> Result<String, GfDocsError>
{
    unquote((atom_of(sexp)?).into())
        .ok_or_else(|| GfDocsError::Parse("expected a string literal".into()))
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

// ── lexicon constant naming and `GF`/text escaping (the shared contract) ────

/// The lexicon constant for a term key.
#[inline]
#[must_use]
pub fn term_const(key: &str) -> String
{
    format!("term_{}", mangle(key))
}

/// The lexicon constant for a cite key.
#[inline]
#[must_use]
pub fn cite_const(key: &str) -> String
{
    format!("cite_{}", mangle(key))
}

/// The lexicon constant for an anchor id.
#[inline]
#[must_use]
pub fn anchor_const(id: &str) -> String
{
    format!("anchor_{}", mangle(id))
}

/// Map a corpus key to a `GF`-safe identifier fragment.
fn mangle(key: &str) -> String
{
    key.replace('-', "_")
}

/// Quote text as a `GF` string literal.
pub(crate) fn gf_str(text: &str) -> String
{
    let mut out = String::with_capacity(text.len().saturating_add(2));
    out.push('"');
    for ch in text.chars() {
        match ch {
            | '"' => out.push_str("\\\""),
            | '\\' => out.push_str("\\\\"),
            | '\n' => out.push_str("\\n"),
            | '\t' => out.push_str("\\t"),
            | '\r' => out.push_str("\\r"),
            | _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// `HTML`-escape text-destined content (`& < >`).
///
/// Text-destined strings (`Txt` prose, titles, captions, display texts) are
/// emitted raw by the linearization; escaping is the content boundary's job,
/// applied exactly once.
pub(crate) fn escape_text(text: &str) -> String
{
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            | '&' => out.push_str("&amp;"),
            | '<' => out.push_str("&lt;"),
            | '>' => out.push_str("&gt;"),
            | other => out.push(other),
        }
    }
    out
}
