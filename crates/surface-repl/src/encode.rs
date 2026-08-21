//! Encode a session submission as a presentation transcript.
//!
//! The encoder reads the submission's merged verdict stream. It does not parse,
//! lower, type, or mark. Highlight spans stay empty: this crate consumes
//! [`HlSpan`] and never produces one.

use gandr_surface_diagnostics::RenderStyle;
use gandr_surface_diagnostics::render_verdict;
use gandr_surface_engine::session::ItemOutcome;
use gandr_surface_engine::session::Submission;
use gandr_surface_engine::session::Verdict;
use gandr_surface_render_remote::present::HlSpan;
use gandr_surface_render_remote::present::OutKind;
use gandr_surface_render_remote::present::TranscriptBlock;
use gandr_surface_syntax::SourceSlice;

/// Encode `submission` of `source` as a transcript block.
///
/// Highlight spans are empty. Semantic tokens come from the language-server
/// face; this encoder does not invent a second role set.
///
/// # Contract
/// - ensures: the block echoes `source` and one line per merged verdict, in
///   stream order, with diagnostics using `render_style`.
/// - provides: the presentation image every face draws.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — a value, a definition, a hole, and a typing failure each
///   produce a distinct line kind.
/// - witness: `loop::tests::a_value_encodes_as_type_and_value_lines`
/// - witness: `loop::tests::a_definition_encodes_as_a_type_line`
/// - witness: `loop::tests::a_hole_encodes_as_a_goal_line`
#[inline]
#[must_use]
pub fn encode_submission(
    source: SourceSlice<'_>,
    submission: &Submission,
    render_style: RenderStyle,
) -> TranscriptBlock
{
    let mut lines = Vec::new();
    for verdict in submission.verdicts() {
        encode_verdict(source, verdict, render_style, &mut lines);
    }
    TranscriptBlock::new(String::from(source.as_ref()), Vec::<HlSpan>::new(), lines)
}

/// Append the lines one merged verdict contributes.
///
/// # Contract
/// - requires: `source` is the exact text that produced `verdict`.
/// - ensures: diagnostic-bearing verdicts are rendered by the shared facade;
///   values, definitions, and goals retain their existing transcript kinds.
/// - provides: one REPL presentation path for source diagnostics.
/// - panics: none.
fn encode_verdict(
    source: SourceSlice<'_>,
    verdict: Verdict<'_>,
    render_style: RenderStyle,
    lines: &mut Vec<(OutKind, String)>,
)
{
    match verdict {
        | Verdict::Outcome(outcome) => encode_outcome(source, outcome, render_style, lines),
        | Verdict::Diagnostic(diagnostic) => {
            let verdict = Verdict::Diagnostic(diagnostic);
            push_diagnostic(source, &verdict, render_style, lines);
        },
        | Verdict::Goal(goal) => {
            let expected = goal.expected.as_deref().unwrap_or("?");
            lines.push((OutKind::Goal, format!("{} : {expected}", goal.hole)));
        },
    }
}

/// Append one facade-rendered diagnostic without exposing renderer types.
fn push_diagnostic(
    source: SourceSlice<'_>,
    verdict: &Verdict<'_>,
    render_style: RenderStyle,
    lines: &mut Vec<(OutKind, String)>,
)
{
    if let Some(rendered) = render_verdict(source, None, verdict, render_style) {
        lines.push((OutKind::Diag, rendered));
    }
}

/// Append the lines one item outcome contributes.
fn encode_outcome(
    source: SourceSlice<'_>,
    outcome: &ItemOutcome,
    render_style: RenderStyle,
    lines: &mut Vec<(OutKind, String)>,
)
{
    match outcome {
        | &ItemOutcome::Definition {
            ref name,
            ref ty,
            bound,
        } => {
            let binding = if bound { "" } else { " (not bound)" };
            lines.push((OutKind::Type, format!("{name} : {ty:?}{binding}")));
        },
        | &ItemOutcome::Expression { ref ty, ref value } => {
            lines.push((OutKind::Type, format!("{ty:?}")));
            lines.push((OutKind::Value, format!("{value:?}")));
        },
        | &ItemOutcome::TypeError { .. } => {
            let verdict = Verdict::Outcome(outcome);
            push_diagnostic(source, &verdict, render_style, lines);
        },
        | &ItemOutcome::Holey => {
            lines.push((
                OutKind::Info,
                String::from("holes present; evaluation declined"),
            ));
        },
    }
}
