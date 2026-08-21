//! Render plans. **Slice three owns this module; it carries no code yet.**
//!
//! The paper this engine follows fuses resolution and rendering by carrying a
//! token function inside each measure. Rust defunctionalizes that function into
//! data, which is what makes a plan inspectable, bounded, and releasable.
//!
//! # The shapes
//!
//! ```text
//! struct PlanId {
//!     slot: PlanSlot,
//!     generation: PlanGeneration,
//! }
//!
//! enum PlanNode {
//!     Empty,
//!     Text(TextId),
//!     Verbatim(VerbatimId),
//!     Newline {
//!         indentation: Indentation,
//!         ending: PhysicalLineEnding,
//!     },
//!     Seq {
//!         left: PlanId,
//!         right: PlanId,
//!     },
//! }
//! ```
//!
//! # Retention
//!
//! The plan arena is reference-counted and generational. A candidate first
//! computes its cost, its ending column, and a stack-local recipe; it allocates
//! a plan node only when the candidate is actually inserted into a frontier or
//! retained by a taint promise. So a candidate that loses never costs plan
//! storage at all.
//!
//! When dominance pruning removes a measure, release records are pushed onto
//! the same private lifetime-bound resolver work vector. Every push charges a
//! resolver work entry, capacity growth is charged to the resolver-stack
//! allocation site, and each popped record decrements plan references and
//! recycles unreachable slots iteratively — never by a recursive drop, which on
//! a deep plan would be the stack overflow this crate spends its whole design
//! avoiding.
//!
//! Memo entries retain their frontier plans and the root retains the winner, so
//! live plan storage is bounded by retained structure rather than by rejected
//! garbage. The live-plan ceiling bounds simultaneous storage and the monotone
//! created-node ceiling stops a sequence of segments from resetting the
//! allocation work it has already done.
//!
//! The generation field is what makes a stale identity refusable rather than
//! silently wrong after a slot is recycled.
