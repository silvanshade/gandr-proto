//! The read-evaluate loop over [`Session`].
//!
//! Cross-line definitions are carried by the landed session engine's
//! checkpoint set. This module does not re-lower an accumulated prelude.

use gandr_surface_engine::lower::LowerError;
use gandr_surface_engine::session::Session;
use gandr_surface_render_remote::present::TranscriptBlock;
use gandr_surface_syntax::SourceSlice;

use crate::completeness::CompletenessError;
use crate::completeness::completeness;
use crate::encode::encode_submission;

/// What the loop did with one offered line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopEvent
{
    /// The buffer is still parse-incomplete; the line was kept.
    Continue,
    /// A complete buffer was submitted and encoded.
    Submitted(TranscriptBlock),
    /// A meta-command produced a message and did not submit.
    Info(String),
    /// The user asked the loop to stop.
    Quit,
}

/// Why the loop could not handle a line.
#[derive(Debug, thiserror::Error)]
pub enum LoopError
{
    /// Completeness could not be decided.
    #[error(transparent)]
    Completeness(#[from] CompletenessError),
    /// The session engine refused the source at the infrastructure boundary.
    #[error(transparent)]
    Lower(#[from] LowerError),
}

/// A read-evaluate loop over one [`Session`].
#[derive(Clone, Debug)]
pub struct SessionLoop
{
    /// The headless session engine.
    session: Session,
    /// Accumulated source waiting for parse completeness.
    pending: String,
}

impl SessionLoop
{
    /// Open a fresh loop over a new session.
    ///
    /// # Contract
    /// - ensures: the loop has no pending source and a prelude-only session.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self {
            session: Session::new(),
            pending: String::new(),
        }
    }

    /// Offer one line to the loop.
    ///
    /// A line that begins with `:` and arrives against an empty pending
    /// buffer is a meta-command. Otherwise the line is appended and, when
    /// parse-complete, submitted.
    ///
    /// # Contract
    /// - ensures: incomplete source is retained; complete source is submitted
    ///   exactly once and then cleared; `:q` / `:quit` yields
    ///   [`LoopEvent::Quit`].
    /// - provides: the loop's one input seam.
    /// - fails: returns [`LoopError`] when completeness or lowering fails.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`LoopError`] when the completeness query or session submit
    /// fails.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — an open form continues, a complete atom submits, a
    ///   quit command stops, and a later line sees an earlier definition.
    /// - witness: `loop::tests::an_open_form_continues`
    /// - witness: `loop::tests::a_complete_atom_submits`
    /// - witness: `loop::tests::quit_stops_the_loop`
    /// - witness: `loop::tests::a_definition_is_visible_on_the_next_line`
    #[inline]
    pub fn offer(
        &mut self,
        line: SourceSlice<'_>,
    ) -> Result<LoopEvent, LoopError>
    {
        let text = line.as_ref();
        if self.pending.is_empty() {
            if let Some(event) = meta_command(line) {
                return Ok(event);
            }
            if text.is_empty() {
                return Ok(LoopEvent::Continue);
            }
        }
        else {
            self.pending.push('\n');
        }
        self.pending.push_str(text);

        let source = SourceSlice::from(self.pending.as_str());
        if !bool::from(completeness(source)?) {
            return Ok(LoopEvent::Continue);
        }
        let submission = self.session.submit(self.pending.as_str())?;
        let block = encode_submission(source, &submission);
        self.pending.clear();
        Ok(LoopEvent::Submitted(block))
    }
}

impl Default for SessionLoop
{
    #[inline]
    fn default() -> Self
    {
        Self::new()
    }
}

/// Interpret a meta-command offered against an empty pending buffer.
fn meta_command(text: SourceSlice<'_>) -> Option<LoopEvent>
{
    match text.as_ref() {
        | ":q" | ":quit" => Some(LoopEvent::Quit),
        | ":help" => Some(LoopEvent::Info(String::from(
            "Enter submits a parse-complete buffer. Holes are typeable. :q leaves.",
        ))),
        | _ => None,
    }
}
