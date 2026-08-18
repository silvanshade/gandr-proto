//! The disciplines the checker enforces beside the typing judgement proper.
//!
//! [`subtype`] is the subsumption relation, consistent rather than strict
//! because holes make `Unknown` a two-sided neighbour. [`mark`] is the totality
//! discipline: it recovers from a type error by marking the node rather than
//! aborting the run, so an editor state always has a typing.
//!
//! The usage and representation disciplines they spend — the grade semiring
//! and the semantic-wrapper boundary — are substrate rather than rules the
//! checker applies, and live in `gandr-core-term` beside the terms they
//! classify.
//!
//! They are grouped because each is a rule the checker applies rather than a
//! judgement it derives.

pub mod mark;
pub mod subtype;
