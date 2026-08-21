//! The render machine. **Slice three owns this module; it carries no code
//! yet.**
//!
//! The winning plan executes on a first-order machine with an explicit stack of
//! plan identities. There is no choiceless document tree, no candidate string,
//! and no recursion.
//!
//! - Every popped identity charges one machine step.
//! - Before a sequence node pushes its right and then its left child, the
//!   machine checks its stack ceiling.
//! - Text and verbatim nodes append their exact bytes.
//! - A newline node appends its recorded physical ending and then its
//!   indentation spaces.
//! - Output bytes are charged before each append.
//!
//! The selected measure's checked byte count is compared against the output
//! ceiling and passed to a single exact fallible reservation. If the counter
//! and the measure disagree, that is an internal arithmetic overflow and it is
//! reported as one — never as partial output. A renderer that emits half a
//! document and returns an error has already destroyed the property the whole
//! metering discipline exists to guarantee.
//!
//! Trivially empty sequences may be eliminated, but a rewrite must preserve
//! sharing and taint semantics. No unconstrained partial evaluator belongs
//! here.
