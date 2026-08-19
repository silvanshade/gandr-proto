//! Interactive read-evaluate loop on a line editor.

use std::io::Write;

use gandr_surface_syntax::SourceSlice;
use reedline::DefaultPrompt;
use reedline::Reedline;
use reedline::Signal;

use crate::batch::BatchStatus;
use crate::session_loop::LoopEvent;
use crate::session_loop::SessionLoop;

/// Run the loop on a terminal line editor until quit or end-of-file.
///
/// # Contract
/// - ensures: each submitted buffer is written to `output`; `:q` and
///   end-of-file leave with [`BatchStatus::COMPLETED`].
/// - provides: the interactive face.
/// - fails: returns [`BatchStatus::FAILED`] when the editor or the loop fails.
/// - panics: none.
#[inline]
pub fn run_interactive<Output>(output: &mut Output) -> BatchStatus
where
    Output: Write,
{
    let mut editor = Reedline::create();
    let prompt = DefaultPrompt::default();
    let mut session = SessionLoop::new();
    loop {
        match editor.read_line(&prompt) {
            | Ok(Signal::Success(buffer)) => {
                match session.offer(SourceSlice::from(buffer.as_str())) {
                    | Ok(LoopEvent::Continue) => {},
                    | Ok(LoopEvent::Submitted(block)) => {
                        crate::batch::write_block(output, &block);
                    },
                    | Ok(LoopEvent::Info(message)) => {
                        drop(writeln!(output, "{message}"));
                    },
                    | Ok(LoopEvent::Quit) => return BatchStatus::COMPLETED,
                    | Err(error) => {
                        drop(writeln!(output, "! {error}"));
                        return BatchStatus::FAILED;
                    },
                }
            },
            | Ok(Signal::CtrlD) => return BatchStatus::COMPLETED,
            | Ok(_) => {},
            | Err(error) => {
                drop(writeln!(output, "! line editor failed: {error}"));
                return BatchStatus::FAILED;
            },
        }
    }
}
