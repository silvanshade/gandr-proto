//! Terminal lifecycle and the smoke launch path.

use std::io::Write;

use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::backend::TestBackend;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::EnterAlternateScreen;
use ratatui::crossterm::terminal::LeaveAlternateScreen;
use ratatui::crossterm::terminal::disable_raw_mode;
use ratatui::crossterm::terminal::enable_raw_mode;

use crate::app::App;
use crate::app::AppKey;
use crate::app::InputChar;
use crate::view::draw;

/// The text the smoke face prints.
pub const SMOKE_NOTE: &str = "gandr tui: ready\n";

/// Why the terminal face stopped.
#[derive(Debug, thiserror::Error)]
pub enum TuiError
{
    /// The terminal backend failed.
    #[error("terminal failed: {0}")]
    Terminal(#[from] std::io::Error),
}

/// Draw one test-backend frame and write the launch note.
///
/// # Contract
/// - ensures: a frame is drawn and [`SMOKE_NOTE`] is written once.
/// - provides: the observable TUI smoke path.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TuiError`] when the note cannot be written.
///
/// # Adequacy
/// - hypothesis: L3 — the smoke path writes the launch note and leaves
///   successfully.
/// - witness: `launch::tests::smoke_writes_the_launch_note`
#[inline]
pub fn run_smoke<Output>(output: &mut Output) -> Result<(), TuiError>
where
    Output: Write,
{
    let mut terminal = match Terminal::new(TestBackend::new(80, 24)) {
        | Ok(terminal) => terminal,
        | Err(void) => match void {},
    };
    let app = App::new();
    match terminal.draw(|frame| draw(frame, &app)) {
        | Ok(_) => {},
        | Err(void) => match void {},
    }
    output.write_all(SMOKE_NOTE.as_bytes())?;
    Ok(())
}

/// Run the interactive face on the process terminal.
///
/// # Contract
/// - ensures: the alternate screen is restored on every path, including quit.
/// - provides: the interactive TUI.
/// - fails: returns [`TuiError`] when the terminal cannot be claimed.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TuiError`] when raw mode or the backend fails.
#[inline]
pub fn run() -> Result<(), TuiError>
{
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let result = run_on(Terminal::new(backend)?);
    drop(disable_raw_mode());
    let mut stdout = std::io::stdout();
    drop(execute!(stdout, LeaveAlternateScreen));
    result
}

/// Drive `terminal` until the application stops.
///
/// # Contract
/// - ensures: each key is applied to the model and the frame is redrawn.
/// - fails: returns a backend or input error.
/// - panics: none.
///
/// # Errors
///
/// Returns [`TuiError`] when drawing or reading input fails.
fn run_on(mut terminal: Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<(), TuiError>
{
    let mut app = App::new();
    loop {
        terminal.draw(|frame| draw(frame, &app))?;
        let Event::Key(key) = crossterm::event::read()?
        else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let mapped = match key.code {
            | KeyCode::Esc => Some(AppKey::Quit),
            | KeyCode::Char('q') if app.input.is_empty() => Some(AppKey::Quit),
            | KeyCode::Enter => Some(AppKey::Enter),
            | KeyCode::Backspace => Some(AppKey::Backspace),
            | KeyCode::Char(ch) => Some(AppKey::Char(InputChar::from(ch))),
            | _ => None,
        };
        if let Some(app_key) = mapped
            && app.handle(app_key).is_some()
        {
            return Ok(());
        }
    }
}
