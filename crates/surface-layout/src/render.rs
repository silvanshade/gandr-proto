//! The render entry point and its budgets. **Slice three owns this module; it
//! carries no code yet.**
//!
//! # The shapes
//!
//! ```text
//! pub enum PhysicalLineEnding {
//!     Lf,
//!     CrLf,
//! }
//!
//! pub struct RenderLimits {
//!     pub max_memo_states: MaxMemoStates,
//!     pub max_frontier_entries: MaxFrontierEntries,
//!     pub max_plan_nodes_created: MaxPlanNodesCreated,
//!     pub max_live_plan_nodes: MaxLivePlanNodes,
//!     pub max_output_bytes: MaxOutputBytes,
//!     pub max_layout_steps: MaxLayoutSteps,
//!     pub max_resolver_work_entries: MaxResolverWorkEntries,
//!     pub max_resolver_stack: MaxResolverStack,
//!     pub max_vm_steps: MaxVmSteps,
//!     pub max_vm_stack: MaxVmStack,
//! }
//!
//! pub struct RenderUsage {
//!     pub memo_states: MemoStatesUsed,
//!     pub frontier_entries: FrontierEntriesUsed,
//!     pub plan_nodes_created: PlanNodesCreated,
//!     pub peak_live_plan_nodes: PeakLivePlanNodes,
//!     pub output_bytes: OutputBytesUsed,
//!     pub layout_steps: LayoutStepsUsed,
//!     pub resolver_work_entries: ResolverWorkEntriesUsed,
//!     pub peak_resolver_stack: PeakResolverStack,
//!     pub vm_steps: VmStepsUsed,
//!     pub peak_vm_stack: PeakVmStack,
//! }
//!
//! pub struct RenderMeter {
//!     limits: RenderLimits,
//!     used: RenderUsage,
//! }
//!
//! pub struct Rendered {
//!     pub text: RenderedText,
//!     pub cost: LayoutCost,
//!     pub width_tainted: WidthTaint,
//! }
//!
//! impl RenderMeter {
//!     pub fn try_new(limits: RenderLimits) -> Result<Self, RenderError>;
//!     pub fn usage(&self) -> RenderUsage;
//! }
//!
//! pub fn render(
//!     arena: &DocArena,
//!     root: DocId,
//!     options: &LayoutOptions,
//!     meter: &mut RenderMeter,
//! ) -> Result<Rendered, RenderError>;
//! ```
//!
//! `WidthTaint` is a two-valued nominal enum rather than a `bool`.
//!
//! # The binding defaults
//!
//! | limit                                        | default                 |
//! | -------------------------------------------- | ----------------------- |
//! | handle, column, and indentation memo states  | 1,000,000               |
//! | frontier entries retained across memo states | 4,000,000               |
//! | plan nodes created, and peak live            | 16,000,000 / 8,000,000  |
//! | output bytes                                 | 64 MiB                  |
//! | layout transitions, edges, frontier steps    | 100,000,000             |
//! | resolver work entries, and peak stack        | 100,000,000 / 1,000,000 |
//! | render machine instructions                  | 100,000,000             |
//! | render machine stack entries                 | 1,000,000               |
//!
//! # The render error space, closed
//!
//! `RenderLimitKind` is exactly memo states, frontier entries, plan nodes
//! created, live plan nodes, output bytes, layout steps, resolver work entries,
//! resolver stack, machine steps, and machine stack.
//!
//! `RenderAllocationSite` is exactly the memo table, the frontier, the plan
//! arena, the resolver stack, the machine stack, and the output.
//!
//! `RenderArithmetic` is exactly column, indentation, squared overflow, line
//! breaks, output bytes, the step counter, the resolver work counter, and the
//! plan reference count.
//!
//! `RenderError` distinguishes an unknown handle, an invalid width, a checked
//! arithmetic overflow naming its operation, an allocation failure naming its
//! site, and a limit exceeded naming its kind and that ceiling. There is no
//! generic step budget and no unreachable one.
//!
//! # Physical endings
//!
//! Line and hard-line nodes emit exactly the configured ending, which
//! contributes one or two bytes to output accounting and zero columns. Verbatim
//! text always emits its own stored endings, so a mixed or comment-internal
//! ending is not rewritten by the caller's choice. There is no global
//! post-processing pass that converts endings, because such a pass would have
//! to reach inside verbatim bytes to work and must not.
//!
//! # Metering discipline
//!
//! Render meter fields are private, the meter is neither `Clone` nor `Default`,
//! and nothing resets or decrements a cumulative counter. Each render call
//! mutably borrows one meter. A standing client creates one build meter and one
//! render meter per output; a segmented client reuses the same pair across
//! every segment. Live-plan, resolver-stack, and machine-stack gauges fall as
//! work is released, while their peak observations and every other counter stay
//! monotone.
