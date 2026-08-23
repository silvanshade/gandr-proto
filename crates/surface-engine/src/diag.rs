//! Diagnostics, goals, and marks as one versioned JSON report — the envelope
//! the inspection surface
//! (`spec:implementation/inspection-protocol.md`) projects from.
//!
//! The report combines marks, obligations, and goals in one envelope. It maps
//! a typing failure
//! ([`gandr_core_machine::FailureState`] +
//! [`gandr_core_term::error::TypeError`]) together with the source identity
//! in an [`OriginMap`](crate::origin) to a structured, source-ranged
//! [`Diagnostic`], and carries the hole **goals** ([`crate::goals`]) in the
//! same versioned [`Report`] envelope. The incremental-pipeline design's
//! *marks* slot is **populated** ([`marks`]): one [`MarkReport`] per node
//! mark from the total semantic marking layer
//! ([`gandr_core_checker::discipline::mark`]), source-ranged through the same
//! `OriginMap`. The *obligations* slot is populated too ([`obligations`]): one
//! [`ObligationReport`] per repair the melder made while recovering the source,
//! carried from the parse through [`Lowered`] with its class and responsible
//! byte span intact. It is the envelope's one syntactic surface — every other
//! field describes the tree that recovery produced.
//!
//! The marks and diagnostics are **complementary** realizations of the same
//! type system: diagnostics are the machine's fail-fast derivation view (first
//! failure per item, with ordered, source-annotated machine contexts); marks
//! are the marker's *total* per-node decoration (every node, every type error
//! localized and recovered). They detect the same type errors, while the
//! diagnostic driver additionally carries the exact active [`OriginNodeId`]
//! chain beside the parser-free machine so both `Descend`- and
//! `Return`-position failures retain source provenance.
//!
//! # serde placement (decision tree)
//!
//! [SPECULATIVE DECISION — the report decision under-specifies placement.] The
//! report decision says only "output is serde-JSON"; it does not say *where*
//! the serde derives live. Keeping `gandr-core-checker` dependency-free and
//! WASM-portable ("core stays parser-free / minimal dependency surface"), the
//! JSON types live **here**, as mirror structs over the core's
//! `FailureState` / `TypeError` / [`Mark`] / [`Goal`] data — *not* as
//! `Serialize` derives on the core types. A checkpoint serde feature on the
//! core can then land without colliding with anything in this module.
//! Rejected alternative: derive `Serialize` on the core error/type enums
//! now — it pulls `serde` into `gandr-core-checker` early, couples the two
//! layers, and bakes the wire format into the verification anchor.
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
//! # Span resolution
//!
//! The core typing machine remains parser- and span-free. The surface
//! diagnostic driver pairs its control trace with the origin map's
//! deterministic preorder: each `Descend` enters the next exact origin node,
//! and each successful `Return` transition leaves the completed node. The
//! active origin ID at a failure is therefore the offending term occurrence
//! even when equal sibling terms repeat. Structural equality only checks that
//! the two traversals remain synchronized; it never selects an occurrence. If
//! they diverge, provenance is dropped and the diagnostic is honestly unlocated
//! rather than borrowing an enclosing or guessed span.

use core::ops::Range;

use gandr_core_checker::discipline::mark::Mark;
use gandr_core_checker::discipline::mark::Marking;
use gandr_core_checker::discipline::mark::mark_comp;
use gandr_core_checker::discipline::mark::mark_value;
use gandr_core_checker::judgements::control::Control;
use gandr_core_checker::judgements::control::Dir;
use gandr_core_incremental::region::Item;
use gandr_core_machine::FailureState;
use gandr_core_machine::Frame;
use gandr_core_machine::Outcome;
use gandr_core_machine::step;
use gandr_core_term::ctx::Ctx;
use gandr_core_term::error::TypeError;
use gandr_core_term::syntax::Term;
use gandr_core_term::types::CompType;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;
use gandr_surface_parser::Oblig;
use gandr_surface_parser::ObligationInstance;
pub use gandr_surface_render_remote::diagnostic::DiagnosticCode;
pub use gandr_surface_render_remote::diagnostic::DiagnosticMessage;
use gandr_surface_render_remote::present::ObligationClass;

use crate::attributes;
use crate::boundary::AttributeName;
use crate::boundary::ContextLength;
use crate::boundary::ContextRole;
use crate::boundary::ItemIndex;
use crate::boundary::RecursiveMarkDepthExceeded;
use crate::boundary::SourceRange;
use crate::goals::Goal;
use crate::goals::goal_item_flags;
use crate::goals::goals_report_with_contexts;
use crate::goals::initial_state;
use crate::lower::Lowered;
use crate::lower::obligation_range;
use crate::origin::OriginFacetKind;
use crate::origin::OriginNodeId;
use crate::origin::TermRef;
use crate::origin::resolve;
use crate::render;

/// The schema version of the [`Report`] envelope.
///
/// Bumped whenever a field is renamed, removed, or changes meaning (additive
/// fields do not require a bump). Consumers must check it before parsing.
///
/// `2` — the reserved `marks` slot changed from an opaque `serde_json::Value`
/// array to a typed [`MarkReport`] array; a meaning change, hence a bump.
///
/// `3` — the reserved `obligations` slot, always `[]`, became the parse's live
/// recovery obligations as a typed [`ObligationReport`] array; a meaning change
/// on the same precedent, hence a bump.
///
/// `4` — rendered `message` text became a stable `code` plus a typed message
/// template and arguments; consumers render the prose they need.
///
/// `5` — the single `span` and unlocated context strings became ordered,
/// role-bearing annotations on both the diagnostic and each machine context.
pub const SCHEMA_VERSION: u32 = 5;

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

/// The semantic role of one source annotation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "codecs",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "lowercase")
)]
pub enum DiagnosticAnnotationKind
{
    /// A locus directly participating in the reported failure.
    Primary,
    /// A broader source locus explaining one pending machine obligation.
    Context,
}

/// One exact, backend-independent source annotation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct DiagnosticAnnotation
{
    /// Whether this locus is primary or contextual.
    pub kind: DiagnosticAnnotationKind,
    /// Exact half-open UTF-8 byte range.
    pub span: Span,
    /// Operand-specific explanation for this locus, when available.
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub label: Option<String>,
    /// Lowering elaboration attached to this locus, when synthesized.
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub elaboration: Option<String>,
}

/// Builds the common single-primary annotation set for source-owned findings.
fn single_primary(
    span: Span,
    label: String,
) -> Vec<DiagnosticAnnotation>
{
    vec![DiagnosticAnnotation {
        kind: DiagnosticAnnotationKind::Primary,
        span,
        label: Some(label),
        elaboration: None,
    }]
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
/// Every [`TypeError`] is fatal, so typing contributes only
/// [`Severity::Error`]. [`Severity::Warning`] arrives from **recognition**: a
/// declaration or binder that takes a prelude or host name is accepted under
/// the warn-and-allow policy and reported at this severity, which is the whole
/// reason the level exists rather than a reserved slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "codecs", serde(rename_all = "lowercase"))]
pub enum Severity
{
    /// A fatal typing error.
    Error,
    /// A non-fatal finding: the program was accepted, and something about it is
    /// worth saying anyway.
    Warning,
}

/// The kind-specific payload of a [`Diagnostic`], tagged by `kind` with the
/// variant data under `data`.
///
/// One variant per *reachable* [`TypeError`] constructor, plus the two that
/// mirror no typing failure at all: [`Self::Attribute`], from the attribute
/// pass, and [`Self::ShadowedName`], from recognition. Both arrive after typing
/// and carry their own payload rather than a [`FailureState`]'s.
///
/// The machine's two
/// polarity-guard `ShapeMismatch` descriptions
/// ([`text::SHAPE_VALUE`](gandr_core_term::error::text::SHAPE_VALUE) /
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
    /// A declaration or binder took a prelude or host name, and the
    /// warn-and-allow policy let it.
    ///
    /// The two cases differ in what follows, and the message says which. A
    /// **declaration** shadows: the name and its whole subtree now mean the
    /// declaration, so `list.each` after a `module list` is the user's
    /// component. A **binder** does not: the lowerer carries no value
    /// environment, so `env.get` under a parameter named `env` still resolves
    /// to the host module, and this diagnostic is the only thing that says so.
    ShadowedName
    {
        /// The path the declaration or binder took.
        path: String,
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

/// One pending machine obligation with its own exact source annotations.
///
/// Contexts retain the failure frame stack's domain semantics in
/// **outermost-first** order: the first entry is the outermost pending
/// obligation, and the last is the frame directly containing the failure.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct DiagnosticContext
{
    /// The machine frame's name (e.g. `AppFn`), for stable machine parsing.
    pub role: String,
    /// A prose description of the pending obligation.
    pub prose: String,
    /// The frame's binder name, when the frame carries real binder information.
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub binder: Option<String>,
    /// Ordered source loci belonging to this pending obligation.
    pub annotations: Vec<DiagnosticAnnotation>,
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
    /// The lowered item this diagnostic rejects, when it comes from item
    /// typing.
    ///
    /// Recognition and attribute diagnostics are report-level findings and do
    /// not claim an item outcome.
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub item: Option<usize>,
    /// Stable registry code.
    pub code: DiagnosticCode,
    /// Fatality classification.
    pub severity: Severity,
    /// Localizable template identity and typed arguments.
    pub message: DiagnosticMessage,
    /// Ordered exact source loci directly participating in this failure.
    ///
    /// Multiple primary loci are permitted when their relationship is the
    /// error. Backend adapters may choose a lead range without discarding the
    /// remaining annotations from the domain report.
    pub annotations: Vec<DiagnosticAnnotation>,
    /// The offending sub-term, rendered, for a `Descend`-position failure;
    /// absent for a `Return`-position failure (the control register is a
    /// type, not a term — [`Self::contexts`] retains its enclosing reasons).
    #[cfg_attr(
        feature = "codecs",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub expr: Option<String>,
    /// Pending machine obligations, outermost first, each with its own loci.
    pub contexts: Vec<DiagnosticContext>,
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
/// The core stays serde-free, so the wire image of every
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
    /// An empty hole `?u` in **pattern** position — complete-but-incomplete,
    /// **not** an error ([`Mark::PatternHole`]). Kept apart from
    /// [`MarkDetail::EmptyHole`] because an unfinished test and an unfinished
    /// value license different conclusions.
    PatternHole
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
/// The [`Mark`] (from [`gandr_core_checker::discipline::mark`], `semantic
/// marker`) is mapped through its node's structural path to a source span via
/// the [`OriginMap`](crate::origin): one `MarkReport` per node mark, in (item,
/// path, mark) order. A mark whose node is not `origin::resolve`-addressable —
/// a reified-stack interior decorated as a bonus entry — has no source
/// identity and is dropped from this surface until a stack-descent resync
/// lands; every reported mark therefore carries a span in source. The
/// incremental `dirty` bit
/// ([`gandr_core_checker::discipline::mark::NodeFacts`]) is not surfaced here:
/// its producer is the order-maintenance-backed edit layer, which is designed
/// and not built, so the field would be a dead `false`.
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

/// One parser recovery obligation: the class of repair the melder made, and the
/// source bytes responsible for it.
///
/// The row is the melder's [`ObligationInstance`] projected to the wire — the
/// class name from the shared renderer vocabulary ([`ObligationClass`]) and the
/// exact responsible byte span, with nothing rendered and nothing opaque. A
/// consumer reads the class as data and spells it itself.
///
/// The rows are the agent stream's *syntactic* half: they say what the parse
/// had to repair to produce a tree at all, where [`Diagnostic`] and
/// [`MarkReport`] say what typing found in the tree that resulted. A recovery
/// hole's [`GoalReport`] is the same repair seen from the term side — the hole
/// the lowerer put where the damage was — so a row and a goal can name one
/// region without either being derived from the other.
///
/// [`ObligationInstance`]: gandr_surface_parser::ObligationInstance
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
pub struct ObligationReport
{
    /// The obligation's class.
    pub class: ObligationClass,
    /// The smallest source span responsible for the obligation.
    pub span: Span,
}

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
    /// One goal per hole.
    pub goals: Vec<GoalReport>,
    /// The incremental-pipeline design's semantic *marks*: one
    /// [`MarkReport`] per `origin`-addressable node mark from the total
    /// marking layer, in (item, path, mark) order.
    pub marks: Vec<MarkReport>,
    /// The resolved entity attributes (proposal-attributes.md §5): one
    /// [`AttrReport`] per well-formed `@[…]` attribute, in source order. An
    /// additive field (no [`SCHEMA_VERSION`] bump): a consumer that ignores it
    /// reads the report unchanged.
    pub attributes: Vec<AttrReport>,
    /// The parse's recovery *obligations*: one [`ObligationReport`] per repair
    /// the melder made, in source order ([`obligations`]). Empty for a source
    /// that parsed clean.
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
/// - provides: the versioned report envelope plus the incremental-pipeline
///   design's marks.
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
        let item_index = ItemIndex::from(item_index);
        let item_base = ctx.clone();
        let base_len = item_base.bindings().len();
        item_bases.push(item_base.clone());
        match item_machine_result(item_index, item, lowered, &item_base) {
            | Ok(ty) => {
                let holey = hole_items.get(item_index.0).copied().unwrap_or(true);
                if !holey
                    && let Some(ref name) = item.name
                    && let Some(value_type) = bound_value_type(&ty)
                {
                    ctx.bind(name.clone(), value_type);
                }
            },
            | Err(failure) => {
                diagnostics.push(build_diagnostic(
                    item_index,
                    &failure,
                    lowered,
                    base_len.into(),
                ));
            },
        }
        push_marks_for_item(item_index, item, &item_base, lowered, &mut marks);
    }
    diagnostics.extend(attr_pass.findings.iter().map(attribute_diagnostic));
    diagnostics.extend(lowered.shadowed_builtins().iter().map(shadowed_diagnostic));
    Report {
        schema_version: SCHEMA_VERSION,
        diagnostics,
        goals: goals_report_with_contexts(lowered, &item_bases)
            .iter()
            .map(goal_to_report)
            .collect(),
        marks,
        attributes: attr_pass.resolved.iter().map(attr_to_report).collect(),
        obligations: obligations(lowered),
    }
}

/// Computes the parse's recovery obligation rows: one [`ObligationReport`] per
/// repair the melder made, in source order.
///
/// The rows are a projection of [`Lowered::obligations`] — the parse's own
/// buffer, carried through lowering — so this function invents no obligation,
/// drops none, and re-spans none. What it adds is the *order*: the parse
/// buffers by severity (the minimization order it selects repairs under), which
/// is the wrong order to read a file in, so the rows are sorted by start, then
/// end, then class. That comparison is total on the row content, so two runs
/// over one source produce byte-identical rows.
///
/// # Contract
/// - requires: none.
/// - ensures: returns one row per obligation in `lowered`, each carrying the
///   class and the exact responsible byte span; the rows are ordered by span
///   start, then span end, then class severity; the result is empty exactly
///   when the parse was clean.
/// - provides: the report envelope's obligation surface, and the source-ordered
///   view a renderer or agent reads.
/// - fails: never.
/// - panics: none.
/// - intension: the order is a total comparison over `(start, end, class)`, so
///   it is deterministic without depending on the sort's stability or on the
///   melder's buffering order.
///
/// # Adequacy
/// - hypothesis: L4 — a recovering source (non-empty rows with exact class and
///   span), a clean source (empty rows), a source whose severity order and
///   source order disagree (the sort is observed, not inherited), and a
///   repeated lowering (determinism) each exercise a distinct decision.
/// - witness: `diag_obligations::tests::rows::rows_carry_the_class_and_the_exact_span`
/// - witness: `diag_obligations::tests::rows::a_clean_source_reports_no_obligations`
/// - witness: `diag_obligations::tests::rows::rows_are_in_source_order_not_severity_order`
/// - witness: `diag_obligations::tests::rows::rows_are_deterministic_across_lowerings`
#[inline]
#[must_use]
pub fn obligations(lowered: &Lowered) -> Vec<ObligationReport>
{
    let mut rows: Vec<ObligationReport> = lowered
        .obligations()
        .iter()
        .map(obligation_report)
        .collect();
    rows.sort_by(|left, right| {
        left.span
            .start
            .cmp(&right.span.start)
            .then_with(|| left.span.end.cmp(&right.span.end))
            .then_with(|| left.class.cmp(&right.class))
    });
    rows
}

/// Projects one buffered melder obligation onto its report row.
fn obligation_report(instance: &ObligationInstance) -> ObligationReport
{
    ObligationReport {
        class: obligation_class(instance.class),
        span: Span::from(obligation_range(instance)),
    }
}

/// Maps the parser's obligation taxonomy onto the shared renderer vocabulary.
///
/// This is the single crossing between the two: [`Oblig`] stays the semantic
/// authority (the melder minimizes over it), and [`ObligationClass`] is the
/// name the report and the render bus publish. The match is exhaustive by
/// construction, so a class added upstream is a compile error here rather than
/// a row that silently loses its class.
///
/// # Contract
/// - requires: none.
/// - ensures: returns the same-named class; the mapping is a bijection onto
///   [`ObligationClass`] and preserves the severity ladder, because both
///   enumerations declare the classes in the same low-to-high order.
/// - provides: the report and render-bus class vocabulary for a parser
///   obligation.
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — mapping every [`Oblig`] class and comparing the images
///   pairwise separates a swapped pair (name preservation) from a reordered
///   ladder (severity preservation).
/// - witness: `diag_obligations::tests::authority::every_parser_class_maps_to_its_own_name_and_rank`
#[inline]
#[must_use]
pub const fn obligation_class(class: Oblig) -> ObligationClass
{
    match class {
        | Oblig::MissingMeld => ObligationClass::MissingMeld,
        | Oblig::MissingTile => ObligationClass::MissingTile,
        | Oblig::IncompleteTile => ObligationClass::IncompleteTile,
        | Oblig::UnmoldedTok => ObligationClass::UnmoldedTok,
        | Oblig::InconMeld => ObligationClass::InconMeld,
        | Oblig::ExtraMeld => ObligationClass::ExtraMeld,
        | Oblig::ReservedKeyword => ObligationClass::ReservedKeyword,
        | Oblig::AmbiguousPrec => ObligationClass::AmbiguousPrec,
    }
}

/// Maps one recorded shadowing of a prelude or host name to its warning.
///
/// # Contract
/// - ensures: the diagnostic carries [`Severity::Warning`], the shadowed path,
///   and the declaration's span; it never carries a context chain, because
///   nothing was being typed when recognition recorded it.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: the recognition record is complete enough to report without
///   consulting the typing pass.
/// - witness: `gandr-surface-engine` `tests/recognition.rs` —
///   `a_shadowed_builtin_is_reported_as_a_warning`
fn shadowed_diagnostic(shadowed: &crate::recognition::ShadowedBuiltin) -> Diagnostic
{
    let path = format!("{}", shadowed.path);
    let message = DiagnosticMessage::ShadowedName { path: path.clone() };
    let label = format!("this declaration shadows {path}");
    Diagnostic {
        detail: DiagnosticDetail::ShadowedName { path },
        item: None,
        code: message.code(),
        severity: Severity::Warning,
        message,
        annotations: single_primary(
            Span {
                start: shadowed.byte_range.0.start,
                end: shadowed.byte_range.0.end,
            },
            label,
        ),
        expr: None,
        contexts: Vec::new(),
        ctx: Vec::new(),
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
            DiagnosticMessage::UnknownAttribute {
                name: name.clone(),
                suggestion: suggestion.clone(),
            },
            span,
        ),
        | attributes::AttrFinding::Duplicate { ref name, ref span } => {
            attribute_problem_diagnostic(
                name.into(),
                AttributeProblem::Duplicate,
                DiagnosticMessage::DuplicateAttribute { name: name.clone() },
                span,
            )
        },
        | attributes::AttrFinding::MissingPayload { ref name, ref span } => {
            attribute_problem_diagnostic(
                name.into(),
                AttributeProblem::MissingPayload,
                DiagnosticMessage::MissingAttributePayload { name: name.clone() },
                span,
            )
        },
        | attributes::AttrFinding::NonValuePayload { ref name, ref span } => {
            attribute_problem_diagnostic(
                name.into(),
                AttributeProblem::NonValuePayload,
                DiagnosticMessage::NonValueAttributePayload { name: name.clone() },
                span,
            )
        },
        | attributes::AttrFinding::IllTypedPayload {
            ref error,
            ref span,
            ..
        } => {
            let message = message_of(error);
            let detail = detail_of(error);
            let label = primary_label(&detail);
            Diagnostic {
                detail,
                item: None,
                code: message.code(),
                severity: Severity::Error,
                message,
                annotations: single_primary(Span::from(span.clone()), label),
                expr: None,
                contexts: Vec::new(),
                ctx: Vec::new(),
            }
        },
    }
}

/// Assembles an attribute-problem [`Diagnostic`] (the non-ill-typed findings).
fn attribute_problem_diagnostic(
    name: AttributeName<'_>,
    problem: AttributeProblem,
    message: DiagnosticMessage,
    span: &SourceRange,
) -> Diagnostic
{
    let label = match &problem {
        | &AttributeProblem::Unknown { .. } => "unknown attribute",
        | &AttributeProblem::Duplicate => "duplicate attribute",
        | &AttributeProblem::MissingPayload => "payload required",
        | &AttributeProblem::NonValuePayload => "payload must be a value",
    };
    Diagnostic {
        detail: DiagnosticDetail::Attribute {
            name: name.0.to_owned(),
            problem,
        },
        code: message.code(),
        item: None,
        severity: Severity::Error,
        message,
        annotations: single_primary(Span::from(span.clone()), label.to_owned()),
        expr: None,
        contexts: Vec::new(),
        ctx: Vec::new(),
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
        let item_index = ItemIndex::from(item_index);
        if let Some(failure) = first_failure(item_index, item, lowered, base) {
            out.push(build_diagnostic(
                item_index,
                &failure,
                lowered,
                base_len.into(),
            ));
        }
    }
    out
}

/// One machine failure with its exact active surface-origin identity.
struct MachineFailure
{
    /// The core typing failure.
    error: TypeError,
    /// The machine state at the failed step.
    state: FailureState,
    /// Exact active origin chain, outermost through the failing occurrence.
    origins: Vec<OriginNodeId>,
}

/// One source-origin node paired with its item-relative core-term path.
struct OriginLocus<'origin>
{
    /// Stable identity retained after the compatibility path is consumed.
    id: OriginNodeId,
    /// Borrowed core-term path used only to verify traversal synchronization.
    term_path: &'origin [u32],
}

/// Carries exact source identity beside the span-free typing machine.
///
/// # Contract
/// - requires: `pending` is the item's core-term origin nodes in preorder.
/// - ensures: [`Self::current_chain`] returns the exact active occurrence chain
///   while the machine and origin preorder agree; any disagreement clears
///   provenance.
/// - provides: occurrence identity for both `Descend` and `Return` failures
///   without changing the core machine's parser-free interface.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — return-position localization, repeated equal siblings,
///   and a direct descend failure distinguish cursor, stack, and desync errors.
/// - witness: `diag::tests::provenance::force_shape_failure_points_to_its_argument`
struct FailureLocusTracker<'term, 'origin>
{
    /// Original item term used to verify each preorder entry.
    term: &'term Term,
    /// Unvisited loci, reversed so the next preorder node is popped in O(1).
    pending: Vec<OriginLocus<'origin>>,
    /// Entered term occurrences not yet completed, outermost first.
    active: Vec<OriginNodeId>,
}

impl<'term, 'origin> FailureLocusTracker<'term, 'origin>
{
    /// Builds the tracker for one lowered item.
    fn new(
        item_index: ItemIndex,
        item: &'term Item,
        lowered: &'origin Lowered,
    ) -> Self
    {
        let target = u32::try_from(item_index.0).ok();
        let mut pending = lowered
            .origin
            .iter_paths()
            .filter_map(|(path, id, _entry)| {
                let (&first, term_path) = path.0.split_first()?;
                if Some(first) != target || resolve(&item.term, term_path).is_none() {
                    return None;
                }
                Some(OriginLocus { id, term_path })
            })
            .collect::<Vec<_>>();
        pending.reverse();
        Self {
            term: &item.term,
            pending,
            active: Vec::new(),
        }
    }

    /// Enters the next exact occurrence when `control` descends a term.
    fn enter(
        &mut self,
        control: &Control,
    )
    {
        let descending = matches!(
            *control,
            Control::DescendValue { .. } | Control::DescendComp { .. }
        );
        if !descending {
            return;
        }
        let Some(locus) = self.pending.pop()
        else {
            self.active.clear();
            return;
        };
        let matched = match (control, resolve(self.term, locus.term_path)) {
            | (&Control::DescendValue { ref value, .. }, Some(TermRef::Value(found))) => {
                value == found
            },
            | (&Control::DescendComp { ref comp, .. }, Some(TermRef::Comp(found))) => comp == found,
            | _ => false,
        };
        if matched {
            self.active.push(locus.id);
        }
        else {
            self.pending.clear();
            self.active.clear();
        }
    }

    /// Leaves the term occurrence whose successful type is being consumed.
    fn finish(&mut self)
    {
        self.active.pop();
    }

    /// Returns the exact active occurrence chain at the current control.
    fn current_chain(&self) -> Vec<OriginNodeId>
    {
        self.active.clone()
    }
}

/// Drives one item through the machine to its first failure.
fn first_failure(
    item_index: ItemIndex,
    item: &Item,
    lowered: &Lowered,
    base: &Ctx,
) -> Option<MachineFailure>
{
    match item_machine_result(item_index, item, lowered, base) {
        | Err(failure) => Some(*failure),
        | Ok(_) => None,
    }
}

/// Drives one item through the machine while retaining exact source identity.
fn item_machine_result(
    item_index: ItemIndex,
    item: &Item,
    lowered: &Lowered,
    base: &Ctx,
) -> Result<Ty, Box<MachineFailure>>
{
    let mut state = initial_state(item, base);
    let mut loci = FailureLocusTracker::new(item_index, item, lowered);
    loci.enter(state.control());
    loop {
        let returning = matches!(*state.control(), Control::Return { .. });
        match step(state) {
            | Outcome::Step(next) => {
                if returning {
                    loci.finish();
                }
                loci.enter(next.control());
                state = next;
            },
            | Outcome::Done(ty) => return Ok(ty),
            | Outcome::Error {
                error,
                state: failure,
            } => {
                return Err(Box::new(MachineFailure {
                    error,
                    state: failure,
                    origins: loci.current_chain(),
                }));
            },
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
    failure: &MachineFailure,
    lowered: &Lowered,
    base_len: ContextLength,
) -> Diagnostic
{
    let detail = detail_of(&failure.error);
    let message = message_of(&failure.error);
    let annotations = failure
        .origins
        .last()
        .and_then(|&origin| {
            failure_annotation(&failure.error, origin, primary_label(&detail), lowered)
        })
        .into_iter()
        .collect();
    let stack = failure.state.stack();
    let enclosing = failure
        .origins
        .get(.. failure.origins.len().saturating_sub(1))
        .filter(|origins| origins.len() == stack.len());
    let contexts = stack
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            diagnostic_context(
                frame,
                enclosing.and_then(|origins| origins.get(index)),
                lowered,
            )
        })
        .collect();
    Diagnostic {
        detail,
        item: Some(usize::from(item_index)),
        code: message.code(),
        severity: Severity::Error,
        message,
        annotations,
        expr: offending_expr(failure.state.control()),
        contexts,
        ctx: bindings_beyond(failure.state.ctx(), base_len),
    }
}

/// Maps a [`TypeError`] to its [`DiagnosticDetail`].
fn detail_of(error: &TypeError) -> DiagnosticDetail
{
    match *error {
        | TypeError::TypeMismatch(ref mismatch) => DiagnosticDetail::TypeMismatch {
            expected: render_type_operand(&mismatch.expected),
            actual: render_type_operand(&mismatch.actual),
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
        // PLACEHOLDER. The formation refusal is threaded through the catch-all
        // this variant exists for, so nothing is lost and nothing is
        // misattributed -- but this is NOT the diagnostic the arc owes.
        //
        // What it owes is a structured, source-ranged detail naming the
        // undeclared type AT THE SIGNATURE that mentions it. The defect being
        // repaired is precisely a diagnostic that is confidently wrong about
        // which of two things is broken: today `def k : NoSuchType` with
        // `def k = 1` refuses with a TypeMismatch blaming the BODY, telling the
        // author their `1` is not a `NoSuchType`. Routing this to `Other`
        // stops the engine asserting the wrong half; it does not yet point at
        // the right one. Owned by `gandr-h19l`.
        | TypeError::IllFormedType(_) => DiagnosticDetail::Other,
    }
}

/// The shortest useful semantic label for a diagnostic's primary locus.
fn primary_label(detail: &DiagnosticDetail) -> String
{
    match *detail {
        | DiagnosticDetail::TypeMismatch {
            ref expected,
            ref actual,
        } => format!("expected {expected}, found {actual}"),
        | DiagnosticDetail::ShapeMismatch {
            ref expected_shape,
            ref actual,
        } => format!("expected {expected_shape}, found {actual}"),
        | DiagnosticDetail::StuckExpr { ref hint } => hint.clone(),
        | DiagnosticDetail::UnboundVariable { .. } => "not found in this scope".to_owned(),
        | DiagnosticDetail::GradeError {
            ref lower,
            ref upper,
        } => format!("required {lower} ≤ {upper}"),
        | DiagnosticDetail::Attribute {
            problem: AttributeProblem::Unknown { .. },
            ..
        } => "unknown attribute".to_owned(),
        | DiagnosticDetail::Attribute {
            problem: AttributeProblem::Duplicate,
            ..
        } => "duplicate attribute".to_owned(),
        | DiagnosticDetail::Attribute {
            problem: AttributeProblem::MissingPayload,
            ..
        } => "payload required".to_owned(),
        | DiagnosticDetail::Attribute {
            problem: AttributeProblem::NonValuePayload,
            ..
        } => "payload must be a value".to_owned(),
        | DiagnosticDetail::ShadowedName { ref path } => {
            format!("this declaration shadows {path}")
        },
        | DiagnosticDetail::Other => "reported here".to_owned(),
    }
}

/// The localizable message template and arguments for a type error.
///
/// Declared datatype operands use their surface spelling; every other operand
/// retains the core error's stable `Debug` representation.
#[inline]
#[must_use]
pub fn message_of(error: &TypeError) -> DiagnosticMessage
{
    match *error {
        | TypeError::TypeMismatch(ref mismatch) => DiagnosticMessage::TypeMismatch {
            expected: render_type_operand(&mismatch.expected),
            actual: render_type_operand(&mismatch.actual),
        },
        | TypeError::ShapeMismatch {
            expected,
            ref actual,
        } => DiagnosticMessage::ShapeMismatch {
            expected_shape: expected.to_owned(),
            actual: render_type_operand(actual),
        },
        | TypeError::StuckExpr { ref expr, hint } => DiagnosticMessage::StuckExpression {
            expression: format!("{expr:?}"),
            hint: hint.to_owned(),
        },
        | TypeError::UnboundVariable { ref name } => {
            DiagnosticMessage::UnboundVariable { name: name.clone() }
        },
        // PLACEHOLDER, paired with the `DiagnosticDetail::Other` arm above; see
        // the comment there for what this owes. The refusal's own Display text
        // survives here, so the message names the offending type even while
        // the structured detail does not.
        | TypeError::IllFormedType(ref refusal) => DiagnosticMessage::Other {
            message: refusal.to_string(),
        },
        | TypeError::GradeError { lower, upper } => DiagnosticMessage::GradeOrder {
            lower: format!("{lower:?}"),
            upper: format!("{upper:?}"),
        },
    }
}

/// Renders a diagnostic type operand through the shared presentation
/// renderer ([`render::ty`]) — the same spelling the REPL transcript and the
/// language-server hover read — whenever every node has a shared spelling.
/// Unsupported nodes fall back to the raw `Debug` form.
fn render_type_operand(ty: &Ty) -> String
{
    render::faithful_ty(ty).unwrap_or_else(|| format!("{ty:?}"))
}

/// Resolves the exact failing occurrence to a primary source annotation.
///
/// A grade error selects the explicit grade facet retained by lowering; every
/// other error uses the semantic node's own range. No missing location is
/// replaced with an enclosing-item fiction.
fn failure_annotation(
    error: &TypeError,
    origin: OriginNodeId,
    label: String,
    lowered: &Lowered,
) -> Option<DiagnosticAnnotation>
{
    let entry = lowered.origin.get(origin)?;
    let byte_range = if matches!(*error, TypeError::GradeError { .. }) {
        lowered
            .origin
            .facets(origin)
            .iter()
            .find(|facet| facet.kind == OriginFacetKind::Grade)
            .map_or_else(
                || entry.byte_range.clone(),
                |facet| facet.byte_range.clone(),
            )
    }
    else {
        entry.byte_range.clone()
    };
    Some(DiagnosticAnnotation {
        kind: DiagnosticAnnotationKind::Primary,
        span: Span::from(byte_range),
        label: Some(label),
        elaboration: entry.elaboration.map(|elab| format!("{elab:?}")),
    })
}

/// Resolves one pending machine obligation to its own contextual annotation.
fn context_annotation(
    origin: OriginNodeId,
    lowered: &Lowered,
) -> Option<DiagnosticAnnotation>
{
    let entry = lowered.origin.get(origin)?;
    Some(DiagnosticAnnotation {
        kind: DiagnosticAnnotationKind::Context,
        span: Span::from(entry.byte_range.clone()),
        label: None,
        elaboration: entry.elaboration.map(|elab| format!("{elab:?}")),
    })
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

/// Renders one machine frame as a semantic diagnostic context.
fn diagnostic_context(
    frame: &Frame,
    origin: Option<&OriginNodeId>,
    lowered: &Lowered,
) -> DiagnosticContext
{
    let (role, prose, binder) = frame_description(frame);
    DiagnosticContext {
        role: role.0.to_owned(),
        prose,
        binder,
        annotations: origin
            .and_then(|&origin| context_annotation(origin, lowered))
            .into_iter()
            .collect(),
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
/// Marks come from [`gandr_core_checker::discipline::mark`] (`semantic marker`)
/// and retain (item, path, mark) order.
///
/// Each item is marked against its ascription (checking mode) or in inference
/// mode — exactly the direction [`initial_state`](crate::goals) drives the
/// machine with (via `mark_item`) — so the marks detect the same type errors
/// as the diagnostics (their spans and some kind labels differ), while the
/// marking additionally decorates *every* node (it is total over type errors).
/// A node's structural path is mapped to a source span through the
/// [`OriginMap`](crate::origin) compatibility path index; a mark whose node is
/// not addressable there (a reified-stack interior) is dropped.
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
/// - provides: the incremental-pipeline design's marks for the report envelope.
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
        // interior bonus entry). Surface lowering never mints a `Stk`, so this
        // is forward-compat only.
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
        | Mark::PatternHole(hole) => MarkDetail::PatternHole { hole },
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
