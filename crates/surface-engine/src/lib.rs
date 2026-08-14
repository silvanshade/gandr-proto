//! The surface engine for the gandr language: the CST-to-core front end the
//! incremental typing pipeline
//! (`spec:implementation/incremental-pipeline.md`) runs on.
//!
//! The pipeline faces this crate implements:
//!
//! - **CST → core lowering** over the covered fragment, with source identity
//!   kept in a side table ([`origin::OriginMap`]) — `gandr-core-checker` syntax
//!   stays span-free and parser-free by decision, so positions live here and
//!   never in the core.
//! - **Import namespace lowering**: `import "URI" as name ;` is retained in
//!   source order and its `as` clause runs through
//!   [`namespace::Modifier::alias_as`] into the lowering's visible
//!   [`namespace::Scope`]. This boundary records imports but performs no
//!   resolution, fetch, or runnable-item synthesis.
//! - **Total lowering and goals**: [`lower::lower_source_total`] lowers *every*
//!   parseable input — syntax errors and out-of-fragment constructs become
//!   holes carrying [`origin::HoleNote`]s (the pipeline spec's §"Holes": a hole
//!   is a term with a typing rule, not a parse failure with a placeholder) —
//!   and [`goals::goals_report`] lists every hole with its span, expected type,
//!   and local `Γ`.
//! - **The diagnostics and goals surface**: [`diag::report`] maps typing
//!   failures (`FailureState` + `TypeError` + the [`origin::OriginMap`]) and
//!   hole goals into one versioned, serde-JSON [`diag::Report`] — the report
//!   envelope the inspection surface
//!   (`spec:implementation/inspection-protocol.md`) projects from.
//! - **Entity attributes** ([`attributes`]): the `@[…]` marker's registry,
//!   payload checker path (iterative, the attribute contract), and inert side
//!   table, projected into [`diag::Report::attributes`]
//!   (`spec:surface-language/attributes.md`). Hash-neutral — an inert attribute
//!   never enters an item's core-IR term.
//! - **The parser-agnostic item seam** ([`item_source`]): the melder-and-
//!   lowering front end as an implementation of
//!   [`gandr_core_incremental::region::ItemSource`], so the item-granular
//!   incremental typer can be driven against real surface source without
//!   depending on this crate or naming a parser. A lowering already carries
//!   that crate's [`gandr_core_incremental::region::Item`], so crossing the
//!   seam drops the surface faces (origins, attributes, declaration tables) and
//!   projects nothing else. `tests/incremental` resumes over real source
//!   through the seam against the surface prelude, the differential gate's
//!   front-end half.
//! - **Kernel admission** ([`kernel`]): the engine's crossing from checked core
//!   into the certified kernel. A [`session::Session`] offers every typed
//!   definition to [`gandr_core_checker::kernel_bridge`] and the kernel's
//!   `add_decl` choke point, accumulating the definitions that lower into the
//!   closed S1 vocabulary as one [`kernel::KernelAdmissions`] environment and
//!   reporting one [`kernel::KernelVerdict`] per item. A session is the first
//!   consumer to populate the bridge's naming environment, so one definition
//!   can reach another through a kernel constant. The crossing observes and
//!   never decides: typing and evaluation are complete without it.
//! - **Edit-action reconstruction** ([`edit`]): the localized structured diff
//!   of two lowerings — the "edit-action" the incremental-typing and
//!   structure-editor literature both consume but leave out of scope
//!   (`spec:implementation/incremental-pipeline.md` §"pipeline-decision-02" and
//!   §"pipeline-decision-04"). It needs neither the checkpoint engine nor a
//!   solver, so it stands as the seam they consume.
//!
//! The streaming driver is designed and not built; `gandr-core-incremental`
//! owns the item-granular checkpoint engine, and per-term-node solver-coupled
//! granularity above its item-level base is likewise designed direction.
//!
//! Entry points: [`lower::lower_source`] (strict) and
//! [`lower::lower_source_total`] (total). Lowering is syntax-directed
//! and total-or-structured-error ([`lower::LowerError`]); see [`lower`] for
//! the covered fragment, the recorded sort-mediation decisions, and the
//! strict-error → hole conversion table.
//!
//! [`prelude`] provides the typing context ([`prelude::prelude_ctx`]) and the
//! eval binding-environment ([`prelude::prelude_env`]) for the operators and
//! the module-qualified native builtins that elaboration targets.
//! [`host`] provides the canonical host effect signatures (`Exec` / `Fs` /
//! `Env` / `Proc`) and the reserved host modules (`fs` / `env` / `proc`)
//! whose member calls elaborate to performs against them.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "unit-test modules share fixture helpers (tree, field, item0, lowered) that are called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair"
    )
)]

extern crate alloc;

pub mod attributes;
pub mod boundary;
pub mod circuit;
pub(crate) mod cst_read;
pub mod desc_cells;
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
pub mod item_source;
pub mod kernel;
pub mod link;
pub mod lower;
pub mod namespace;
pub mod origin;
pub mod prelude;
pub mod render;
pub mod run;
pub mod session;
pub mod synnode;

pub use crate::prelude::prelude_ctx;
pub use crate::prelude::prelude_env;
