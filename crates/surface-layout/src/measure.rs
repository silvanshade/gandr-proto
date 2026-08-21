//! Measure and cost. **Slice two owns this module; it carries no code yet.**
//!
//! A measure summarizes one candidate layout of one subdocument in the exact
//! context it was resolved in: what it costs, where it leaves the column, which
//! render plan produces it, and how many bytes that plan emits.
//!
//! # The shapes
//!
//! ```text
//! pub struct LayoutCost {
//!     pub squared_overflow: SquaredOverflow,
//!     pub line_breaks: LineBreaks,
//! }
//!
//! pub struct LayoutOptions {
//!     pub page_width: PageWidth,
//!     pub computation_width: ComputationWidth,
//!     pub line_ending: PhysicalLineEnding,
//! }
//!
//! struct Measure {
//!     last_column: Column,
//!     cost: LayoutCost,
//!     plan: PlanId,
//!     output_bytes: OutputBytes,
//! }
//! ```
//!
//! `PageWidth` defaults to 100 columns and `ComputationWidth` to 120, and the
//! computation width must be at least the page width. The default physical
//! ending is a line feed.
//!
//! # The cost rule
//!
//! Costs order lexicographically by squared overflow, then by line count. For
//! text extending a line from column `c` by `len`, the incremental overflow is
//! `max(0, c + len - page_width)^2 - max(0, c - page_width)^2`. A layout-owned
//! newline adds one line break, and the indentation it emits contributes
//! overflow as text starting from column zero.
//!
//! Squaring is what makes the rule prefer many small overruns to one large one,
//! which is the behaviour a reader actually wants from a printer that cannot
//! fit everything.
//!
//! For verbatim records, charge the text rule from the incoming column across
//! the first fragment, then `max(0, width - page_width)^2` from absolute column
//! zero for every later fragment. The count of records carrying an ending is
//! added to the line breaks. The ending column is the incoming column plus the
//! sole fragment's width when there is only one, and otherwise the last
//! fragment's width. Output bytes add the exact stored bytes.
//!
//! Overflow is charged for **every** fragment, so an over-wide middle line
//! costs and taints the candidate even when the final line is short. A rule
//! that looked only at the ending column would call such a layout free, which
//! is precisely wrong.
//!
//! Every addition, square, width, indentation, and byte count is checked.
//!
//! # The frontier
//!
//! One measure dominates another exactly when its cost is no greater and its
//! ending column is no greater, with at least one of the two strict. An
//! untainted measure set is a non-empty, mutually non-dominating frontier
//! sorted by strictly increasing cost and therefore strictly decreasing ending
//! column. Equal cost and equal ending column keeps the left, earlier plan, so
//! a tie is resolved by construction order rather than by whichever candidate
//! happened to arrive second.
//!
//! Choice merges two sorted frontiers linearly. Concatenation combines each
//! left measure with the memoized result of the right taken at that left's
//! ending column, pruning as it goes rather than materializing a product. The
//! root of an untainted result selects the least-cost measure.
//!
//! The ending-column dimension is not an optimization, it is what makes the
//! answer correct: under unaligned concatenation a locally more expensive
//! layout can leave a column that makes everything after it cheaper, and a
//! frontier that tracked only cost would have already discarded it.
//!
//! Measures and mutable frontiers are private. A client sees the chosen cost on
//! the rendered result and nothing else.
