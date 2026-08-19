//! The read-evaluate loop over the headless session engine.
//!
//! The line editor owns raw text. The validator gates on parse completeness
//! only. The landed [`Session`] carries typed context across lines. This
//! crate encodes the report; it does not parse, lower, type, or mark.

#![cfg_attr(not(test), warn(clippy::print_stdout, clippy::print_stderr))]

pub mod batch;
pub mod completeness;
pub mod encode;
pub mod interactive;
pub mod session_loop;

pub use batch::BatchStatus;
pub use batch::run_batch;
pub use completeness::CompletenessError;
pub use completeness::completeness;
pub use encode::encode_submission;
pub use interactive::run_interactive;
pub use session_loop::LoopError;
pub use session_loop::LoopEvent;
pub use session_loop::SessionLoop;
