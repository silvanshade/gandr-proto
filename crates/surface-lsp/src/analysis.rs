//! One whole-file recheck: parse, highlight, lower, report.
//!
//! This module calls the pipeline. It does not parse, lower, type, or mark
//! on its own.

use alloc::string::String;
use alloc::vec::Vec;

use gandr_surface_engine::diag::Report;
use gandr_surface_engine::diag::Severity;
use gandr_surface_engine::diag::report;
use gandr_surface_engine::lower::lower_source_total;
use gandr_surface_engine::prelude_ctx;
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
use crate::protocol::DiagnosticSeverity;
use crate::protocol::Hover;
use crate::protocol::MarkupContent;
use crate::protocol::Position;
use crate::protocol::Range;
use crate::protocol::SemanticTokens;
use crate::tokens::encode;

/// The projections one whole-file recheck yields.
#[derive(Clone, Debug)]
pub struct Analysis
{
    /// Source text this analysis was computed for.
    source: String,
    /// Highlight spans over that source.
    highlights: Vec<HlSpan>,
    /// Pipeline report for that source.
    report: Report,
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
        let highlights = match built_in() {
            | Ok(pbg) => match parse(&pbg, SourceSlice::from(source.as_str())) {
                | Ok(parsed) => highlight(&pbg, parsed.cst()),
                | Err(_) => Vec::new(),
            },
            | Err(_) => Vec::new(),
        };
        let report = match lower_source_total(source.as_str().into()) {
            | Ok(lowered) => report(&lowered, &prelude_ctx()),
            | Err(_) => empty_report(),
        };
        Self {
            source,
            highlights,
            report,
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

    /// Project the report into editor diagnostics.
    ///
    /// # Contract
    /// - ensures: one diagnostic per report diagnostic that has a span, plus
    ///   one warning per obligation row.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn diagnostics(
        &self,
        encoding: PositionEncoding,
    ) -> Vec<Diagnostic>
    {
        let index = LineIndex::new(SourceText::from(self.source.as_str()));
        let mut out = Vec::new();
        for diagnostic in &self.report.diagnostics {
            let Some(span) = diagnostic.span.as_ref()
            else {
                continue;
            };
            out.push(Diagnostic {
                range: range_of_bytes(
                    SourceText::from(self.source.as_str()),
                    &index,
                    ByteOffset::from(span.start),
                    ByteOffset::from(span.end),
                    encoding,
                ),
                severity: match diagnostic.severity {
                    | Severity::Error => DiagnosticSeverity::ERROR,
                    | Severity::Warning => DiagnosticSeverity::WARNING,
                },
                message: diagnostic.message.clone(),
            });
        }
        for obligation in &self.report.obligations {
            out.push(Diagnostic {
                range: range_of_bytes(
                    SourceText::from(self.source.as_str()),
                    &index,
                    ByteOffset::from(obligation.span.start),
                    ByteOffset::from(obligation.span.end),
                    encoding,
                ),
                severity: DiagnosticSeverity::WARNING,
                message: format!("parse repaired: {:?}", obligation.class),
            });
        }
        out
    }

    /// Hover at `position`, if a goal or diagnostic covers it.
    ///
    /// # Contract
    /// - ensures: prefers a hole goal whose span contains the cursor, else a
    ///   diagnostic whose span contains it, else `None`.
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
        for goal in &self.report.goals {
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
        for diagnostic in &self.report.diagnostics {
            let Some(span) = diagnostic.span.as_ref()
            else {
                continue;
            };
            if bool::from(contains(
                ByteOffset::from(span.start),
                ByteOffset::from(span.end),
                byte,
            )) {
                return Some(Hover {
                    contents: MarkupContent::markdown(diagnostic.message.clone()),
                });
            }
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
        for goal in &self.report.goals {
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

/// An empty report used when lowering is unavailable.
fn empty_report() -> Report
{
    Report {
        schema_version: gandr_surface_engine::diag::SCHEMA_VERSION,
        diagnostics: Vec::new(),
        goals: Vec::new(),
        marks: Vec::new(),
        attributes: Vec::new(),
        obligations: Vec::new(),
    }
}

#[cfg(test)]
mod tests
{
    use super::Analysis;
    use crate::position::PositionEncoding;
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
}
