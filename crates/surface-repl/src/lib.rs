//! The read-evaluate loop over the headless session engine.
//!
//! The line editor owns raw text. The validator gates on parse completeness
//! only. The landed session engine carries typed context across lines.
//!
//! This crate is an encoder over checker state: it implements no parser, no
//! lowering, no typing, and no marker. It does *invoke* the landed parser —
//! [`mod@completeness`] asks it whether more tokens are expected, and
//! [`highlight`] asks the grammar's normative highlighter to classify a
//! committed CST — exactly as the language-server face does. Owning a parser
//! and calling one are different things, and the distinction is stated here
//! rather than left to "does not parse", which a reader checks against
//! [`mod@completeness`] molding a token stream and finds false.

#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::print_stderr))]

pub mod batch;
pub mod completeness;
pub mod encode;
pub mod highlight;
pub mod interactive;
pub mod session_loop;

pub use batch::BatchStatus;
pub use batch::run_batch;
pub use completeness::CompletenessError;
pub use completeness::completeness;
pub use encode::encode_submission;
pub use highlight::highlight_source;
pub use interactive::run_interactive;
pub use session_loop::LoopError;
pub use session_loop::LoopEvent;
pub use session_loop::SessionLoop;
