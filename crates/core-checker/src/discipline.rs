//! The disciplines the checker enforces beside the typing judgement proper.
//!
//! [`subtype`] is the subsumption relation, consistent rather than strict
//! because holes make `Unknown` a two-sided neighbour. [`grade`] is the usage
//! discipline over thunks. [`mark`] is the totality discipline: it recovers
//! from a type error by marking the node rather than aborting the run, so an
//! editor state always has a typing. [`boundary`] is the representation
//! discipline — the semantic wrappers every crate-defined signature crosses,
//! which is what keeps anonymous primitives out of this crate's public surface.
//!
//! They are grouped because each is a rule the checker applies rather than a
//! judgement it derives, and because the later reorganization slices want the
//! boundary between this layer and the judgements measurable.

pub mod boundary;
pub mod grade;
pub mod mark;
pub mod subtype;
