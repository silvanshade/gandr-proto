//! Prices a general dependency-tracking engine against the hand-rolled
//! validated resume, at the granularity of the top-level item.
//!
//! # What this crate is, and is not
//!
//! **This crate is an assessment instrument. Nothing in the shipping tree
//! depends on it, and nothing should.** It exists to answer one question with
//! numbers instead of argument: *would a general on-demand query engine pay for
//! gandr's item-granular incremental typing, and what would adopting one cost?*
//! Its output is a measurement table and a recommendation; the decision those
//! serve is taken elsewhere.
//!
//! It is therefore the **only** crate in the workspace that names the engine
//! under assessment. [`manifests`] makes that a checked property rather than an
//! intention: the engine is confined here, and if a later change lets it reach
//! a second manifest, the crate's own suite fails.
//!
//! # The granularity frame
//!
//! The assessment runs at **item granularity and coarser, never below it.** A
//! general engine carries per-query bookkeeping that a per-node judgement
//! cannot amortize — the measured finding of the incremental type-checking
//! literature this frame is taken from — so a node-grained engine is out of
//! scope by rule, not by oversight. The top-level item is the coarsest unit at
//! which gandr's reuse decision is actually made, which makes it the unit where
//! an engine has a chance of paying.
//!
//! # What is measured
//!
//! Two paths compute the same thing and are compared on the same programs:
//!
//! - the **baseline** ([`baseline`]) —
//!   [`gandr_core_incremental::checkpoint::resume_with`], the hand-rolled
//!   validated resume as built;
//! - the **engine path** ([`engine`]) — the same item typing expressed as
//!   memoized queries over a database keyed by item identity, with each item's
//!   dependency footprint projected into its query's declared dependencies.
//!
//! **The recheck-traversal floor is a property of the demand, not of the
//! engine**, which is why the comparison is run under two demand shapes
//! ([`measure::DemandShape`]): asking for every item's typing, which is what
//! the baseline's signature always produces, and asking for one item's typing,
//! which the baseline cannot express at all. Reporting only the first would
//! price the engine on the workload least able to show what it buys; reporting
//! only the second would compare against a capability the baseline never
//! claimed.
//!
//! # Why the counters are assertions
//!
//! A comparison whose inputs never reach the code under test is green and
//! worthless. Every measurement here is therefore accompanied by a **work
//! count** ([`ledger`]) asserted against the workload's known structure — how
//! many query bodies executed, how many memos were reused, how many items were
//! visited. The sharpest of these is the backdating assertion: the engine's
//! whole advantage on a value-only edit rests on an equal recomputed binding
//! stopping the invalidation wave, and a configuration that silently disables
//! equality cutoff produces a plausible-looking cost table with the headline
//! mechanism dead. [`measure`] asserts the wave stops.
//!
//! # The ownership boundary
//!
//! gandr's core terms and types are reference-counted ([`alloc::rc::Rc`]) and
//! so are neither [`Send`] nor [`Sync`]; the engine requires both of every
//! value it retains. Every value crossing into the query graph is therefore
//! encoded through the checkpoint codec `gandr-core-incremental` already ships
//! for persistence, and decoded on the way out. That encode/decode traffic is
//! not an implementation detail of this harness — **it is the adoption cost,
//! measured** ([`ledger::Ledger::boundary_bytes`]).
//!
//! # Module map
//!
//! - [`boundary`] — the semantic wrappers these signatures carry in place of
//!   bare primitives.
//! - [`workload`] — the generated programs and the edit vocabulary applied to
//!   them, shaped so that every dirty set is known in advance and can be
//!   asserted rather than observed.
//! - [`ledger`] — the work counters and the typed failures the harness reports.
//! - [`arena`] — the out-of-database item store the query graph reads through,
//!   the engine's documented on-demand-input pattern adapted to values that
//!   cannot enter the database.
//! - [`engine`] — the database, its inputs, and the tracked queries: the model
//!   under assessment.
//! - [`baseline`] — the hand-rolled path's measured recheck.
//! - [`measure`] — the runner that produces the comparison rows, and the table
//!   they are reported as.
//! - [`manifests`] — the engine-confinement check.

extern crate alloc;

pub mod arena;
pub mod baseline;
pub mod boundary;
pub mod engine;
pub mod ledger;
pub mod manifests;
pub mod measure;
pub mod workload;
