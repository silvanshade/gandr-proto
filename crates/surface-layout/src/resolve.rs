//! Memoized iterative resolution. **Slice two owns this module; it carries no
//! code yet.**
//!
//! Resolution is a dynamic program over the document graph. Only in-bound
//! states are memoized, under the key
//!
//! ```text
//! (DocId, Column, Indentation)
//! ```
//!
//! with the layout options constant for one invocation and both numeric fields
//! at most the computation width. The handle includes the arena key; an
//! implementation may store the private node identity once that key has been
//! validated. Flattening is already represented in the finalized arena, so
//! flatten mode is not a further key dimension.
//!
//! A shared handle reached again in the **same** column and indentation
//! consumes one state and reuses its retained frontier. The same handle in a
//! different context consumes a distinct state, because unaligned concatenation
//! makes that result observably different — this is the whole reason the
//! context is in the key.
//!
//! # Iterative, and not by preference
//!
//! The resolver is an explicit enter-and-resume work machine over every node
//! form, and it never calls itself. A document's depth is caller-supplied
//! input, so native stack depth is not a resource this crate may spend on it.
//! The resolver owns exactly one private work vector for state evaluation,
//! child continuation, frontier merging, and plan retain and release. Before
//! **every** push it checked-increments the cumulative work-entry counter,
//! checks the stack ceiling against the resulting live length, and reserves
//! fallibly against the resolver-stack allocation site. A pop never refunds the
//! cumulative counter.
//!
//! No callback, helper, frontier operation, plan-release traversal, or memo
//! miss may allocate a second unmetered work vector or place an unmetered entry
//! on another stack. That rule is what makes the accounting a bound rather than
//! an estimate.
//!
//! # Charging
//!
//! Each state creation charges a memo state. Every entered or resumed node,
//! every traversed edge including a second edge to a shared handle, every
//! frontier comparison and insertion, every plan retain and release, and every
//! forced-promise step charges one layout step. So construction charges each
//! stored node and byte once, while layout charges the context-sensitive work
//! actually done, without ever re-allocating the shared document.
//!
//! # Complexity, before the budgets stop the work
//!
//! For `n` finalized nodes and computation width `W`:
//!
//! - flatten finalization is linear in `n` and adds at most `O(n)` nodes;
//! - full unaligned resolution is `O(n W^4)` time;
//! - memo and frontier storage is `O(n W^3)`;
//! - the aligned-only subset would be `O(n W^3)` time, and this engine
//!   implements the full unaligned bound rather than the subset;
//! - emission is linear in output bytes, one charged instruction per popped
//!   plan node.
//!
//! Bytes are stored once per arena identity, scratch frontier buffers are
//! reused, and no layout enumeration and no per-candidate string is ever
//! materialized. The exhaustive oracle that checks this resolver is test-only
//! and may enumerate layouts for deliberately small documents; nothing that
//! ships enumerates anything.
