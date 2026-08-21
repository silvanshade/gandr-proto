//! The shared document-layout engine.
//!
//! A client builds an immutable document in a sealed arena, the resolver picks
//! a Pareto-optimal layout for a page width, and a first-order machine renders
//! the winning plan. Every client of the layout engine — the presentation
//! printer, the source formatter, the language-server faces — shares this one
//! implementation, so a layout decision is made in exactly one place.
//!
//! The engine is deliberately more expressive than a greedy Wadler printer: it
//! carries arbitrary choice and unaligned concatenation. Those two features
//! cannot be added to a greedy representation afterwards without rewriting
//! every client document, which is why they are present from the first slice.
//!
//! # What is built
//!
//! The crate lands in three slices, each of which is a complete crate that
//! passes the merge wall on its own. This module map is the plan of record;
//! later render-machine modules remain contract-only until their slice.
//!
//! | slice | modules                                | public surface it adds                                                                                     |
//! | ----- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
//! | one   | [`units`], [`error`], [`limits`], [`arena`], [`build`] | the document algebra, builder, sealed arena, build limits, and build errors |
//! | two   | [`measure`], [`taint`], [`plan`], [`mod@resolve`], [`limits`] | cost, taint, memoized resolution, plan identities, and render budgets |
//! | three | [`vm`], [`render`]                      | the render machine, entry point, rendered bytes, and tainted fallback execution                          |
//!
//! # The boundary this crate owns
//!
//! Document nodes, physical line emission, identity validity, flattening,
//! choice resolution, width taint, cost and frontier order, memoization, render
//! plans, and every resource limit belong here, because they must be identical
//! for every client. Which syntactic form groups, aligns, or nests is a
//! language decision and belongs to the client. The requested page width and
//! the client's presentation budget belong to the caller: a batch run, a
//! language server, a read-evaluate loop, and a narrow terminal pane each have
//! a different one.
//!
//! The default cost order is squared overflow followed by line count, and a
//! client may add choices and nesting but may not replace that order. A
//! configurable cost factory is an extension to consider only after the fixed
//! cost has an adequacy-tested implementation.
//!
//! # Totality
//!
//! Nothing in this crate panics, diverges, or truncates. Construction and
//! rendering are metered against explicit limits, every arithmetic step is
//! checked, every store checks its limit and then reserves fallibly, and a
//! failure returns a typed error rather than partial output. A document too
//! wide for its computation width is reported as width-tainted and is still
//! rendered in full; taint marks a layout as outside the optimality theorem,
//! never as a candidate to cut short.

pub mod arena;
pub mod build;
pub mod error;
pub mod limits;
pub mod measure;
pub mod plan;
pub mod render;
pub mod resolve;
pub mod taint;
pub mod units;
pub use error::RenderError;
pub use limits::RenderLimits;
pub use limits::RenderMeter;
pub use limits::RenderUsage;
pub use limits::vm;
pub use measure::LayoutCost;
pub use measure::LayoutOptions;
pub use measure::PhysicalLineEnding;
pub use measure::WidthTaint;
pub use plan::PlanId;
pub use resolve::Resolved;
pub use resolve::resolve;
pub use units::ComputationWidth;
pub use units::PageWidth;
