//! The incremental typing pipeline for the gandr language (milestone A2,
//! `docs/gandr/spec/roadmap.md`; design:
//! `docs/gandr/spec/incremental-pipeline.md`).
//!
//! This crate implements the pipeline's first boxes:
//!
//! - **A2.1 — CST → core lowering** over the Stage-1-typeable fragment, with
//!   source identity kept in a side table ([`origin::OriginMap`], decision D4 —
//!   `gandr-core-checker` syntax stays span-free and parser-free).
//! - **A2.2 — total lowering and goals**: [`lower::lower_source_total`] lowers
//!   *every* parseable input — syntax errors and out-of-fragment constructs
//!   become holes carrying [`origin::HoleNote`]s (D5; pipeline spec §7) — and
//!   [`goals::goals_report`] lists every hole with its span, expected type, and
//!   local `Γ` (the v0 agent-stream seed).
//! - **A2.4 — diagnostics and goals surface**: [`diag::report`] maps typing
//!   failures (`FailureState` + `TypeError` + the [`origin::OriginMap`]) and
//!   hole goals into one versioned, serde-JSON [`diag::Report`] — the v0 of the
//!   agent stream (D7).
//! - **Entity attributes** ([`attributes`]): the `@[…]` marker's registry,
//!   payload checker path (iterative, the attribute contract), and inert side
//!   table, projected into [`diag::Report::attributes`]
//!   (proposal-attributes.md). Hash-neutral — an inert attribute never enters
//!   an item's core-IR term.
//! - **A2.3 — dependency-validated checkpoints** ([`footprint`],
//!   [`checkpoint`]): [`footprint::footprint_of`] captures the dependency
//!   footprint of a lowered item — the context names its term read
//!   (`incremental-pipeline.md` §4 at item granularity) — and [`checkpoint`]
//!   layers a changed-region → dirty-frontier incremental typer over the
//!   melder-based lower path: it diffs the edited item list against a base
//!   [`checkpoint::Checkpoints`], validates each unchanged item's footprint
//!   against the bindings the edit changed, and *adopts* (reuses) or *re-types*
//!   accordingly — the §5 validated resume. The gate is the differential
//!   `resume(base, edited) == checkpoint_source(edited)` (`tests/incremental`).
//!   Edit-surviving item identity and the frontier order run on
//!   [`gandr_theory_orders`] (§7 Porter disposition).
//! - **Edit-action reconstruction** ([`edit`]): the localized structured diff
//!   of two lowerings — the Porter/Pantograph "edit-action" both consume but
//!   leave out of scope (`incremental-pipeline.md` §7). A
//!   foundation-independent A2 brick: it needs neither the checkpoint base
//!   (A2.3) nor the solver, so it lands ahead of them as the seam they will
//!   consume.
//!
//! Later rungs add the streaming driver (A2.5) and the per-term-node,
//! solver-coupled checkpoint granularity above the item-level A2.3 base here.
//!
//! Entry points: [`lower::lower_source`] (strict, A2.1) and
//! [`lower::lower_source_total`] (total, A2.2). Lowering is syntax-directed
//! and total-or-structured-error ([`lower::LowerError`]); see [`lower`] for
//! the covered fragment, the recorded sort-mediation decisions, and the
//! strict-error → hole conversion table.
//!
//! [`prelude`] provides the typing context ([`prelude::prelude_ctx`]) and the
//! eval binding-environment ([`prelude::prelude_env`]) for the operators and
//! the module-qualified native builtins that elaboration targets (the design
//! record). [`host`] provides the canonical host effect signatures (`Exec` /
//! `Fs` / `Env` / `Proc`) and the reserved host modules (`fs` / `env` / `proc`)
//! whose member calls elaborate to performs against them.

#![cfg_attr(
    test,
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "unit-test modules share fixture helpers (tree, field, item0, lowered) that are called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair"
    )
)]

extern crate alloc;

pub mod attributes;
pub mod boundary;
#[cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the worklist modules place their public drivers before the step helpers for readability; the caller-before-callee rule conflicts with that deliberate top-down layout pending a layout redesign"
    )
)]
pub mod checkpoint;
pub mod desc_elab;
#[cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the worklist modules place their public drivers before the step helpers for readability; the caller-before-callee rule conflicts with that deliberate top-down layout pending a layout redesign"
    )
)]
pub mod diag;
#[cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the worklist modules place their public drivers before the step helpers for readability; the caller-before-callee rule conflicts with that deliberate top-down layout pending a layout redesign"
    )
)]
pub mod edit;
pub mod ffi;
#[cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the worklist modules place their public drivers before the step helpers for readability; the caller-before-callee rule conflicts with that deliberate top-down layout pending a layout redesign"
    )
)]
pub mod footprint;
#[cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the worklist modules place their public drivers before the step helpers for readability; the caller-before-callee rule conflicts with that deliberate top-down layout pending a layout redesign"
    )
)]
pub mod goals;
#[cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the worklist modules place their public drivers before the step helpers for readability; the caller-before-callee rule conflicts with that deliberate top-down layout pending a layout redesign"
    )
)]
pub mod host;
pub mod link;
pub mod lower;
pub mod origin;
pub mod prelude;
pub mod render;
pub mod run;
pub mod session;
pub mod synnode;

pub use crate::prelude::prelude_ctx;
pub use crate::prelude::prelude_env;
