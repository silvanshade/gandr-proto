//! Prose-pacing metrics over the document trees (proposal
//! `docs/gandr/spec/proposal-docs-gf-pipeline.md` §3.5, the gandr-aaq arc).
//!
//! Two lanes, both exact about what they measure:
//!
//! * **Lane A — tree walks (exact by construction).** Section shape, block mix,
//!   weave compliance (every payload block gets an introducing prose
//!   paragraph), emphasis density and placement (bold/italic mark the
//!   load-bearing ideas — the doctrine targets roughly one per paragraph), and
//!   term/cross-reference chains between adjacent paragraphs (the entity-grid
//!   coherence signal, exact from the tree — no embeddings).
//! * **Lane B — rhythm on linearized prose.** Sentence-length and
//!   paragraph-length distributions from the prose as it will be read: `Txt`
//!   text with entities resolved, `TermRef`/`XRef` display text from the
//!   runtime's lexicon views, `CiteRef` and inline math counted as reference
//!   markers rather than prose words. Sentence boundaries are `UAX` #29
//!   (`unicode-segmentation`); words are whitespace tokens containing at least
//!   one alphanumeric character, so code tokens like `f[<]` count as one word.
//!
//! Deliberately absent: syllable- and complex-word-based readability scores
//! (Flesch/Fog and friends). This is a mathematics corpus — complex words
//! are not the enemy; density, pacing, concept placement at paragraph
//! boundaries, and flow are (owner direction, 2026-07-23). The proposal's
//! clause-level lane (RGL-parsed structure) lands separately, gated on
//! coverage data.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;

use gandr_workflow_grammatical_framework::rt::CategoryName;
use gandr_workflow_grammatical_framework::rt::ExprText;
use gandr_workflow_grammatical_framework::rt::GfRuntime;
use gandr_workflow_grammatical_framework::sexp::Sexp;
use gandr_workflow_grammatical_framework::sexp::unquote;
use unicode_segmentation::UnicodeSegmentation;

use crate::error::GfDocsError;

/// Borrowed sample set for one word-count distribution.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct WordCountSamples<'samples>(&'samples [u32]);

impl<'samples> From<&'samples [u32]> for WordCountSamples<'samples>
{
    #[inline]
    fn from(samples: &'samples [u32]) -> Self
    {
        Self(samples)
    }
}

/// Word count for one measured prose fragment.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct WordCount(u32);

impl From<WordCount> for u32
{
    #[inline]
    fn from(count: WordCount) -> Self
    {
        count.0
    }
}

/// Median value derived from a word-count sample set.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct Median(f64);

impl From<Median> for f64
{
    #[inline]
    fn from(median: Median) -> Self
    {
        median.0
    }
}

/// Nesting depth of one measured section.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SectionDepth(u32);

impl SectionDepth
{
    /// Return the depth of a nested section.
    #[must_use]
    fn child(self) -> Self
    {
        Self(self.0.saturating_add(1))
    }
}

/// Whether the preceding block introduced the next payload with prose.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PreviousBlockWasProse(bool);

/// Whether the leading block of a nested container is prose.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LeadingBlockIsProse(bool);

impl From<LeadingBlockIsProse> for bool
{
    #[inline]
    fn from(value: LeadingBlockIsProse) -> Self
    {
        value.0
    }
}

/// Whether an inline appears within emphasis.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct EmphasisState(bool);

/// Borrowed prose text participating in word measurement.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ParagraphText<'text>(&'text str);

impl<'text> From<&'text str> for ParagraphText<'text>
{
    #[inline]
    fn from(text: &'text str) -> Self
    {
        Self(text)
    }
}

/// Borrowed prose text carrying encoded `HTML` entities.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct EscapedProse<'text>(&'text str);

impl<'text> From<&'text str> for EscapedProse<'text>
{
    #[inline]
    fn from(text: &'text str) -> Self
    {
        Self(text)
    }
}

/// Expected S-expression constructor.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ExpectedConstructor<'text>(&'text str);

impl<'text> From<&'text str> for ExpectedConstructor<'text>
{
    #[inline]
    fn from(expected: &'text str) -> Self
    {
        Self(expected)
    }
}

/// One bare S-expression atom.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct SexpAtom<'text>(&'text str);

impl AsRef<str> for SexpAtom<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

/// Constructor tag of one encoded `GF` list.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ListTag<'text>(&'text str);

impl<'text> From<&'text str> for ListTag<'text>
{
    #[inline]
    fn from(tag: &'text str) -> Self
    {
        Self(tag)
    }
}

/// The display texts the linearized-prose lane resolves references through.
///
/// Term constants map to their rendered text, anchor constants to their
/// target titles — both extracted from the compiled grammar by the runtime
/// (never parsed from the generated modules; the bindings-first doctrine).
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct LexiconViews
{
    /// Term constant name to rendered text (`{ text = … }`).
    terms: BTreeMap<String, String>,
    /// Anchor constant name to target title (`{ title = … }`).
    anchors: BTreeMap<String, String>,
    /// Anchor constant name to its `HTML` id (`{ id = … }`).
    anchor_ids: BTreeMap<String, String>,
}

impl LexiconViews
{
    /// Build the views from the runtime: enumerate the `Term` and `Anchor`
    /// functions and tabular-linearize each bare constant.
    ///
    /// # Errors
    /// [`GfDocsError::Parse`] when a constant's linearization lacks the
    /// expected record field; [`GfDocsError::Python`] on interop failure.
    #[inline]
    pub fn load<R>(runtime: &R) -> Result<Self, GfDocsError>
    where
        R: GfRuntime + ?Sized,
    {
        let mut terms = BTreeMap::new();
        for name in runtime.functions_by_cat(&CategoryName::new("Term"))? {
            let fields = runtime.tabular_linearize(&ExprText::new(name.to_string()))?;
            let text = fields
                .get("text")
                .ok_or_else(|| GfDocsError::Parse(format!("{name}: no text field")))?
                .clone();
            terms.insert(name.to_string(), text);
        }
        let mut anchors = BTreeMap::new();
        let mut anchor_ids = BTreeMap::new();
        for name in runtime.functions_by_cat(&CategoryName::new("Anchor"))? {
            let fields = runtime.tabular_linearize(&ExprText::new(name.to_string()))?;
            let title = fields
                .get("title")
                .ok_or_else(|| GfDocsError::Parse(format!("{name}: no title field")))?
                .clone();
            let id = fields
                .get("id")
                .ok_or_else(|| GfDocsError::Parse(format!("{name}: no id field")))?
                .clone();
            anchors.insert(name.to_string(), title);
            anchor_ids.insert(name.to_string(), id);
        }
        Ok(Self {
            terms,
            anchors,
            anchor_ids,
        })
    }

    /// The anchor id map (constant → `HTML` id), for the table-of-contents
    /// lane.
    #[inline]
    #[must_use]
    pub fn anchor_ids(&self) -> &BTreeMap<String, String>
    {
        &self.anchor_ids
    }
}

/// A distribution of word counts (sentence or paragraph lengths): the
/// rhythm signal of Lane B.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Dist
{
    /// The sample count.
    pub count: u32,
    /// The smallest sample.
    pub min: u32,
    /// The largest sample.
    pub max: u32,
    /// The arithmetic mean.
    pub mean: f64,
    /// The median (mean of the two middle samples at even counts).
    pub median: f64,
    /// The 90th percentile (nearest-rank).
    pub p90: f64,
    /// The population standard deviation.
    pub stdev: f64,
}

impl Dist
{
    /// Compute the distribution of one sample set (`None` when empty).
    #[inline]
    #[must_use]
    pub fn from_samples(samples: WordCountSamples<'_>) -> Option<Self>
    {
        let (first, rest) = samples.0.split_first()?;
        let mut sorted = samples.0.to_vec();
        sorted.sort_unstable();
        let count = u32::try_from(sorted.len()).unwrap_or(u32::MAX);
        let sum = samples
            .0
            .iter()
            .fold(0_u32, |acc, sample| acc.saturating_add(*sample));
        let mean = f64::from(sum) / f64::from(count);
        let variance = sorted
            .iter()
            .map(|sample| {
                let delta = f64::from(*sample) - mean;
                delta * delta
            })
            .sum::<f64>()
            / f64::from(count);
        let median = f64::from(median_of(sorted.as_slice().into()));
        let p90_rank = count
            .saturating_mul(9)
            .saturating_add(9)
            .checked_div(10)
            .unwrap_or(0);
        let p90_index = usize::try_from(p90_rank.saturating_sub(1)).unwrap_or(0);
        let p90 = sorted
            .get(p90_index.min(sorted.len().saturating_sub(1)))
            .copied()
            .unwrap_or(*first);
        let min = *rest.iter().fold(first, |acc, sample| acc.min(sample));
        let max = *rest.iter().fold(first, |acc, sample| acc.max(sample));
        Some(Self {
            count,
            min,
            max,
            mean,
            median,
            p90: f64::from(p90),
            stdev: variance.sqrt(),
        })
    }
}

/// The median of a sorted, nonempty sample slice.
fn median_of(sorted: WordCountSamples<'_>) -> Median
{
    let middle = sorted.0.len().checked_div(2).unwrap_or(0);
    if sorted.0.len() % 2 == 1 {
        return Median(sorted.0.get(middle).copied().map_or(0.0, f64::from));
    }
    let lower = sorted.0.get(middle.saturating_sub(1)).copied().unwrap_or(0);
    let upper = sorted.0.get(middle).copied().unwrap_or(0);
    Median(f64::midpoint(f64::from(lower), f64::from(upper)))
}

/// One section's shape (Lane A): counts only, in reading order.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SectionReport
{
    /// The section title.
    pub title: String,
    /// The nesting depth (1 for top-level sections).
    pub depth: u32,
    /// Prose paragraphs (prose blocks plus definition blocks).
    pub paragraphs: u32,
    /// Sentences across those paragraphs.
    pub sentences: u32,
    /// Prose words across those paragraphs.
    pub prose_words: u32,
    /// Payload blocks (everything but prose/definition/nested-section).
    pub payload_blocks: u32,
    /// Emphasis spans (bold plus italic) inside the prose.
    pub emphasis_spans: u32,
    /// Paragraphs carrying at least one emphasis span.
    pub paragraphs_with_emphasis: u32,
}

/// The document-level pacing report (Lanes A and B).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Report
{
    /// Every section in reading order (nested sections flattened, depth
    /// recorded).
    pub sections: Vec<SectionReport>,
    /// Total prose words.
    pub prose_words: u32,
    /// Total prose paragraphs.
    pub paragraphs: u32,
    /// Sentence-length distribution (words per sentence).
    pub sentence_words: Option<Dist>,
    /// Paragraph-length distribution (words per paragraph).
    pub paragraph_words: Option<Dist>,
    /// Mean sentences per paragraph.
    pub sentences_per_paragraph_mean: f64,
    /// Emphasis spans per 100 prose words.
    pub emphasis_per_100_words: f64,
    /// Share of paragraphs carrying at least one emphasis span (0–1).
    pub paragraphs_with_emphasis: f64,
    /// Total payload blocks.
    pub payload_blocks: u32,
    /// Payload blocks lacking an introducing prose paragraph.
    pub weave_violations: u32,
    /// Term/cross references (unresolved constants included) across prose.
    pub term_refs: u32,
    /// Cross references to anchors.
    pub xrefs: u32,
    /// Citation references.
    pub cites: u32,
    /// Mean shared term/cross references between adjacent prose paragraphs
    /// (the entity-grid-style coherence signal).
    pub adjacent_shared_refs_mean: f64,
    /// Human-readable findings (weave violations and friends).
    pub findings: Vec<String>,
}

/// What one prose paragraph's inline walk accumulates.
#[derive(Default)]
struct ParagraphMeasure
{
    /// The linearized paragraph text (references resolved).
    text: String,
    /// Emphasis (bold/italic) spans.
    emphasis_spans: u32,
    /// Words inside emphasis spans.
    emphasis_words: u32,
    /// `TermRef` occurrences.
    term_refs: u32,
    /// `XRef` occurrences.
    xrefs: u32,
    /// `CiteRef` occurrences.
    cites: u32,
    /// The constants this paragraph references (for the chain signal).
    ref_constants: BTreeSet<String>,
}

/// Per-document accumulation across the block walk.
#[derive(Default)]
struct DocumentAccum
{
    /// Section reports in reading order.
    sections: Vec<SectionReport>,
    /// Sentence word counts (every prose paragraph, in order).
    sentence_words: Vec<u32>,
    /// Paragraph word counts (in order).
    paragraph_words: Vec<u32>,
    /// Sentences per paragraph (in order).
    sentences_per_paragraph: Vec<u32>,
    /// Emphasis spans and emphasized words.
    emphasis_spans: u32,
    /// Words inside emphasis spans.
    emphasis_words: u32,
    /// Paragraphs carrying at least one emphasis span.
    paragraphs_with_emphasis: u32,
    /// Reference totals.
    term_refs: u32,
    /// Cross-reference totals.
    xrefs: u32,
    /// Citation totals.
    cites: u32,
    /// Per-paragraph reference constant sets (in order).
    paragraph_refs: Vec<BTreeSet<String>>,
    /// Payload block totals.
    payload_blocks: u32,
    /// Weave violations.
    weave_violations: u32,
    /// Findings lines.
    findings: Vec<String>,
}

/// Analyze one component tree (read by the runtime) into its pacing report.
///
/// # Errors
/// [`GfDocsError::Parse`] when the tree departs from the expected
/// constructor shapes (root not `MkComponent`, malformed lists).
#[inline]
pub fn analyze(
    tree: &Sexp,
    views: &LexiconViews,
) -> Result<Report, GfDocsError>
{
    let args = expect_app(tree, "MkComponent".into())?;
    let [
        ref _anchor,
        ref _title,
        ref _status,
        ref _grounds,
        ref _derives,
        ref sections,
        ref _refs,
    ] = *args
    else {
        return Err(GfDocsError::Parse("MkComponent arity is not seven".into()));
    };
    let mut accum = DocumentAccum::default();
    walk_list(sections, "Section".into(), &mut |section| {
        measure_section(section, SectionDepth(1), views, &mut accum)
    })?;
    Ok(finish(accum))
}

/// The linearized prose paragraphs of one component tree (references
/// resolved through the views, entities unescaped): the parse lane's input.
///
/// # Errors
/// [`GfDocsError::Parse`] when the tree departs from the expected
/// constructor shapes.
#[inline]
pub fn paragraph_texts(
    tree: &Sexp,
    views: &LexiconViews,
) -> Result<Vec<String>, GfDocsError>
{
    let args = expect_app(tree, "MkComponent".into())?;
    let [
        ref _anchor,
        ref _title,
        ref _status,
        ref _grounds,
        ref _derives,
        ref sections,
        ref _refs,
    ] = *args
    else {
        return Err(GfDocsError::Parse("MkComponent arity is not seven".into()));
    };
    let mut paragraphs = Vec::new();
    walk_list(sections, "Section".into(), &mut |section| {
        harvest_section_paragraphs(section, views, &mut paragraphs)
    })?;
    Ok(paragraphs)
}

/// Harvest one section's prose paragraphs (recursing into nested sections
/// and examples).
///
/// # Termination
/// - reason: structural recursion mirrors the section tree's own shape.
/// - measure: the section nesting depth strictly decreases per level.
/// - boundedness: a finite document has finite nesting.
/// - input recursion: structural descent over the input section tree.
fn harvest_section_paragraphs(
    section: &Sexp,
    views: &LexiconViews,
    paragraphs: &mut Vec<String>,
) -> Result<(), GfDocsError>
{
    let args = expect_app(section, "MkSection".into())?;
    let [ref _anchor, ref _title, ref _status, ref blocks] = *args
    else {
        return Err(GfDocsError::Parse("MkSection arity is not four".into()));
    };
    walk_list(blocks, "Block".into(), &mut |block| {
        harvest_block_paragraphs(block, views, paragraphs)
    })
}

/// Harvest one block's prose paragraphs (prose and definition blocks yield;
/// examples recurse; payloads skip).
///
/// # Termination
/// - reason: structural recursion mirrors the block tree's own shape.
/// - measure: the block tree's height strictly decreases per level.
/// - boundedness: a finite document has finite block height.
/// - input recursion: structural descent over the input block tree.
fn harvest_block_paragraphs(
    block: &Sexp,
    views: &LexiconViews,
    paragraphs: &mut Vec<String>,
) -> Result<(), GfDocsError>
{
    let Sexp::App { ref head, ref args } = *block
    else {
        return Err(GfDocsError::Parse("block is not an application".into()));
    };
    match head.as_str() {
        | "ProseBlock" | "DefinitionBlock" => {
            let inlines = args
                .last()
                .ok_or_else(|| GfDocsError::Parse(format!("{head} has no inline argument")))?;
            let measure = measure_inlines(inlines, views)?;
            paragraphs.push(measure.text);
        },
        | "ExampleBlock" => {
            let blocks = args
                .get(1)
                .ok_or_else(|| GfDocsError::Parse("ExampleBlock arity is not two".into()))?;
            walk_list(blocks, "Block".into(), &mut |inner| {
                harvest_block_paragraphs(inner, views, paragraphs)
            })?;
        },
        | "NestedSection" => {
            let nested = args.first().ok_or_else(|| {
                GfDocsError::Parse("NestedSection has no section argument".into())
            })?;
            harvest_section_paragraphs(nested, views, paragraphs)?;
        },
        | _ => {},
    }
    Ok(())
}

/// Render one report as a human-readable text table.
#[inline]
#[must_use]
pub fn render(report: &Report) -> String
{
    use core::fmt::Write as _;
    let mut out = String::new();
    let _res = writeln!(
        out,
        "sections: {}  paragraphs: {}  prose words: {}  payload blocks: {} (weave violations: {})",
        report.sections.len(),
        report.paragraphs,
        report.prose_words,
        report.payload_blocks,
        report.weave_violations
    );
    if let Some(ref dist) = report.sentence_words {
        let _res = writeln!(
            out,
            "sentence words: mean {:.1}  median {:.1}  p90 {:.1}  max {}  stdev {:.1}",
            dist.mean, dist.median, dist.p90, dist.max, dist.stdev
        );
    }
    if let Some(ref dist) = report.paragraph_words {
        let _res = writeln!(
            out,
            "paragraph words: mean {:.1}  median {:.1}  p90 {:.1}  max {}  stdev {:.1}",
            dist.mean, dist.median, dist.p90, dist.max, dist.stdev
        );
    }
    let _res = writeln!(
        out,
        "sentences/paragraph: mean {:.2}  emphasis: {:.1} spans/100 words, {:.0}% paragraphs marked",
        report.sentences_per_paragraph_mean,
        report.emphasis_per_100_words,
        report.paragraphs_with_emphasis * 100.0_f64
    );
    let _res = writeln!(
        out,
        "refs: term {}  xref {}  cite {}  adjacent-paragraph shared refs mean {:.2}",
        report.term_refs, report.xrefs, report.cites, report.adjacent_shared_refs_mean
    );
    for section in &report.sections {
        let indent = "  ".repeat(usize::try_from(section.depth).unwrap_or(1));
        let _res = writeln!(
            out,
            "{indent}{}: {} para, {} sent, {} words, {} payload, {} emph",
            section.title,
            section.paragraphs,
            section.sentences,
            section.prose_words,
            section.payload_blocks,
            section.emphasis_spans
        );
    }
    if !report.findings.is_empty() {
        let _res = writeln!(out, "findings:");
        for finding in &report.findings {
            let _res = writeln!(out, "  {finding}");
        }
    }
    out
}

/// Fold the accumulation into the public report.
fn finish(accum: DocumentAccum) -> Report
{
    let prose_words = accum
        .paragraph_words
        .iter()
        .fold(0_u32, |acc, words| acc.saturating_add(*words));
    let paragraphs = u32::try_from(accum.paragraph_words.len()).unwrap_or(u32::MAX);
    let sentences: u32 = accum
        .sentences_per_paragraph
        .iter()
        .fold(0_u32, |acc, count| acc.saturating_add(*count));
    let sentences_per_paragraph_mean = if paragraphs == 0 {
        0.0_f64
    }
    else {
        f64::from(sentences) / f64::from(paragraphs)
    };
    let emphasis_per_100_words = if prose_words == 0 {
        0.0_f64
    }
    else {
        f64::from(accum.emphasis_spans) * 100.0_f64 / f64::from(prose_words)
    };
    let paragraphs_with_emphasis = if paragraphs == 0 {
        0.0_f64
    }
    else {
        f64::from(accum.paragraphs_with_emphasis) / f64::from(paragraphs)
    };
    let mut shared_total = 0_u32;
    let mut shared_pairs = 0_u32;
    for pair in accum.paragraph_refs.windows(2) {
        let (Some(previous), Some(next)) = (pair.first(), pair.get(1))
        else {
            continue;
        };
        let shared = previous.intersection(next).count();
        shared_total = shared_total.saturating_add(u32::try_from(shared).unwrap_or(u32::MAX));
        shared_pairs = shared_pairs.saturating_add(1);
    }
    let adjacent_shared_refs_mean = if shared_pairs == 0 {
        0.0_f64
    }
    else {
        f64::from(shared_total) / f64::from(shared_pairs)
    };
    Report {
        sections: accum.sections,
        prose_words,
        paragraphs,
        sentence_words: Dist::from_samples(accum.sentence_words.as_slice().into()),
        paragraph_words: Dist::from_samples(accum.paragraph_words.as_slice().into()),
        sentences_per_paragraph_mean,
        emphasis_per_100_words,
        paragraphs_with_emphasis,
        payload_blocks: accum.payload_blocks,
        weave_violations: accum.weave_violations,
        term_refs: accum.term_refs,
        xrefs: accum.xrefs,
        cites: accum.cites,
        adjacent_shared_refs_mean,
        findings: accum.findings,
    }
}

/// Measure one section (its own blocks, then its nested sections) into the
/// accumulation.
///
/// # Termination
/// - reason: structural recursion mirrors the section tree's own shape.
/// - measure: the section nesting depth strictly decreases per level.
/// - boundedness: a finite document has finite nesting.
/// - input recursion: structural descent over the input section tree.
fn measure_section(
    section: &Sexp,
    depth: SectionDepth,
    views: &LexiconViews,
    accum: &mut DocumentAccum,
) -> Result<(), GfDocsError>
{
    let args = expect_app(section, "MkSection".into())?;
    let [ref _anchor, ref title, ref _status, ref blocks] = *args
    else {
        return Err(GfDocsError::Parse("MkSection arity is not four".into()));
    };
    let title = quoted(title)?;
    let mut report = SectionReport {
        title: title.clone(),
        depth: depth.0,
        paragraphs: 0,
        sentences: 0,
        prose_words: 0,
        payload_blocks: 0,
        emphasis_spans: 0,
        paragraphs_with_emphasis: 0,
    };
    let mut prev_was_prose = PreviousBlockWasProse::default();
    walk_list(blocks, "Block".into(), &mut |block| {
        measure_block(
            block,
            &title,
            &mut prev_was_prose,
            views,
            accum,
            &mut report,
        )
    })?;
    accum.sections.push(report);
    Ok(())
}

/// Measure one block: prose carriers feed Lane B, payloads feed block mix
/// and the weave check, containers recurse.
///
/// # Termination
/// - reason: structural recursion mirrors the block tree's own shape (examples
///   and nested sections nest finitely).
/// - measure: the block tree's height strictly decreases per level.
/// - boundedness: a finite document has finite block height.
/// - input recursion: structural descent over the input block tree.
fn measure_block(
    block: &Sexp,
    section_title: &String,
    prev_was_prose: &mut PreviousBlockWasProse,
    views: &LexiconViews,
    accum: &mut DocumentAccum,
    report: &mut SectionReport,
) -> Result<(), GfDocsError>
{
    let Sexp::App { ref head, ref args } = *block
    else {
        return Err(GfDocsError::Parse("block is not an application".into()));
    };
    match head.as_str() {
        | "ProseBlock" | "DefinitionBlock" => {
            let inlines = args
                .last()
                .ok_or_else(|| GfDocsError::Parse(format!("{head} has no inline argument")))?;
            let measure = measure_inlines(inlines, views)?;
            absorb_paragraph(measure, accum, report);
            prev_was_prose.0 = true;
        },
        | "NestedSection" => {
            let nested = args.first().ok_or_else(|| {
                GfDocsError::Parse("NestedSection has no section argument".into())
            })?;
            measure_section(nested, SectionDepth(report.depth).child(), views, accum)?;
        },
        | "ExampleBlock" => {
            let blocks = args
                .get(1)
                .ok_or_else(|| GfDocsError::Parse("ExampleBlock arity is not two".into()))?;
            accum.payload_blocks = accum.payload_blocks.saturating_add(1);
            report.payload_blocks = report.payload_blocks.saturating_add(1);
            if !bool::from(leading_block_is_prose(blocks)) {
                accum.weave_violations = accum.weave_violations.saturating_add(1);
                accum.findings.push(format!(
                    "weave: {section_title}: {head} opens without inner prose"
                ));
            }
            prev_was_prose.0 = true;
            let mut inner_prev = PreviousBlockWasProse::default();
            walk_list(blocks, "Block".into(), &mut |inner| {
                measure_block(inner, section_title, &mut inner_prev, views, accum, report)
            })?;
        },
        | _ => {
            check_weave(head, section_title, *prev_was_prose, accum, report);
            prev_was_prose.0 = false;
        },
    }
    Ok(())
}

/// Record one payload block against the weave rule (every payload block gets
/// an introducing prose paragraph).
fn check_weave(
    head: &String,
    section_title: &String,
    prev_was_prose: PreviousBlockWasProse,
    accum: &mut DocumentAccum,
    report: &mut SectionReport,
)
{
    accum.payload_blocks = accum.payload_blocks.saturating_add(1);
    report.payload_blocks = report.payload_blocks.saturating_add(1);
    if !prev_was_prose.0 {
        accum.weave_violations = accum.weave_violations.saturating_add(1);
        accum.findings.push(format!(
            "weave: {section_title}: {head} lacks a prose introduction"
        ));
    }
}

/// Whether a block list's leading block is prose (the self-introducing
/// example pattern).
fn leading_block_is_prose(blocks: &Sexp) -> LeadingBlockIsProse
{
    let Sexp::App { ref args, .. } = *blocks
    else {
        return LeadingBlockIsProse(false);
    };
    LeadingBlockIsProse(args.first().is_some_and(|block| {
        matches!(*block, Sexp::App { ref head, .. } if head == "ProseBlock" || head == "DefinitionBlock")
    }))
}

/// Fold one prose paragraph's measure into the accumulations.
fn absorb_paragraph(
    measure: ParagraphMeasure,
    accum: &mut DocumentAccum,
    report: &mut SectionReport,
)
{
    let sentences: Vec<&str> =
        UnicodeSegmentation::unicode_sentences(measure.text.as_str()).collect();
    let sentence_count = u32::try_from(sentences.len()).unwrap_or(u32::MAX);
    let mut paragraph_words = 0_u32;
    for sentence in sentences {
        let words = u32::from(count_words(sentence.into()));
        paragraph_words = paragraph_words.saturating_add(words);
        accum.sentence_words.push(words);
    }
    accum.paragraph_words.push(paragraph_words);
    accum.sentences_per_paragraph.push(sentence_count);
    accum.emphasis_spans = accum.emphasis_spans.saturating_add(measure.emphasis_spans);
    accum.emphasis_words = accum.emphasis_words.saturating_add(measure.emphasis_words);
    if measure.emphasis_spans > 0 {
        accum.paragraphs_with_emphasis = accum.paragraphs_with_emphasis.saturating_add(1);
        report.paragraphs_with_emphasis = report.paragraphs_with_emphasis.saturating_add(1);
    }
    accum.term_refs = accum.term_refs.saturating_add(measure.term_refs);
    accum.xrefs = accum.xrefs.saturating_add(measure.xrefs);
    accum.cites = accum.cites.saturating_add(measure.cites);
    accum.paragraph_refs.push(measure.ref_constants);
    report.paragraphs = report.paragraphs.saturating_add(1);
    report.sentences = report.sentences.saturating_add(sentence_count);
    report.prose_words = report.prose_words.saturating_add(paragraph_words);
    report.emphasis_spans = report.emphasis_spans.saturating_add(measure.emphasis_spans);
}

/// Linearize and measure one prose carrier's inline list.
///
/// # Termination
/// - reason: structural recursion mirrors the inline tree's own shape (emphasis
///   constructors nest finitely).
/// - measure: the inline tree's height strictly decreases per level.
/// - boundedness: a finite paragraph has finite inline height.
/// - input recursion: structural descent over the input inline tree.
fn measure_inlines(
    inlines: &Sexp,
    views: &LexiconViews,
) -> Result<ParagraphMeasure, GfDocsError>
{
    let mut measure = ParagraphMeasure::default();
    walk_list(inlines, "Inline".into(), &mut |inline| {
        measure_inline(inline, views, &mut measure, EmphasisState::default())
    })?;
    measure.text = unescape(measure.text.as_str().into());
    Ok(measure)
}

/// Measure one inline, appending its linearized text to the paragraph.
///
/// # Termination
/// - reason: structural recursion mirrors the inline tree's own shape (emphasis
///   constructors nest finitely).
/// - measure: the inline tree's height strictly decreases per level.
/// - boundedness: a finite paragraph has finite inline height.
/// - input recursion: structural descent over the input inline tree.
fn measure_inline(
    inline: &Sexp,
    views: &LexiconViews,
    measure: &mut ParagraphMeasure,
    emphasized: EmphasisState,
) -> Result<(), GfDocsError>
{
    let Sexp::App { ref head, ref args } = *inline
    else {
        return Err(GfDocsError::Parse("inline is not an application".into()));
    };
    match head.as_str() {
        | "Txt" => {
            let [ref text] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("Txt arity is not one".into()));
            };
            let text = quoted(text)?;
            push_words(text.as_str().into(), measure, emphasized);
        },
        | "Bold" | "Italic" => {
            let [ref children] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse(format!("{head} arity is not one")));
            };
            measure.emphasis_spans = measure.emphasis_spans.saturating_add(1);
            walk_list(children, "Inline".into(), &mut |child| {
                measure_inline(child, views, measure, EmphasisState(true))
            })?;
        },
        | "TermDef" => {
            let [ref term, ref display] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("TermDef arity is not two".into()));
            };
            measure
                .ref_constants
                .insert(atom_of(term)?.as_ref().to_owned());
            let display = quoted(display)?;
            push_words(display.as_str().into(), measure, emphasized);
        },
        | "TermRef" => {
            let [ref term] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("TermRef arity is not one".into()));
            };
            let name = atom_of(term)?;
            measure.term_refs = measure.term_refs.saturating_add(1);
            measure.ref_constants.insert(name.as_ref().to_owned());
            let text = views
                .terms
                .get(name.as_ref())
                .cloned()
                .unwrap_or_else(|| name.as_ref().to_owned());
            push_words(text.as_str().into(), measure, emphasized);
        },
        | "XRef" => {
            let [ref anchor] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("XRef arity is not one".into()));
            };
            let name = atom_of(anchor)?;
            measure.xrefs = measure.xrefs.saturating_add(1);
            measure.ref_constants.insert(name.as_ref().to_owned());
            let text = views
                .anchors
                .get(name.as_ref())
                .cloned()
                .unwrap_or_else(|| name.as_ref().to_owned());
            push_words(text.as_str().into(), measure, emphasized);
        },
        | "CiteRef" => {
            measure.cites = measure.cites.saturating_add(1);
        },
        | "MathInline" | "CodeInline" => {},
        | other => {
            return Err(GfDocsError::Parse(format!(
                "unknown inline constructor {other}"
            )));
        },
    }
    Ok(())
}

/// Append text to the paragraph and count its words (tracking emphasized
/// words separately).
fn push_words(
    text: ParagraphText<'_>,
    measure: &mut ParagraphMeasure,
    emphasized: EmphasisState,
)
{
    if !measure.text.is_empty() && !text.0.is_empty() {
        measure.text.push(' ');
    }
    measure.text.push_str(text.0);
    if emphasized.0 {
        let words = u32::from(count_words(text));
        measure.emphasis_words = measure.emphasis_words.saturating_add(words);
    }
}

/// Count words: whitespace tokens containing at least one alphanumeric
/// character (code tokens count as one word; stray punctuation as none).
fn count_words(text: ParagraphText<'_>) -> WordCount
{
    let count = text
        .0
        .split_whitespace()
        .filter(|token| token.chars().any(char::is_alphanumeric))
        .count();
    WordCount(u32::try_from(count).unwrap_or(u32::MAX))
}

/// Resolve the `HTML`-entity forms the tree stores (`&lt;` `&gt;` `&amp;`)
/// back to the characters a reader reads.
fn unescape(text: EscapedProse<'_>) -> String
{
    text.0
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The arguments of a constructor application, or a parse error.
fn expect_app<'tree>(
    tree: &'tree Sexp,
    expected: ExpectedConstructor<'_>,
) -> Result<&'tree [Sexp], GfDocsError>
{
    let Sexp::App { ref head, ref args } = *tree
    else {
        return Err(GfDocsError::Parse(format!(
            "expected {}, found an atom",
            expected.0,
        )));
    };
    if head != expected.0 {
        return Err(GfDocsError::Parse(format!(
            "expected {}, found {head}",
            expected.0,
        )));
    }
    Ok(args)
}

/// The text of a bare-atom node.
fn atom_of(tree: &Sexp) -> Result<SexpAtom<'_>, GfDocsError>
{
    let Sexp::Atom(ref atom) = *tree
    else {
        return Err(GfDocsError::Parse("expected a bare atom".into()));
    };
    Ok(SexpAtom(atom))
}

/// The unquoted text of a string-literal atom.
fn quoted(tree: &Sexp) -> Result<String, GfDocsError>
{
    let Sexp::Atom(ref atom) = *tree
    else {
        return Err(GfDocsError::Parse("expected a string literal".into()));
    };
    unquote((atom).into()).ok_or_else(|| GfDocsError::Parse("expected a string literal".into()))
}

/// Walk one list chain (`Cons<Tag>`/`Cons<Tag>Glued` … `Base<Tag>`),
/// applying `visit` to each element. The glue-boundary cons (a
/// punctuation-leading text binds to its left neighbor) is a rendering
/// distinction the metrics do not make.
fn walk_list(
    tree: &Sexp,
    tag: ListTag<'_>,
    visit: &mut dyn FnMut(&Sexp) -> Result<(), GfDocsError>,
) -> Result<(), GfDocsError>
{
    let tag = tag.0;
    let mut cursor = tree;
    loop {
        match cursor {
            | &Sexp::Atom(ref atom) if *atom == format!("Base{tag}") => return Ok(()),
            | &Sexp::App { ref head, ref args }
                if *head == format!("Cons{tag}") || *head == format!("Cons{tag}Glued") =>
            {
                let [ref element, ref tail] = *args.as_slice()
                else {
                    return Err(GfDocsError::Parse(format!("Cons{tag} arity is not two")));
                };
                visit(element)?;
                cursor = tail;
            },
            | _ => return Err(GfDocsError::Parse(format!("malformed [{tag}] list"))),
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// Build views by hand (unit tests never touch the runtime).
    fn views() -> LexiconViews
    {
        let terms = BTreeMap::from([("term_frozen_core".to_owned(), "frozen core".to_owned())]);
        let anchors = BTreeMap::from([(
            "anchor_rs_ladder".to_owned(),
            "Marker semantics along the productivity ladder".to_owned(),
        )]);
        LexiconViews {
            terms,
            anchors,
            anchor_ids: BTreeMap::new(),
        }
    }

    /// Wrap inlines into a prose block inside a one-section component.
    fn component_with_blocks(blocks: Vec<Sexp>) -> Sexp
    {
        let mut chain = Sexp::atom("BaseBlock");
        for block in blocks.into_iter().rev() {
            chain = Sexp::app("ConsBlock", vec![block, chain]);
        }
        let section = Sexp::app("MkSection", vec![
            Sexp::atom("anchor_s"),
            Sexp::string("Section title"),
            Sexp::atom("StatusPartial"),
            chain,
        ]);
        Sexp::app("MkComponent", vec![
            Sexp::atom("anchor_c"),
            Sexp::string("Component title"),
            Sexp::atom("StatusPartial"),
            Sexp::atom("BaseCiteKey"),
            Sexp::atom("BaseAnchor"),
            Sexp::app("ConsSection", vec![section, Sexp::atom("BaseSection")]),
            Sexp::atom("BaseCiteRef"),
        ])
    }

    /// One prose block from inline nodes.
    fn prose(inlines: Vec<Sexp>) -> Sexp
    {
        let mut chain = Sexp::atom("BaseInline");
        for inline in inlines.into_iter().rev() {
            chain = Sexp::app("ConsInline", vec![inline, chain]);
        }
        Sexp::app("ProseBlock", vec![chain])
    }

    /// A text inline.
    fn txt(text: ParagraphText<'_>) -> Sexp
    {
        Sexp::app("Txt", vec![Sexp::string(text.0)])
    }

    #[test]
    fn dist_computes_exact_statistics()
    {
        let dist =
            Dist::from_samples(WordCountSamples::from(&[10, 20, 30, 40][..])).expect("nonempty");
        assert_eq!(dist.count, 4);
        assert_eq!(dist.min, 10);
        assert_eq!(dist.max, 40);
        assert!((dist.mean - 25.0).abs() < f64::EPSILON);
        assert!((dist.median - 25.0).abs() < f64::EPSILON);
        assert!((dist.p90 - 40.0).abs() < f64::EPSILON);
        assert!((dist.stdev - 11.180_339_887_498_949_f64).abs() < 1.0e-9_f64);
    }

    #[test]
    fn dist_handles_odd_and_single_samples()
    {
        let odd = Dist::from_samples(WordCountSamples::from(&[5, 1, 9][..])).expect("nonempty");
        assert!((odd.median - 5.0).abs() < f64::EPSILON);
        assert!((odd.p90 - 9.0).abs() < f64::EPSILON);
        let single = Dist::from_samples(WordCountSamples::from(&[7][..])).expect("nonempty");
        assert!((single.mean - 7.0).abs() < f64::EPSILON);
        assert!((single.stdev - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn copattern_dots_do_not_split_sentences()
    {
        let text = "The record is (head =&gt; e, tail =&gt; e). The second sentence follows.";
        assert_eq!(UnicodeSegmentation::unicode_sentences(text).count(), 2);
    }

    #[test]
    fn word_count_treats_code_tokens_as_one_word()
    {
        assert_eq!(
            u32::from(count_words(
                "the eliminator f[<](p, rec n ih) produces".into(),
            )),
            7,
        );
        assert_eq!(u32::from(count_words("— ; ...".into())), 0);
    }

    #[test]
    fn term_and_cross_references_resolve_display_text()
    {
        let tree = component_with_blocks(vec![prose(vec![
            txt("The".into()),
            Sexp::app("TermRef", vec![Sexp::atom("term_frozen_core")]),
            txt("ride the".into()),
            Sexp::app("XRef", vec![Sexp::atom("anchor_rs_ladder")]),
            txt("presentation.".into()),
        ])]);
        let report = analyze(&tree, &views()).expect("analyze");
        // 1 + 2 + 2 + 6 + 1 words across the resolved references.
        assert_eq!(report.prose_words, 12);
        assert_eq!(report.term_refs, 1);
        assert_eq!(report.xrefs, 1);
    }

    #[test]
    fn emphasis_density_counts_spans_and_words()
    {
        let mut bold_children = Sexp::atom("BaseInline");
        bold_children = Sexp::app("ConsInline", vec![
            txt("exactly one idea".into()),
            bold_children,
        ]);
        let tree = component_with_blocks(vec![prose(vec![
            txt("A paragraph carries".into()),
            Sexp::app("Bold", vec![bold_children]),
            txt("per the doctrine.".into()),
        ])]);
        let report = analyze(&tree, &views()).expect("analyze");
        assert_eq!(report.sections.first().map(|s| s.emphasis_spans), Some(1));
        assert!((report.paragraphs_with_emphasis - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn weave_flags_unintroduced_payload_blocks()
    {
        let payload = Sexp::app("JudgementsBlock", vec![
            Sexp::string("Judgement forms"),
            Sexp::atom("BaseTxt"),
            Sexp::atom("BaseTxt"),
        ]);
        let tree = component_with_blocks(vec![
            prose(vec![txt("An introduction.".into())]),
            payload.clone(),
            payload,
        ]);
        let report = analyze(&tree, &views()).expect("analyze");
        assert_eq!(report.payload_blocks, 2);
        assert_eq!(report.weave_violations, 1);
        assert_eq!(report.findings.len(), 1);
        assert!(report.findings[0].contains("JudgementsBlock"));
    }

    #[test]
    fn adjacent_paragraphs_share_reference_chains()
    {
        let first = prose(vec![
            txt("The".into()),
            Sexp::app("TermRef", vec![Sexp::atom("term_frozen_core")]),
            txt("enters.".into()),
        ]);
        let second = prose(vec![
            txt("Again the".into()),
            Sexp::app("TermRef", vec![Sexp::atom("term_frozen_core")]),
            txt("returns.".into()),
        ]);
        let tree = component_with_blocks(vec![first, second]);
        let report = analyze(&tree, &views()).expect("analyze");
        assert!((report.adjacent_shared_refs_mean - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn entities_unescape_before_word_counting()
    {
        let text = "add[&lt;](p, n) computes";
        let measure_tree = component_with_blocks(vec![prose(vec![txt(text.into())])]);
        let report = analyze(&measure_tree, &views()).expect("analyze");
        assert_eq!(report.prose_words, 3);
    }
}
