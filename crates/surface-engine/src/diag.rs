//! Diagnostics and goals as a JSON agent stream (`A2-PLAN.md` §A2.4,
//! decision D7).
//!
//! This is the **v0 of the agent stream** (`VISION.md` §4: marks +
//! obligations + goals). It maps a typing failure
//! ([`gandr_core_checker::machine::FailureState`] +
//! [`gandr_core_checker::error::TypeError`]) together with the source identity
//! in an [`OriginMap`](crate::origin) to a structured, source-ranged
//! [`Diagnostic`], and carries the hole **goals** (`A2-PLAN.md` §A2.2,
//! [`crate::goals`]) in the same versioned [`Report`] envelope. The
//! incremental-pipeline design *marks* slot is now **populated** ([`marks`],
//! `semantic-marks work`): one [`MarkReport`] per node mark from the total
//! semantic marking layer ([`gandr_core_checker::mark`], `semantic marker`),
//! source-ranged through the same `OriginMap`. The *obligations* slot stays
//! reserved (empty) so the schema is forward-compatible.
//!
//! The marks and the diagnostics are **complementary** realizations of the same
//! type system: the diagnostics are the machine's fail-fast derivation view
//! (first failure per item, with the partial-derivation `context_chain`); the
//! marks are the marker's *total* per-node decoration (every node, every type
//! error localized and recovered). They detect the same type errors, but their
//! spans and some kind labels differ — diagnostics fall back to the enclosing
//! item's span for `Return`-position failures, and the marker specializes some
//! kinds (e.g. `Thunkability` vs the diagnostic's `GradeError`).
//!
//! # serde placement (decision tree)
//!
//! [SPECULATIVE DECISION, D7 under-specification.] D7 says only "output is
//! serde-JSON"; it does not say *where* the serde derives live. The plan
//! reserves a `serde` feature on `gandr-core-checker` for **A2.3**
//! (checkpointing of the machine `State`). To keep A2.4 parallel-safe with A2.3
//! (the plan's §3 requirement) and to keep `gandr-core-checker` dependency-free
//! and WASM-portable (decision D3's "core stays parser-free / minimal
//! dependency surface"), the JSON types live **here**, as mirror structs over
//! the core's `FailureState` / `TypeError` / [`Mark`] / [`Goal`] data — *not*
//! as `Serialize` derives on the core types. A2.3 can add its `State` serde
//! feature without colliding
//! with anything in this module. Rejected alternative: derive `Serialize` on
//! the core error/type enums now — it pulls `serde` into `gandr-core-checker`
//! early, couples the two rungs, and bakes the wire format into the
//! verification anchor.
//!
//! # Rendering (decision tree)
//!
//! [SPECULATIVE DECISION.] Types, terms, and grades are rendered to strings
//! via their derived [`core::fmt::Debug`] — the same choice
//! [`OriginMap::snapshot`](crate::origin::OriginMap::snapshot) and the goals
//! golden tests already make. This is deterministic (golden-stable), faithful
//! to the core representation, and respects grade-carrier boundary's
//! grade-carrier seal (it does not *match* the `Grade` representation outside
//! `grade.rs`). A surface-syntax pretty-printer is a forward enhancement,
//! deferred to avoid duplicating — and risking divergence from — the lowerer's
//! spelling decisions.
//!
//! # Span resolution (decision tree)
//!
//! [SPECULATIVE DECISION, D4 reversal-trigger (b) noted, not fired.] The
//! `OriginMap` stores CST origins by stable origin IDs and exposes legacy path
//! readback for diagnostics; a [`FailureState`] carries the offending sub-term
//! in its control register but **no path**. For a `Descend`-position failure
//! (the offending term is in the control register) the precise sub-node span is
//! recovered by a structural match of the offending term against the recorded
//! origin nodes of its item, in compatibility path order — so
//! `UnboundVariable`, every `StuckExpr`, the axiom `TypeMismatch`, and the
//! `Descend`-site `GradeError` get exact sub-node spans (with the elaboration
//! tag, satisfying D7's "elaborated-node failures report the surface range with
//! the elaboration noted"). For a `Return`-position failure (the control
//! register is a *type*, the failing frame is on the stack) the diagnostic
//! falls back to the **enclosing item's** span — always within the source — and
//! relies on the [`Diagnostic::context_chain`] (the partial derivation) to
//! localize the error structurally. Structurally-identical sibling sub-terms
//! resolve to the first in path order; this is a known v0 imprecision, not a
//! soundness issue (every reported span lies within the source). D4's reversal
//! trigger — "diagnostics need spans the origin map cannot address" — is
//! therefore **not** fired: the item-span fallback keeps every range valid.

use core::ops::Range;

use gandr_core_checker::control::Control;
use gandr_core_checker::control::Dir;
use gandr_core_checker::ctx::Ctx;
use gandr_core_checker::error::TypeError;
use gandr_core_checker::machine::FailureState;
use gandr_core_checker::machine::Frame;
use gandr_core_checker::machine::Outcome;
use gandr_core_checker::machine::step;
use gandr_core_checker::mark::Mark;
use gandr_core_checker::mark::Marking;
use gandr_core_checker::mark::mark_comp;
use gandr_core_checker::mark::mark_value;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::CompType;
use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;
use gandr_core_incrementality::region::Item;

use crate::attributes;
use crate::boundary::AttributeName;
use crate::boundary::ContextLength;
use crate::boundary::ContextRole;
use crate::boundary::DataMention;
use crate::boundary::ItemIndex;
use crate::boundary::RecursiveMarkDepthExceeded;
use crate::boundary::SourceRange;
use crate::goals::Goal;
use crate::goals::goal_item_flags;
use crate::goals::goals_report_with_contexts;
use crate::goals::initial_state;
use crate::lower::Lowered;
use crate::origin::TermRef;
use crate::origin::resolve;
use crate::render;

/// The schema version of the [`Report`] envelope.
///
/// Bumped whenever a field is renamed, removed, or changes meaning (additive
/// fields do not require a bump). Consumers must check it before parsing.
///
/// `2` — the reserved `marks` slot changed from an opaque `serde_json::Value`
/// array to a typed [`MarkReport`] array (`semantic-marks work`); a meaning
/// change, hence a bump.
pub const SCHEMA_VERSION: u32 = 2;

/// A byte span `[start, end)` in the source.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct Span
{
    /// The inclusive start byte offset.
    pub start: usize,
    /// The exclusive end byte offset.
    pub end: usize,
}

impl From<Range<usize>> for Span
{
    #[inline]
    fn from(range: Range<usize>) -> Self
    {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<SourceRange> for Span
{
    #[inline]
    fn from(range: SourceRange) -> Self
    {
        Self::from(range.0)
    }
}

/// One typing-context hypothesis `name : type`, rendered for the agent
/// stream.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct Binding
{
    /// The bound name.
    pub name: String,
    /// The bound type, rendered (see the module doc's rendering decision).
    pub ty: String,
}

/// The severity of a [`Diagnostic`].
///
/// Only [`Severity::Error`] occurs at Stage 1 (every [`TypeError`] is fatal);
/// the enum reserves room for warnings/hints as later stages add
/// non-fatal marks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "codecs", serde(rename_all = "lowercase"))]
pub enum Severity
{
    /// A fatal typing error.
    Error,
}

/// The kind-specific payload of a [`Diagnostic`], tagged by `kind` with the
/// variant data under `data`.
///
/// One variant per *reachable* [`TypeError`] constructor. The machine's two
/// polarity-guard `ShapeMismatch` descriptions
/// ([`text::SHAPE_VALUE`](gandr_core_checker::error::text::SHAPE_VALUE) /
/// `SHAPE_COMP`) are unreachable by construction (`error.rs` module doc; a
/// conformance meta-test pins this), so they need no dedicated shape here —
/// they would arrive as an ordinary [`Self::ShapeMismatch`] if they ever did.
/// [`Self::Other`] is the reserved catch-all for an upstream variant this
/// mirror does not model: unconstructable today (the [`detail_of`] match is
/// total — an added upstream variant is a compile-visible change there) and
/// retained so the serde wire shape stays stable.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "codecs", serde(tag = "kind", content = "data"))]
pub enum DiagnosticDetail
{
    /// Subsumption failed: the term's type is not consistent with the
    /// expected type.
    TypeMismatch
    {
        /// The expected (checked-against) type, rendered.
        expected: String,
        /// The type the term actually has, rendered.
        actual: String,
    },
    /// An elimination's principal premise inferred a type of the wrong shape.
    ShapeMismatch
    {
        /// A description of the expected type constructor (e.g. "an arrow
        /// type").
        expected_shape: String,
        /// The type actually inferred, rendered.
        actual: String,
    },
    /// No typing rule applies to the term in this direction.
    StuckExpr
    {
        /// A hint for making progress (e.g. "annotate this injection").
        hint: String,
    },
    /// A variable was used with no hypothesis in scope.
    UnboundVariable
    {
        /// The variable's name.
        name: String,
    },
    /// A grade-order requirement `lower ⊑ upper` failed.
    GradeError
    {
        /// The left-hand side of the failed `⊑`, rendered.
        lower: String,
        /// The right-hand side of the failed `⊑`, rendered.
        upper: String,
    },
    /// An attribute-specific failure (proposal-attributes.md §3.2): an unknown
    /// name, a single-valued duplicate, a missing payload, or a non-value
    /// payload. An *ill-typed* payload is not here — it is the ordinary
    /// [`Self::TypeMismatch`] of the record/scalar/list rules, surfaced at the
    /// payload node.
    Attribute
    {
        /// The offending attribute's name.
        name: String,
        /// What is wrong with it.
        problem: AttributeProblem,
    },
    /// A forward-compatible catch-all for a future [`TypeError`] variant; the
    /// human-readable [`Diagnostic::message`] still describes it.
    Other,
}

/// The kind of an attribute-specific diagnostic
/// ([`DiagnosticDetail::Attribute`]; proposal-attributes.md §3.2).
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub enum AttributeProblem
{
    /// The name resolves to no registry entry, with a did-you-mean over the
    /// registry (`UnknownAttribute`).
    Unknown
    {
        /// The nearest registry name, when one is close enough.
        #[cfg_attr(
            feature = "codecs",
            serde(default, skip_serializing_if = "Option::is_none")
        )]
        suggestion: Option<String>,
    },
    /// A single-valued attribute repeated on one entity (`DuplicateAttribute`).
    Duplicate,
    /// A schema that requires a payload was written as a bare marker.
    MissingPayload,
    /// The payload is a computation, not a value (attribute purity is locality,
    /// §3.3).
    NonValuePayload,
}

/// One frame of the partial derivation, rendered as agent-facing context
/// (D7's `context_chain`).
///
/// The chain is the failure frame stack in **outermost-first** order (the
/// reading "while checking the body of `square` … while checking the argument
/// of …"): the first entry is the outermost pending obligation, the last is
/// the frame the failure occurred under.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct ContextFrame
{
    /// The machine frame's name (e.g. `AppFn`), for stable machine parsing.
    pub role: String,
    /// A prose description of the pending obligation.
    pub prose: String,
    /// The frame's binder name, when it carries one (abstractions, binds).
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub binder: Option<String>,
}

/// One source-ranged diagnostic: a [`TypeError`] mapped through its
/// [`FailureState`] and the [`OriginMap`](crate::origin).
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct Diagnostic
{
    /// The kind-specific payload (`kind` + `data`).
    #[cfg_attr(feature = "codecs", serde(flatten))]
    pub detail: DiagnosticDetail,
    /// The severity (always [`Severity::Error`] at Stage 1).
    pub severity: Severity,
    /// The human-readable message — the [`TypeError`]'s `Display`.
    pub message: String,
    /// The source span, where resolvable (see the module doc's span
    /// decision); a precise sub-node span for `Descend` failures, else the
    /// enclosing item's span.
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub span: Option<Span>,
    /// The elaboration tag of the span's origin node, when it is a
    /// synthesized node (`def` sugar, operator elaboration, …).
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub elaboration: Option<String>,
    /// The offending sub-term, rendered, for a `Descend`-position failure;
    /// absent for a `Return`-position failure (the control register is a
    /// type, not a term — the [`Self::context_chain`] localizes it instead).
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub expr: Option<String>,
    /// The partial derivation as a context chain (outermost first).
    pub context_chain: Vec<ContextFrame>,
    /// `Γ` at the failure point, beyond the caller's base context (the base —
    /// e.g. the prelude — is implied, as in the goals report).
    pub ctx: Vec<Binding>,
}

/// One hole goal, rendered for the agent stream (the JSON image of
/// [`Goal`]).
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct GoalReport
{
    /// The hole's identifier (unique within one [`Lowered`]).
    pub hole: u32,
    /// The index of the item containing the hole.
    pub item: usize,
    /// The elided region's source span.
    pub span: Span,
    /// What was elided, rendered (the [`HoleNote`](crate::origin::HoleNote)).
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub note: Option<String>,
    /// The expected type at the hole, rendered, when checking-position and
    /// reached by the machine; absent for inference-position or unreached
    /// holes.
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub expected: Option<String>,
    /// The bindings local to the hole (beyond the base context), when the
    /// machine reached it.
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub ctx_local: Option<Vec<Binding>>,
}

/// The kind-specific payload of a [`MarkReport`], tagged by `kind` with the
/// variant data under `data` — the serde mirror of [`Mark`].
///
/// The core stays serde-free (decision D3), so the wire image of every
/// [`Mark`] kind lives here, exactly as [`DiagnosticDetail`] mirrors
/// [`TypeError`]. Types and grades are rendered to strings via
/// [`core::fmt::Debug`] (the module's rendering decision). [`Self::Other`] is
/// the reserved catch-all for a [`Mark`] kind this mirror does not model:
/// unconstructable today (the [`mark_detail`] match is total — an added
/// upstream kind is a compile-visible change there) and retained so the serde
/// wire shape stays stable. The [`MarkReport::is_error`] flag (read from
/// [`Mark::is_error`]) still classifies such a mark authoritatively.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "codecs", serde(tag = "kind", content = "data"))]
pub enum MarkDetail
{
    /// An empty hole `?u` — a complete-but-incomplete node, **not** an error
    /// ([`Mark::EmptyHole`]).
    EmptyHole
    {
        /// The hole's identifier.
        hole: u32,
    },
    /// Subsumption failed: the synthesized type is not a consistent subtype of
    /// the analyzed type (the typed error-boundary `{t}_{actual ⇐ expected}`;
    /// [`Mark::TypeMismatch`]).
    TypeMismatch
    {
        /// The expected (analyzed) type, rendered.
        expected: String,
        /// The synthesized type the node produced, rendered.
        actual: String,
    },
    /// An effect-row mismatch: consistent returner payloads, but the
    /// synthesized row is not a subset of the expected row
    /// ([`Mark::EffectRowMismatch`]).
    EffectRowMismatch
    {
        /// The expected returner type `F^ε′ A`, rendered.
        expected: String,
        /// The synthesized returner type `F^ε A`, rendered.
        actual: String,
    },
    /// An elimination's principal premise synthesized a type of the wrong
    /// shape ([`Mark::ShapeMismatch`]).
    ShapeMismatch
    {
        /// A description of the expected type constructor (e.g. "an arrow
        /// type").
        expected_shape: String,
        /// The type the principal premise actually synthesized, rendered.
        actual: String,
    },
    /// A variable used with no hypothesis in scope ([`Mark::FreeVariable`]).
    FreeVariable
    {
        /// The variable's name.
        name: String,
    },
    /// A grade-budget requirement `required ⊑ available` failed
    /// ([`Mark::GradeBudget`]).
    GradeBudget
    {
        /// The demanded grade, rendered.
        required: String,
        /// The thunk's available grade, rendered.
        available: String,
    },
    /// A thunk cannot be forced at all (`1 ⊑ r` failed; the specialized
    /// grade-budget case; [`Mark::Thunkability`]).
    Thunkability
    {
        /// The thunk's available grade, rendered.
        available: String,
    },
    /// A well-formed node with no typing rule in this direction
    /// ([`Mark::Stuck`]).
    Stuck
    {
        /// A hint for making progress.
        hint: String,
    },
    /// A forward-compatible catch-all for a future [`Mark`] kind; the
    /// [`MarkReport::is_error`] flag still classifies it.
    Other,
}

/// One source-ranged semantic [`Mark`] from the total marking layer.
///
/// The [`Mark`] (from [`gandr_core_checker::mark`], `semantic marker`) is
/// mapped through its node's structural path to a source span via the
/// [`OriginMap`](crate::origin): one `MarkReport` per node mark, in (item,
/// path, mark) order. A mark whose node is not `origin::resolve`-addressable —
/// a reified-stack interior decorated as a bonus entry (`reified-stack marking
/// residual`) — has no source identity and is dropped from this surface until
/// the Stk-descent resync lands (`semantic-marks work` step 5); every reported
/// mark therefore carries a span in source. The incremental `dirty` bit
/// ([`gandr_core_checker::mark::NodeFacts`]) is not surfaced here: its producer
/// is the gated edit / order-maintenance layer (`semantic-marks work` step 3,
/// gated on `dirty-frontier work` / `CST-resynchronization work`), so the field
/// would be a dead `false` until then.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct MarkReport
{
    /// The kind-specific payload (`kind` + `data`).
    #[cfg_attr(feature = "codecs", serde(flatten))]
    pub detail: MarkDetail,
    /// Whether this mark is an **error** ([`Mark::is_error`]) — `false` only
    /// for an empty hole. Authoritative even when [`Self::detail`] is the
    /// mirror's [`MarkDetail::Other`] catch-all.
    pub is_error: bool,
    /// The index of the item the marked node belongs to.
    pub item: usize,
    /// The marked node's source span.
    pub span: Span,
    /// The elaboration tag of the node's origin, when it is a synthesized node
    /// (`def` sugar, operator elaboration, …).
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub elaboration: Option<String>,
    /// The node's analyzed (checked-against) type, rendered, when typed in
    /// checking mode; absent in inference mode.
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub analyzed: Option<String>,
    /// The node's synthesized type, rendered, when present.
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub synthesized: Option<String>,
}

/// One projected entity attribute, the plain-data image of a
/// [`crate::attributes::ResolvedAttr`] (proposal-attributes.md §5).
///
/// This is the `Report.attributes` field's element. A renderer (LSP hover, the
/// TUI, the agent stream, a REPL `:attrs` command) reads this and renders it —
/// it parses nothing, lowers nothing, types nothing (host-effect boundary D6,
/// the renderer firewall).
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct AttrReport
{
    /// The annotated item's stable id (its index in `Lowered::items`; see the
    /// [`crate::attributes`] storage-identity note).
    pub node: usize,
    /// The resolved schema's name (the `SchemaRef`).
    pub schema: String,
    /// The checked payload, rendered with the module's deterministic `Debug`
    /// projection until the shared surface pretty-printer lands.
    pub payload: String,
    /// The schema's identity tier (`inert` for every MVP schema).
    pub tier: attributes::AttrTier,
    /// The `@[…]` block's source span.
    pub span: Span,
}

/// Reserved obligation payload for the agent stream.
///
/// This zero-variant type keeps reports usable as Rust values when codecs are
/// disabled. With codecs enabled, the empty `Vec<ObligationReport>` serializes
/// to the same JSON `[]` reserved slot as the former `Vec<serde_json::Value>`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub enum ObligationReport {}

/// The versioned agent-stream envelope: diagnostics, goals, marks, and
/// attributes for one lowered file, with a reserved slot for obligations.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct Report
{
    /// The schema version ([`SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// One diagnostic per item that fails to type, plus one per malformed
    /// attribute (proposal-attributes.md §3.2).
    pub diagnostics: Vec<Diagnostic>,
    /// One goal per hole (`A2-PLAN.md` §A2.2).
    pub goals: Vec<GoalReport>,
    /// The incremental-pipeline design semantic *marks* (`semantic marker` /
    /// `semantic-marks work`): one [`MarkReport`] per `origin`-addressable
    /// node mark from the total marking layer, in (item, path, mark) order.
    pub marks: Vec<MarkReport>,
    /// The resolved entity attributes (proposal-attributes.md §5): one
    /// [`AttrReport`] per well-formed `@[…]` attribute, in source order. An
    /// additive field (no [`SCHEMA_VERSION`] bump): a consumer that ignores it
    /// reads the report unchanged.
    pub attributes: Vec<AttrReport>,
    /// Reserved for *obligations* (empty at A2.4; D7 reserves the slot).
    pub obligations: Vec<ObligationReport>,
}

impl Report
{
    /// Serializes this report to a stable, pretty-printed JSON string (the
    /// golden surface and the on-the-wire agent stream).
    ///
    /// # Contract
    /// - ensures: returns a deterministic, pretty-printed JSON rendering of the
    ///   report.
    /// - provides: the serialized agent stream (the golden surface).
    /// - fails: propagates a `serde_json::Error` if serialization fails (it
    ///   cannot for these value types, but the signature stays honest).
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Propagates a [`serde_json::Error`] if serialization fails (it cannot
    /// for the value types in this module, but the fallible signature keeps
    /// the surface honest).
    #[cfg(feature = "codecs")]
    #[inline]
    pub fn to_json(&self) -> Result<String, serde_json::Error>
    {
        serde_json::to_string_pretty(self)
    }
}

/// Builds the full agent-stream [`Report`] for a lowered file, typing each item
/// under the context produced by the successful, bindable items before it.
///
/// # Contract
/// - requires: `base` is the typing context before the first lowered item (e.g.
///   `prelude_ctx`).
/// - ensures: returns the `Report` envelope at `SCHEMA_VERSION` — one
///   diagnostic per failing item, one goal per hole, one mark per
///   `origin`-addressable node mark — with each item checked against the same
///   ordered context a [`crate::session::Session`] uses for outcomes.
/// - provides: the v0 agent stream (D7) plus the incremental-pipeline design
///   marks (`semantic-marks work`).
/// - panics: none.
#[inline]
#[must_use]
pub fn report(
    lowered: &Lowered,
    base: &Ctx,
) -> Report
{
    let attr_pass = attributes::run(lowered, base);
    let hole_items = goal_item_flags(lowered);
    let mut item_bases = Vec::with_capacity(lowered.items.len());
    let mut diagnostics = Vec::new();
    let mut marks = Vec::new();
    let mut ctx = base.clone();
    for (item_index, item) in lowered.items.iter().enumerate() {
        let item_base = ctx.clone();
        let base_len = item_base.bindings().len();
        item_bases.push(item_base.clone());
        match item_machine_result(item, &item_base) {
            | Ok(ty) => {
                let holey = hole_items.get(item_index).copied().unwrap_or(true);
                if !holey
                    && let Some(ref name) = item.name
                    && let Some(value_type) = bound_value_type(&ty)
                {
                    ctx.bind(name.clone(), value_type);
                }
            },
            | Err(pair) => {
                let (error, failure) = *pair;
                diagnostics.push(build_diagnostic(
                    item_index.into(),
                    &error,
                    &failure,
                    lowered,
                    base_len.into(),
                ));
            },
        }
        push_marks_for_item(item_index.into(), item, &item_base, lowered, &mut marks);
    }
    diagnostics.extend(attr_pass.findings.iter().map(attribute_diagnostic));
    Report {
        schema_version: SCHEMA_VERSION,
        diagnostics,
        goals: goals_report_with_contexts(lowered, &item_bases)
            .iter()
            .map(goal_to_report)
            .collect(),
        marks,
        attributes: attr_pass.resolved.iter().map(attr_to_report).collect(),
        obligations: Vec::new(),
    }
}

/// Maps one resolved attribute to its plain-data [`AttrReport`] projection
/// (the renderer-firewall image, proposal-attributes.md §5).
fn attr_to_report(resolved: &attributes::ResolvedAttr) -> AttrReport
{
    AttrReport {
        node: resolved.node,
        schema: resolved.schema.clone(),
        payload: format!("{:?}", resolved.payload),
        tier: resolved.tier,
        span: Span::from(resolved.span.clone()),
    }
}

/// Maps one attribute finding to a source-ranged [`Diagnostic`]
/// (proposal-attributes.md §3.2). An ill-typed payload becomes the ordinary
/// type-error diagnostic ([`detail_of`]); the attribute-specific problems
/// become a [`DiagnosticDetail::Attribute`].
fn attribute_diagnostic(finding: &attributes::AttrFinding) -> Diagnostic
{
    match *finding {
        | attributes::AttrFinding::Unknown {
            ref name,
            ref suggestion,
            ref span,
        } => attribute_problem_diagnostic(
            name.into(),
            AttributeProblem::Unknown {
                suggestion: suggestion.clone(),
            },
            unknown_attribute_message(name.into(), suggestion.as_deref().map(Into::into)),
            span,
        ),
        | attributes::AttrFinding::Duplicate { ref name, ref span } => {
            attribute_problem_diagnostic(
                name.into(),
                AttributeProblem::Duplicate,
                format!("duplicate attribute `{name}` (single-valued)"),
                span,
            )
        },
        | attributes::AttrFinding::MissingPayload { ref name, ref span } => {
            attribute_problem_diagnostic(
                name.into(),
                AttributeProblem::MissingPayload,
                format!("attribute `{name}` requires a payload"),
                span,
            )
        },
        | attributes::AttrFinding::NonValuePayload { ref name, ref span } => {
            attribute_problem_diagnostic(
                name.into(),
                AttributeProblem::NonValuePayload,
                format!("attribute `{name}` payload must be a value, not a computation"),
                span,
            )
        },
        | attributes::AttrFinding::IllTypedPayload {
            ref error,
            ref span,
            ..
        } => Diagnostic {
            detail: detail_of(error),
            severity: Severity::Error,
            message: error.to_string(),
            span: Some(Span::from(span.clone())),
            elaboration: None,
            expr: None,
            context_chain: Vec::new(),
            ctx: Vec::new(),
        },
    }
}

/// Assembles an attribute-problem [`Diagnostic`] (the non-ill-typed findings).
fn attribute_problem_diagnostic(
    name: AttributeName<'_>,
    problem: AttributeProblem,
    message: String,
    span: &SourceRange,
) -> Diagnostic
{
    Diagnostic {
        detail: DiagnosticDetail::Attribute {
            name: name.0.to_owned(),
            problem,
        },
        severity: Severity::Error,
        message,
        span: Some(Span::from(span.clone())),
        elaboration: None,
        expr: None,
        context_chain: Vec::new(),
        ctx: Vec::new(),
    }
}

/// The `UnknownAttribute` message, with a did-you-mean when a registry name is
/// close enough.
fn unknown_attribute_message(
    name: AttributeName<'_>,
    suggestion: Option<AttributeName<'_>>,
) -> String
{
    match suggestion {
        | Some(candidate) => {
            format!(
                "unknown attribute `{}`; did you mean `{}`?",
                name.0, candidate.0
            )
        },
        | None => format!("unknown attribute `{}`", name.0),
    }
}

/// Computes the diagnostics: one [`Diagnostic`] for each item whose typing
/// fails on the machine (an item that types to `Done` — including any
/// hole-bearing item — contributes none).
///
/// # Contract
/// - requires: `base` is the typing context the items were lowered against
///   (e.g. `prelude_ctx`).
/// - ensures: returns one `Diagnostic` per item whose typing fails on the
///   machine; an item that types to `Done` (including any hole-bearing item)
///   contributes none.
/// - provides: the source-ranged diagnostics for the agent stream.
/// - panics: none.
#[inline]
#[must_use]
pub fn diagnostics(
    lowered: &Lowered,
    base: &Ctx,
) -> Vec<Diagnostic>
{
    let base_len = base.bindings().len();
    let mut out = Vec::new();
    for (item_index, item) in lowered.items.iter().enumerate() {
        if let Some((error, failure)) = first_failure(item, base) {
            out.push(build_diagnostic(
                item_index.into(),
                &error,
                &failure,
                lowered,
                base_len.into(),
            ));
        }
    }
    out
}

/// Drives one item through the machine to its first failure, returning the
/// error and the captured [`FailureState`], or [`None`] if it types to
/// `Done`.
fn first_failure(
    item: &Item,
    base: &Ctx,
) -> Option<(TypeError, FailureState)>
{
    match item_machine_result(item, base) {
        | Err(pair) => Some(*pair),
        | Ok(_) => None,
    }
}

/// Drives one item through the machine, returning the typed terminal result
/// or the first failure.
fn item_machine_result(
    item: &Item,
    base: &Ctx,
) -> Result<Ty, Box<(TypeError, FailureState)>>
{
    let mut state = initial_state(item, base);
    loop {
        match step(state) {
            | Outcome::Step(next) => state = next,
            | Outcome::Done(ty) => return Ok(ty),
            | Outcome::Error {
                error,
                state: failure,
            } => return Err(Box::new((error, failure))),
        }
    }
}

/// The value type a definition binds into scope, or [`None`] when it has no
/// nameable value type.
fn bound_value_type(ty: &Ty) -> Option<ValueType>
{
    match *ty {
        | Ty::Value(ref value_type) => Some(value_type.clone()),
        | Ty::Comp(CompType::F(ref payload, _)) => Some((**payload).clone()),
        | _ => None,
    }
}

/// Assembles one [`Diagnostic`] from a captured failure.
fn build_diagnostic(
    item_index: ItemIndex,
    error: &TypeError,
    failure: &FailureState,
    lowered: &Lowered,
    base_len: ContextLength,
) -> Diagnostic
{
    let (span, elaboration) = resolve_span(item_index, failure, lowered);
    Diagnostic {
        detail: detail_of(error),
        severity: Severity::Error,
        message: message_of(error),
        span,
        elaboration,
        expr: offending_expr(failure.control()),
        context_chain: failure.stack().iter().map(context_frame).collect(),
        ctx: bindings_beyond(failure.ctx(), base_len),
    }
}

/// Maps a [`TypeError`] to its [`DiagnosticDetail`].
fn detail_of(error: &TypeError) -> DiagnosticDetail
{
    match *error {
        | TypeError::TypeMismatch {
            ref expected,
            ref actual,
        } => DiagnosticDetail::TypeMismatch {
            expected: render_type_operand(expected),
            actual: render_type_operand(actual),
        },
        | TypeError::ShapeMismatch {
            expected,
            ref actual,
        } => DiagnosticDetail::ShapeMismatch {
            expected_shape: expected.to_owned(),
            actual: render_type_operand(actual),
        },
        | TypeError::StuckExpr { expr: _, hint } => DiagnosticDetail::StuckExpr {
            hint: hint.to_owned(),
        },
        | TypeError::UnboundVariable { ref name } => {
            DiagnosticDetail::UnboundVariable { name: name.clone() }
        },
        | TypeError::GradeError { lower, upper } => DiagnosticDetail::GradeError {
            lower: format!("{lower:?}"),
            upper: format!("{upper:?}"),
        },
    }
}

/// The human message for a type error. Identical to the core `Display`
/// (`TypeError::to_string`), EXCEPT a `TypeMismatch` / `ShapeMismatch` whose
/// operands mention a declared datatype renders those operands by their surface
/// spelling ([`render_type_operand`]) — so a nominal mismatch reads
/// `Maybe(Integer)` / `Celsius`, never the raw `Debug` of a `DataId`
/// (declared-data design Decision 2's O(1)-render corollary). Every
/// non-declared-data mismatch keeps the core `Display` byte-for-byte (the
/// golden report snapshots pin it).
fn message_of(error: &TypeError) -> String
{
    match *error {
        | TypeError::TypeMismatch {
            ref expected,
            ref actual,
        } if mentions_data(expected).0 || mentions_data(actual).0 => {
            format!(
                "type mismatch: expected {}, actual {}",
                render_type_operand(expected),
                render_type_operand(actual)
            )
        },
        | TypeError::ShapeMismatch {
            expected,
            ref actual,
        } if mentions_data(actual).0 => {
            format!(
                "type shape mismatch: expected {expected}, actual {}",
                render_type_operand(actual)
            )
        },
        | _ => error.to_string(),
    }
}

/// Whether a type mentions a declared-data nominal handle
/// ([`ValueType::Data`]) at any depth — the gate that routes a diagnostic
/// operand to its surface spelling rather than raw `Debug`.
fn mentions_data(ty: &Ty) -> DataMention
{
    DataMention(match *ty {
        | Ty::Value(ref value) => value_mentions_data(value).0,
        | Ty::Comp(ref comp) => comp_mentions_data(comp).0,
    })
}

/// Renders a diagnostic type operand: its SURFACE spelling ([`render::ty`])
/// when the type mentions a declared datatype and that rendering is faithful
/// (holds no `?` wildcard), else the `Debug` form the non-declared-data
/// diagnostics (and their golden snapshots) have always used. This keeps the
/// declared-data nominal spelling out of raw `Debug` while never degrading a
/// type the shared renderer cannot yet spell.
fn render_type_operand(ty: &Ty) -> String
{
    if mentions_data(ty).0 {
        let rendered = render::ty(ty);
        if !rendered.contains('?') {
            return rendered;
        }
    }
    format!("{ty:?}")
}

/// Whether a value type mentions [`ValueType::Data`] at any depth.
fn value_mentions_data(ty: &ValueType) -> DataMention
{
    type_mentions_data(TypeNode::Value(ty))
}

/// Whether a computation type mentions [`ValueType::Data`] at any depth.
fn comp_mentions_data(ty: &CompType) -> DataMention
{
    type_mentions_data(TypeNode::Comp(ty))
}

/// One pending node in an iterative value/computation-type traversal.
enum TypeNode<'ty>
{
    /// A value-type node.
    Value(&'ty ValueType),
    /// A computation-type node.
    Comp(&'ty CompType),
}

/// Iteratively scans a finite type tree for a declared-data node.
fn type_mentions_data(root: TypeNode<'_>) -> DataMention
{
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        match node {
            | TypeNode::Value(node) => match *node {
                | ValueType::Data { .. } => return DataMention(true),
                | ValueType::Prod(ref fst, ref snd) | ValueType::Sum(ref fst, ref snd) => {
                    pending.push(TypeNode::Value(snd));
                    pending.push(TypeNode::Value(fst));
                },
                | ValueType::List(ref element) => {
                    pending.push(TypeNode::Value(element));
                },
                | ValueType::Record(ref fields) => {
                    pending.extend(fields.values().map(|field| TypeNode::Value(field.as_ref())));
                },
                | ValueType::Thunk(_, ref body) => {
                    pending.push(TypeNode::Comp(body));
                },
                | ValueType::Stk(ref consumes, ref delivers) => {
                    pending.push(TypeNode::Comp(delivers));
                    pending.push(TypeNode::Comp(consumes));
                },
                | ValueType::Path {
                    ty: ref carrier, ..
                } => {
                    pending.push(TypeNode::Value(carrier));
                },
                | _ => {},
            },
            | TypeNode::Comp(node) => match *node {
                | CompType::F(ref of, _) => {
                    pending.push(TypeNode::Value(of));
                },
                | CompType::Arrow(ref arg, ref res) => {
                    pending.push(TypeNode::Comp(res));
                    pending.push(TypeNode::Value(arg));
                },
                | CompType::With(ref fst, ref snd) => {
                    pending.push(TypeNode::Comp(snd));
                    pending.push(TypeNode::Comp(fst));
                },
                | _ => {},
            },
        }
    }
    DataMention(false)
}

/// Resolves the source span for a failure: a precise sub-node span for a
/// `Descend` failure, else the enclosing item's span (see the module doc's
/// span decision). The second element is the span node's elaboration tag,
/// when present.
fn resolve_span(
    item_index: ItemIndex,
    failure: &FailureState,
    lowered: &Lowered,
) -> (Option<Span>, Option<String>)
{
    if let Some((range, elaboration)) = precise_span(item_index, failure, lowered) {
        return (
            Some(Span::from(range)),
            elaboration.map(|elab| format!("{elab:?}")),
        );
    }
    // Fallback: the enclosing item's root origin entry — always in-source.
    let item_path = [u32::try_from(item_index.0).unwrap_or(u32::MAX)];
    match lowered.origin.get_path(&item_path) {
        | Some(entry) => (
            Some(Span::from(entry.byte_range.clone())),
            entry.elaboration.map(|elab| format!("{elab:?}")),
        ),
        | None => (None, None),
    }
}

/// Renders the offending sub-term of a `Descend`-position failure; [`None`]
/// for a `Return` failure (the control register carries a type, not a term).
fn offending_expr(control: &Control) -> Option<String>
{
    match *control {
        | Control::DescendValue { ref value, .. } => Some(format!("{value:?}")),
        | Control::DescendComp { ref comp, .. } => Some(format!("{comp:?}")),
        // A `Return` failure has no offending term; the context chain
        // localizes it instead.
        | _ => None,
    }
}

/// The bindings of `ctx` beyond the first `base_len` (the caller's base
/// context, treated as implied — consistent with the goals report).
fn bindings_beyond(
    ctx: &Ctx,
    base_len: ContextLength,
) -> Vec<Binding>
{
    ctx.bindings()
        .get(base_len.0 ..)
        .unwrap_or(&[])
        .iter()
        .map(|entry| {
            let (ref name, ref ty) = *entry;
            Binding {
                name: name.clone(),
                ty: format!("{ty:?}"),
            }
        })
        .collect()
}

/// Renders one frame as a [`ContextFrame`].
fn context_frame(frame: &Frame) -> ContextFrame
{
    let (role, prose, binder) = frame_description(frame);
    ContextFrame {
        role: role.0.to_owned(),
        prose,
        binder,
    }
}

/// The `(role, prose, binder)` triple for a frame (the partial-derivation
/// vocabulary).
fn frame_description(frame: &Frame) -> (ContextRole<'static>, String, Option<String>)
{
    match *frame {
        | Frame::Abs { ref var, .. } => (
            ContextRole("Abs"),
            format!("checking the body of an abstraction binding `{var}`"),
            Some(var.clone()),
        ),
        | Frame::AppFn { .. } => (
            ContextRole("AppFn"),
            "checking the function of an application".to_owned(),
            None,
        ),
        | Frame::AppArg { .. } => (
            ContextRole("AppArg"),
            "checking the argument of an application".to_owned(),
            None,
        ),
        | Frame::PairFst { .. } => (
            ContextRole("PairFst"),
            "checking the first component of a pair".to_owned(),
            None,
        ),
        | Frame::PairSnd { .. } => (
            ContextRole("PairSnd"),
            "checking the second component of a pair".to_owned(),
            None,
        ),
        | Frame::Inj { .. } => (
            ContextRole("Inj"),
            "checking the payload of an injection".to_owned(),
            None,
        ),
        | Frame::Thunk { .. } => (
            ContextRole("Thunk"),
            "checking the body of a thunk".to_owned(),
            None,
        ),
        | Frame::Force { .. } => (
            ContextRole("Force"),
            "checking the forced value".to_owned(),
            None,
        ),
        | Frame::Ret { .. } => (
            ContextRole("Ret"),
            "checking the returned value".to_owned(),
            None,
        ),
        | Frame::Bind { ref var, .. } => (
            ContextRole("Bind"),
            format!("checking the bound computation of `run {var} <- …`"),
            Some(var.clone()),
        ),
        | Frame::BindBody { .. } => (
            ContextRole("BindBody"),
            "checking the continuation of a `let`-binding".to_owned(),
            None,
        ),
        | Frame::CaseScrut { .. } => (
            ContextRole("CaseScrut"),
            "checking the scrutinee of a case".to_owned(),
            None,
        ),
        | Frame::CaseArm1 { .. } => (
            ContextRole("CaseArm1"),
            "checking the first arm of a case".to_owned(),
            None,
        ),
        | Frame::CaseArm2 => (
            ContextRole("CaseArm2"),
            "checking the second arm of a case".to_owned(),
            None,
        ),
        | Frame::Split {
            ref fst_name,
            ref snd_name,
            ..
        } => (
            ContextRole("Split"),
            format!("checking the scrutinee of a split binding `({fst_name}, {snd_name})`"),
            None,
        ),
        | Frame::SplitBody { .. } => (
            ContextRole("SplitBody"),
            "checking the body of a split".to_owned(),
            None,
        ),
        | Frame::With1 { .. } => (
            ContextRole("With1"),
            "checking the first component of a lazy pair".to_owned(),
            None,
        ),
        | Frame::With2 { .. } => (
            ContextRole("With2"),
            "checking the second component of a lazy pair".to_owned(),
            None,
        ),
        | Frame::Prj { .. } => (
            ContextRole("Prj"),
            "checking the target of a projection".to_owned(),
            None,
        ),
        | Frame::Annot { .. } => (
            ContextRole("Annot"),
            "checking an annotated value".to_owned(),
            None,
        ),
        // Frames without a dedicated rendering above share the generic
        // obligation prose.
        | _ => (
            ContextRole("Frame"),
            "checking a pending obligation".to_owned(),
            None,
        ),
    }
}

/// The offending sub-term of a `Descend` failure, borrowed.
enum Offending<'term>
{
    /// A value sub-term.
    Value(&'term Value),
    /// A computation sub-term.
    Comp(&'term Comp),
}

/// Finds the recorded origin node of `item_index` whose term structurally
/// equals the failure's offending control sub-term, returning its byte range
/// and elaboration. [`None`] for a `Return` failure or an unmatched term.
fn precise_span(
    item_index: ItemIndex,
    failure: &FailureState,
    lowered: &Lowered,
) -> Option<(SourceRange, Option<crate::origin::ElabKind>)>
{
    let offending = match *failure.control() {
        | Control::DescendValue { ref value, .. } => Offending::Value(value),
        | Control::DescendComp { ref comp, .. } => Offending::Comp(comp),
        | _ => return None,
    };
    let item = lowered.items.get(item_index.0)?;
    let target = u32::try_from(item_index.0).ok()?;
    for (path, _id, entry) in lowered.origin.iter_paths() {
        let Some((&first, term_path)) = path.split_first()
        else {
            continue;
        };
        if first != target {
            continue;
        }
        let Some(node) = resolve(&item.term, term_path)
        else {
            continue;
        };
        let matched = match (&offending, node) {
            | (&Offending::Value(value), TermRef::Value(found)) => value == found,
            | (&Offending::Comp(comp), TermRef::Comp(found)) => comp == found,
            | _ => false,
        };
        if matched {
            return Some((entry.byte_range.clone(), entry.elaboration));
        }
    }
    None
}

/// Renders one [`Goal`] as a [`GoalReport`].
fn goal_to_report(goal: &Goal) -> GoalReport
{
    GoalReport {
        hole: goal.hole,
        item: goal.item,
        span: Span::from(goal.byte_range.clone()),
        note: goal.note.as_ref().map(|note| format!("{note:?}")),
        expected: goal.expected.as_ref().map(|ty| format!("{ty:?}")),
        ctx_local: goal.ctx_local.as_ref().map(|bindings| {
            bindings
                .iter()
                .map(|entry| {
                    let (ref name, ref ty) = *entry;
                    Binding {
                        name: name.clone(),
                        ty: format!("{ty:?}"),
                    }
                })
                .collect()
        }),
    }
}

/// Computes the semantic marks for a lowered file: one [`MarkReport`] per
/// `origin`-addressable node mark from the total marking layer.
///
/// Marks come from [`gandr_core_checker::mark`] (`semantic marker`) and retain
/// (item, path, mark) order.
///
/// Each item is marked against its ascription (checking mode) or in inference
/// mode — exactly the direction [`initial_state`](crate::goals) drives the
/// machine with (via `mark_item`) — so the marks detect the same type errors
/// as the diagnostics (their spans and some kind labels differ), while the
/// marking additionally decorates *every* node (it is total over type errors).
/// A node's structural path is mapped to a source span through the
/// [`OriginMap`](crate::origin) compatibility path index; a mark whose node is
/// not addressable there (a reified-stack interior, `reified-stack marking
/// residual`) is dropped (`semantic-marks work` step 5).
///
/// A node carries at most one mark under surface lowering (the
/// absorbing-`Unknown` recovery discipline precludes a rule-mark plus a
/// finish-mark on one node), so the per-node fan-out below is forward-compat
/// for the multi-mark core forms (handlers, reified stacks) the pipeline does
/// not yet see.
///
/// # Contract
/// - requires: `base` is the typing context the items were lowered against
///   (e.g. `prelude_ctx`).
/// - ensures: returns one `MarkReport` per `origin`-addressable node mark, in
///   (item, path, mark) order; an item whose term sort is unknown contributes
///   none.
/// - provides: the incremental-pipeline design marks for the agent stream
///   (`VISION.md` §4).
/// - panics: none. The marker is *type*-total (every node decorated, every
///   abort site recovered) but still direct-style; the pipeline therefore
///   fail-closes by skipping marks for items whose origin path depth exceeds
///   [`MAX_RECURSIVE_MARK_DEPTH`], so adversarial-depth interactive submissions
///   still return diagnostics/goals without overflowing.
#[inline]
#[must_use]
pub fn marks(
    lowered: &Lowered,
    base: &Ctx,
) -> Vec<MarkReport>
{
    let mut out = Vec::new();
    for (item_index, item) in lowered.items.iter().enumerate() {
        push_marks_for_item(item_index.into(), item, base, lowered, &mut out);
    }
    out
}

/// Appends marks for one lowered item, preserving source item identity.
fn push_marks_for_item(
    item_index: ItemIndex,
    item: &Item,
    base: &Ctx,
    lowered: &Lowered,
    out: &mut Vec<MarkReport>,
)
{
    if recursive_mark_depth_exceeded(item_index, lowered).0 {
        // Interim safety contract: the core marker is still direct-style.
        // Rather than risk aborting the long-lived session on generated or
        // adversarial nesting, omit the optional incremental-pipeline design marks for
        // this item. Machine-backed diagnostics and goals above still preserve
        // source identity and type failures for the submission.
        return;
    }
    let marking = mark_item(item, base);
    let target = u32::try_from(item_index.0).unwrap_or(u32::MAX);
    for (node_path, node_id) in marking.compatibility_paths() {
        let Some(facts) = marking.get(*node_id)
        else {
            continue;
        };
        if facts.marks.is_empty() {
            continue;
        }
        let mut path = Vec::with_capacity(node_path.len().saturating_add(1));
        path.push(target);
        path.extend_from_slice(node_path);
        // Drop a mark whose node has no source identity (a reified-stack
        // interior bonus entry; `reified-stack marking residual` / `semantic-marks
        // work` step 5). Surface lowering never mints a `Stk`, so this is
        // forward-compat only.
        let Some(entry) = lowered.origin.get_path(&path)
        else {
            continue;
        };
        let span = Span::from(entry.byte_range.clone());
        let elaboration = entry.elaboration.map(|elab| format!("{elab:?}"));
        let analyzed = facts.analyzed.as_ref().map(|ty| format!("{ty:?}"));
        let synthesized = facts.synthesized.as_ref().map(|ty| format!("{ty:?}"));
        for mark in &facts.marks {
            out.push(MarkReport {
                detail: mark_detail(mark),
                is_error: bool::from(mark.is_error()),
                item: item_index.0,
                span: span.clone(),
                elaboration: elaboration.clone(),
                analyzed: analyzed.clone(),
                synthesized: synthesized.clone(),
            });
        }
    }
}
/// Maximum origin-path depth sent to the direct-style marker from the
/// interactive diagnostics path.
///
/// This is deliberately a pipeline guard, not a core typing rule: diagnostics
/// and goals are already machine-backed and remain available for deeper input,
/// while marks are advisory decoration until the marker is made iterative.
const MAX_RECURSIVE_MARK_DEPTH: usize = 1024;

/// Returns `true` when an item's origin-addressable term is deep enough that
/// calling the direct-style core marker is not safe for a long-lived process.
fn recursive_mark_depth_exceeded(
    item_index: ItemIndex,
    lowered: &Lowered,
) -> RecursiveMarkDepthExceeded
{
    let Ok(target) = u32::try_from(item_index.0)
    else {
        return RecursiveMarkDepthExceeded(true);
    };
    for (path, _id, _entry) in lowered.origin.iter_paths() {
        let Some((&first, rest)) = path.split_first()
        else {
            continue;
        };
        if first == target && rest.len() > MAX_RECURSIVE_MARK_DEPTH {
            return RecursiveMarkDepthExceeded(true);
        }
    }
    RecursiveMarkDepthExceeded(false)
}

/// Marks one lowered item against its recorded ascription when the sorts match,
/// otherwise in inference mode — the marking counterpart of
/// [`initial_state`](crate::goals). The dispatch is total over `Term`'s two
/// sorts (its upstream growth point is retired; an added sort is a
/// compile-visible change here).
fn mark_item(
    item: &Item,
    base: &Ctx,
) -> Marking
{
    match (&item.term, &item.ascription) {
        | (&Term::Value(ref value), &Some(Ty::Value(ref expected))) => {
            mark_value(base.clone(), value.clone(), Dir::Check(expected.clone()))
        },
        | (&Term::Value(ref value), _) => mark_value(base.clone(), value.clone(), Dir::Infer),
        | (&Term::Comp(ref comp), &Some(Ty::Comp(ref expected))) => {
            mark_comp(base.clone(), comp.clone(), Dir::Check(expected.clone()))
        },
        | (&Term::Comp(ref comp), _) => mark_comp(base.clone(), comp.clone(), Dir::Infer),
    }
}

/// Maps a [`Mark`] to its [`MarkDetail`] (the serde mirror; rendering follows
/// the module's `Debug` decision).
fn mark_detail(mark: &Mark) -> MarkDetail
{
    match *mark {
        | Mark::EmptyHole(hole) => MarkDetail::EmptyHole { hole },
        | Mark::TypeMismatch(ref boundary) => MarkDetail::TypeMismatch {
            expected: format!("{:?}", boundary.expected),
            actual: format!("{:?}", boundary.actual),
        },
        | Mark::EffectRowMismatch(ref boundary) => MarkDetail::EffectRowMismatch {
            expected: format!("{:?}", boundary.expected),
            actual: format!("{:?}", boundary.actual),
        },
        | Mark::ShapeMismatch {
            expected,
            ref actual,
        } => MarkDetail::ShapeMismatch {
            expected_shape: expected.to_owned(),
            actual: format!("{actual:?}"),
        },
        | Mark::FreeVariable { ref name } => MarkDetail::FreeVariable { name: name.clone() },
        | Mark::GradeBudget {
            required,
            available,
        } => MarkDetail::GradeBudget {
            required: format!("{required:?}"),
            available: format!("{available:?}"),
        },
        | Mark::Thunkability { available } => MarkDetail::Thunkability {
            available: format!("{available:?}"),
        },
        | Mark::Stuck { hint } => MarkDetail::Stuck {
            hint: hint.to_owned(),
        },
    }
}
