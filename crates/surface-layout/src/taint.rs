//! Width taint. **Slice two owns this module; it carries no code yet.**
//!
//! The optimality theorem behind the resolver holds while the context stays
//! inside the computation width. Outside it, the engine keeps going and says
//! so. Taint marks a candidate as outside the theorem; it never truncates,
//! never fails, and never silently drops output.
//!
//! # The shapes
//!
//! ```text
//! enum MeasureSet {
//!     Frontier(Vec<Measure>),
//!     Tainted(TaintPromise),
//! }
//!
//! enum TaintPromise {
//!     Ready(Measure),
//!     Deferred {
//!         doc: NodeId,
//!         column: Column,
//!         indentation: Indentation,
//!     },
//! }
//! ```
//!
//! # The operations, which bind exactly
//!
//! 1. Tainting a frontier keeps its first, least-cost measure as a ready
//!    promise.
//! 2. Tainting an already tainted set returns the same promise.
//! 3. Merging a frontier with a tainted set returns the frontier, from either
//!    side.
//! 4. Merging two tainted sets returns the **left** promise, forcing neither.
//! 5. A subproblem that goes out of bounds becomes a deferred promise carrying
//!    its exact document, column, and indentation. It is not inserted into the
//!    in-bound memo table and is never collapsed with another out-of-bound
//!    context.
//! 6. If the root is tainted, only the retained promise is forced. Fallback
//!    execution resolves choices left-first without building frontiers,
//!    preserves the promise's exact columns and indentation for cost and
//!    output, and stays subject to every render limit.
//!
//! Rule 5 is the one that is easy to get wrong and expensive to get wrong. A
//! shared text node reached at one column past the computation width and at two
//! columns past it yields two distinct promises and two distinct measures. No
//! context-erasing sentinel exists, because collapsing those two contexts would
//! make the engine emit one of them and report the other's cost.
//!
//! Rule 4 is why the vertical branch of a group sits on the left: when both
//! branches taint, the left-biased merge is what makes a document too wide to
//! lay out fall back to its vertical form.
//!
//! A rendered result reports that its chosen root came from a promise. Raising
//! the computation width is the prescribed way to seek an untainted optimum.
