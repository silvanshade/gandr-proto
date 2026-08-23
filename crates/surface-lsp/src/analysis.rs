//! One whole-file recheck: parse, highlight, and submit.
//!
//! This module calls the pipeline. It does not parse, lower, type, or mark
//! on its own.

use alloc::string::String;
use alloc::vec::Vec;
use std::sync::OnceLock;

use gandr_surface_engine::diag::DiagnosticAnnotation;
use gandr_surface_engine::diag::DiagnosticAnnotationKind;
use gandr_surface_engine::diag::DiagnosticContext;
use gandr_surface_engine::diag::DiagnosticMessage;
use gandr_surface_engine::diag::Report;
use gandr_surface_engine::diag::Severity;
use gandr_surface_engine::diag::message_of;
use gandr_surface_engine::render;
use gandr_surface_engine::session::ItemOutcome;
use gandr_surface_engine::session::Session;
use gandr_surface_engine::session::Submission;
use gandr_surface_engine::session::Verdict;
use gandr_surface_grammar::Pbg;
use gandr_surface_grammar::built_in;
use gandr_surface_grammar::highlight;
use gandr_surface_parser::parse;
use gandr_surface_render_remote::present::ByteOffset;
use gandr_surface_render_remote::present::HlSpan;
use gandr_surface_render_remote::present::SourceText;
use gandr_surface_syntax::SourceSlice;

use crate::position::LineIndex;
use crate::position::PositionEncoding;
use crate::position::byte_of_position;
use crate::position::position_of_byte;
use crate::protocol::CompletionItem;
use crate::protocol::Diagnostic;
use crate::protocol::DiagnosticRelatedInformation;
use crate::protocol::DiagnosticSeverity;
use crate::protocol::DocumentUri;
use crate::protocol::Hover;
use crate::protocol::Location;
use crate::protocol::MarkupContent;
use crate::protocol::Position;
use crate::protocol::Range;
use crate::protocol::SemanticTokens;
use crate::tokens::encode;

/// The built-in grammar, constructed once per process.
///
/// [`built_in`] rebuilds the whole precedence-bounded grammar from its rule
/// tables and does not read the document, so every recheck was paying for the
/// same immutable value. Measured on 2026-08-22 it costs about sixteen
/// milliseconds and is CONSTANT in document size, which made it the largest
/// single term in an LSP recheck for any document below roughly a hundred and
/// fifty items — and an editor pays it once per keystroke. Caching it took one
/// recheck of a ten-definition document from 16.6 ms to 0.59 ms, and of a
/// fifty-definition document from 19.2 ms to 3.7 ms. What remains is whole-file
/// typing, which is superlinear in document size and is engine-side work.
///
/// A construction failure is cached as `None` rather than retried. The grammar
/// is a static rule table: if it fails to build once it fails every time, and
/// retrying would reintroduce the per-keystroke cost on exactly the path that
/// can least afford it.
fn grammar() -> Option<&'static Pbg>
{
    static GRAMMAR: OnceLock<Option<Pbg>> = OnceLock::new();
    GRAMMAR.get_or_init(|| built_in().ok()).as_ref()
}

/// The projections one whole-file recheck yields.
#[derive(Clone, Debug)]
pub struct Analysis
{
    /// Source text this analysis was computed for.
    source: String,
    /// Highlight spans over that source.
    highlights: Vec<HlSpan>,
    /// Pipeline submission for that source.
    submission: Submission,
}

impl Analysis
{
    /// Recheck `source` through the public pipeline.
    ///
    /// # Contract
    /// - ensures: always returns an analysis. A grammar, parse, or lower
    ///   infrastructure failure yields empty highlights and no report rows
    ///   rather than aborting the editor session.
    /// - provides: the encoder input for tokens, diagnostics, hover, and
    ///   completion.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn check(source: String) -> Self
    {
        let highlights = match grammar() {
            | Some(pbg) => match parse(pbg, SourceSlice::from(source.as_str())) {
                | Ok(parsed) => highlight(pbg, parsed.cst()),
                | Err(_) => Vec::new(),
            },
            | None => Vec::new(),
        };
        let submission = Session::new()
            .submit(source.as_str())
            .unwrap_or_else(|_| empty_submission());
        Self {
            source,
            highlights,
            submission,
        }
    }

    /// Encode highlight spans as semantic tokens.
    ///
    /// # Contract
    /// - ensures: returns the delta-encoded stream for `encoding`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn semantic_tokens(
        &self,
        encoding: PositionEncoding,
    ) -> SemanticTokens
    {
        let index = LineIndex::new(SourceText::from(self.source.as_str()));
        SemanticTokens {
            data: encode(
                SourceText::from(self.source.as_str()),
                &index,
                encoding,
                &self.highlights,
            ),
        }
    }

    /// Encode the highlight spans overlapping `range` as semantic tokens.
    ///
    /// # Contract
    /// - requires: `range` is a client range over this analysis's source.
    /// - ensures: returns the delta-encoded stream for `encoding` restricted to
    ///   the spans that overlap `range`. A span is included when it overlaps
    ///   the range at all, so a token straddling either edge is returned whole
    ///   rather than clipped — the protocol permits a superset and a clipped
    ///   token would misstate the classified extent. Deltas are chained from
    ///   the document origin exactly as in the full stream, which is what the
    ///   protocol requires: the range restricts which tokens are sent, never
    ///   the coordinate system they are sent in. An empty or inverted range
    ///   yields an empty stream.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn semantic_tokens_in_range(
        &self,
        range: Range,
        encoding: PositionEncoding,
    ) -> SemanticTokens
    {
        let text = SourceText::from(self.source.as_str());
        let index = LineIndex::new(text);
        let start = usize::from(byte_of_position(text, &index, range.start, encoding));
        let end = usize::from(byte_of_position(text, &index, range.end, encoding));
        if end <= start {
            return SemanticTokens::default();
        }
        let visible: Vec<HlSpan> = self
            .highlights
            .iter()
            .filter(|span| {
                usize::from(span.range.start) < end && start < usize::from(span.range.end)
            })
            .cloned()
            .collect();
        SemanticTokens {
            data: encode(text, &index, encoding, &visible),
        }
    }

    /// Project the merged verdict stream into editor diagnostics.
    ///
    /// # Contract
    /// - ensures: one diagnostic per visible error verdict; the first primary
    ///   annotation is the LSP lead, every remaining root or context annotation
    ///   becomes related information, context loci retain both their annotation
    ///   label and owning cause, and genuinely unlocated refusals use the
    ///   protocol-required zero-width origin range rather than claiming the
    ///   whole document.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn diagnostics(
        &self,
        encoding: PositionEncoding,
        uri: &DocumentUri,
    ) -> Vec<Diagnostic>
    {
        let index = LineIndex::new(SourceText::from(self.source.as_str()));
        let mut out = Vec::new();
        for verdict in self.submission.verdicts() {
            let (start, end, severity, code, message, related_information) = match verdict {
                | Verdict::Diagnostic(diagnostic) => {
                    let lead = diagnostic
                        .annotations
                        .iter()
                        .position(|annotation| annotation.kind == DiagnosticAnnotationKind::Primary)
                        .or_else(|| (!diagnostic.annotations.is_empty()).then_some(0));
                    let (start, end) = lead
                        .and_then(|index| diagnostic.annotations.get(index))
                        .map_or((0, 0), |annotation| {
                            (annotation.span.start, annotation.span.end)
                        });
                    let mut message = diagnostic.message.to_string();
                    let mut related_information = diagnostic
                        .annotations
                        .iter()
                        .enumerate()
                        .filter_map(|(index, annotation)| {
                            (Some(index) != lead).then_some(annotation)
                        })
                        .map(|annotation| {
                            related(
                                uri,
                                SourceText::from(self.source.as_str()),
                                &index,
                                encoding,
                                &annotation.span,
                                annotation
                                    .label
                                    .clone()
                                    .unwrap_or_else(|| diagnostic.message.to_string()),
                            )
                        })
                        .collect::<Vec<_>>();
                    for context in &diagnostic.contexts {
                        if context.annotations.is_empty() {
                            message.push_str("\nwhile ");
                            message.push_str(&context.prose);
                        }
                        related_information.extend(context.annotations.iter().map(|annotation| {
                            related(
                                uri,
                                SourceText::from(self.source.as_str()),
                                &index,
                                encoding,
                                &annotation.span,
                                context_annotation_message(annotation, context),
                            )
                        }));
                    }
                    (
                        start,
                        end,
                        match diagnostic.severity {
                            | Severity::Error => DiagnosticSeverity::ERROR,
                            | Severity::Warning => DiagnosticSeverity::WARNING,
                        },
                        diagnostic.code,
                        message,
                        related_information,
                    )
                },
                | Verdict::Outcome(&ItemOutcome::TypeError { ref error }) => {
                    let message = message_of(error);
                    (
                        0_usize,
                        0_usize,
                        DiagnosticSeverity::ERROR,
                        message.code(),
                        message.to_string(),
                        Vec::new(),
                    )
                },
                | Verdict::Outcome(_) | Verdict::Goal(_) => continue,
            };
            out.push(Diagnostic {
                range: range_of_bytes(
                    SourceText::from(self.source.as_str()),
                    &index,
                    ByteOffset::from(start),
                    ByteOffset::from(end),
                    encoding,
                ),
                severity,
                code,
                message,
                related_information,
            });
        }
        for obligation in &self.submission.report.obligations {
            let message = DiagnosticMessage::ParseRepair {
                class: format!("{:?}", obligation.class),
            };
            out.push(Diagnostic {
                range: range_of_bytes(
                    SourceText::from(self.source.as_str()),
                    &index,
                    ByteOffset::from(obligation.span.start),
                    ByteOffset::from(obligation.span.end),
                    encoding,
                ),
                code: message.code(),
                severity: DiagnosticSeverity::WARNING,
                message: message.to_string(),
                related_information: Vec::new(),
            });
        }
        out
    }
    /// Hover at `position`.
    ///
    /// Preference order: a hole goal whose span contains the cursor, else a
    /// diagnostic annotation that contains it, else — when the cursor sits on
    /// a name this submission defined — that definition's rendered type. The
    /// definition arm is what makes hover answer on an otherwise clean file,
    /// where no goal or diagnostic exists to cover anything.
    ///
    /// # Contract
    /// - ensures: returns the first preference that applies, and [`None`] only
    ///   when the cursor sits on nothing this analysis knows.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn hover(
        &self,
        position: Position,
        encoding: PositionEncoding,
    ) -> Option<Hover>
    {
        let index = LineIndex::new(SourceText::from(self.source.as_str()));
        let byte = byte_of_position(
            SourceText::from(self.source.as_str()),
            &index,
            position,
            encoding,
        );
        for goal in &self.submission.report.goals {
            if bool::from(contains(
                ByteOffset::from(goal.span.start),
                ByteOffset::from(goal.span.end),
                byte,
            )) {
                let mut body = String::from("**goal**");
                if let Some(expected) = goal.expected.as_ref() {
                    body.push_str("\n\nexpected: `");
                    body.push_str(expected);
                    body.push('`');
                }
                if let Some(note) = goal.note.as_ref() {
                    body.push_str("\n\n");
                    body.push_str(note);
                }
                return Some(Hover {
                    contents: MarkupContent::markdown(body),
                });
            }
        }
        for diagnostic in &self.submission.report.diagnostics {
            for annotation in &diagnostic.annotations {
                let span = &annotation.span;
                if bool::from(contains(
                    ByteOffset::from(span.start),
                    ByteOffset::from(span.end),
                    byte,
                )) {
                    return Some(Hover {
                        contents: MarkupContent::markdown(diagnostic.message.to_string()),
                    });
                }
            }
            for context in &diagnostic.contexts {
                for annotation in &context.annotations {
                    let span = &annotation.span;
                    if bool::from(contains(
                        ByteOffset::from(span.start),
                        ByteOffset::from(span.end),
                        byte,
                    )) {
                        return Some(Hover {
                            contents: MarkupContent::markdown(format!("while {}", context.prose)),
                        });
                    }
                }
            }
        }
        if let Some(word) = word_at(SourceText::from(self.source.as_str()), byte) {
            // Later definitions shadow earlier ones, so the last match wins.
            let (name, ty, bound) =
                self.submission
                    .outcomes
                    .iter()
                    .rev()
                    .find_map(|outcome| match *outcome {
                        | ItemOutcome::Definition {
                            ref name,
                            ref ty,
                            ref bound,
                        } if *name == word.as_ref() => Some((name, ty, bound)),
                        | _ => None,
                    })?;
            let mut body = String::from("```gandr\n");
            body.push_str(name);
            body.push_str(" : ");
            body.push_str(&render::ty(ty));
            body.push_str("\n```");
            if !*bound {
                body.push_str("\n\ncomputation-typed, so not bound in scope: thunk it to name it");
            }
            return Some(Hover {
                contents: MarkupContent::markdown(body),
            });
        }
        None
    }

    /// Completion candidates at `position`.
    ///
    /// # Contract
    /// - ensures: if the cursor sits in a hole goal, returns that goal's local
    ///   bindings; otherwise returns an empty list.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn completions(
        &self,
        position: Position,
        encoding: PositionEncoding,
    ) -> Vec<CompletionItem>
    {
        let index = LineIndex::new(SourceText::from(self.source.as_str()));
        let byte = byte_of_position(
            SourceText::from(self.source.as_str()),
            &index,
            position,
            encoding,
        );
        for goal in &self.submission.report.goals {
            if !bool::from(contains(
                ByteOffset::from(goal.span.start),
                ByteOffset::from(goal.span.end),
                byte,
            )) {
                continue;
            }
            let Some(bindings) = goal.ctx_local.as_ref()
            else {
                return Vec::new();
            };
            return bindings
                .iter()
                .map(|binding| CompletionItem {
                    label: binding.name.clone(),
                    detail: Some(binding.ty.clone()),
                })
                .collect();
        }
        Vec::new()
    }
}

/// The identifier-like run containing `byte`, if any.
///
/// A cursor sitting just past a word still hovers it — the character under
/// the cursor may be the word's trailing delimiter — so both the byte at
/// `byte` and the one before it are considered.
///
/// # Contract
/// - ensures: returns the longest `[A-Za-z0-9_]` run containing or ending at
///   `byte`, and [`None`] when the cursor is not on a name.
/// - panics: none.
fn word_at(
    text: SourceText<'_>,
    byte: ByteOffset,
) -> Option<SourceSlice<'_>>
{
    let source: &str = text.into();
    let byte = usize::from(byte);
    let is_word = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
    // Snap to a char boundary, exactly as the position conversions do, so a
    // cursor handed an interior UTF-8 byte still lands on its character.
    let mut snapped = byte.min(source.len());
    while snapped > 0 && !source.is_char_boundary(snapped) {
        snapped = snapped.saturating_sub(1);
    }
    let before = source.get(.. snapped)?;
    let after = source.get(snapped ..)?;
    let on_word = after.chars().next().is_some_and(is_word);
    let past_word = before.chars().next_back().is_some_and(is_word);
    if !on_word && !past_word {
        return None;
    }
    let start = before
        .char_indices()
        .rev()
        .take_while(|&(_, ch)| is_word(ch))
        .last()
        .map_or(snapped, |(index, _)| index);
    let end = after
        .char_indices()
        .take_while(|&(_, ch)| is_word(ch))
        .last()
        .map_or(0, |(index, ch)| index.saturating_add(ch.len_utf8()));
    source
        .get(start .. snapped.saturating_add(end))
        .map(SourceSlice::from)
}

/// Whether `byte` sits in `[start, end)`.
fn contains(
    start: ByteOffset,
    end: ByteOffset,
    byte: ByteOffset,
) -> crate::boundary::ContainsByte
{
    let start = usize::from(start);
    let end = usize::from(end);
    let byte = usize::from(byte);
    crate::boundary::ContainsByte::from(byte >= start && byte < end)
}

/// Map a byte span onto an LSP range.
fn range_of_bytes(
    text: SourceText<'_>,
    index: &LineIndex,
    start: ByteOffset,
    end: ByteOffset,
    encoding: PositionEncoding,
) -> Range
{
    Range::new(
        position_of_byte(text, index, start, encoding),
        position_of_byte(text, index, end, encoding),
    )
}

/// Builds standard LSP related information for one engine-owned locus.
fn related(
    uri: &DocumentUri,
    text: SourceText<'_>,
    index: &LineIndex,
    encoding: PositionEncoding,
    span: &gandr_surface_engine::diag::Span,
    message: String,
) -> DiagnosticRelatedInformation
{
    DiagnosticRelatedInformation {
        location: Location {
            uri: uri.clone(),
            range: range_of_bytes(
                text,
                index,
                ByteOffset::from(span.start),
                ByteOffset::from(span.end),
                encoding,
            ),
        },
        message,
    }
}

/// Composes one locus-specific label with the cause that owns the annotation.
fn context_annotation_message(
    annotation: &DiagnosticAnnotation,
    context: &DiagnosticContext,
) -> String
{
    match annotation.label.as_deref() {
        | Some(label) => format!("{label}; while {}", context.prose),
        | None => format!("while {}", context.prose),
    }
}

/// An empty submission used when lowering is unavailable.
fn empty_submission() -> Submission
{
    Submission {
        report: Report {
            schema_version: gandr_surface_engine::diag::SCHEMA_VERSION,
            diagnostics: Vec::new(),
            goals: Vec::new(),
            marks: Vec::new(),
            attributes: Vec::new(),
            obligations: Vec::new(),
        },
        outcomes: Vec::new(),
        kernel: Vec::new(),
        matches: Vec::new(),
    }
}

#[cfg(test)]
mod tests
{
    /// The grammar is built once per process, not once per recheck.
    ///
    /// Ablation (2026-08-22): building a fresh grammar per call — by leaking
    /// one from `built_in()` on each invocation rather than reading the
    /// `OnceLock` — turns this red, because the two calls then hand back
    /// different addresses. Without it the cache could be removed and every
    /// other witness in this crate would stay green while an editor paid
    /// sixteen milliseconds a keystroke again, which is how the cost got here
    /// in the first place.
    #[test]
    fn the_grammar_is_built_once_per_process()
    {
        let first = super::grammar().expect("the built-in grammar builds");
        let second = super::grammar().expect("the built-in grammar builds");
        assert!(
            core::ptr::eq(first, second),
            "each recheck must read one shared grammar rather than build its own"
        );
    }

    use super::Analysis;
    use crate::position::PositionEncoding;
    use crate::protocol::CharacterOffset;
    use crate::protocol::DocumentUri;
    use crate::protocol::LineNumber;
    use crate::protocol::Position;

    #[test]
    fn a_definition_produces_semantic_tokens()
    {
        let analysis = Analysis::check(String::from("def f = 42;\n"));
        let tokens = analysis.semantic_tokens(PositionEncoding::Utf16);
        assert!(
            !tokens.data.is_empty(),
            "a definition must produce at least one token"
        );
    }

    #[test]
    fn hover_at_the_origin_is_total()
    {
        let analysis = Analysis::check(String::from("def f = 42;\n"));
        let _hover = analysis.hover(Position::default(), PositionEncoding::Utf16);
    }

    #[test]
    fn causal_contexts_become_lsp_related_information()
    {
        let analysis = Analysis::check(String::from("(ret 1)(2)\n"));
        let uri = DocumentUri::from(String::from("file:///shape.gandr"));
        let diagnostics = analysis.diagnostics(PositionEncoding::Utf16, &uri);
        assert_eq!(1, diagnostics.len());
        assert_eq!(1, diagnostics[0].related_information.len());
        assert!(
            diagnostics[0].related_information[0]
                .message
                .contains("function of an application")
        );
    }

    #[test]
    fn a_labeled_context_keeps_its_locus_and_cause_in_related_information()
    {
        let mut analysis = Analysis::check(String::from("(ret 1)(2)\n"));
        analysis.submission.report.diagnostics[0].contexts[0].annotations[0].label =
            Some(String::from("application head"));
        let uri = DocumentUri::from(String::from("file:///shape.gandr"));
        let diagnostics = analysis.diagnostics(PositionEncoding::Utf16, &uri);
        assert_eq!(1, diagnostics.len());
        assert_eq!(
            "application head; while checking the function of an application",
            diagnostics[0].related_information[0].message
        );
    }

    /// Hovering a defined name answers with its rendered type — the arm that
    /// makes hover useful on a clean file, where no goal or diagnostic covers
    /// anything.
    #[test]
    fn hovering_a_defined_name_reports_its_type()
    {
        let analysis = Analysis::check(String::from("def f = 42;\n"));
        // `f` spans bytes 4..5; hover from inside the name.
        let hover = analysis.hover(
            Position::new(LineNumber::from(0_u32), CharacterOffset::from(4_u32)),
            PositionEncoding::Utf8,
        );
        let Some(hover) = hover
        else {
            panic!("a defined name must hover");
        };
        assert!(
            hover.contents.value.contains('f'),
            "the hover names the definition: {hover:?}"
        );
        assert!(
            hover.contents.value.contains("Integer"),
            "the hover renders the definition's type: {hover:?}"
        );
    }

    /// A name this submission never defined hovers nothing.
    #[test]
    fn hovering_an_unknown_name_is_none()
    {
        let analysis = Analysis::check(String::from("def f = 42;\n"));
        let hover = analysis.hover(
            Position::new(LineNumber::from(1_u32), CharacterOffset::from(0_u32)),
            PositionEncoding::Utf8,
        );
        assert!(hover.is_none(), "nothing is defined at end of file");
    }

    /// The word scan reaches past word boundaries: a cursor at a name's
    /// trailing edge still hovers it.
    #[test]
    fn the_word_scan_accepts_the_cursor_at_either_edge()
    {
        for character in [4_u32, 5] {
            let analysis = Analysis::check(String::from("def f = 42;\n"));
            let hover = analysis.hover(
                Position::new(LineNumber::from(0_u32), CharacterOffset::from(character)),
                PositionEncoding::Utf8,
            );
            assert!(hover.is_some(), "character {character} sits on or at `f`");
        }
    }

    /// A later definition of one name shadows the earlier one in hover.
    #[test]
    fn a_redefined_name_hovers_its_latest_type()
    {
        // Assembled from pieces because embedded-syntax fixtures must be raw
        // strings, and a raw string's normalized indentation would shift the
        // byte positions this hover test reads.
        let mut source = String::from("def x = 1;");
        source.push('\n');
        source.push_str("def x = true;");
        source.push('\n');
        let analysis = Analysis::check(source);
        // Line 1's `x` (bytes 13..14) — the second binding.
        let hover = analysis.hover(
            Position::new(LineNumber::from(1_u32), CharacterOffset::from(4_u32)),
            PositionEncoding::Utf8,
        );
        let Some(hover) = hover
        else {
            panic!("a defined name must hover");
        };
        assert!(
            !hover.contents.value.contains("Integer"),
            "the shadowed type must not win: {hover:?}"
        );
    }
}
