//! Draw the application onto a ratatui frame.

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

use crate::app::App;

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
    let mut transcript = String::new();
    for block in &app.transcript {
        transcript.push_str(&block.source);
        transcript.push('\n');
        for pair in &block.lines {
            transcript.push_str(&pair.1);
            transcript.push('\n');
        }
    }
    if let Some(area) = layout.first() {
        frame.render_widget(
            Paragraph::new(transcript)
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
