//! Terminal diagnostics over the surface engine's merged verdict stream.
//!
//! This crate owns the dependency on `annotate-snippets`. The surface engine
//! remains responsible for parsing, lowering, typing, and source spans; this
//! facade only projects those facts into a terminal report.

use std::path::Path;

use annotate_snippets::AnnotationKind;
use annotate_snippets::Group;
use annotate_snippets::Level;
use annotate_snippets::Origin;
use annotate_snippets::Renderer;
use annotate_snippets::Snippet;
use annotate_snippets::renderer::DecorStyle;
use gandr_core_term::error::TypeError;
use gandr_surface_engine::diag::Diagnostic;
use gandr_surface_engine::diag::DiagnosticAnnotation;
use gandr_surface_engine::diag::DiagnosticAnnotationKind;
use gandr_surface_engine::diag::Severity;
use gandr_surface_engine::diag::Span;
use gandr_surface_engine::diag::message_of;
use gandr_surface_engine::session::ItemOutcome;
use gandr_surface_engine::session::Submission;
use gandr_surface_engine::session::Verdict;
use gandr_surface_syntax::SourceSlice;

/// Color policy selected by a terminal face without exposing renderer types.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RenderStyle
{
    /// Deterministic text with no terminal escape sequences.
    #[default]
    Plain,
    /// Annotate-snippets' styled terminal presentation.
    Styled,
}

impl RenderStyle
{
    /// Selects styled output exactly when the destination supports a terminal.
    #[inline]
    #[must_use]
    pub const fn for_terminal(is_terminal: bool) -> Self
    {
        if is_terminal {
            Self::Styled
        }
        else {
            Self::Plain
        }
    }
}

/// Shared render request passed to the report renderer.
struct RenderReport<'text>
{
    /// Source text being rendered.
    source: SourceSlice<'text>,
    /// Optional terminal path prefix for the report header.
    path: Option<&'text Path>,
    /// Plain or styled backend policy selected by the terminal face.
    style: RenderStyle,
    /// Severity from the surface engine's diagnostic stream.
    severity: Severity,
    /// Machine diagnostic code to show as the report id.
    code: String,
    /// Human-facing title text for the report.
    title_text: String,
    /// Ordered source annotations to project into the snippet backend.
    annotations: Vec<RenderAnnotation>,
    /// Optional notes displayed below the main report.
    notes: Vec<String>,
}

/// Backend projection of one domain annotation.
struct RenderAnnotation
{
    /// Domain role retained until the annotate-snippets boundary.
    kind: DiagnosticAnnotationKind,
    /// Exact source range.
    span: Span,
    /// Locus-specific label.
    label: Option<String>,
}

/// Render every diagnostic in a submitted source's merged verdict stream.
///
/// # Contract
/// - requires: `source` is the exact source text used to produce `submission`;
///   `path`, when present, identifies that source for the terminal header.
/// - ensures: returns one rendered report for every warning, report diagnostic,
///   and outcome-only type error in merged verdict order, using `style`.
/// - provides: terminal-facing diagnostic strings without exposing
///   `annotate-snippets` types in the public API.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — report diagnostics and outcome-only type errors both
///   produce located reports, while values and goals produce no diagnostics.
/// - witness: `diagnostics::a_type_mismatch_renders_as_a_located_report`
#[must_use]
#[inline]
pub fn render_submission(
    source: SourceSlice<'_>,
    path: Option<&Path>,
    submission: &Submission,
    style: RenderStyle,
) -> Vec<String>
{
    submission
        .verdicts()
        .filter_map(|verdict| render_verdict(source, path, &verdict, style))
        .collect()
}

/// Render one diagnostic-bearing verdict, if `verdict` is a refusal or finding.
///
/// # Contract
/// - requires: `source` is the source that supplied every span in `verdict`.
/// - ensures: returns a report using `style` for a report diagnostic or an
///   outcome-only type error, and `None` for values, definitions, and hole
///   goals.
/// - provides: one terminal report with a stable code, source location when
///   available, and labels naming the diagnostic's operands.
/// - panics: none.
#[must_use]
#[inline]
pub fn render_verdict(
    source: SourceSlice<'_>,
    path: Option<&Path>,
    verdict: &Verdict<'_>,
    style: RenderStyle,
) -> Option<String>
{
    match *verdict {
        | Verdict::Outcome(&ItemOutcome::TypeError { ref error }) => {
            Some(render_type_error(source, path, error, style))
        },
        | Verdict::Diagnostic(diagnostic) => {
            Some(render_diagnostic(source, path, diagnostic, style))
        },
        | Verdict::Outcome(_) | Verdict::Goal(_) => None,
    }
}

/// Render one source-ranged report diagnostic.
fn render_diagnostic(
    source: SourceSlice<'_>,
    path: Option<&Path>,
    diagnostic: &Diagnostic,
    style: RenderStyle,
) -> String
{
    let mut annotations = diagnostic
        .annotations
        .iter()
        .map(project_annotation)
        .collect::<Vec<_>>();
    let mut notes = Vec::new();
    for context in &diagnostic.contexts {
        if context.annotations.is_empty() {
            notes.push(format!("while {}", context.prose));
            continue;
        }
        annotations.extend(
            context
                .annotations
                .iter()
                .map(|annotation| RenderAnnotation {
                    kind: annotation.kind,
                    span: annotation.span.clone(),
                    label: Some(context_annotation_label(
                        annotation.label.as_deref(),
                        context.prose.as_str(),
                    )),
                }),
        );
    }
    render_report(RenderReport {
        source,
        path,
        style,
        severity: diagnostic.severity,
        code: diagnostic.code.to_string(),
        title_text: diagnostic.message.to_string(),
        annotations,
        notes,
    })
}

/// Render an outcome-only type error that has no report-owned span.
fn render_type_error(
    source: SourceSlice<'_>,
    path: Option<&Path>,
    error: &TypeError,
    style: RenderStyle,
) -> String
{
    let message = message_of(error);
    render_report(RenderReport {
        source,
        style,
        path,
        severity: Severity::Error,
        code: message.code().to_string(),
        title_text: message.to_string(),
        annotations: Vec::new(),
        notes: Vec::new(),
    })
}

/// Render one report using the adopted terminal renderer.
fn render_report(
    RenderReport {
        source,
        path,
        style,
        severity,
        code,
        title_text,
        annotations,
        notes,
    }: RenderReport<'_>
) -> String
{
    let title = level_for(severity).primary_title(title_text).id(code);
    let mut group = Group::with_title(title);
    let valid = annotations
        .iter()
        .filter_map(|annotation| {
            valid_range(source, &annotation.span).map(|range| (annotation, range))
        })
        .collect::<Vec<_>>();
    if !valid.is_empty() {
        let snippet_path = path.map_or_else(
            || "<input>".to_owned(),
            |path| path.to_string_lossy().into_owned(),
        );
        let mut snippet = Snippet::source(source.as_ref()).path(snippet_path);
        for (annotation, range) in valid {
            let kind = match annotation.kind {
                | DiagnosticAnnotationKind::Primary => AnnotationKind::Primary,
                | DiagnosticAnnotationKind::Context => AnnotationKind::Context,
            };
            let mut rendered = kind.span(range.start .. range.end);
            if let Some(ref label) = annotation.label {
                rendered = rendered.label(label.clone());
            }
            snippet = snippet.annotation(rendered);
        }
        group = group.element(snippet);
    }
    else if let Some(path) = path {
        group = group.element(Origin::path(path.to_string_lossy().into_owned()));
    }
    for note in notes {
        group = group.element(Level::NOTE.message(note));
    }
    let groups = [group];
    let renderer = match style {
        | RenderStyle::Plain => Renderer::plain(),
        | RenderStyle::Styled => Renderer::styled(),
    };
    renderer.decor_style(DecorStyle::Unicode).render(&groups)
}

/// Select a renderer level for the surface severity.
fn level_for(severity: Severity) -> Level<'static>
{
    match severity {
        | Severity::Error => Level::ERROR,
        | Severity::Warning => Level::WARNING,
    }
}

/// Keep a source span only when it is a valid UTF-8 byte range in `source`.
fn valid_range(
    source: SourceSlice<'_>,
    span: &Span,
) -> Option<Span>
{
    if span.start > span.end {
        return None;
    }
    source.as_ref().get(span.start .. span.end)?;
    Some(span.clone())
}

/// Projects one engine annotation without leaking the backend into the domain.
fn project_annotation(annotation: &DiagnosticAnnotation) -> RenderAnnotation
{
    RenderAnnotation {
        kind: annotation.kind,
        span: annotation.span.clone(),
        label: annotation.label.clone(),
    }
}

/// Composes one locus-specific label with the cause that owns the annotation.
fn context_annotation_label(
    label: Option<&str>,
    prose: &str,
) -> String
{
    match label {
        | Some(label) => format!("{label}; while {prose}"),
        | None => format!("while {prose}"),
    }
}
