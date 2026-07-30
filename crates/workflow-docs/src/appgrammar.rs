//! The domain application grammar (gandr-739): a small generated `GF`
//! lexicon grammar composed with the `RGL` Lang subset — the clause-level
//! metrics lane's grammar.
//!
//! The vocabulary is curated to the corpus: the docs lexicon's terms
//! (checked display texts), a corpus-seeded general supplement (prose words
//! the `RGL` Lang lexicon does not know, classified by a documented
//! suffix/`POS` heuristic with an override map), and proper names. Curated
//! vocabulary is the point: the parse search space stays small (the libpgf
//! C-stack crash is the 65k-lemma configuration's, per
//! `docs/gandr/spec/internalizing-gf.md`'s observation log), domain terms
//! are first-class lexemes, and the same modules are the vocabulary
//! substrate the compiler-feedback grammar (gandr-2a5) reuses.
//!
//! This module holds the pure generation and rendering logic; the
//! `app-grammar` lane in `main.rs` orchestrates the `IO` (corpus reading,
//! runtime calls, the `gf` driver).

use alloc::collections::BTreeMap;

use crate::lexicon::Lexicon;

/// The category a domain entry declares.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DomainCat
{
    /// Common noun (the default for terms and general words).
    CN,
    /// Proper name.
    ON,
    /// Noun lexeme (general supplement).
    N,
    /// Adjective lexeme.
    A,
    /// Verb lexeme.
    V,
    /// Adverb lexeme.
    Adv,
}

impl AsRef<str> for DomainCat
{
    #[inline]
    fn as_ref(&self) -> &'static str
    {
        match *self {
            | Self::CN => "CN",
            | Self::ON => "ON",
            | Self::N => "N",
            | Self::A => "A",
            | Self::V => "V",
            | Self::Adv => "Adv",
        }
    }
}

/// One domain-lexicon entry: a `GF` fun and its English linearization.
#[derive(Clone, Debug)]
pub struct DomainEntry
{
    /// The fun name (namespaced: `term_…`, `gen_…`, `on_…`).
    pub fun: String,
    /// The declared category.
    pub cat: DomainCat,
    /// The English display form.
    pub text: String,
}

impl DomainEntry
{
    /// Render this entry's concrete `GF` linearization.
    #[must_use]
    fn gf_lin(&self) -> String
    {
        let quoted = gf_quote(GfStringText::from(self.text.as_str()));
        match self.cat {
            | DomainCat::CN => format!("mkCN (ParadigmsEng.mkN {quoted})"),
            | DomainCat::ON => format!("ParadigmsEng.mkON {quoted}"),
            | DomainCat::N => format!("ParadigmsEng.mkN {quoted}"),
            | DomainCat::A => format!("ParadigmsEng.mkA {quoted}"),
            | DomainCat::V => format!("ParadigmsEng.mkV {quoted}"),
            | DomainCat::Adv => format!("ParadigmsEng.mkAdv {quoted}"),
        }
    }
}

/// Maximum number of general-supplement entries to retain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SeedLimit(usize);

impl From<usize> for SeedLimit
{
    #[inline]
    fn from(limit: usize) -> Self
    {
        Self(limit)
    }
}

impl From<SeedLimit> for usize
{
    #[inline]
    fn from(limit: SeedLimit) -> Self
    {
        limit.0
    }
}

/// One lowercased word considered by the suffix classifier.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct SeedWord<'text>(&'text str);

impl<'text> From<&'text str> for SeedWord<'text>
{
    #[inline]
    fn from(word: &'text str) -> Self
    {
        Self(word)
    }
}

/// Text quoted into one generated `GF` string literal.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct GfStringText<'text>(&'text str);

impl<'text> From<&'text str> for GfStringText<'text>
{
    #[inline]
    fn from(text: &'text str) -> Self
    {
        Self(text)
    }
}

/// Classify the docs lexicon's term records into domain entries.
///
/// Single-token capitalized display texts (Idris, Agda, Clojure, …) become
/// proper names; everything else becomes a common noun. Multi-word `CN`
/// morphology caveat: `mkN "frozen core"` inflects only the string's tail
/// (`frozen cores`), which is right for head-final compounds and wrong for
/// head-first phrases — documented, accepted for v1, and exactly the class
/// the parse lane's inflection checks will later grade.
#[inline]
#[must_use]
pub fn term_entries(lexicon: &Lexicon) -> Vec<DomainEntry>
{
    lexicon
        .term_records()
        .iter()
        .map(|(constant, key_text)| {
            let text = &key_text.1;
            let single_token = !text.contains(char::is_whitespace);
            let capitalized = text.chars().next().is_some_and(char::is_uppercase);
            let cat = if single_token && capitalized {
                DomainCat::ON
            }
            else {
                DomainCat::CN
            };
            DomainEntry {
                fun: constant.clone(),
                cat,
                text: text.clone(),
            }
        })
        .collect()
}

/// `POS` overrides for the general supplement: words the suffix heuristic
/// classifies wrongly (verbs carry no reliable suffix signal).
const POS_OVERRIDES: &[(&str, DomainCat)] = &[
    ("unify", DomainCat::V),
    ("parse", DomainCat::V),
    ("prove", DomainCat::V),
    ("check", DomainCat::V),
    ("elaborate", DomainCat::V),
    ("promote", DomainCat::V),
    ("weaken", DomainCat::V),
    ("substitute", DomainCat::V),
    ("lower", DomainCat::V),
    ("recurse", DomainCat::V),
    ("match", DomainCat::V),
    ("render", DomainCat::V),
    ("compile", DomainCat::V),
    ("adjoin", DomainCat::V),
    ("discharge", DomainCat::V),
    ("instantiate", DomainCat::V),
    ("split", DomainCat::V),
    ("quote", DomainCat::V),
    ("elide", DomainCat::V),
    ("linearize", DomainCat::V),
    ("validate", DomainCat::V),
    ("record", DomainCat::V),
    ("embed", DomainCat::V),
    ("project", DomainCat::V),
    ("compare", DomainCat::V),
    ("hold", DomainCat::V),
    ("mean", DomainCat::V),
];

/// Proper-name phrases the seeder cannot recover from single tokens.
const PROPER_PHRASES: &[&str] = &["Liquid Haskell", "Simply Typed"];

/// Classify one lowercased word by suffix (the documented heuristic:
/// adverbs `-ly`; adjectives `-ive -ous -al -ent -ant -able -ible -full
/// -less -ic`; verbs `-ize -ise -ate -ify`; otherwise noun).
fn classify_suffix(word: SeedWord<'_>) -> DomainCat
{
    const ADJ: &[&str] = &[
        "ive", "ous", "al", "ent", "ant", "able", "ible", "full", "less", "ic",
    ];
    const VERB: &[&str] = &["ize", "ise", "ate", "ify"];
    if word.0.ends_with("ly") {
        return DomainCat::Adv;
    }
    if ADJ.iter().any(|suffix| word.0.ends_with(suffix)) {
        return DomainCat::A;
    }
    if VERB.iter().any(|suffix| word.0.ends_with(suffix)) {
        return DomainCat::V;
    }
    DomainCat::N
}

/// The per-word tally the seeder accumulates: total count and form counts
/// (to recover the dominant capitalization).
#[derive(Default)]
struct Tally
{
    /// Total occurrences of the lowercase form.
    total: u32,
    /// Occurrences per surface form.
    forms: BTreeMap<String, u32>,
}

/// Seed the general supplement from prose paragraph texts: lowercase
/// alphabetic words (length ≥ 3) the `is_known` predicate rejects, counted
/// and classified, capped at `limit` entries by frequency.
///
/// `is_known` is the caller's `lookupMorpho` against the `RGL` Lang lexicon
/// (the subtraction that keeps the supplement to words Lang lacks).
/// Classification: dominant capitalized form → `ON`; override map → its
/// `POS`; suffix heuristic otherwise. Words containing no lowercase form at
/// all (acronyms, `ALLCAPS` code) are skipped.
#[inline]
#[must_use]
pub fn seed_general<K>(
    texts: &[String],
    mut is_known: K,
    limit: SeedLimit,
) -> Vec<DomainEntry>
where
    K: FnMut(&str) -> bool,
{
    let limit = usize::from(limit);
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for text in texts {
        for token in text.split_whitespace() {
            let word: String = token
                .chars()
                .filter(|ch| ch.is_alphabetic() || *ch == '-' || *ch == '\'')
                .collect();
            // Strip edge hyphens (prose artifacts like "(η-categories)");
            // `GF` identifiers are `ASCII`, and non-`ASCII` tokens (Greek
            // letters, math symbols) are the math register's, not the
            // lexicon's — skipped here, reported as unknown by the parse
            // lane's coverage accounting (documented v1 posture).
            let word = word.trim_matches('-');
            if word.chars().count() < 3 || !word.chars().any(char::is_lowercase) || !word.is_ascii()
            {
                continue;
            }
            let word = word.to_owned();
            let lower = word.to_lowercase();
            if is_known(&lower) {
                continue;
            }
            let tally = tallies.entry(lower).or_default();
            tally.total = tally.total.saturating_add(1);
            let count = tally.forms.entry(word).or_insert(0);
            *count = (*count).saturating_add(1);
        }
    }
    let mut ranked: Vec<(String, Tally)> = tallies.into_iter().collect();
    ranked.sort_by(|a, b| b.1.total.cmp(&a.1.total).then_with(|| a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .take(limit)
        .filter_map(|(lower, tally)| {
            let (form, _count) = tally
                .forms
                .iter()
                .max_by(|&(fa, ca), &(fb, cb)| ca.cmp(cb).then_with(|| fb.cmp(fa)))?;
            let capitalized = form.chars().next().is_some_and(char::is_uppercase);
            let cat = if capitalized {
                DomainCat::ON
            }
            else {
                POS_OVERRIDES
                    .iter()
                    .find(|&&(word, _)| word == lower)
                    .map_or_else(
                        || classify_suffix(SeedWord::from(lower.as_str())),
                        |&(_, cat)| cat,
                    )
            };
            let prefix = if cat == DomainCat::ON { "on" } else { "gen" };
            Some(DomainEntry {
                fun: format!("{prefix}_{}", lower.replace(['-', '\''], "_")),
                cat,
                text: form.clone(),
            })
        })
        .collect()
}

/// The proper-name phrase entries (constant list).
#[inline]
#[must_use]
pub fn proper_phrase_entries() -> Vec<DomainEntry>
{
    PROPER_PHRASES
        .iter()
        .map(|text| DomainEntry {
            fun: format!("on_{}", text.to_lowercase().replace(' ', "_")),
            cat: DomainCat::ON,
            text: (*text).to_owned(),
        })
        .collect()
}

/// The `--# -path=` line for the generated modules: the `RGL` sources the
/// composition extends (absolute, since the modules are generated outside
/// the `RGL` tree).
#[inline]
#[must_use]
pub fn path_line(rgl_src: &std::path::Path) -> String
{
    let join = |dir: &str| rgl_src.join(dir).display().to_string();
    format!(
        "--# -path=.:{}:{}:{}:{}:{}\n",
        join("abstract"),
        join("common"),
        join("english"),
        join("prelude"),
        join("api")
    )
}

/// Render the abstract lexicon module (`GandrTermsAbs.gf`).
#[inline]
#[must_use]
pub fn render_abstract(entries: &[DomainEntry]) -> String
{
    use core::fmt::Write as _;
    let mut by_cat: BTreeMap<DomainCat, Vec<&str>> = BTreeMap::new();
    for entry in entries {
        by_cat.entry(entry.cat).or_default().push(&entry.fun);
    }
    let mut out = String::from("abstract GandrTermsAbs = Cat ** {\n\x20 fun\n");
    for (cat, funs) in by_cat {
        let _res = writeln!(out, "    {} : {} ;", funs.join(" , "), cat.as_ref());
    }
    out.push_str("}\n");
    out
}

/// Render the concrete lexicon module (`GandrTermsEng.gf`).
#[inline]
#[must_use]
pub fn render_concrete(entries: &[DomainEntry]) -> String
{
    use core::fmt::Write as _;
    let mut out = String::from(
        "concrete GandrTermsEng of GandrTermsAbs = CatEng ** open ParadigmsEng, SyntaxEng in {\n\x20 flags coding = utf8 ;\n\x20 lin\n",
    );
    for entry in entries {
        let _res = writeln!(out, "    {} = {} ;", entry.fun, entry.gf_lin());
    }
    out.push_str("}\n");
    out
}

/// Render the composition abstract module (`GandrAppLex.gf`).
#[inline]
#[must_use]
pub fn render_composition_abstract() -> String
{
    "abstract GandrAppLex = Lang, GandrTermsAbs ;\n".to_owned()
}

/// Render the composition concrete module (`GandrAppLexEng.gf`).
#[inline]
#[must_use]
pub fn render_composition_concrete() -> String
{
    "concrete GandrAppLexEng of GandrAppLex = LangEng, GandrTermsEng ;\n".to_owned()
}

/// Quote text as a `GF` string literal.
fn gf_quote(text: GfStringText<'_>) -> String
{
    let mut out = String::with_capacity(text.0.len().saturating_add(2));
    out.push('"');
    for ch in text.0.chars() {
        match ch {
            | '"' => out.push_str("\\\""),
            | '\\' => out.push_str("\\\\"),
            | other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn suffix_heuristic_classifies()
    {
        assert_eq!(classify_suffix("convergent".into()), DomainCat::A);
        assert_eq!(classify_suffix("recursive".into()), DomainCat::A);
        assert_eq!(classify_suffix("normalize".into()), DomainCat::V);
        assert_eq!(classify_suffix("locally".into()), DomainCat::Adv);
        assert_eq!(classify_suffix("corecursion".into()), DomainCat::N);
        assert_eq!(classify_suffix("eliminator".into()), DomainCat::N);
    }

    #[test]
    fn seeder_classifies_dominant_forms()
    {
        let texts = vec![
            "Idris handles elaboration; idris-like systems recur".to_owned(),
            "corecursion corecursion corecursion".to_owned(),
        ];
        let entries = seed_general(&texts, |_| false, 10_usize.into());
        let idris = entries.iter().find(|entry| entry.fun == "on_idris");
        assert!(idris.is_some());
        assert_eq!(idris.map(|entry| entry.cat), Some(DomainCat::ON));
        let corecursion = entries.iter().find(|entry| entry.fun == "gen_corecursion");
        assert_eq!(corecursion.map(|entry| entry.cat), Some(DomainCat::N));
    }

    #[test]
    fn seeder_subtracts_known_words()
    {
        let texts = vec!["the cat corecursion".to_owned()];
        let entries = seed_general(&texts, |word| word == "cat", 10_usize.into());
        assert!(entries.iter().all(|entry| entry.fun != "gen_cat"));
        assert!(entries.iter().any(|entry| entry.fun == "gen_corecursion"));
    }

    #[test]
    fn render_shapes_are_gf_modules()
    {
        let entries = vec![DomainEntry {
            fun: "term_frozen_core".to_owned(),
            cat: DomainCat::CN,
            text: "frozen core".to_owned(),
        }];
        let abstract_module = render_abstract(&entries);
        assert!(abstract_module.contains("abstract GandrTermsAbs = Cat ** {"));
        assert!(abstract_module.contains("term_frozen_core : CN ;"));
        let concrete = render_concrete(&entries);
        assert!(concrete.contains("term_frozen_core = mkCN (ParadigmsEng.mkN \"frozen core\") ;"));
    }

    #[test]
    fn gf_quote_escapes()
    {
        assert_eq!(
            gf_quote("say \"hi\" \\ done".into()),
            "\"say \\\"hi\\\" \\\\ done\""
        );
    }
}
