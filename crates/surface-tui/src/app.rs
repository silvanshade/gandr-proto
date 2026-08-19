//! Application model and event update.

use gandr_surface_render_remote::present::TranscriptBlock;
use gandr_surface_repl::LoopEvent;
use gandr_surface_repl::SessionLoop;
use gandr_surface_syntax::SourceSlice;

/// One key the view can dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppKey
{
    /// Insert a character into the input buffer.
    Char(InputChar),
    /// Remove the last input character.
    Backspace,
    /// Submit the input buffer.
    Enter,
    /// Leave the face.
    Quit,
}

/// A single inserted character.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputChar(char);

impl From<char> for InputChar
{
    #[inline]
    fn from(value: char) -> Self
    {
        Self(value)
    }
}

impl From<InputChar> for char
{
    #[inline]
    fn from(value: InputChar) -> Self
    {
        value.0
    }
}

/// Why the application stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppStop
{
    /// The user asked to leave.
    Quit,
}

/// The TUI model.
#[derive(Debug)]
pub struct App
{
    /// The shared read-evaluate loop.
    loop_state: SessionLoop,
    /// The current input buffer.
    input: String,
    /// Submitted transcript blocks, newest last.
    transcript: Vec<TranscriptBlock>,
    /// Status text drawn in the footer.
    status: String,
}

impl App
{
    /// Open a fresh application.
    ///
    /// # Contract
    /// - ensures: empty input and transcript over a new session.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            loop_state: SessionLoop::new(),
            input: String::new(),
            transcript: Vec::new(),
            status: String::from("Enter submits · q quits"),
        }
    }

    /// Borrow the input buffer.
    #[inline]
    #[must_use]
    pub fn input(&self) -> &str
    {
        &self.input
    }

    /// Borrow the transcript.
    #[inline]
    #[must_use]
    pub fn transcript(&self) -> &[TranscriptBlock]
    {
        &self.transcript
    }

    /// Borrow the status line.
    #[inline]
    #[must_use]
    pub fn status(&self) -> &str
    {
        &self.status
    }

    /// Apply one key.
    ///
    /// # Contract
    /// - ensures: `q` with an empty buffer stops; Enter offers the buffer to
    ///   the loop; other characters extend the buffer.
    /// - provides: the Elm update step.
    /// - panics: none.
    #[inline]
    pub fn handle(
        &mut self,
        key: AppKey,
    ) -> Option<AppStop>
    {
        match key {
            | AppKey::Quit => {
                if self.input.is_empty() {
                    return Some(AppStop::Quit);
                }
                self.input.clear();
                None
            },
            | AppKey::Backspace => {
                self.input.pop();
                None
            },
            | AppKey::Char(ch) => {
                self.input.push(char::from(ch));
                None
            },
            | AppKey::Enter => {
                self.submit_input();
                None
            },
        }
    }

    /// Offer the current buffer to the loop and record the result.
    fn submit_input(&mut self)
    {
        let line = core::mem::take(&mut self.input);
        match self.loop_state.offer(SourceSlice::from(line.as_str())) {
            | Ok(LoopEvent::Continue) => {
                self.status = String::from("waiting for parse completeness");
                self.input = line;
                self.input.push('\n');
            },
            | Ok(LoopEvent::Submitted(block)) => {
                self.status = String::from("submitted");
                self.transcript.push(block);
            },
            | Ok(LoopEvent::Info(message)) => self.status = message,
            | Ok(LoopEvent::Quit) => self.status = String::from("quit requested"),
            | Err(error) => self.status = format!("{error}"),
        }
    }
}

impl Default for App
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}
