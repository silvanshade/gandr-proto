//! Interactive read-evaluate loop on a line editor.

use std::io::Write;

use gandr_surface_diagnostics::RenderStyle;
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
/// - ensures: each submitted buffer is written to `output` using
///   `render_style`; a buffer still pending at `:q` or end-of-file is submitted
///   rather than dropped; both leave with [`BatchStatus::COMPLETED`] when
///   nothing failed.
/// - provides: the interactive face.
/// - fails: returns [`BatchStatus::FAILED`] when the editor or the loop fails.
/// - panics: none.
#[inline]
pub fn run_interactive<Output>(
    output: &mut Output,
    render_style: RenderStyle,
) -> BatchStatus
where
    Output: Write,
{
    let mut editor = Reedline::create();
    let prompt = DefaultPrompt::default();
    let mut session = SessionLoop::with_render_style(render_style);
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
                    | Ok(LoopEvent::Quit) => return finish(output, &mut session),
                    | Err(error) => {
                        drop(writeln!(output, "! {error}"));
                        return BatchStatus::FAILED;
                    },
                }
            },
            | Ok(Signal::CtrlD) => return finish(output, &mut session),
            | Ok(_) => {},
            | Err(error) => {
                drop(writeln!(output, "! line editor failed: {error}"));
                return BatchStatus::FAILED;
            },
        }
    }
}

/// Close the loop, reporting whatever is still pending.
///
/// # Contract
/// - ensures: a pending buffer is written once; an empty one writes nothing.
/// - fails: returns [`BatchStatus::FAILED`] after writing the error.
/// - panics: none.
fn finish<Output>(
    output: &mut Output,
    session: &mut SessionLoop,
) -> BatchStatus
where
    Output: Write,
{
    match session.finish() {
        | Ok(Some(LoopEvent::Submitted(block))) => {
            crate::batch::write_block(output, &block);
            BatchStatus::COMPLETED
        },
        | Ok(_) => BatchStatus::COMPLETED,
        | Err(error) => {
            drop(writeln!(output, "! {error}"));
            BatchStatus::FAILED
        },
    }
}
