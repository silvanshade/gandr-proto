//! Draw the application onto a ratatui frame.

use gandr_surface_render_remote::present::TranscriptBlock;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::theme::style_of;

/// Draw `app` onto `frame`.
///
/// # Contract
/// - ensures: the frame receives a transcript, an input, and a status pane.
/// - panics: none.
#[inline]
pub fn draw(
    frame: &mut Frame<'_>,
    app: &App,
)
{
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());
    let mut lines = Vec::new();
    for block in &app.transcript {
        lines.push(paint_source(block));
        for pair in &block.lines {
            lines.push(Line::from(pair.1.clone()));
        }
    }
    if let Some(area) = layout.first() {
        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().title(" transcript ").borders(Borders::ALL)),
            *area,
        );
    }
    if let Some(area) = layout.get(1) {
        frame.render_widget(
            Paragraph::new(app.input.as_str())
                .block(Block::default().title(" input ").borders(Borders::ALL)),
            *area,
        );
    }
    if let Some(area) = layout.get(2) {
        frame.render_widget(Paragraph::new(app.status.as_str()), *area);
    }
}

/// Paint `block.source` with its highlight spans.
fn paint_source(block: &TranscriptBlock) -> Line<'static>
{
    if block.source_hl.is_empty() {
        return Line::from(block.source.clone());
    }
    let source = block.source.as_str();
    let mut spans = Vec::new();
    let mut cursor = 0_usize;
    for span in &block.source_hl {
        let start = usize::from(span.range.start);
        let end = usize::from(span.range.end);
        let well_formed = start >= cursor
            && start < end
            && end <= source.len()
            && source.is_char_boundary(start)
            && source.is_char_boundary(end);
        if !well_formed {
            continue;
        }
        if start > cursor {
            spans.push(Span::raw(source[cursor .. start].to_owned()));
        }
        spans.push(Span::styled(
            source[start .. end].to_owned(),
            style_of(span.role),
        ));
        cursor = end;
    }
    if cursor < source.len() && source.is_char_boundary(cursor) {
        spans.push(Span::raw(source[cursor ..].to_owned()));
    }
    if spans.is_empty() {
        Line::from(block.source.clone())
    }
    else {
        Line::from(spans)
    }
}
