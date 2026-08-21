//! Batch read-evaluate loop over standard input.

use std::io::BufRead;
use std::io::Write;

use gandr_surface_diagnostics::RenderStyle;
use gandr_surface_render_remote::present::OutKind;
use gandr_surface_render_remote::present::TranscriptBlock;
use gandr_surface_syntax::SourceSlice;

use crate::session_loop::LoopError;
use crate::session_loop::LoopEvent;
use crate::session_loop::SessionLoop;

/// How the batch loop left.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchStatus(i32);

impl BatchStatus
{
    /// The loop finished every line.
    pub const COMPLETED: Self = Self(0);
    /// The last submission failed to lower or a completeness query failed.
    pub const FAILED: Self = Self(1);
}

impl From<BatchStatus> for i32
{
    #[inline]
    fn from(status: BatchStatus) -> Self
    {
        status.0
    }
}

/// Run the loop over `input`, writing transcripts to `output`.
///
/// # Contract
/// - ensures: each complete submission is written once using `render_style`;
///   incomplete source at end-of-file is submitted as-is if non-empty.
/// - provides: the non-interactive smoke path.
/// - fails: returns [`BatchStatus::FAILED`] after writing the error.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — a value line prints a value, and a quit line stops.
/// - witness: `gandr` `cli::tests::piped_value_prints_a_transcript`
#[inline]
pub fn run_batch<Input, Output>(
    input: Input,
    output: &mut Output,
    render_style: RenderStyle,
) -> BatchStatus
where
    Input: BufRead,
    Output: Write,
{
    let mut session = SessionLoop::with_render_style(render_style);
    for line in input.lines() {
        let Ok(line) = line
        else {
            drop(writeln!(output, "! failed to read input"));
            return BatchStatus::FAILED;
        };
        match session.offer(SourceSlice::from(line.as_str())) {
            | Ok(LoopEvent::Continue) => {},
            | Ok(LoopEvent::Submitted(block)) => write_block(output, &block),
            | Ok(LoopEvent::Info(message)) => {
                drop(writeln!(output, "{message}"));
            },

            | Ok(LoopEvent::Quit) => return BatchStatus::COMPLETED,
            | Err(error) => {
                write_error(output, &error);
                return BatchStatus::FAILED;
            },
        }
    }
    BatchStatus::COMPLETED
}

/// Write one transcript block as plain text.
pub(crate) fn write_block<Output>(
    output: &mut Output,
    block: &TranscriptBlock,
) where
    Output: Write,
{
    drop(writeln!(output, "▸ {}", block.source));
    for &(kind, ref line) in &block.lines {
        let mark = match kind {
            | OutKind::Source => "▸",
            | OutKind::Type => ":",
            | OutKind::Value => "=",
            | OutKind::Blame | OutKind::Diag => "!",
            | OutKind::Stuck | OutKind::Info => "·",
            | OutKind::Goal => "?",
        };
        drop(writeln!(output, "{mark} {line}"));
    }
}

/// Write a loop error.
fn write_error<Output>(
    output: &mut Output,
    error: &LoopError,
) where
    Output: Write,
{
    drop(writeln!(output, "! {error}"));
}
