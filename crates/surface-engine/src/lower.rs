//! CST → core-CBPV lowering.
//!
//! [`lower_source`] parses with the melder front-end
//! ([`crate::synnode::SynTree`], the lowering contract) viewed through the
//! [`crate::synnode::SynNode`] adapter, walks the CST, and produces [`Lowered`]
//! — items plus an [`crate::origin::OriginMap`]. Lowering is **syntax-directed
//! and total-or-structured-error**: every construct either lowers or yields a
//! [`LowerError`]; nothing panics.
//!
//! # Strictness
//!
//! Two modes ([`Strictness`]):
//!
//! - [`Strictness::Strict`] ([`lower_source`]): syntax errors and
//!   out-of-fragment constructs are [`LowerError`]s.
//! - [`Strictness::Total`] ([`lower_source_total`]): lowering is **total on
//!   parseable input** — every input-shaped failure becomes a
//!   [`Value::Hole`]/[`Comp::Hole`] carrying a [`crate::origin::HoleNote`]
//!   (what was elided) and the elided region's byte range in the origin map;
//!   `ERROR`/`MISSING` CST nodes lower to holes wherever they occur (item-,
//!   statement-, and expression-local — recovery granularity is the consuming
//!   position, at least statement-local). Only the two *infrastructure*
//!   failures ([`LowerError::ParserUnavailable`], [`LowerError::ParseFailed`])
//!   survive as errors; neither depends on the input text.
//!
//! Conversion table (strict error → total-mode lowering):
//!
//! | Strict `LowerError`        | Total mode                                                                |
//! | -------------------------- | ------------------------------------------------------------------------- |
//! | `Syntax` (ERROR/MISSING)   | hole at the error region (`HoleNote::SyntaxError`)                        |
//! | `Unsupported { kind }`     | hole at the construct (`HoleNote::UnsupportedForm`)                       |
//! | `MissingCaseArm`           | the missing arm's body is a hole (`HoleNote::MissingCaseArm`)             |
//! | `EmptyBlock`               | the missing tail is a hole (`HoleNote::EmptyBlock`)                       |
//! | `InvalidIntegerLiteral`    | hole at the literal (`HoleNote::InvalidIntegerLiteral`)                   |
//! | `InvalidGrade`             | the whole graded construct is a hole (`HoleNote::InvalidGrade`)           |
//! | `DanglingSignature`        | an item whose term is a hole, ascribed the signature (`MissingDefinition`)|
//! | `DuplicateModuleMember`    | duplicate member is elided as a hole in sequence (module keeps first field) |
//! | `TypeSortMismatch`         | the type position lowers to `Unknown` (no note; the `Unknown` *is* the signal) |
//! | `MalformedNode`            | hole at the node (`HoleNote::MalformedNode`)                              |
//!
//! **User-written holes** (`?` / `?name`, [`node_kinds::HOLE`]) are the one
//! hole source that is *not* a recovered [`LowerError`]: a hole the user typed
//! is a legitimate axiom, lowered to a [`Value::Hole`]/[`Comp::Hole`] carrying
//! [`HoleNote::UserHole`] in **both** strict and total mode (it never gates on
//! [`Strictness::Total`]). Its sort follows the consuming position exactly as a
//! recovery hole's does; see [`Lowerer::user_value_hole`].
//!
//! [SPECULATIVE DECISION] **Type-position elisions carry no note**:
//! out-of-fragment, sort-mismatched, or damaged *types* lower to
//! `ValueType::Unknown`/`CompType::Unknown` in total mode (types are not
//! terms, so they cannot be holes), and the `Unknown` inside the
//! ascription/annotation is itself the visible signal — it surfaces in the
//! goals report as a partially-unknown expected type.
//!
//! [SPECULATIVE DECISION] **Skipped-arm coarseness**: a `case` arm or `co`
//! field that cannot lower in total mode (unknown constructor, duplicate,
//! malformed pattern) is *skipped*; if that leaves an `Inl`/`Inr` arm or
//! `fst`/`snd` field unfilled, the unfilled slot becomes a hole with its
//! missing-slot note. The skipped arm's own content is elided without a
//! dedicated note (one note slot per origin entry); tylr-style obligations
//! (D2 reopen trigger 2) are the principled upgrade.
//!
//! # Sort mediation (the two syntax-directed coercions)
//!
//! Core CBPV separates values from computations; the surface does not. Two
//! coercions, both decidable at lowering with no type information, mediate:
//!
//! - **Force sugar** (the design's rule): a call head — or, by the same
//!   rationale, a projection target — that lowers to a *value* is wrapped in
//!   `Force`, because the principal premise must be a computation.
//! - **`Ret`/`Bind` mediation** [SPECULATIVE DECISION; the design requires `ret
//!   (x * x)` to lower but does not name the mechanism]: a value in computation
//!   position (block tail, arm body, …) is wrapped in `Ret`
//!   ([`crate::origin::ElabKind::RetCoercion`]); a computation in value
//!   position (a `ret` payload, call argument, tuple component, …) is *hoisted*
//!   — bound to a fresh `%tmpN` variable by a synthesized `Bind` around the
//!   nearest enclosing computation, in left-to-right evaluation order
//!   ([`crate::origin::ElabKind::BindHoist`]). Fresh names start with `%`,
//!   which no surface identifier can, so capture is impossible.
//!
//! # Node kinds
//!
//! All node-kind strings live in the one table of [`node_kinds`] (§4.2: the
//! lowerer keys on node kinds, never keyword spellings).

pub(crate) mod codata;
pub(crate) mod data;
pub mod node_kinds;
mod recursion_surface;
#[cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the worklist modules place their public drivers before the step helpers for readability; the caller-before-callee rule conflicts with that deliberate top-down layout pending a layout redesign"
    )
)]
mod recursive;
#[cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the type-lowering module places its public drivers before the schedule/assemble step helpers for readability; the caller-before-callee rule conflicts with that deliberate top-down layout pending a layout redesign"
    )
)]
pub(crate) mod types;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::rc::Rc;

use gandr_core_checker::grade::Grade;
use gandr_core_checker::nominal::GandrSort;
use gandr_core_checker::prim::NativePrim;
use gandr_core_checker::syntax::ArenaBridgeError;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::CompNodeId;
use gandr_core_checker::syntax::FlatArena;
use gandr_core_checker::syntax::Side;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::syntax::ValueNodeId;
use gandr_core_checker::syntax::WalkBase;
use gandr_core_checker::syntax::WalkMotive;
use gandr_core_checker::types::CompType;
use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;
use gandr_core_incremental::region::Item;
use gandr_surface_parser::Oblig;
use gandr_surface_parser::ObligationInstance;
use gandr_surface_syntax::NodeId;
use gandr_surface_syntax::StableHash;
use gandr_theory_nominal_automata::Gensym;
use thiserror::Error;

use crate::boundary::DefinitionName;
use crate::boundary::FreshHoleId;
use crate::boundary::HostEscapeFlag;
use crate::boundary::ItemIndex;
use crate::boundary::LambdaArity;
use crate::boundary::ListCaseFlag;
use crate::boundary::NodeText;
use crate::boundary::OperatorText;
use crate::boundary::PipelineSource;
use crate::boundary::ShellWordContinuation;
use crate::boundary::SignificantIndex;
use crate::boundary::SourceOffset;
use crate::boundary::SourceRange;
use crate::boundary::SyntaxField;
use crate::boundary::SyntaxKind;
use crate::boundary::TotalMode;
use crate::ffi::CType;
use crate::ffi::ForeignFn;
use crate::ffi::ForeignModule;
use crate::ffi::ForeignParam;
use crate::namespace::Binding;
use crate::namespace::Collision;
use crate::namespace::EventKind;
use crate::namespace::EventRejection;
use crate::namespace::Modifier;
use crate::namespace::NamePath;
use crate::namespace::NamespaceEventHandler;
use crate::namespace::RejectionReason;
use crate::namespace::Scope;
use crate::namespace::ScopeError;
use crate::namespace::Segment;
use crate::namespace::Trie;
use crate::origin::ElabKind;
use crate::origin::HoleNote;
use crate::origin::OriginEntry;
use crate::origin::OriginMap;
use crate::origin::OriginNode;
use crate::synnode::SynNode;
use crate::synnode::SynTree;

/// Result type for lowering operations.
pub type LowerResult<T> = Result<T, LowerError>;

/// A structured lowering failure: one constructor per failure mode, in the
/// `gandr-core-checker` error convention. Lowering never panics — every input
/// is either [`Lowered`] or exactly one of these.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum LowerError
{
    /// The tree-sitter parser rejected the gandr grammar (version skew).
    #[error("parser initialization failed: {detail}")]
    ParserUnavailable
    {
        /// The underlying `tree_sitter::LanguageError`, rendered.
        detail: String,
    },

    /// Parsing returned no tree (only possible under cancellation/timeout,
    /// which this crate does not configure; kept for totality).
    #[error("parsing produced no tree")]
    ParseFailed,

    /// The parse tree contains an `ERROR` or `MISSING` node. Strict mode stops
    /// here; total mode lowers these regions to holes instead.
    #[error("syntax error at bytes {byte_range:?}")]
    Syntax
    {
        /// The byte range of the first `ERROR`/`MISSING` node.
        byte_range: SourceRange,
    },

    /// A construct outside the covered fragment (sessions, sharing,
    /// worlds, `leta`, type abstraction/instantiation, shell blocks,
    /// strings, …). Total mode converts these to holes-with-notes.
    #[error("unsupported construct `{kind}` at bytes {byte_range:?}")]
    Unsupported
    {
        /// The node kind of the unsupported construct.
        kind: SyntaxKind,
        /// The construct's byte range.
        byte_range: SourceRange,
    },

    /// A bare fix-bound name occurred inside its recursive scope.
    #[error(
        "unmarked recursive reference `{name}` at bytes {byte_range:?}; did you mean `{suggestion}`?"
    )]
    UnmarkedRecursiveReference
    {
        /// The fix-bound name that required call-site evidence.
        name: String,
        /// The marked spelling suggested to the author.
        suggestion: String,
        /// The bare occurrence's source range.
        byte_range: SourceRange,
    },

    /// A direction marker targeted no definition in the enclosing recursive
    /// scope.
    #[error(
        "marked reference `{target}` at bytes {byte_range:?} does not target the enclosing recursive scope"
    )]
    MarkedReferenceOutsideRecursiveScope
    {
        /// The marked expression target.
        target: String,
        /// The marked occurrence's source range.
        byte_range: SourceRange,
    },

    /// Named descent measures are reserved but not implemented.
    #[error(
        "recursion marker resident `{resident}` at bytes {byte_range:?} is reserved for named measures and is not implemented"
    )]
    ReservedNamedMeasure
    {
        /// The declined resident.
        resident: String,
        /// The resident's source range.
        byte_range: SourceRange,
    },

    /// Explicit erased instantiation is reserved but not implemented.
    #[error(
        "recursion marker resident `{resident}` at bytes {byte_range:?} is reserved for explicit instantiation and is not implemented"
    )]
    ReservedExplicitInstantiation
    {
        /// The declined resident.
        resident: String,
        /// The resident's source range.
        byte_range: SourceRange,
    },

    /// Explicit size instantiation is reserved but not implemented.
    #[error(
        "recursion marker resident `{resident}` at bytes {byte_range:?} is reserved for explicit sizes and is not implemented"
    )]
    ReservedExplicitSize
    {
        /// The declined resident.
        resident: String,
        /// The resident's source range.
        byte_range: SourceRange,
    },

    /// Per-call cost bounds are reserved but not implemented.
    #[error(
        "recursion marker resident `{resident}` at bytes {byte_range:?} is reserved for cost bounds and is not implemented"
    )]
    ReservedCostBound
    {
        /// The declined resident.
        resident: String,
        /// The resident's source range.
        byte_range: SourceRange,
    },

    /// Tail-call assertions are reserved but not implemented.
    #[error(
        "recursion marker resident `tail` at bytes {byte_range:?} is reserved for tail-call assertions and is not implemented"
    )]
    ReservedTailAssertion
    {
        /// The resident's source range.
        byte_range: SourceRange,
    },

    /// A `number` token that is not an `i64` integer literal (floats,
    /// exponents, overflow) — `Value::Int` is the only literal in core.
    #[error("invalid integer literal `{text}` at bytes {byte_range:?}")]
    InvalidIntegerLiteral
    {
        /// The literal's source text.
        text: String,
        /// The literal's byte range.
        byte_range: SourceRange,
    },

    /// A grade annotation that is not a `u64` numeral or `ω` (grade
    /// variables are Stage 2).
    #[error("invalid grade `{text}` at bytes {byte_range:?}")]
    InvalidGrade
    {
        /// The grade's source text.
        text: String,
        /// The grade's byte range.
        byte_range: SourceRange,
    },

    /// A `case` over `Inl`/`Inr` is missing one arm (an error in strict
    /// mode; it becomes a hole body in total mode).
    #[error("missing `{constructor}` case arm at bytes {byte_range:?}")]
    MissingCaseArm
    {
        /// The constructor whose arm is missing (`Inl` or `Inr`).
        constructor: &'static str,
        /// The `case` expression's byte range.
        byte_range: SourceRange,
    },

    /// A block with no tail expression: the bind-chain has no final
    /// computation. [SPECULATIVE DECISION] No default tail is invented
    /// (`ret ()` would be a semantic choice); total mode makes this a hole.
    #[error("block has no tail expression at bytes {byte_range:?}")]
    EmptyBlock
    {
        /// The block's byte range.
        byte_range: SourceRange,
    },

    /// A `def name : T;` signature with no following matching definition.
    /// [SPECULATIVE DECISION] Signatures attach to the nearest *following*
    /// `def` of the same name; unmatched ones are errors rather than
    /// silently dropped items (a [`Item`] has no term to carry).
    #[error("signature for `{name}` has no matching definition at bytes {byte_range:?}")]
    DanglingSignature
    {
        /// The signature's name.
        name: String,
        /// The signature's byte range.
        byte_range: SourceRange,
    },

    /// A checked module body defines the same field name more than once.
    /// Strict mode rejects at the duplicate definition; total mode keeps the
    /// first field and lowers the duplicate site to a discarded hole so the
    /// recovery remains source-ranged and ordered.
    #[error("module member `{name}` is defined more than once at bytes {byte_range:?}")]
    DuplicateModuleMember
    {
        /// The duplicated member name.
        name: String,
        /// The duplicate definition's byte range.
        byte_range: SourceRange,
    },

    /// A type of the wrong polarity for its position (e.g. `F` applied to a
    /// computation type, or a computation-sorted ascription on a value
    /// annotation).
    #[error("expected {expected} at bytes {byte_range:?}, but `{kind}` lowers to the other sort")]
    TypeSortMismatch
    {
        /// The required sort, as prose ("a value type" / "a computation
        /// type").
        expected: &'static str,
        /// The node kind of the offending type.
        kind: SyntaxKind,
        /// The offending type's byte range.
        byte_range: SourceRange,
    },

    /// A node missing grammar-guaranteed structure (a required field or
    /// child). Unreachable on error-free grammar-conformant trees — the
    /// upfront [`LowerError::Syntax`] scan rejects the rest — but kept as a
    /// structured error so lowering stays total on any tree.
    #[error("malformed `{kind}` node at bytes {byte_range:?}")]
    MalformedNode
    {
        /// The malformed node's kind.
        kind: SyntaxKind,
        /// The malformed node's byte range.
        byte_range: SourceRange,
    },

    /// A checked flat-arena bridge allocation or readback failed.
    #[error("arena bridge failed: {error:?}")]
    ArenaBridge
    {
        /// The arena bridge failure.
        error: ArenaBridgeError,
    },

    /// A copattern clause `.π => e` names an observation `π` the codata type
    /// `C` (from the definition's `-> C` result) does not declare (the design
    /// record §4.1 coverage). Strict mode rejects; total mode drops the
    /// clause (the skipped-arm coarseness).
    #[error(
        "copattern clause observes `{observation}`, not declared by codata `{codata}`, at bytes {byte_range:?}"
    )]
    UnknownObservation
    {
        /// The undeclared observation the clause projects.
        observation: String,
        /// The codata type named by the definition's result.
        codata: String,
        /// The clause's byte range.
        byte_range: SourceRange,
    },

    /// Two copattern clauses answer the same observation `π` (the lowering
    /// contract §4.1: an observation is answered *exactly once*). Strict
    /// mode rejects; total mode keeps the last clause (last-wins, as
    /// duplicate record fields).
    #[error(
        "observation `{observation}` is answered by more than one copattern clause at bytes {byte_range:?}"
    )]
    DuplicateObservation
    {
        /// The duplicated observation.
        observation: String,
        /// The duplicate clause's byte range.
        byte_range: SourceRange,
    },

    /// A copattern definition against codata `C` leaves an observation `π`
    /// unanswered (the copattern-coverage contract: coverage requires every
    /// observation answered). Strict mode rejects here; total mode omits
    /// the field and lets the carrier ascription surface the gap as a
    /// record type mismatch naming the missing observation (the coverage
    /// diagnostic).
    #[error(
        "codata `{codata}` definition does not answer observation `{observation}` at bytes {byte_range:?}"
    )]
    MissingObservation
    {
        /// The unanswered observation.
        observation: String,
        /// The codata type named by the definition's result.
        codata: String,
        /// The definition's byte range.
        byte_range: SourceRange,
    },

    /// The namespace engine rejected an import transformation or merge.
    #[error("import namespace lowering failed at bytes {byte_range:?}: {error}")]
    Namespace
    {
        /// The failed namespace operation.
        error: ScopeError,
        /// The import declaration's source range.
        byte_range: SourceRange,
    },
    /// The persistent import table cannot represent another source-order
    /// declaration index.
    #[error("too many import declarations at bytes {byte_range:?}")]
    ImportIndexOverflow
    {
        /// The import declaration's source range.
        byte_range: SourceRange,
    },
}

impl From<ArenaBridgeError> for LowerError
{
    #[inline]
    fn from(error: ArenaBridgeError) -> Self
    {
        Self::ArenaBridge { error }
    }
}

/// The lowering mode: fail-fast or total (see the module doc's
/// conversion table).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Strictness
{
    /// Fail-fast: syntax errors and out-of-fragment constructs are
    /// structured [`LowerError`]s.
    #[default]
    Strict,
    /// Total on parseable input — input-shaped failures
    /// lower to holes carrying [`HoleNote`]s.
    Total,
}

/// One entity attribute lexed off a `def` item's leading `@[…]` block, before
/// registry resolution and payload typing (proposal-attributes.md §2).
///
/// Lowering is deliberately name-and-payload only: the schema lookup, the
/// payload typing against that schema, duplicate/arity checks, and the
/// hash-neutral side table are the [`crate::attributes`] pass's job (the
/// lowerer stays typing-free, mirroring the CST → core discipline). The
/// payload is lowered here to its value fragment so the attribute pass can
/// type it with the ordinary checker without re-walking the CST.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawAttr
{
    /// The index of the item this attribute annotates (into
    /// [`Lowered::items`]), the stable-`NodeId` stand-in the side table
    /// keys on (see the [`crate::attributes`] as-built note).
    pub item: usize,
    /// The attribute name (`doc`, `deprecated`, …).
    pub name: String,
    /// The name token's byte range (the diagnostic anchor for an unknown or
    /// duplicate attribute).
    pub name_range: SourceRange,
    /// The whole `@[…]` block's byte range (the projection span).
    pub block_range: SourceRange,
    /// The lowered payload, absent for a bare marker (`@[name]`).
    pub payload: Option<RawPayload>,
}

/// A lowered attribute payload: its value-fragment term plus whether it is in
/// the value fragment at all (proposal-attributes.md §3.3 — a payload is data,
/// never an `F`-computation).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawPayload
{
    /// The lowered payload value (a placeholder [`Value::Unit`] when
    /// [`Self::is_value_fragment`] is `false` — the attribute pass rejects it
    /// before reading the value).
    pub value: Value,
    /// Whether the payload lowered to a pure value with no hoisted
    /// computations; `false` for a computation payload (a shell block, a
    /// binary operator, …), which the attribute pass rejects as non-value.
    pub is_value_fragment: bool,
    /// The payload's byte range (the ill-typed-payload diagnostic anchor).
    pub range: SourceRange,
}

/// The source-order index of an import declaration in a lowered file.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ImportIndex(pub usize);

/// Namespace policy for source imports: one alias names one source.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ImportNamespaceHandler;

impl NamespaceEventHandler<ImportIndex, SourceRange> for ImportNamespaceHandler
{
    type Label = ();

    #[inline]
    fn not_found(
        &mut self,
        _path: &NamePath,
    ) -> Result<(), EventRejection>
    {
        Ok(())
    }

    #[inline]
    fn shadow(
        &mut self,
        path: &NamePath,
        _collision: Collision<ImportIndex, SourceRange>,
    ) -> Result<Binding<ImportIndex, SourceRange>, EventRejection>
    {
        Err(EventRejection::new(
            EventKind::Shadow,
            path.clone(),
            RejectionReason::from("an import alias must name one source"),
        ))
    }

    #[inline]
    fn hook(
        &mut self,
        _path: &NamePath,
        _label: &(),
        subject: Trie<ImportIndex, SourceRange>,
    ) -> Result<Trie<ImportIndex, SourceRange>, EventRejection>
    {
        Ok(subject)
    }
}

/// One lowered `import "URI" as name ;` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportDeclaration
{
    /// The URI string with its surrounding quotes removed.
    pub uri: String,
    /// The one-segment alias supplied by the `as` clause.
    pub alias: Segment,
}

/// The result of lowering a source file: items plus the origin side table.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Lowered
{
    /// The lowered items, in source order.
    ///
    /// The item type is the incrementality crate's parser-agnostic
    /// [`Item`] — a name, an ascription, and a lowered term — so a lowering
    /// crosses the changed-region seam without a projection. This lowerer fills
    /// [`Item::ascription`] from an explicit `def name : T;` signature or from
    /// the type the `def`-function sugar derives; the signature wins when both
    /// exist, because it is the user's stated contract.
    pub items: Vec<Item>,
    /// The foreign modules declared by `extern` blocks, in source order
    /// (proposal-ffi.md §2). `extern` blocks are declarations, not runnable
    /// items, so they contribute no [`Item`]; a native FFI handler
    /// consumes these to resolve C symbols and marshal the
    /// boundary (§5.1).
    pub foreign: Vec<ForeignModule>,
    /// Import declarations in source order. Each declaration contributes a
    /// root binding to [`Self::import_scope`] and qualifies it through
    /// [`Modifier::alias_as`]; imports are declarations, not runnable items.
    pub imports: Vec<ImportDeclaration>,
    /// The `codata` blocks declared by *this* source, keyed by codata type name
    /// (the codata-declaration contract). Like [`Self::foreign`], a `codata`
    /// block is a declaration, not a runnable item — it yields no
    /// [`Item`]. Read through [`Self::codata`]: a REPL
    /// [`crate::session::Session`] merges these into its persistent
    /// registry so a later submission's copattern definition sees the
    /// declaration (the negative-declaration analogue of the `extern`
    /// bridge).
    codata: BTreeMap<String, codata::CodataDecl>,
    /// The `data` blocks declared by *this* source, keyed by datatype name
    /// (the data-declaration contract). Like [`Self::codata`], a `data` block
    /// is a declaration, not a runnable item — it yields no
    /// [`Item`]. Read through [`Self::data`]: a renderer consults
    /// the declaration table's constructor enumeration and minted `DataId`
    /// to print a declared-constructor value as its `tag + name`
    /// (`Some(3)`, `Red`) rather than the structural carrier.
    data: BTreeMap<String, data::DataDecl>,
    /// Stable-ID-keyed CST origins, with compatibility path readback (see
    /// [`crate::origin`]).
    pub origin: OriginMap,
    /// The raw entity attributes off each item's leading `@[…]` blocks, in
    /// source order (proposal-attributes.md §2); resolved and typed by the
    /// [`crate::attributes`] pass.
    pub attributes: Vec<RawAttr>,
    /// The namespace scope after this source's imports have been merged with
    /// the scope supplied to seeded lowering. Imports touch only its visible
    /// namespace, so the export namespace remains empty.
    import_scope: Scope<ImportIndex, SourceRange>,
}

impl Lowered
{
    /// This source's own `codata` declarations, keyed by codata type name —
    /// the negative-declaration bridge a REPL session persists across
    /// submissions, analogous to `extern` declarations.
    #[inline]
    #[must_use]
    pub(crate) fn codata(&self) -> &BTreeMap<String, codata::CodataDecl>
    {
        &self.codata
    }

    /// This source's own `data` declarations (the data-declaration contract),
    /// keyed by datatype name — the decl table (constructor enumeration +
    /// minted `DataId`) a renderer resolves a declared-constructor value's
    /// `tag` against to print its constructor name.
    #[inline]
    #[must_use]
    pub(crate) fn data(&self) -> &BTreeMap<String, data::DataDecl>
    {
        &self.data
    }

    /// The namespace scope built while lowering this source's imports.
    #[inline]
    #[must_use]
    pub const fn import_scope(&self) -> &Scope<ImportIndex, SourceRange>
    {
        &self.import_scope
    }

    /// A deterministic rendering for golden tests: items in debug format
    /// plus the [`OriginMap::snapshot`] (which records each entry's
    /// reproducible merkle `cst_hash` and omits the positional CST arena
    /// slot).
    #[inline]
    #[must_use]
    pub fn debug_snapshot(&self) -> String
    {
        let mut lines: Vec<String> = Vec::new();
        for (index, item) in self.items.iter().enumerate() {
            lines.push(format!("=== item {index}: {:?} ===\n", item.name));
            lines.push(format!("ascription: {:#?}\n", item.ascription));
            lines.push(format!("term: {:#?}\n", item.term));
        }
        lines.push("=== origin ===\n".to_owned());
        lines.push(self.origin.snapshot());
        lines.concat()
    }
}

/// Parses `source` and lowers it to core CBPV in [`Strictness::Strict`]
/// mode.
///
/// # Contract
/// - ensures: on success every top-level item lowers, yielding a `Lowered`
///   (items in source order plus the `OriginMap`); no holes are synthesized.
/// - provides: the strict lowering of `source`.
/// - fails: the first out-of-fragment construct, `ERROR`/`MISSING` region, or
///   malformed node aborts lowering; per-region recovery is the `Total` mode.
/// - panics: none.
///
/// # Errors
///
/// Returns the first [`LowerError`] encountered, walking items in source
/// order (fail-fast over the whole file; per-region recovery is
/// [`lower_source_total`]'s hole story).
#[inline]
pub fn lower_source(source: PipelineSource<'_>) -> LowerResult<Lowered>
{
    lower_source_with(source, Strictness::Strict)
}

/// Parses `source` and lowers it to core CBPV in [`Strictness::Total`] mode:
/// total on parseable input — see the module doc's conversion
/// table.
///
/// # Contract
/// - ensures: every parseable input lowers to a `Lowered`; out-of-fragment
///   constructs and error regions become `Value::Hole`/`Comp::Hole` holes, each
///   carrying a `HoleNote` and its elided byte range in the `OriginMap` (the
///   module doc's conversion table).
/// - provides: total lowering on parseable input (the goals-report seed).
/// - fails: only the input-independent infrastructure failures
///   `LowerError::ParserUnavailable` and `LowerError::ParseFailed`.
/// - panics: none.
///
/// # Errors
///
/// Only the infrastructure failures [`LowerError::ParserUnavailable`] and
/// [`LowerError::ParseFailed`] — neither depends on the input text; every
/// parseable input lowers.
#[inline]
pub fn lower_source_total(source: PipelineSource<'_>) -> LowerResult<Lowered>
{
    lower_source_with(source, Strictness::Total)
}

/// Parses `source` and lowers it to core CBPV under `strictness`.
///
/// # Contract
/// - ensures: lowers `source` under `strictness` — `Strictness::Strict` fails
///   fast on the first error, `Strictness::Total` recovers errors to holes.
/// - provides: the shared lowering entry that `lower_source` and
///   `lower_source_total` wrap.
/// - fails: per `strictness` — any `LowerError` in strict mode, or only the two
///   infrastructure failures in total mode.
/// - panics: none.
///
/// # Errors
///
/// As [`lower_source`] (strict) or [`lower_source_total`] (total).
#[inline]
pub fn lower_source_with(
    source: PipelineSource<'_>,
    strictness: Strictness,
) -> LowerResult<Lowered>
{
    lower_source_seeded(
        source,
        strictness,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &BTreeMap::new(),
        &Scope::new(),
        ImportIndex(0),
    )
}

/// Parses `source` and lowers it totally, seeded with `foreign` modules
/// declared by earlier submissions (the REPL session's persistent `extern`
/// registry — proposal-ffi.md §2).
///
/// The seeded modules are visible to this source's foreign-call elaboration
/// (§3.1), exactly as an earlier `def` is visible through the session context;
/// the source's own `extern` blocks are added on top. The returned
/// [`Lowered::foreign`] carries only *this* source's declared blocks (what the
/// caller should merge into its accumulating registry), not the seed.
///
/// # Contract
/// - ensures: a call `m.op(args)` whose `m` is in `foreign` (or declared in
///   `source`) elaborates to a `perform`; `Lowered::foreign` is this source's
///   own `extern` blocks.
/// - fails: only the two infrastructure failures (total mode).
/// - panics: none.
///
/// # Errors
///
/// [`LowerError::ParserUnavailable`] / [`LowerError::ParseFailed`].
#[inline]
pub fn lower_source_total_with_foreign(
    source: PipelineSource<'_>,
    foreign: &BTreeMap<String, ForeignModule>,
) -> LowerResult<Lowered>
{
    lower_source_seeded(
        source,
        Strictness::Total,
        foreign,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &Scope::new(),
        ImportIndex(0),
    )
}

/// Parses `source` and lowers it totally, seeded with both the `foreign`
/// (`extern`) and `codata` declaration registries a REPL session accumulated
/// from earlier submissions (proposal-ffi.md §2; the codata-declaration
/// contract). A later line's copattern definition `def rec f() -> C` is
/// coverage-checked against, and a later `s.π` observation is disambiguated by,
/// a `codata C` block declared on an earlier line — exactly as an `extern`
/// module or a `def` bridges lines. The returned [`Lowered::foreign`] /
/// [`Lowered::codata`] carry only *this* source's own declarations (what the
/// caller merges into its registries).
///
/// # Contract
/// - ensures: a copattern definition or observation elaborates against a
///   `codata` block in `codata` (or declared in `source`); `Lowered::codata` is
///   this source's own blocks.
/// - fails: only the two infrastructure failures (total mode).
/// - panics: none.
///
/// # Errors
///
/// [`LowerError::ParserUnavailable`] / [`LowerError::ParseFailed`].
#[inline]
pub(crate) fn lower_source_total_seeded(
    source: PipelineSource<'_>,
    foreign: &BTreeMap<String, ForeignModule>,
    codata: &BTreeMap<String, codata::CodataDecl>,
    data: &BTreeMap<String, data::DataDecl>,
    import_scope: &Scope<ImportIndex, SourceRange>,
    import_index_base: ImportIndex,
) -> LowerResult<Lowered>
{
    lower_source_seeded(
        source,
        Strictness::Total,
        foreign,
        codata,
        data,
        import_scope,
        import_index_base,
    )
}

/// The shared lowering entry, seeded with foreign-module, declaration, and
/// import-namespace registries (empty for the standalone [`lower_source`] /
/// [`lower_source_total`] entries).
fn lower_source_seeded(
    source: PipelineSource<'_>,
    strictness: Strictness,
    foreign: &BTreeMap<String, ForeignModule>,
    codata: &BTreeMap<String, codata::CodataDecl>,
    data: &BTreeMap<String, data::DataDecl>,
    import_scope: &Scope<ImportIndex, SourceRange>,
    import_index_base: ImportIndex,
) -> LowerResult<Lowered>
{
    // The melder CST front-end, viewed through the `SynNode` adapter in place
    // of the retired tree-sitter parse. The parse is total: any input yields a
    // tree plus its severity-ordered obligations.
    let tree = SynTree::parse(source.as_ref())?;
    let obligations = tree.obligations().to_vec();
    // Strict mode fails fast on the first syntactic obligation (in source
    // order, mirroring the retired first-`ERROR` scan). Total mode carries the
    // obligations into the lowerer, which resolves each recovery hole's note
    // from the obligation responsible for its span.
    if matches!(strictness, Strictness::Strict)
        && let Some(first) = first_obligation_in_source_order(&obligations)
    {
        return Err(LowerError::Syntax {
            byte_range: obligation_range(first),
        });
    }
    // Seed the projection-disambiguation set from the persisted declarations'
    // observation names, so an `s.π` in this source that observes a codata type
    // declared on an earlier submission routes to the observation elaboration.
    let observations = codata
        .values()
        .flat_map(codata::CodataDecl::observation_names)
        .map(ToOwned::to_owned)
        .collect();
    // Seed the constructor-resolution map from the persisted `data`
    // declarations, so a constructor application `C(v̄)` or `case v { … }` in
    // this source resolves against a `data D` block declared on an earlier
    // submission — the `codata` bridge applied to declared data (the design
    // record). The `collect_data` pre-pass then adds this source's own new
    // blocks.
    let constructors = data
        .iter()
        .flat_map(|(name, decl)| {
            decl.ctors
                .iter()
                .enumerate()
                .map(move |(tag, ctor)| (ctor.clone(), (name.clone(), tag)))
        })
        .collect();
    Lowerer {
        source,
        hoist: Gensym::new(GandrSort::TmpHoist),
        holes: Gensym::new(GandrSort::HoleAddr),
        strictness,
        foreign: foreign.clone(),
        codata: codata.clone(),
        observations,
        data: data.clone(),
        constructors,
        obligations,
        import_scope: import_scope.clone(),
        import_index_base,
    }
    .source_file(tree.root())
}

/// The obligation with the earliest start (then shortest span), the source-
/// order "first error" the strict entry reports — the melder buffers
/// obligations in accumulation order, not source order.
fn first_obligation_in_source_order(
    obligations: &[ObligationInstance]
) -> Option<&ObligationInstance>
{
    obligations
        .iter()
        .min_by_key(|obligation| (obligation.span.start(), obligation.span.end()))
}

/// Maps the obligation responsible for a grout-leaf recovery hole to the
/// [`HoleNote`] the hole carries (the melder's obligation-class → note
/// table). The
/// obligation carries only its class and span ([`ObligationInstance`]); the
/// fielded notes the design sketched (`MissingDelimiter { expected }`,
/// `IncompleteKeyword { expected }`, `AmbiguousOperatorPrecedence {
/// candidates }`) are derived from the span and
/// `source` where recoverable and otherwise simplified — the melder's
/// `expected`/`candidates` payload is not carried on `ObligationInstance`
/// (enriching it would thread new fields through the obligation hot path and
/// the CST persistence format), so `MissingDelimiter` keeps only `opened_at`
/// (the obligation span, which points at the unclosed opener),
/// `IncompleteKeyword` keeps only the `typed` span text, and
/// `AmbiguousOperatorPrecedence` is unfielded. `InconMeld` — a sort transition,
/// which resolves to `Unknown` with no note in type position — maps in the
/// residual term position to [`HoleNote::UnsupportedForm`] over `node_kind`,
/// matching the retired `TypeSortMismatch` note.
fn obligation_hole_note(
    obligation: ObligationInstance,
    node_kind: SyntaxKind,
    source: PipelineSource<'_>,
) -> HoleNote
{
    match obligation.class {
        | Oblig::MissingMeld => HoleNote::MissingOperand,
        | Oblig::MissingTile => HoleNote::MissingDelimiter {
            opened_at: obligation_range(&obligation),
        },
        | Oblig::IncompleteTile => HoleNote::IncompleteKeyword {
            typed: source
                .get(obligation_range(&obligation).0)
                .unwrap_or_default()
                .to_owned(),
        },
        | Oblig::UnmoldedTok => HoleNote::UnrecognizedToken,
        | Oblig::InconMeld => HoleNote::UnsupportedForm { kind: node_kind },
        | Oblig::ExtraMeld => HoleNote::AdjacentTerms,
        | Oblig::ReservedKeyword => HoleNote::ReservedKeyword,
        | Oblig::AmbiguousPrec => HoleNote::AmbiguousOperatorPrecedence,
    }
}

/// The `usize` byte range of an obligation's source span.
fn obligation_range(obligation: &ObligationInstance) -> SourceRange
{
    let start = usize::try_from(u32::from(obligation.span.start())).unwrap_or(0);
    let end = usize::try_from(u32::from(obligation.span.end())).unwrap_or(start);
    SourceRange(start .. end)
}

// --- Lowering outputs --------------------------------------------------------

/// A lowered value paired with its origin shadow tree.
struct VOut
{
    /// Canonical arena backing the lowered value root.
    arena: FlatArena,
    /// Root value node in [`Self::arena`].
    root: ValueNodeId,
    /// The value's origin shadow tree.
    origin: OriginNode,
}

impl VOut
{
    /// Allocates a legacy structural value into the canonical private carrier.
    fn from_legacy_value(
        value: &Value,
        origin: OriginNode,
    ) -> LowerResult<Self>
    {
        let mut arena = FlatArena::new();
        let root = arena.alloc_value(value)?;
        Ok(Self {
            arena,
            root,
            origin,
        })
    }

    /// Explicit compatibility boundary: reads the canonical root back to legacy
    /// structure.
    fn readback_value(&self) -> LowerResult<Value>
    {
        self.arena.value(self.root).map_err(LowerError::from)
    }
}

/// A lowered computation paired with its origin shadow tree.
struct COut
{
    /// Canonical arena backing the lowered computation root.
    arena: FlatArena,
    /// Root computation node in [`Self::arena`].
    root: CompNodeId,
    /// The computation's origin shadow tree.
    origin: OriginNode,
}

impl COut
{
    /// Allocates a legacy structural computation into the canonical private
    /// carrier.
    fn from_legacy_comp(
        comp: &Comp,
        origin: OriginNode,
    ) -> LowerResult<Self>
    {
        let mut arena = FlatArena::new();
        let root = arena.alloc_comp(comp)?;
        Ok(Self {
            arena,
            root,
            origin,
        })
    }

    /// Explicit compatibility boundary: reads the canonical root back to legacy
    /// structure.
    fn readback_comp(&self) -> LowerResult<Comp>
    {
        self.arena.comp(self.root).map_err(LowerError::from)
    }
}

/// A lowered expression of either sort.
enum EOut
{
    /// The expression lowered to a value.
    Value(
        /// The value output.
        VOut,
    ),
    /// The expression lowered to a computation. By construction these carry
    /// no pending hoists — computation contexts consume their own.
    Comp(
        /// The computation output.
        COut,
    ),
}

/// A pending hoist: a computation that occurred in value position, to be
/// bound by a synthesized `Bind` around the nearest enclosing computation.
struct Hoist
{
    /// The fresh `%tmpN` binder.
    name: String,
    /// The hoisted computation.
    bound: COut,
}

/// One source-ordered checked-module binding.
///
/// # Contract
/// - ensures: `bound` is evaluated exactly once and binds its result to
///   `binder` before the rest of the module body lowers.
/// - panics: none.
struct ModuleBinding
{
    /// The binder introduced by the generated `Bind`.
    binder: String,
    /// The computation that obtains the member value.
    bound: COut,
}

/// One candidate field of the checked-module record.
///
/// # Contract
/// - ensures: absent an explicit structural signature, `value` is returned
///   under `label`; repacking may omit it while `origin` continues to mirror
///   every value that remains.
/// - panics: none.
struct ModuleField
{
    /// The record label.
    label: String,
    /// The field value used in the final record.
    value: Value,
    /// The field value's origin shadow.
    origin: OriginNode,
}

/// Source-ordered state accumulated while lowering one module body.
///
/// # Contract
/// - ensures: signatures, definitions, bindings, and candidate return fields
///   remain associated with exactly one module declaration.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: separate collections preserve source evaluation while
///   structural matching filters only the final record payload.
/// - mutants: filter bindings with fields; share state across declarations.
/// - witnesses: `module_signature_repacking_hides_extra_members` and
///   `module_members_bind_in_source_order_and_return_record`.
#[derive(Default)]
struct ModuleBody
{
    /// Member signatures awaiting their matching definitions.
    sigs: Vec<PendingSig>,
    /// First definition range for each occupied member name.
    definitions: BTreeMap<String, SourceRange>,
    /// Computations evaluated in source order.
    bindings: Vec<ModuleBinding>,
    /// Candidate fields from which the final record is repacked.
    fields: Vec<ModuleField>,
}

/// A validated definition member ready for expression lowering.
///
/// # Contract
/// - ensures: `name` is reserved and `explicit` is the matching preceding
///   signature, when present.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: validation consumes a preceding signature exactly once.
/// - mutants: discard `explicit`; consume a signature after lowering.
/// - witnesses: `nested_member_signature_constrains_the_parent_binding` and
///   `duplicate_module_member_definition_is_rejected`.
struct ModuleMemberPlan
{
    /// The member's record label and generated binder.
    name: String,
    /// The matching member signature, when one preceded the definition.
    explicit: Option<Ty>,
}

/// One simple shell command accepted by the bootstrap shell lowering.
struct HostCommand
{
    /// The program name passed to `Exec::exec`.
    program: String,
    /// The argument vector passed to `Exec::exec`.
    args: Vec<Value>,
}

/// One decoded shell command fragment before lexical word boundaries are
/// restored.
struct ShellPart
{
    /// The decoded value payload for this fragment.
    value: ShellPartValue,
    /// The origin shadow for the value payload.
    origin: OriginNode,
    /// The source byte range occupied by this fragment.
    range: SourceRange,
}

/// One shell word after adjacent fragments have been regrouped.
struct ShellWord
{
    /// Fragments in source order.
    parts: Vec<ShellPart>,
    /// The source byte range occupied by the whole word.
    range: SourceRange,
}

/// The value payload carried by one decoded shell command fragment.
enum ShellPartValue
{
    /// A shell-literal word or quoted string.
    Literal(String),
    /// A host-language escape lowered to one typed value fragment.
    HostEscape(Value),
}

impl ShellPart
{
    /// Builds a literal command fragment with the source token as its origin.
    fn literal(
        text: String,
        node: SynNode<'_>,
    ) -> Self
    {
        Self {
            value: ShellPartValue::Literal(text),
            origin: OriginNode::leaf(entry(node, None)),
            range: node.byte_range(),
        }
    }

    /// Whether this fragment came from `$(...)`.
    fn is_host_escape(&self) -> HostEscapeFlag
    {
        matches!(self.value, ShellPartValue::HostEscape(_)).into()
    }
}

impl ShellWord
{
    /// Starts a word from its first fragment.
    fn new(part: ShellPart) -> Self
    {
        let range = part.range.clone();
        Self {
            parts: vec![part],
            range,
        }
    }

    /// Appends an adjacent fragment to this word.
    fn push(
        &mut self,
        part: ShellPart,
    )
    {
        self.range = SourceRange(self.range.start .. part.range.end);
        self.parts.push(part);
    }
}

/// A shell command plus its lowered `Exec::exec` origin.
struct ShellCommand<'tree>
{
    /// The decoded command payload.
    command: HostCommand,
    /// The host-expression computations that must run before this command.
    hoists: Vec<Hoist>,
    /// The source command node.
    node: SynNode<'tree>,
    /// The `Exec::exec` perform origin.
    origin: OriginNode,
}

/// One parsed arm of a list-`case` (the lowering contract): the empty-list arm
/// `Nil`, or the cons arm `Cons(head, tail)` with its two binders.
enum ListArm
{
    /// The `Nil` arm: its body, run on the empty list.
    Nil(
        /// The `nil` arm body.
        COut,
    ),
    /// The `Cons(head, tail)` arm: its two binders and body.
    Cons(
        /// The `head` binder, bound to the element.
        String,
        /// The `tail` binder, bound to the rest of the list.
        String,
        /// The `cons` arm body.
        COut,
    ),
}

/// A `def name : T;` awaiting its definition.
struct PendingSig
{
    /// The signature's name.
    name: String,
    /// The lowered ascription.
    ty: Ty,
    /// The signature's byte range (for the dangling error / hole).
    byte_range: SourceRange,
    /// The signature's CST node identity (for the total-mode hole's origin).
    node: NodeId,
    /// The signature's CST node merkle hash (for the total-mode hole's origin).
    hash: StableHash,
    /// Whether a later definition consumed it.
    used: bool,
}

/// Consumes the first unused signature matching `name`.
fn take_sig(
    sigs: &mut [PendingSig],
    name: DefinitionName<'_>,
) -> Option<Ty>
{
    sigs.iter_mut()
        .find(|sig| !sig.used && sig.name == name.as_ref())
        .map(|sig| {
            sig.used = true;
            sig.ty.clone()
        })
}

// --- The lowerer
// --------------------------------------------------------------

/// Lowering state: the source text (for node text extraction), the hoist-binder
/// allocator, the hole-id allocator (total mode), and the strictness — both
/// allocators are `Gensym`s over the nominal atom substrate (the design
/// record).
#[derive(Clone)]
struct Lowerer<'src>
{
    /// The source text being lowered.
    source: PipelineSource<'src>,
    /// Allocator for `%tmpN` hoist binders — a monotone [`Gensym`] over the
    /// [`GandrSort::TmpHoist`] atom sort (the lowering contract).
    hoist: Gensym<GandrSort>,
    /// Allocator for hole identifiers (total mode) — a monotone [`Gensym`] over
    /// the [`GandrSort::HoleAddr`] atom sort (the lowering contract).
    /// Identifiers are minted in lowering-attempt order; statement-level
    /// recovery may discard a failed attempt's mints, so identifiers are
    /// unique and deterministic but not necessarily contiguous.
    holes: Gensym<GandrSort>,
    /// The lowering mode.
    strictness: Strictness,
    /// The foreign modules declared by `extern` blocks, keyed by namespace
    /// (proposal-ffi.md §2). Built in a pre-pass at the start of
    /// [`Self::source_file`] so a later item's call `m.op(args)` can elaborate
    /// against the module declared earlier in the same source; a call selecting
    /// a member of one of these elaborates to a `perform` (§3.1).
    foreign: BTreeMap<String, ForeignModule>,
    /// Namespace bindings accumulated by earlier sources, plus this source's
    /// accepted imports as lowering proceeds.
    import_scope: Scope<ImportIndex, SourceRange>,
    /// Source-order index assigned to this source's first accepted import.
    import_index_base: ImportIndex,
    /// The `codata` blocks declared in this source, keyed by codata type name
    /// (the codata-declaration contract). Built in a pre-pass at the start of
    /// [`Self::source_file`] (like [`Self::foreign`]) so a later copattern
    /// definition `def rec f() -> C { … }` is elaborated and
    /// coverage-checked against `C`'s observation set regardless of source
    /// order. A `codata` block is a declaration, not a runnable item — it
    /// contributes no [`Item`].
    codata: BTreeMap<String, codata::CodataDecl>,
    /// The union of every declared observation name (across all `codata`
    /// blocks). A `.π` projection whose field is a declared observation lowers
    /// to the codata observation `force(RecordProj(s, π))` rather than a plain
    /// record projection ([`Self::projection`] dispatch, design §3.1) — the
    /// label-driven analogue of module-select disambiguation.
    observations: BTreeSet<String>,
    /// The `data` blocks declared in this source, keyed by datatype name (the
    /// positive-declaration analogue of [`Self::codata`]; the lowering contract
    /// Decision 4). Built by the [`Self::collect_data`] pre-pass at the
    /// start of [`Self::source_file`] so a later item's constructor
    /// application `C(v̄)` or `case v { … }` resolves against the datatype
    /// declared elsewhere in the same source. The registry holds the decl
    /// table (constructor enumeration) the frozen core deliberately does
    /// not carry, and its `DataId` is the core-local id the runtime `Data`
    /// former compares on.
    data: BTreeMap<String, data::DataDecl>,
    /// Every declared constructor name mapped to its `(datatype name, tag)` —
    /// the constructor-resolution map a `C(v̄)` call / bare `C` / `case` arm
    /// consults (the lowering contract). Populated alongside [`Self::data`].
    constructors: BTreeMap<String, (String, usize)>,
    /// The parse's severity-ordered obligations. Total-mode recovery resolves
    /// each grout-leaf hole's [`HoleNote`] from the obligation responsible for
    /// the hole's span.
    obligations: Vec<ObligationInstance>,
}

impl Lowerer<'_>
{
    /// Whether total-mode recovery is active.
    fn total(&self) -> TotalMode
    {
        matches!(self.strictness, Strictness::Total).into()
    }

    /// Mints a fresh hole identifier — the next [`GandrSort::HoleAddr`] atom's
    /// identity, projected to the
    /// [`HoleId`](gandr_core_checker::syntax::HoleId) addressing handle the
    /// IR carries.
    fn fresh_hole(&mut self) -> FreshHoleId
    {
        u32::from(
            self.holes
                .fresh()
                .map_or_else(|error| error.exhausted_id(), |atom| atom.id()),
        )
        .into()
    }

    /// Builds the origin entry for a hole synthesized at `node` from
    /// `error`: the consuming node's CST identity, the *elided region's*
    /// byte range (from the error, which always points at or inside the
    /// consuming node), and the structured note.
    fn hole_entry(
        &self,
        node: SynNode<'_>,
        error: &LowerError,
    ) -> OriginEntry
    {
        let byte_range = error_byte_range(error).unwrap_or_else(|| node.byte_range());
        OriginEntry {
            note: Some(self.note_for(&byte_range, node.kind(), error)),
            cst_node: node.cst_node(),
            cst_hash: node.cst_hash(),
            byte_range,
            elaboration: None,
        }
    }

    /// The [`HoleNote`] a recovery hole at `byte_range` (over a node of
    /// `node_kind`) carries. The *parse-shaped* recovery errors — a
    /// syntax-error region ([`LowerError::Syntax`], the melder's walked
    /// grout leaves) and a structurally malformed node
    /// ([`LowerError::MalformedNode`], whose field the melder degrouted
    /// rather than filled) — take the note of the obligation responsible
    /// for their span (the melder filters grout out of the named surface,
    /// so the responsible obligation, not the leaf, is the signal); every
    /// other (semantic) error keeps its structured [`note_of`]
    /// note. A parse error with no responsible obligation (never observed;
    /// every repair the melder inserts flags one) falls back to
    /// [`HoleNote::MalformedNode`], so [`HoleNote::SyntaxError`] has no
    /// producer in the recovery path.
    fn note_for(
        &self,
        byte_range: &SourceRange,
        node_kind: SyntaxKind,
        error: &LowerError,
    ) -> HoleNote
    {
        if !matches!(
            *error,
            LowerError::Syntax { .. } | LowerError::MalformedNode { .. }
        ) {
            return note_of(error);
        }
        self.responsible_obligation(byte_range)
            .map_or(HoleNote::MalformedNode { kind: node_kind }, |obligation| {
                obligation_hole_note(obligation, node_kind, self.source)
            })
    }

    /// The obligation responsible for a grout-leaf hole at `hole`: the buffered
    /// obligation whose span overlaps or touches `hole`, preferring the largest
    /// intersection, then the closest start, then the highest severity. The
    /// melder's span conventions differ by class (`UnmoldedTok`/`MissingMeld`
    /// coincide with the grout leaf; `MissingTile` points at the unclosed
    /// opener one step before it), so an overlap-or-touch join covers them
    /// uniformly.
    fn responsible_obligation(
        &self,
        hole: &SourceRange,
    ) -> Option<ObligationInstance>
    {
        self.obligations
            .iter()
            .filter(|obligation| {
                let span = obligation_range(obligation);
                span.start <= hole.end && hole.start <= span.end
            })
            .max_by_key(|obligation| {
                let span = obligation_range(obligation);
                let overlap = span
                    .end
                    .min(hole.end)
                    .saturating_sub(span.start.max(hole.start));
                let start_distance = span.start.abs_diff(hole.start);
                (
                    overlap,
                    usize::MAX.saturating_sub(start_distance),
                    obligation.class.index(),
                )
            })
            .copied()
    }

    /// A value hole recovering from `error` at `node` (total mode).
    fn value_hole(
        &mut self,
        node: SynNode<'_>,
        error: &LowerError,
    ) -> LowerResult<VOut>
    {
        let entry = self.hole_entry(node, error);
        VOut::from_legacy_value(
            &Value::Hole(self.fresh_hole().into()),
            OriginNode::leaf(entry),
        )
    }

    /// A computation hole recovering from `error` at `node` (total mode).
    fn comp_hole(
        &mut self,
        node: SynNode<'_>,
        error: &LowerError,
    ) -> LowerResult<COut>
    {
        let entry = self.hole_entry(node, error);
        COut::from_legacy_comp(
            &Comp::Hole(self.fresh_hole().into()),
            OriginNode::leaf(entry),
        )
    }

    /// The `?name` text of a user-written [`node_kinds::HOLE`], if it is named.
    fn hole_name_text(
        &self,
        node: SynNode<'_>,
    ) -> Option<String>
    {
        node.child_by_field_name(node_kinds::FIELD_HOLE_NAME)
            .and_then(|name| self.text(name).ok())
            .map(NodeText::to_owned)
    }

    /// A value hole the *user wrote* (`?` / `?name`). Unlike
    /// [`Self::value_hole`] it is not gated on total mode — a user hole is
    /// a legitimate axiom in every mode. The sort is fixed by the consuming
    /// position ([`Self::value_expr`]); [`Self::user_comp_hole`] is the
    /// dual.
    fn user_value_hole(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<VOut>
    {
        let hole_note = HoleNote::UserHole {
            name: self.hole_name_text(node),
        };
        VOut::from_legacy_value(
            &Value::Hole(self.fresh_hole().into()),
            OriginNode::leaf(user_hole_entry(node, hole_note)),
        )
    }

    /// A computation hole the *user wrote* (`?` / `?name`); the computation-
    /// position dual of [`Self::user_value_hole`]. Producing a `Comp::Hole`
    /// (rather than `Ret` of a value hole) keeps the hole sort-polymorphic, so
    /// it can stand for a non-returner computation — a function or a lazy pair.
    fn user_comp_hole(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let hole_note = HoleNote::UserHole {
            name: self.hole_name_text(node),
        };
        COut::from_legacy_comp(
            &Comp::Hole(self.fresh_hole().into()),
            OriginNode::leaf(user_hole_entry(node, hole_note)),
        )
    }

    /// Mints a fresh hoist binder — the next [`GandrSort::TmpHoist`] atom's
    /// identity, rendered `%tmpN`. `%` is not a surface identifier character,
    /// so fresh names can never capture or be captured.
    fn fresh_name(&mut self) -> String
    {
        let counter = self.hoist.fresh().map_or_else(
            |error| u32::from(error.exhausted_id()),
            |atom| u32::from(atom.id()),
        );
        format!("%tmp{counter}")
    }

    /// Wraps `body` in one synthesized `Bind` per pending hoist (innermost =
    /// last hoisted), tagging each with `host` and
    /// [`ElabKind::BindHoist`]. `host`'s byte range must contain both the
    /// hoisted computations and `body` (it is the surface node whose
    /// lowering consumed the hoists), preserving the range-nesting
    /// invariant.
    fn wrap_hoists_entry(
        hoists: Vec<Hoist>,
        body: COut,
        host: &OriginEntry,
    ) -> LowerResult<COut>
    {
        let mut acc = body;
        for hoist in hoists.into_iter().rev() {
            let comp = Comp::Bind(
                Rc::new({
                    let readback_comp = hoist.bound.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                hoist.name,
                Rc::new({
                    let readback_comp = acc.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            );
            let entry = OriginEntry {
                cst_node: host.cst_node,
                cst_hash: host.cst_hash,
                byte_range: host.byte_range.clone(),
                elaboration: Some(ElabKind::BindHoist),
                note: None,
            };
            acc = COut::from_legacy_comp(
                &comp,
                OriginNode::new(entry, vec![hoist.bound.origin, acc.origin]),
            )?;
        }
        Ok(acc)
    }

    /// [`Self::wrap_hoists_entry`] with `node` as the host.
    fn wrap_hoists(
        hoists: Vec<Hoist>,
        body: COut,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let host = entry(node, Some(ElabKind::BindHoist));
        Self::wrap_hoists_entry(hoists, body, &host)
    }

    /// The text of a CST node.
    fn text(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<NodeText<'_>>
    {
        node_text(self.source, node)
    }

    // --- Expressions ---------------------------------------------------------

    /// Lowers an expression to whichever sort it has, pushing hoists for
    /// computations the *caller* placed in value position into `hoists`.
    fn expr(
        &mut self,
        node: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<EOut>
    {
        recursive::expr(self, node, hoists)
    }

    /// Lowers an expression in *value position*: a computation result is
    /// hoisted to a fresh variable (see the module doc's sort mediation).
    /// In total mode a failed sub-lowering becomes a value hole here —
    /// holes take the sort of the position that consumes them.
    fn value_expr(
        &mut self,
        node: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<VOut>
    {
        recursive::value_expr(self, node, hoists)
    }

    /// Lowers an expression in *computation position*: a value result is
    /// wrapped in `Ret` ([`ElabKind::RetCoercion`]); pending hoists are
    /// consumed here. In total mode a failed sub-lowering becomes a
    /// computation hole here.
    fn comp_expr(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        recursive::comp_expr(self, node)
    }

    /// Lowers a bare `number` token (no type suffix) to a value literal
    /// (the lowering contract).
    ///
    /// A *float-shaped* numeral — one carrying a fractional point or an
    /// exponent — lowers to an `f64` [`Value::Num`] (Rust's default for an
    /// unsuffixed float). An *integer-shaped* one stays the frozen
    /// [`Value::Int`] typed by `Integer`, and an i64-overflowing integer
    /// is an error rather than silently becoming a float. A float whose
    /// magnitude overflows `f64` (parsing to a non-finite `inf`) is likewise an
    /// error, symmetric with the integer case and matching Rust. (A suffixed
    /// literal such as `8080u32` is the separate `typed_number` token,
    /// [`Self::typed_number_literal`].)
    fn number_literal(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<VOut>
    {
        let text = self.text(node)?;
        let float_shaped = text.contains('.') || text.contains('e') || text.contains('E');
        let value = if float_shaped {
            text.parse::<f64>()
                .ok()
                .filter(|float| float.is_finite())
                .map(Value::f64)
        }
        else {
            text.parse::<i64>().ok().map(Value::int)
        };
        match value {
            | Some(value) => VOut::from_legacy_value(&value, OriginNode::leaf(entry(node, None))),
            // An i64-overflowing integer, or an unparseable float numeral.
            | None => Err(LowerError::InvalidIntegerLiteral {
                text: text.to_owned(),
                byte_range: node.byte_range(),
            }),
        }
    }

    /// Lowers a `typed_number` token (a Rust-style type-suffixed numeric
    /// literal, `8080u32` / `1.5f64` / `2f32`) to the corresponding
    /// [`Value::Num`] (the lowering contract).
    ///
    /// The grammar guarantees the text ends in one of the six three-character
    /// primitive suffixes; it is split off and the digit part parsed as that
    /// type. A literal that does not fit (`4294967296u32`), whose digits are
    /// ill-shaped for the suffix (`1.5u32`), or whose float magnitude overflows
    /// to a non-finite value (`1e400f64`) is rejected as an
    /// [`LowerError::InvalidIntegerLiteral`] — the suffixed literal is
    /// monomorphic, so there is no fallback to another type.
    fn typed_number_literal(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<VOut>
    {
        let text = self.text(node)?;
        let parsed = text
            .len()
            .checked_sub(3)
            .map(|cut| text.split_at(cut))
            .and_then(|(digits, suffix)| match suffix {
                | "u32" => digits.parse::<u32>().ok().map(Value::u32),
                | "u64" => digits.parse::<u64>().ok().map(Value::u64),
                | "i32" => digits.parse::<i32>().ok().map(Value::i32),
                | "i64" => digits.parse::<i64>().ok().map(Value::i64),
                | "f32" => digits
                    .parse::<f32>()
                    .ok()
                    .filter(|float| float.is_finite())
                    .map(Value::f32),
                | "f64" => digits
                    .parse::<f64>()
                    .ok()
                    .filter(|float| float.is_finite())
                    .map(Value::f64),
                | _ => None,
            });
        match parsed {
            | Some(value) => VOut::from_legacy_value(&value, OriginNode::leaf(entry(node, None))),
            | None => Err(LowerError::InvalidIntegerLiteral {
                text: text.to_owned(),
                byte_range: node.byte_range(),
            }),
        }
    }

    /// Lowers a `string` token to [`Value::Str`] (value-model ladder, the
    /// design record).
    ///
    /// The `string` node text spans the surrounding double quotes; they are
    /// stripped and the grammar's single-character escape sequences decoded
    /// ([`Self::decode_string_escapes`]). Unlike `number`, every well-formed
    /// `string` is a valid literal, so there is no rejecting branch.
    fn string_literal(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<VOut>
    {
        let text = self.text(node)?;
        // Strip the delimiting quotes. A non-error parse always supplies both,
        // but fall back to the raw text rather than panicking if one is absent.
        let inner = text
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(text.0);
        VOut::from_legacy_value(
            &Value::Str(Self::decode_string_escapes(inner.into())),
            OriginNode::leaf(entry(node, None)),
        )
    }

    /// Decodes the interior of a string literal. Per the grammar
    /// (`grammar.js` `escape_sequence` = `\` followed by `.` or a `\r?\n`), an
    /// escape is a backslash followed by either one character or a line
    /// continuation:
    /// - the recognized escapes `\n` `\t` `\r` `\0` `\\` `\"` `\'` map to their
    ///   control character;
    /// - a backslash before an actual newline (`\<LF>` or `\<CR><LF>`) is a
    ///   line continuation, elided (the backslash and the newline both
    ///   dropped);
    /// - any other `\c` decodes to the literal `c` (the backslash dropped), and
    ///   a trailing lone backslash is dropped.
    fn decode_string_escapes(inner: NodeText<'_>) -> String
    {
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                out.push(ch);
                continue;
            }
            match chars.next() {
                | Some('n') => out.push('\n'),
                | Some('t') => out.push('\t'),
                | Some('r') => out.push('\r'),
                | Some('0') => out.push('\0'),
                | Some('\\') => out.push('\\'),
                | Some('"') => out.push('"'),
                | Some('\'') => out.push('\''),
                // A backslash before a literal newline is a line continuation
                // (the grammar's `\r?\n` escape alternative): elide it. For a
                // CRLF continuation the paired LF follows the CR in the same
                // token, so consume it too. A trailing lone backslash (`None`)
                // likewise yields nothing.
                | Some('\r') => {
                    if chars.peek() == Some(&'\n') {
                        let _consumed = chars.next();
                    }
                },
                | Some('\n') | None => {},
                | Some(other) => out.push(other),
            }
        }
        out
    }

    /// Lowers `true`/`false` to an annotated injection into `1 + 1` (plan:
    /// "Booleans need no core change").
    ///
    /// [SPECULATIVE DECISION] The plan fixes the target type but not the
    /// polarity; convention here: `true` ⇒ `inj1 ()`, `false` ⇒ `inj2 ()`,
    /// so an `if`'s consequence is the first `case` arm.
    fn boolean_literal(node: SynNode<'_>) -> LowerResult<VOut>
    {
        let Some(token) = node.child(SignificantIndex(0))
        else {
            return Err(LowerError::MalformedNode {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        };
        let side = match token.kind() {
            | node_kinds::TRUE => Side::Fst,
            | node_kinds::FALSE => Side::Snd,
            | kind => {
                return Err(LowerError::Unsupported {
                    kind,
                    byte_range: node.byte_range(),
                });
            },
        };
        let elab = Some(ElabKind::BoolLiteral);
        let unit = OriginNode::leaf(entry(node, elab));
        let inj = OriginNode::new(entry(node, elab), vec![unit]);
        let bool_ty = ValueType::sum(ValueType::Unit, ValueType::Unit);
        VOut::from_legacy_value(
            &Value::annot(Value::Inj(side, Rc::new(Value::Unit)), bool_ty),
            OriginNode::new(entry(node, elab), vec![inj]),
        )
    }

    /// Lowers an n-ary tuple to right-nested [`Value::Pair`]s; the inner
    /// synthesized pairs carry [`ElabKind::TupleNest`].
    fn tuple(
        &mut self,
        node: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<VOut>
    {
        recursive::tuple(self, node, hoists)
    }

    /// Lowers a list literal `[v₀, …, vₙ]` to [`Value::List`] (the design
    /// record). Each element lowers in value position (a computation
    /// element is hoisted, as a tuple component is); the element child
    /// order matches `origin::resolve` (`0, 1, …`). The empty list `[]`
    /// lowers to an empty `Value::List`.
    fn list_expr(
        &mut self,
        node: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<VOut>
    {
        recursive::list_expr(self, node, hoists)
    }

    /// Lowers an ascription `(e : T)`, sort-directed by the type.
    ///
    /// A value-sorted `(v : A)` lowers to [`Value::Annot`] as before. A
    /// computation-sorted `(t : B)` has no core annotation node (recorded
    /// decision — core carries only the value annotation), so it elaborates
    /// through the thunk annotation instead: `force ((thunk t) : U_ω B)`.
    /// Checking `thunk t ⇐ U_ω B` checks `t ⇐ B` (the `U`-introduction rule)
    /// and the `force` synthesizes `B` back, so the encoding *is* the
    /// ascription rule — with two extra evaluation steps and no new core
    /// form ([`ElabKind::CompAscription`]).
    /// This is what gives the check-only computations (`case`, `if`, check-only
    /// tails) a source-level expected type outside a `def` signature — an
    /// expression-only program (a shell script) previously had no typeable
    /// spelling for them.
    fn annotation(
        &mut self,
        node: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<EOut>
    {
        recursive::annotation(self, node, hoists)
    }

    /// Lowers `thunk[r] { t }` to [`Value::Thunk`] (grade default `ω`).
    fn thunk(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<VOut>
    {
        recursive::thunk(self, node)
    }

    /// Lowers `fn(x: A, …) { t }` to nested [`Comp::Abs`]; inner synthesized
    /// abstractions carry [`ElabKind::CurrySugar`].
    fn lambda(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        recursive::lambda(self, node)
    }

    /// Folds parameters into nested `Abs` around `body`. `outer_entry` tags
    /// the outermost abstraction; inner ones get `inner_elab` (or
    /// [`ElabKind::CurrySugar`] if [`None`]).
    fn curry_abs(
        params: Vec<(String, Option<ValueType>)>,
        body: COut,
        outer_entry: &OriginEntry,
        inner_elab: Option<ElabKind>,
    ) -> LowerResult<COut>
    {
        let count = params.len();
        let mut acc = body;
        for (index, (name, annot)) in params.into_iter().rev().enumerate() {
            let outermost = index == count.saturating_sub(1);
            let elaboration = if outermost {
                outer_entry.elaboration
            }
            else {
                Some(inner_elab.unwrap_or(ElabKind::CurrySugar))
            };
            let entry = OriginEntry {
                cst_node: outer_entry.cst_node,
                cst_hash: outer_entry.cst_hash,
                byte_range: outer_entry.byte_range.clone(),
                elaboration,
                note: None,
            };
            acc = COut::from_legacy_comp(
                &Comp::Abs(
                    name,
                    annot.map(Rc::new),
                    Rc::new({
                        let readback_comp = acc.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                ),
                OriginNode::new(entry, vec![acc.origin]),
            )?;
        }
        Ok(acc)
    }

    /// Lowers a call. `Inl(v)`/`Inr(v)` constructor calls are injections
    /// (values); any other call is an application spine with the
    /// syntax-directed force sugar on the head.
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        allow(
            unknown_lints,
            non_local_effect_before_unhandled_error,
            reason = "the staged hoist buffers are transaction-local: a failed intro/eliminator/injection drops its buffer with the error (total mode elides, strict mode propagates), so no effect escapes unhandled"
        )
    )]
    fn call(
        &mut self,
        node: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<EOut>
    {
        let function = required_field(node, node_kinds::FIELD_FUNCTION)?;
        let arguments_node = required_field(node, node_kinds::FIELD_ARGUMENTS)?;
        let arguments = named_non_extra_children(arguments_node);

        // The identity intro `here(v)` (the lowering contract): a reserved lowercase
        // call head resolved by name (like the `Inl`/`Inr` constructors),
        // lowering to the value former `Value::Here`. A failed intro (wrong
        // arity) elides the whole call in total mode.
        let function_head = if function.kind() == node_kinds::IDENTIFIER {
            let function_head = self.text(function)?;
            Some(function_head.0)
        }
        else {
            None
        };
        if function_head == Some(node_kinds::NAME_HERE) {
            let mut staged_hoists = Vec::new();
            let here = match self.here_call(node, &arguments, &mut staged_hoists) {
                | Err(ref error) if bool::from(self.total()) => self.value_hole(node, error)?,
                | other => other?,
            };
            hoists.append(&mut staged_hoists);
            return Ok(EOut::Value(here));
        }

        // The identity eliminator `walk(p, fn(x, y, q) { C }, fn(x) { c })`
        // (the lowering contract): a reserved lowercase call head resolved by name —
        // the eliminator is `walk` (not the capitalized `J` of the literature),
        // so like `here` it is recognized here rather than as a constructor
        // call — lowering to the computation former `Comp::Walk`. A failed
        // eliminator (wrong arity, non-lambda motive/base, out-of-rung motive
        // body) elides the whole call in total mode.
        if function_head == Some(node_kinds::NAME_WALK) {
            let j = match self.walk_call(node, &arguments) {
                | Err(ref error) if bool::from(self.total()) => self.comp_hole(node, error)?,
                | other => other?,
            };
            return Ok(EOut::Comp(j));
        }

        if function.kind() == node_kinds::CONSTRUCTOR {
            // A failed injection (unknown constructor, wrong arity) elides
            // the whole call in total mode.
            let mut staged_hoists = Vec::new();
            let injected = match self.injection(node, function, &arguments, &mut staged_hoists) {
                | Err(ref error) if bool::from(self.total()) => self.value_hole(node, error)?,
                | other => other?,
            };
            hoists.append(&mut staged_hoists);
            return Ok(EOut::Value(injected));
        }

        // A call `m.op(args)` whose `m` is an `extern`-declared module
        // elaborates to `perform m.op {payload}` (proposal-ffi.md §3.1) rather
        // than the ordinary force-and-apply spine below; a reserved host
        // module (`fs` / `env` / `proc`) elaborates the same way against its
        // host signature. An `extern`-declared module shadows a host module of
        // the same name (a user declaration wins over the ambient surface). A
        // failed foreign or host call (unknown op, arity mismatch) elides the
        // whole call in total mode.
        if function.kind() == node_kinds::PROJECTION_EXPRESSION {
            match self.foreign_call(node, function, &arguments) {
                | Ok(Some(perform)) => return Ok(EOut::Comp(perform)),
                | Ok(None) => {},
                | Err(ref error) if bool::from(self.total()) => {
                    return Ok(EOut::Comp({
                        let hole = self.comp_hole(node, error)?;
                        core::convert::identity(hole)
                    }));
                },
                | Err(error) => return Err(error),
            }
            match self.host_call(node, function, &arguments) {
                | Ok(Some(perform)) => return Ok(EOut::Comp(perform)),
                | Ok(None) => {},
                | Err(ref error) if bool::from(self.total()) => {
                    return Ok(EOut::Comp({
                        let hole = self.comp_hole(node, error)?;
                        core::convert::identity(hole)
                    }));
                },
                | Err(error) => return Err(error),
            }
        }

        let mut local: Vec<Hoist> = Vec::new();
        // A failed head becomes a computation-hole head in total mode, so
        // the arguments still lower (and report their own goals).
        let lowered_head = match self.expr(function, &mut local) {
            | Err(ref error) if bool::from(self.total()) => EOut::Comp({
                let hole = self.comp_hole(function, error)?;
                core::convert::identity(hole)
            }),
            | other => other?,
        };
        let head = match lowered_head {
            | EOut::Comp(comp) => comp,
            // The force sugar: a value head (variable, thunk literal, …) is
            // wrapped in `Force` — in CBPV the application head must be a
            // computation, decidable at lowering with no type information.
            | EOut::Value(value) => COut::from_legacy_comp(
                &Comp::Force(Rc::new({
                    let readback_value = value.readback_value()?;
                    core::convert::identity(readback_value)
                })),
                OriginNode::new(entry(function, Some(ElabKind::ForceSugar)), vec![
                    value.origin,
                ]),
            )?,
        };
        if arguments.is_empty() {
            return Ok(EOut::Comp({
                let wrapped = Self::wrap_hoists(local, head, node)?;
                core::convert::identity(wrapped)
            }));
        }
        let count = arguments.len();
        let mut acc = head;
        for (index, argument) in arguments.into_iter().enumerate() {
            let arg = self.value_expr(argument, &mut local)?;
            let elaboration = if index == count.saturating_sub(1) {
                None
            }
            else {
                Some(ElabKind::CurrySugar)
            };
            acc = COut::from_legacy_comp(
                &Comp::App(
                    Rc::new({
                        let readback_comp = acc.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                    Rc::new({
                        let readback_value = arg.readback_value()?;
                        core::convert::identity(readback_value)
                    }),
                ),
                OriginNode::new(entry(node, elaboration), vec![acc.origin, arg.origin]),
            )?;
        }
        Ok(EOut::Comp({
            let wrapped = Self::wrap_hoists(local, acc, node)?;
            core::convert::identity(wrapped)
        }))
    }

    /// Elaborates a foreign call `m.op(args)` to `perform m.op {payload}` when
    /// `m` is an `extern`-declared module and `op` one of its functions
    /// (proposal-ffi.md §3.1) — the "a foreign call is an effect op" rule.
    ///
    /// The payload is the argument record keyed by parameter name; the
    /// synthesized `Perform` carries the module's per-library effect signature
    /// (so its effect row `⟨m⟩` records the foreign reach) and
    /// [`ElabKind::ForeignPerform`]. Arguments that lower to computations are
    /// hoisted around the `perform`, as call arguments are.
    ///
    /// # Contract
    /// - ensures: `Some(perform)` when `m.op` selects a declared foreign
    ///   function with matching arity; `None` when the projection is not a
    ///   foreign selection (a genuine record projection or module builtin falls
    ///   through to the ordinary call path).
    /// - fails: [`LowerError::Unsupported`] for a known foreign module with an
    ///   unknown member (the lowering contract — a module namespace is not a
    ///   record) or an arity mismatch against the declared signature.
    /// - panics: none.
    fn foreign_call(
        &mut self,
        call_node: SynNode<'_>,
        function: SynNode<'_>,
        arguments: &[SynNode<'_>],
    ) -> LowerResult<Option<COut>>
    {
        let target_node = required_field(function, node_kinds::FIELD_VALUE)?;
        if target_node.kind() != node_kinds::IDENTIFIER {
            return Ok(None);
        }
        let field_node = required_field(function, node_kinds::FIELD_FIELD)?;
        // Resolve the module/op and clone the pieces we need, releasing the
        // immutable borrow of `self` before lowering arguments (which needs a
        // mutable borrow). A non-foreign projection returns `None` to fall
        // through to the ordinary call path.
        let resolved = {
            let module = self.text(target_node)?;
            let op = self.text(field_node)?;
            match self.foreign.get(module.0) {
                | None => None,
                | Some(foreign_module) => match foreign_module.function(op) {
                    | None => {
                        return Err(LowerError::Unsupported {
                            kind: call_node.kind(),
                            byte_range: call_node.byte_range(),
                        });
                    },
                    | Some(foreign_fn) => {
                        if arguments.len() != foreign_fn.params.len() {
                            return Err(LowerError::Unsupported {
                                kind: call_node.kind(),
                                byte_range: call_node.byte_range(),
                            });
                        }
                        let names: Vec<String> = foreign_fn
                            .params
                            .iter()
                            .map(|param| param.name.clone())
                            .collect();
                        Some((foreign_module.effect_sig(), foreign_fn.op.clone(), names))
                    },
                },
            }
        };
        let Some((sig, op_name, param_names)) = resolved
        else {
            return Ok(None);
        };
        // Lower each argument in value position, keyed by its parameter name;
        // a computation argument is hoisted around the `perform` (as a call
        // argument is around the application spine).
        let mut hoists: Vec<Hoist> = Vec::new();
        let mut fields: Vec<(String, Value)> = Vec::with_capacity(param_names.len());
        let mut origins: Vec<OriginNode> = Vec::with_capacity(param_names.len());
        for (name, &argument) in param_names.iter().zip(arguments.iter()) {
            let arg = self.value_expr(argument, &mut hoists)?;
            fields.push((name.clone(), {
                let readback_value = arg.readback_value()?;
                core::convert::identity(readback_value)
            }));
            origins.push(arg.origin);
        }
        let payload = Value::record(fields);
        let perform = COut::from_legacy_comp(
            &Comp::perform(sig, &op_name, payload),
            OriginNode::new(entry(call_node, Some(ElabKind::ForeignPerform)), origins),
        )?;
        Ok(Some({
            let wrapped = Self::wrap_hoists(hoists, perform, call_node)?;
            core::convert::identity(wrapped)
        }))
    }

    /// Elaborates a host-module call `fs.read(v)` / `env.get(v)` /
    /// `proc.exit(v)` to `perform` of the corresponding host operation
    /// ([`crate::host`]) — the same module-select ⇒ perform rule as
    /// [`Self::foreign_call`], against ambient host signatures instead of a
    /// per-library one (so the effect row records the host reach, `F^⟨Fs⟩`).
    ///
    /// The payload follows the member's declared parameters (matching the
    /// signature's payload types, pinned in [`crate::host`]'s tests): zero
    /// parameters perform `()`, one performs the bare argument value, and
    /// several perform the argument record keyed by parameter name (the FFI
    /// convention). Arguments that lower to computations are hoisted around
    /// the `perform`, as call arguments are.
    ///
    /// # Contract
    /// - ensures: `Some(perform)` when the projection head is a reserved
    ///   host-module name and the field one of its members with matching arity;
    ///   `None` when the head is not a host module (a genuine record projection
    ///   or prelude module builtin falls through to the ordinary call path).
    /// - fails: [`LowerError::Unsupported`] for a known host module with an
    ///   unknown member (the lowering contract — a module namespace is not a
    ///   record) or an arity mismatch against the declared parameters.
    /// - panics: none.
    fn host_call(
        &mut self,
        call_node: SynNode<'_>,
        function: SynNode<'_>,
        arguments: &[SynNode<'_>],
    ) -> LowerResult<Option<COut>>
    {
        let target_node = required_field(function, node_kinds::FIELD_VALUE)?;
        if target_node.kind() != node_kinds::IDENTIFIER {
            return Ok(None);
        }
        let field_node = required_field(function, node_kinds::FIELD_FIELD)?;
        let resolved = {
            let module = self.text(target_node)?;
            let op = self.text(field_node)?;
            match crate::host::host_module(module) {
                | None => None,
                | Some(host_module) => match host_module.member(op) {
                    | None => {
                        return Err(LowerError::Unsupported {
                            kind: call_node.kind(),
                            byte_range: call_node.byte_range(),
                        });
                    },
                    | Some(member) => {
                        if arguments.len() != member.params.len() {
                            return Err(LowerError::Unsupported {
                                kind: call_node.kind(),
                                byte_range: call_node.byte_range(),
                            });
                        }
                        Some((host_module.sig(), member.op, member.params))
                    },
                },
            }
        };
        let Some((sig, op_name, param_names)) = resolved
        else {
            return Ok(None);
        };
        // Lower each argument in value position; a computation argument is
        // hoisted around the `perform`, as a call argument is around the
        // application spine.
        let mut hoists: Vec<Hoist> = Vec::new();
        let mut args: Vec<Value> = Vec::with_capacity(param_names.len());
        let mut origins: Vec<OriginNode> = Vec::with_capacity(param_names.len());
        for &argument in arguments {
            let arg = self.value_expr(argument, &mut hoists)?;
            args.push({
                let readback_value = arg.readback_value()?;
                core::convert::identity(readback_value)
            });
            origins.push(arg.origin);
        }
        // The payload shape follows the declared parameters: `()` for a
        // zero-parameter member, the bare value for one, the parameter-keyed
        // record for several.
        let payload = match (args.len(), param_names) {
            | (0, _) => Value::Unit,
            | (1, _) => {
                let Some(sole) = args.pop()
                else {
                    // Unreachable: `args` was just measured at one element.
                    return Err(LowerError::Unsupported {
                        kind: call_node.kind(),
                        byte_range: call_node.byte_range(),
                    });
                };
                sole
            },
            | (_, params) => Value::record(params.iter().map(|&param| param.to_owned()).zip(args)),
        };
        let perform = COut::from_legacy_comp(
            &Comp::perform(sig, op_name, payload),
            OriginNode::new(entry(call_node, Some(ElabKind::HostPerform)), origins),
        )?;
        Ok(Some({
            let wrapped = Self::wrap_hoists(hoists, perform, call_node)?;
            core::convert::identity(wrapped)
        }))
    }

    /// Lowers an `Inl(v)`/`Inr(v)` constructor call to [`Value::Inj`].
    fn injection(
        &mut self,
        call_node: SynNode<'_>,
        constructor: SynNode<'_>,
        arguments: &[SynNode<'_>],
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<VOut>
    {
        let constructor_text = self.text(constructor)?;
        let side = match constructor_text.0 {
            | node_kinds::NAME_INL => Side::Fst,
            | node_kinds::NAME_INR => Side::Snd,
            // A declared-data constructor `C(v̄)` lowers to the nominal
            // constructor-tagged value (the constructor-lowering contract); an unknown
            // constructor is out of fragment.
            | other if self.is_data_constructor(other.into()).into() => {
                // No enclosing ascription here (a bare constructor application);
                // its fields are lowered unchecked. The field-type discipline
                // applies when an ascription reaches the constructor through
                // [`Self::value_expr_expecting`], itself stuck under inference
                // until then (the constructor-lowering contract is check-only).
                return self.data_constructor(call_node, constructor, arguments, hoists, None);
            },
            | _ => {
                return Err(LowerError::Unsupported {
                    kind: constructor.kind(),
                    byte_range: constructor.byte_range(),
                });
            },
        };
        let (Some(&payload_node), 1) = (arguments.first(), arguments.len())
        else {
            // Injections take exactly one payload.
            return Err(LowerError::Unsupported {
                kind: call_node.kind(),
                byte_range: call_node.byte_range(),
            });
        };
        let mut staged_hoists = Vec::new();
        let payload = self.value_expr(payload_node, &mut staged_hoists)?;
        let payload_value = payload.readback_value()?;
        hoists.append(&mut staged_hoists);
        VOut::from_legacy_value(
            &Value::Inj(side, Rc::new(payload_value)),
            OriginNode::new(entry(call_node, None), vec![payload.origin]),
        )
    }

    /// Lowers the identity intro `here(v)` to [`Value::Here`] (the design
    /// record; rule `Here`). Exactly one witness argument, lowered in value
    /// position (a computation witness hoists, as any call argument would).
    fn here_call(
        &mut self,
        call_node: SynNode<'_>,
        arguments: &[SynNode<'_>],
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<VOut>
    {
        let (Some(&witness_node), 1) = (arguments.first(), arguments.len())
        else {
            // `here` takes exactly one witness.
            return Err(LowerError::Unsupported {
                kind: call_node.kind(),
                byte_range: call_node.byte_range(),
            });
        };
        let mut staged_hoists = Vec::new();
        let witness = self.value_expr(witness_node, &mut staged_hoists)?;
        let witness_value = witness.readback_value()?;
        hoists.append(&mut staged_hoists);
        VOut::from_legacy_value(
            &Value::Here(Rc::new(witness_value)),
            OriginNode::new(entry(call_node, None), vec![witness.origin]),
        )
    }

    /// Lowers the identity eliminator `walk(p, fn(x, y, q) { C }, fn(x) { c })`
    /// to [`Comp::Walk`] (the lowering contract; rule `Walk`, rung 1 — explicit
    /// motives only).
    ///
    /// The motive and base are part of the `Walk` **syntax form** (`Arrow` is
    /// non-dependent, so neither is a first-class function): each must be a
    /// literal `fn` lambda with exactly three / one **unannotated**
    /// parameter(s). The motive's body is a *type* under the binders, lowered
    /// by [`Self::walk_motive_type`]; the base's body is an ordinary
    /// computation. The scrutinee lowers in value position with its hoists
    /// wrapped around the whole eliminator.
    fn walk_call(
        &mut self,
        call_node: SynNode<'_>,
        arguments: &[SynNode<'_>],
    ) -> LowerResult<COut>
    {
        let [scrut_node, motive_node, base_node] = *arguments
        else {
            // `Walk` takes exactly (scrutinee, motive, base).
            return Err(LowerError::Unsupported {
                kind: call_node.kind(),
                byte_range: call_node.byte_range(),
            });
        };
        let (motive_params, motive_body_node) = self.walk_lambda(motive_node, 3_usize.into())?;
        let [ref x, ref y, ref q] = motive_params[..]
        else {
            return Err(LowerError::Unsupported {
                kind: motive_node.kind(),
                byte_range: motive_node.byte_range(),
            });
        };
        let motive_ty = self.walk_motive_type(motive_body_node)?;
        let motive = WalkMotive::new(x, y, q, motive_ty);

        let (base_params, base_body_node) = self.walk_lambda(base_node, 1_usize.into())?;
        let [ref base_binder] = base_params[..]
        else {
            return Err(LowerError::Unsupported {
                kind: base_node.kind(),
                byte_range: base_node.byte_range(),
            });
        };
        let mut hoists = Vec::new();
        let scrut = self.value_expr(scrut_node, &mut hoists)?;
        let scrut_value = scrut.readback_value()?;
        let base_body = self.block(base_body_node)?;
        let base_comp = base_body.readback_comp()?;
        let base = WalkBase::new(base_binder, base_comp);

        let body = COut::from_legacy_comp(
            &Comp::Walk {
                scrut: Rc::new(scrut_value),
                motive: Box::new(motive),
                base,
            },
            OriginNode::new(entry(call_node, None), vec![scrut.origin, base_body.origin]),
        )?;
        Self::wrap_hoists(hoists, body, call_node)
    }

    /// Destructures one `Walk` binder argument: a literal `fn` lambda with
    /// exactly `arity` **unannotated** parameters, returning the binder names
    /// and the body block node. Annotated binders are out of the rung-1
    /// fragment (the motive binders are type-level; the base binder's type is
    /// the scrutinee's carrier, supplied by the typing rule).
    fn walk_lambda<'tree>(
        &self,
        node: SynNode<'tree>,
        arity: LambdaArity,
    ) -> LowerResult<(Vec<String>, SynNode<'tree>)>
    {
        if node.kind() != node_kinds::LAMBDA_EXPRESSION {
            return Err(LowerError::Unsupported {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        }
        let params_node = required_field(node, node_kinds::FIELD_PARAMETERS)?;
        let params = self.parameters(params_node)?;
        if params.len() != usize::from(arity)
            || params.iter().any(|param| {
                let (_, ref annotation) = *param;
                annotation.is_some()
            })
        {
            return Err(LowerError::Unsupported {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        }
        let body = required_field(node, node_kinds::FIELD_BODY)?;
        Ok((
            params.into_iter().map(|(name, _annot)| name).collect(),
            body,
        ))
    }

    /// Lowers a `Walk` motive body — a **type** under the motive binders,
    /// parsed in expression position (the motive is part of the `Walk`
    /// syntax form; the lowering contract). The body block must be a single
    /// tail expression, in one of two rung-1 shapes:
    ///
    /// * **an ascribed hole `(? : C)`** — the ascription's annotation parses in
    ///   genuine *type* position, so the full type grammar is available
    ///   (arrows, grades, `Path` with the rung-1 number/variable endpoints);
    ///   the hole is a pure syntactic carrier, never lowered. A value-sorted
    ///   annotation `A` is the F-wrapped sugar `F A` (the design's value-motive
    ///   elaboration);
    /// * **an `Path(c, e1, e2)` constructor call** — the motive-body identity
    ///   former, whose *endpoints* are ordinary value expressions (richer than
    ///   the type-position rung-1 capture: the motive is Walk's own binder
    ///   form, not a general type position, and the honest `cong` derivations
    ///   need compound endpoints exactly here). The carrier is a `?` hole (the
    ///   gradual `Unknown`) or a non-reserved constructor atom; the result is
    ///   F-wrapped. A hoisting endpoint (an effectful sub-computation) is out
    ///   of fragment — endpoints are values.
    ///
    /// Everything else is [`LowerError::Unsupported`] (the consuming `Walk`
    /// holes in total mode). NO reduction, NO motive synthesis — rung 2's
    /// business.
    fn walk_motive_type(
        &mut self,
        body_node: SynNode<'_>,
    ) -> LowerResult<CompType>
    {
        // The motive body is the block's single tail expression.
        if body_node.kind() != node_kinds::BLOCK {
            return Err(LowerError::Unsupported {
                kind: body_node.kind(),
                byte_range: body_node.byte_range(),
            });
        }
        let children = named_non_extra_children(body_node);
        let (Some(&tail), 1) = (children.first(), children.len())
        else {
            return Err(LowerError::Unsupported {
                kind: body_node.kind(),
                byte_range: body_node.byte_range(),
            });
        };
        match tail.kind() {
            // Shape 1: an ascribed hole `(? : C)` — the annotation is real
            // type position.
            | node_kinds::ANNOTATION_EXPRESSION => {
                let value = required_field(tail, node_kinds::FIELD_VALUE)?;
                if value.kind() != node_kinds::HOLE {
                    return Err(LowerError::Unsupported {
                        kind: value.kind(),
                        byte_range: value.byte_range(),
                    });
                }
                let ty_node = required_field(tail, node_kinds::FIELD_TYPE)?;
                // Route through the declared-data-aware seam so a motive
                // mentioning a declared datatype (`? : F Maybe(Integer)`) sees
                // the nominal handle at any depth (the lowering contract).
                match self.lower_type_node(ty_node)? {
                    | Ty::Comp(comp_ty) => Ok(comp_ty),
                    // A value-sorted motive is the F-wrapped special case.
                    | Ty::Value(value_ty) => Ok(CompType::returner(value_ty)),
                }
            },
            // Shape 2: the motive-body identity former `Path(c, e1, e2)`.
            | node_kinds::CALL_EXPRESSION => {
                let function = required_field(tail, node_kinds::FIELD_FUNCTION)?;
                let function_is_path_type = function.kind() == node_kinds::CONSTRUCTOR && {
                    let function_text = self.text(function)?;
                    function_text.0 == node_kinds::NAME_PATH_TYPE
                };
                if !function_is_path_type {
                    return Err(LowerError::Unsupported {
                        kind: function.kind(),
                        byte_range: function.byte_range(),
                    });
                }
                let arguments_node = required_field(tail, node_kinds::FIELD_ARGUMENTS)?;
                let arguments = named_non_extra_children(arguments_node);
                let [carrier_node, lhs_node, rhs_node] = *arguments
                else {
                    return Err(LowerError::Unsupported {
                        kind: tail.kind(),
                        byte_range: tail.byte_range(),
                    });
                };
                let carrier = self.walk_motive_carrier(carrier_node)?;
                let lhs = self.walk_motive_endpoint(lhs_node)?;
                let rhs = self.walk_motive_endpoint(rhs_node)?;
                Ok(CompType::returner(ValueType::path(carrier, lhs, rhs)))
            },
            | kind => Err(LowerError::Unsupported {
                kind,
                byte_range: tail.byte_range(),
            }),
        }
    }

    /// Lowers a motive-body `Path` carrier: a `?` hole (the gradual `Unknown` —
    /// conversion's consistency absorbs the precise carrier) or a non-reserved
    /// constructor atom.
    fn walk_motive_carrier(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<ValueType>
    {
        match node.kind() {
            | node_kinds::HOLE => Ok(ValueType::Unknown),
            | node_kinds::CONSTRUCTOR => self.text(node).map(|text| ValueType::atom(text.0)),
            | kind => Err(LowerError::Unsupported {
                kind,
                byte_range: node.byte_range(),
            }),
        }
    }

    /// Lowers a motive-body `Path` endpoint: an ordinary value expression (the
    /// motive is Walk's own binder form, so endpoints here may be compound —
    /// lists, thunks, pairs — unlike the rung-1 type-position capture). A
    /// hoisting endpoint (an effectful sub-computation) is out of fragment.
    fn walk_motive_endpoint(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<Value>
    {
        let mut hoists = Vec::new();
        let endpoint = self.value_expr(node, &mut hoists)?;
        if !hoists.is_empty() {
            return Err(LowerError::Unsupported {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        }
        endpoint.readback_value()
    }

    /// Lowers `force v` to [`Comp::Force`].
    fn force(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        recursive::force(self, node)
    }

    /// Lowers `ret v` to [`Comp::Ret`]; a computation payload is hoisted
    /// (`ret (x * x)` ⇒ `(x * x) >>= %tmp. ret %tmp`).
    fn ret(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        recursive::ret(self, node)
    }

    /// Lowers a `case` expression, dispatching on the arm constructors: a
    /// `case` whose arms use `Nil` / `Cons` is the list eliminator
    /// ([`Comp::ListCase`], the lowering contract); otherwise it is the sum
    /// case ([`Comp::Case`], `Inl` / `Inr`). The classification is
    /// syntactic — the constructor names, decidable at lowering with no
    /// type information.
    fn case(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        // A `case` over declared constructors, or an EMPTY `case v {}` (the
        // absurd match over an uninhabited datatype — no arm reveals the
        // constructor family, so it can only be the declared-data eliminator),
        // lowers to `Comp::DataCase` (the lowering contract Decision 3).
        if bool::from(self.case_arms_are_data(node)) || bool::from(Self::case_arms_empty(node)) {
            self.data_case(node)
        }
        else if bool::from(self.case_arms_are_list(node)) {
            self.list_case(node)
        }
        else {
            self.sum_case(node)
        }
    }

    /// Whether any arm of a `case` uses a list constructor (`Nil` / `Cons`), so
    /// the `case` lowers to [`Comp::ListCase`] rather than [`Comp::Case`].
    fn case_arms_are_list(
        &self,
        node: SynNode<'_>,
    ) -> ListCaseFlag
    {
        named_non_extra_children(node)
            .into_iter()
            .filter(|arm| arm.kind() == node_kinds::ARM)
            .filter_map(|arm| arm.child_by_field_name(node_kinds::FIELD_PATTERN))
            .filter(|pattern| pattern.kind() == node_kinds::CONSTRUCTOR_PATTERN)
            .filter_map(|pattern| pattern.child_by_field_name(node_kinds::FIELD_CONSTRUCTOR))
            .any(|constructor| {
                matches!(
                    self.text(constructor).map(|text| text.0),
                    Ok(node_kinds::NAME_NIL | node_kinds::NAME_CONS)
                )
            })
            .into()
    }

    /// Lowers `case v { Inl(x) => t, Inr(y) => u }` to [`Comp::Case`], arms
    /// normalized Inl-then-Inr; a missing arm is
    /// [`LowerError::MissingCaseArm`] in this rung.
    fn sum_case(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let mut hoists = Vec::new();
        let scrut_node = required_field(node, node_kinds::FIELD_VALUE)?;
        let scrut = self.value_expr(scrut_node, &mut hoists)?;

        let mut inl: Option<(String, COut)> = None;
        let mut inr: Option<(String, COut)> = None;
        for arm_node in named_non_extra_children(node) {
            if arm_node.kind() != node_kinds::ARM {
                continue;
            }
            // Total mode skips unlowerable arms (the module doc's recorded
            // skipped-arm coarseness); an unfilled `Inl`/`Inr` slot becomes
            // a hole below.
            let (side, binder, body) = match self.case_arm(arm_node) {
                | Err(_) if bool::from(self.total()) => continue,
                | other => other?,
            };
            let slot = match side {
                | Side::Fst => &mut inl,
                | Side::Snd => &mut inr,
            };
            if slot.is_some() {
                // [SPECULATIVE DECISION] Duplicate arms for one constructor
                // are out of fragment (the design names only the missing-arm
                // case); total mode keeps the first.
                if bool::from(self.total()) {
                    continue;
                }
                return Err(LowerError::Unsupported {
                    kind: arm_node.kind(),
                    byte_range: arm_node.byte_range(),
                });
            }
            *slot = Some((binder, body));
        }
        let left = match inl {
            | Some(left) => left,
            | None => {
                let error = LowerError::MissingCaseArm {
                    constructor: node_kinds::NAME_INL,
                    byte_range: node.byte_range(),
                };
                if !bool::from(self.total()) {
                    return Err(error);
                }
                // The hole *is* the missing arm's body (total mode).
                (node_kinds::DISCARD_BINDER.to_owned(), {
                    let hole = self.comp_hole(node, &error)?;
                    core::convert::identity(hole)
                })
            },
        };
        let right = match inr {
            | Some(right) => right,
            | None => {
                let error = LowerError::MissingCaseArm {
                    constructor: node_kinds::NAME_INR,
                    byte_range: node.byte_range(),
                };
                if !bool::from(self.total()) {
                    return Err(error);
                }
                (node_kinds::DISCARD_BINDER.to_owned(), {
                    let hole = self.comp_hole(node, &error)?;
                    core::convert::identity(hole)
                })
            },
        };
        let body = COut::from_legacy_comp(
            &Comp::Case(
                Rc::new({
                    let readback_value = scrut.readback_value()?;
                    core::convert::identity(readback_value)
                }),
                (
                    left.0,
                    Rc::new({
                        let readback_comp = left.1.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                ),
                (
                    right.0,
                    Rc::new({
                        let readback_comp = right.1.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                ),
            ),
            OriginNode::new(entry(node, None), vec![
                scrut.origin,
                left.1.origin,
                right.1.origin,
            ]),
        )?;
        Self::wrap_hoists(hoists, body, node)
    }

    /// Parses one `case` arm: an `Inl`/`Inr` constructor pattern with
    /// exactly one identifier or wildcard argument, and the arm's body in
    /// computation position.
    fn case_arm(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<(Side, String, COut)>
    {
        let pattern = required_field(node, node_kinds::FIELD_PATTERN)?;
        if pattern.kind() != node_kinds::CONSTRUCTOR_PATTERN {
            // Catch-all and tuple arms are out of the binary-sum fragment.
            return Err(LowerError::Unsupported {
                kind: pattern.kind(),
                byte_range: pattern.byte_range(),
            });
        }
        let constructor = required_field(pattern, node_kinds::FIELD_CONSTRUCTOR)?;
        let constructor_text = self.text(constructor)?;
        let side = match constructor_text.0 {
            | node_kinds::NAME_INL => Side::Fst,
            | node_kinds::NAME_INR => Side::Snd,
            | _ => {
                return Err(LowerError::Unsupported {
                    kind: constructor.kind(),
                    byte_range: constructor.byte_range(),
                });
            },
        };
        let pattern_args: Vec<SynNode<'_>> =
            pattern.children_by_field_name(node_kinds::FIELD_ARGUMENT);
        let (Some(&binder_node), 1) = (pattern_args.first(), pattern_args.len())
        else {
            return Err(LowerError::Unsupported {
                kind: pattern.kind(),
                byte_range: pattern.byte_range(),
            });
        };
        let binder = self.pattern_binder(binder_node)?;
        let body_node = required_field(node, node_kinds::FIELD_BODY)?;
        let body = self.comp_expr(body_node)?;
        Ok((side, binder, body))
    }

    /// Lowers `case v { Nil => t, Cons(h, t) => u }` to [`Comp::ListCase`],
    /// arms normalized nil-then-cons; a missing arm is
    /// [`LowerError::MissingCaseArm`] in strict mode (a hole body in total
    /// mode), exactly as [`Self::sum_case`] (the lowering contract). The term
    /// children are the scrutinee (0), the `nil` body (1), and the `cons`
    /// body (2) — the `origin::resolve` order.
    fn list_case(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let mut hoists = Vec::new();
        let scrut_node = required_field(node, node_kinds::FIELD_VALUE)?;
        let scrut = self.value_expr(scrut_node, &mut hoists)?;

        let mut nil: Option<COut> = None;
        let mut cons: Option<(String, String, COut)> = None;
        for arm_node in named_non_extra_children(node) {
            if arm_node.kind() != node_kinds::ARM {
                continue;
            }
            // Total mode skips unlowerable arms (the recorded skipped-arm
            // coarseness); an unfilled `Nil`/`Cons` slot becomes a hole below.
            let arm = match self.list_case_arm(arm_node) {
                | Err(_) if bool::from(self.total()) => continue,
                | other => other?,
            };
            let slot_taken = match arm {
                | ListArm::Nil(body) => {
                    let taken = nil.is_some();
                    if !taken {
                        nil = Some(body);
                    }
                    taken
                },
                | ListArm::Cons(head, tail, body) => {
                    let taken = cons.is_some();
                    if !taken {
                        cons = Some((head, tail, body));
                    }
                    taken
                },
            };
            // A duplicate arm for one constructor is out of fragment; total mode
            // keeps the first (as `sum_case`).
            if slot_taken && !bool::from(self.total()) {
                return Err(LowerError::Unsupported {
                    kind: arm_node.kind(),
                    byte_range: arm_node.byte_range(),
                });
            }
        }

        let nil = match nil {
            | Some(nil) => nil,
            | None => {
                let error = LowerError::MissingCaseArm {
                    constructor: node_kinds::NAME_NIL,
                    byte_range: node.byte_range(),
                };
                if !bool::from(self.total()) {
                    return Err(error);
                }
                self.comp_hole(node, &error)?
            },
        };
        let (head, tail, cons) = match cons {
            | Some(cons) => cons,
            | None => {
                let error = LowerError::MissingCaseArm {
                    constructor: node_kinds::NAME_CONS,
                    byte_range: node.byte_range(),
                };
                if !bool::from(self.total()) {
                    return Err(error);
                }
                (
                    node_kinds::DISCARD_BINDER.to_owned(),
                    node_kinds::DISCARD_BINDER.to_owned(),
                    {
                        let hole = self.comp_hole(node, &error)?;
                        core::convert::identity(hole)
                    },
                )
            },
        };

        let body = COut::from_legacy_comp(
            &Comp::ListCase {
                scrut: Rc::new({
                    let readback_value = scrut.readback_value()?;
                    core::convert::identity(readback_value)
                }),
                nil: Rc::new({
                    let readback_comp = nil.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                head,
                tail,
                cons: Rc::new({
                    let readback_comp = cons.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            },
            OriginNode::new(entry(node, None), vec![
                scrut.origin,
                nil.origin,
                cons.origin,
            ]),
        )?;
        Self::wrap_hoists(hoists, body, node)
    }

    /// Parses one list-`case` arm: a `Nil` (no arguments) or `Cons(head, tail)`
    /// (exactly two identifier/wildcard arguments) constructor pattern, and the
    /// arm's body in computation position (the lowering contract).
    fn list_case_arm(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<ListArm>
    {
        let pattern = required_field(node, node_kinds::FIELD_PATTERN)?;
        if pattern.kind() != node_kinds::CONSTRUCTOR_PATTERN {
            return Err(LowerError::Unsupported {
                kind: pattern.kind(),
                byte_range: pattern.byte_range(),
            });
        }
        let constructor = required_field(pattern, node_kinds::FIELD_CONSTRUCTOR)?;
        let arguments: Vec<SynNode<'_>> =
            pattern.children_by_field_name(node_kinds::FIELD_ARGUMENT);
        let body_node = required_field(node, node_kinds::FIELD_BODY)?;
        let constructor_text = self.text(constructor)?;
        match constructor_text.0 {
            // `Nil` binds nothing.
            | node_kinds::NAME_NIL => {
                if !arguments.is_empty() {
                    return Err(LowerError::Unsupported {
                        kind: pattern.kind(),
                        byte_range: pattern.byte_range(),
                    });
                }
                Ok(ListArm::Nil({
                    let comp = self.comp_expr(body_node)?;
                    core::convert::identity(comp)
                }))
            },
            // `Cons(head, tail)` binds exactly two patterns.
            | node_kinds::NAME_CONS => {
                let (Some(&head_node), Some(&tail_node), 2) =
                    (arguments.first(), arguments.get(1), arguments.len())
                else {
                    return Err(LowerError::Unsupported {
                        kind: pattern.kind(),
                        byte_range: pattern.byte_range(),
                    });
                };
                let head = self.pattern_binder(head_node)?;
                let tail = self.pattern_binder(tail_node)?;
                Ok(ListArm::Cons(head, tail, {
                    let comp = self.comp_expr(body_node)?;
                    core::convert::identity(comp)
                }))
            },
            | _ => Err(LowerError::Unsupported {
                kind: constructor.kind(),
                byte_range: constructor.byte_range(),
            }),
        }
    }

    /// Lowers `if c { t } else { u }` to a `case` on `1 + 1`
    /// ([`ElabKind::IfSugar`]); the consequence is the `inj1` (`true`) arm,
    /// matching [`Self::boolean_literal`]'s polarity.
    fn if_sugar(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let mut hoists = Vec::new();
        let condition_node = required_field(node, node_kinds::FIELD_CONDITION)?;
        let condition = self.value_expr(condition_node, &mut hoists)?;
        let consequence_node = required_field(node, node_kinds::FIELD_CONSEQUENCE)?;
        let consequence = self.block(consequence_node)?;
        let alternative_node = required_field(node, node_kinds::FIELD_ALTERNATIVE)?;
        let alternative = if alternative_node.kind() == node_kinds::BLOCK {
            self.block(alternative_node)?
        }
        else {
            // `else if …` chains: the alternative is itself an
            // `if_expression`, already a computation.
            self.comp_expr(alternative_node)?
        };
        let body = COut::from_legacy_comp(
            &Comp::Case(
                Rc::new({
                    let readback_value = condition.readback_value()?;
                    core::convert::identity(readback_value)
                }),
                (
                    node_kinds::DISCARD_BINDER.to_owned(),
                    Rc::new({
                        let readback_comp = consequence.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                ),
                (
                    node_kinds::DISCARD_BINDER.to_owned(),
                    Rc::new({
                        let readback_comp = alternative.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                ),
            ),
            OriginNode::new(entry(node, Some(ElabKind::IfSugar)), vec![
                condition.origin,
                consequence.origin,
                alternative.origin,
            ]),
        )?;
        Self::wrap_hoists(hoists, body, node)
    }

    /// Lowers `co { fst = t, snd = u }` to [`Comp::With`], fields normalized
    /// fst-then-snd.
    fn co(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let mut fst: Option<COut> = None;
        let mut snd: Option<COut> = None;
        for field in named_non_extra_children(node) {
            if field.kind() != node_kinds::CO_FIELD {
                continue;
            }
            // Total mode skips unlowerable fields (the module doc's
            // skipped-arm coarseness); an unfilled `fst`/`snd` slot becomes
            // a hole below.
            let (Some(name_node), Some(value_node)) = (
                field.child_by_field_name(node_kinds::FIELD_NAME),
                field.child_by_field_name(node_kinds::FIELD_VALUE),
            )
            else {
                if bool::from(self.total()) {
                    continue;
                }
                return Err(LowerError::MalformedNode {
                    kind: field.kind(),
                    byte_range: field.byte_range(),
                });
            };
            let name_text = self.text(name_node)?;
            let slot = match name_text.0 {
                | node_kinds::NAME_FST => &mut fst,
                | node_kinds::NAME_SND => &mut snd,
                // [SPECULATIVE DECISION] n-ary/other-named lazy products are
                // out of fragment (the design covers exactly `fst`/`snd`);
                // total mode skips them.
                | _ => {
                    if bool::from(self.total()) {
                        continue;
                    }
                    return Err(LowerError::Unsupported {
                        kind: field.kind(),
                        byte_range: field.byte_range(),
                    });
                },
            };
            if slot.is_some() {
                if bool::from(self.total()) {
                    continue;
                }
                return Err(LowerError::Unsupported {
                    kind: field.kind(),
                    byte_range: field.byte_range(),
                });
            }
            *slot = Some({
                let comp = self.comp_expr(value_node)?;
                core::convert::identity(comp)
            });
        }
        let missing = |byte_range| LowerError::Unsupported {
            kind: node.kind(),
            byte_range,
        };
        let (fst_out, snd_out) = match (fst, snd) {
            | (Some(fst_out), Some(snd_out)) => (fst_out, snd_out),
            | (fst, snd) if bool::from(self.total()) => {
                let fst_out = match fst {
                    | Some(out) => out,
                    | None => self.comp_hole(node, &missing(node.byte_range()))?,
                };
                let snd_out = match snd {
                    | Some(out) => out,
                    | None => self.comp_hole(node, &missing(node.byte_range()))?,
                };
                (fst_out, snd_out)
            },
            | _ => return Err(missing(node.byte_range())),
        };
        COut::from_legacy_comp(
            &Comp::With(
                Rc::new({
                    let readback_comp = fst_out.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                Rc::new({
                    let readback_comp = snd_out.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            ),
            OriginNode::new(entry(node, None), vec![fst_out.origin, snd_out.origin]),
        )
    }

    /// Lowers a record literal `#{ ℓ = v, … }` to [`Value::Record`] (the design
    /// record D3).
    ///
    /// Each field value lowers in *value position* (a computation field is
    /// hoisted, as a tuple component is). The fields are keyed by label into a
    /// canonical (sorted) [`BTreeMap`], and the origin children follow that
    /// sorted order — the order the checker / machine / mark descend the field
    /// values (`gandr_core_checker::checker`'s `rule_record`), so the per-field
    /// origin indices line up across the faces. The empty record `#{}`
    /// lowers to an empty [`Value::Record`]; a duplicate label is an error
    /// (strict) and keeps the last field (total). The source fields are
    /// deduplicated to their last-wins survivor *before* lowering, so a
    /// discarded duplicate field is never lowered — its hoisted effects
    /// would otherwise run dead in total mode.
    fn record_expr(
        &mut self,
        node: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<VOut>
    {
        let mut field_nodes: BTreeMap<String, SynNode<'_>> = BTreeMap::new();
        for field in named_non_extra_children(node) {
            if field.kind() != node_kinds::RECORD_FIELD {
                continue;
            }
            let (Some(name_node), Some(value_node)) = (
                field.child_by_field_name(node_kinds::FIELD_NAME),
                field.child_by_field_name(node_kinds::FIELD_VALUE),
            )
            else {
                if bool::from(self.total()) {
                    continue;
                }
                return Err(LowerError::MalformedNode {
                    kind: field.kind(),
                    byte_range: field.byte_range(),
                });
            };
            let label = {
                let text = self.text(name_node)?;
                core::convert::identity(text)
            }
            .to_owned();
            if field_nodes.insert(label, value_node).is_some() && !bool::from(self.total()) {
                return Err(LowerError::Unsupported {
                    kind: field.kind(),
                    byte_range: field.byte_range(),
                });
            }
        }
        // Lower the surviving fields in canonical (sorted) label order — the
        // order the checker / machine / mark and the origin descend.
        let mut value_fields: BTreeMap<String, Rc<Value>> = BTreeMap::new();
        let mut origins: Vec<OriginNode> = Vec::with_capacity(field_nodes.len());
        for (label, value_node) in field_nodes {
            let lowered = self.value_expr(value_node, hoists)?;
            value_fields.insert(
                label,
                Rc::new({
                    let readback_value = lowered.readback_value()?;
                    core::convert::identity(readback_value)
                }),
            );
            origins.push(lowered.origin);
        }
        VOut::from_legacy_value(
            &Value::Record(value_fields),
            OriginNode::new(entry(node, None), origins),
        )
    }

    /// Lowers a functional record update `#{ r | ℓ = v, … }` (value-semantics
    /// MVP, `proposal-value-semantics-mvp.md` §3.1) to a fresh-record rebuild
    /// `recordupdate r #{ ℓ = v, … }` over
    /// [`gandr_core_checker::prim::NativePrim::RecordUpdate`]
    /// ([`ElabKind::RecordUpdate`]).
    ///
    /// The base `r` and the overrides record both lower in *value position* (a
    /// computation base / field value is hoisted, as a record literal's is),
    /// and the overrides reuse [`Self::record_expr`]'s canonicalizing
    /// (sorted, last-wins) field lowering — the base child is not a
    /// `record_field`, so it is skipped there. The native overlays the
    /// overrides onto the base and returns a **fresh** record, so no prior
    /// binding of `r` observes the update. This is a derived form over the
    /// existing record former — no frozen-core slot is touched.
    fn record_update(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<EOut>
    {
        let base_node = required_field(node, node_kinds::FIELD_BASE)?;
        let mut hoists = Vec::new();
        let base = self.value_expr(base_node, &mut hoists)?;
        let overrides = self.record_expr(node, &mut hoists)?;
        let elab = Some(ElabKind::RecordUpdate);
        let head = COut::from_legacy_comp(
            &Comp::native(NativePrim::RecordUpdate),
            OriginNode::leaf(entry(node, elab)),
        )?;
        let partial = COut::from_legacy_comp(
            &Comp::App(
                Rc::new({
                    let readback_comp = head.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                Rc::new({
                    let readback_value = base.readback_value()?;
                    core::convert::identity(readback_value)
                }),
            ),
            OriginNode::new(entry(node, elab), vec![head.origin, base.origin]),
        )?;
        let body = COut::from_legacy_comp(
            &Comp::App(
                Rc::new({
                    let readback_comp = partial.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                Rc::new({
                    let readback_value = overrides.readback_value()?;
                    core::convert::identity(readback_value)
                }),
            ),
            OriginNode::new(entry(node, elab), vec![partial.origin, overrides.origin]),
        )?;
        Ok(EOut::Comp({
            let wrapped = Self::wrap_hoists(hoists, body, node)?;
            core::convert::identity(wrapped)
        }))
    }

    /// Lowers a record field projection `record.ℓ` to [`Comp::RecordProj`]
    /// (the lowering contract): the target lowers in *value position* (the
    /// record is a value) and the projection is the resulting computation.
    /// A capitalized target token is accepted here as a variable so
    /// `Module.field` uses the same ordinary projection path as
    /// `module.field` without admitting bare constructors elsewhere. The
    /// dispatch reaches here from [`Self::projection`] for any field name
    /// that is neither a module member (the lowering contract module-select)
    /// nor the structural `fst` / `snd`.
    fn record_projection(
        &mut self,
        node: SynNode<'_>,
        target_node: SynNode<'_>,
        label: String,
    ) -> LowerResult<EOut>
    {
        let mut hoists = Vec::new();
        let record = if target_node.kind() == node_kinds::CONSTRUCTOR {
            let name = {
                let text = self.text(target_node)?;
                core::convert::identity(text)
            }
            .to_owned();
            VOut::from_legacy_value(
                &Value::Var(name),
                OriginNode::leaf(entry(target_node, None)),
            )?
        }
        else {
            self.value_expr(target_node, &mut hoists)?
        };
        let body = COut::from_legacy_comp(
            &Comp::RecordProj {
                record: Rc::new({
                    let readback_value = record.readback_value()?;
                    core::convert::identity(readback_value)
                }),
                label,
            },
            OriginNode::new(entry(node, None), vec![record.origin]),
        )?;
        Ok(EOut::Comp({
            let wrapped = Self::wrap_hoists(hoists, body, node)?;
            core::convert::identity(wrapped)
        }))
    }

    /// Lowers `t.fst` / `t.snd` to [`Comp::Prj`], with the force sugar on a
    /// value target (same rationale as call heads: the principal premise
    /// must be a computation).
    fn projection(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<EOut>
    {
        let field_node = required_field(node, node_kinds::FIELD_FIELD)?;
        let target_node = required_field(node, node_kinds::FIELD_VALUE)?;
        // Module select: a `M.l` whose value is a known module name and whose
        // field is one of its members
        // elaborates to the flat qualified `Var("M.l")` — pure elaboration, no
        // core change (`Value::Var` is already a `String`). Gated on BOTH
        // halves via the prelude registry, so a genuine record projection (once
        // records exist) still falls through to the structural / hole path
        // below.
        let field_text = self.text(field_node)?;
        let target_is_known_module = if target_node.kind() == node_kinds::IDENTIFIER {
            let module = self.text(target_node)?;
            if crate::prelude::is_module_member(module, field_text).0 {
                let mut qualified = module.to_owned();
                qualified.push('.');
                qualified.push_str(field_text.0);
                return Ok(EOut::Value({
                    let selected_value = VOut::from_legacy_value(
                        &Value::var(&qualified),
                        OriginNode::leaf(entry(node, Some(ElabKind::ModuleSelect))),
                    )?;
                    core::convert::identity(selected_value)
                }));
            }
            // A host module (`fs` / `env` / `proc`) has no value-level
            // members: its members exist only as calls (lowered by `host_call`
            // before this path is reached), so any bare selection takes the
            // declined path below.
            crate::prelude::is_module(module).0 || crate::host::is_host_module(module).0
        }
        else {
            false
        };
        // Otherwise it is a structural projection `t.fst` / `t.snd`, the record
        // field projection `t.ℓ` (the lowering contract), or — for a known module with
        // an unknown member — a declined hole (the lowering contract: a module
        // namespace is not a record value, so a non-member is an error rather
        // than a projection). The owned label releases the `field_text` borrow
        // so the record path can take `&mut self`.
        let record_label = match field_text.0 {
            | node_kinds::NAME_FST | node_kinds::NAME_SND => None,
            | _ if target_is_known_module => {
                return Err(LowerError::Unsupported {
                    kind: node.kind(),
                    byte_range: node.byte_range(),
                });
            },
            | other => Some(other.to_owned()),
        };
        if let Some(label) = record_label {
            // A `.π` whose field is a declared codata observation is an
            // observation `force(RecordProj(s, π))` (design §3.1), not a plain
            // record projection — the label-driven analogue of module-select
            // disambiguation. A plain record field that happens to
            // share an observation name would also route here (the MVP carrier
            // has no nominal opacity); forcing a non-thunk field is a defined
            // `Stuck`, and route (b) removes the ambiguity.
            if self.is_observation(label.as_str().into()).0 {
                return self.codata_observation(node, target_node, label);
            }
            return self.record_projection(node, target_node, label);
        }
        let side = match field_text.0 {
            | node_kinds::NAME_FST => Side::Fst,
            | node_kinds::NAME_SND => Side::Snd,
            // Unreachable: the record-field case returned above; only `fst` /
            // `snd` remain.
            | _ => {
                return Err(LowerError::Unsupported {
                    kind: node.kind(),
                    byte_range: node.byte_range(),
                });
            },
        };
        let mut hoists = Vec::new();
        // A failed target becomes a computation-hole target in total mode
        // (same recovery shape as a call head).
        let lowered_target = match self.expr(target_node, &mut hoists) {
            | Err(ref error) if bool::from(self.total()) => EOut::Comp({
                let hole = self.comp_hole(target_node, error)?;
                core::convert::identity(hole)
            }),
            | other => other?,
        };
        let target = match lowered_target {
            | EOut::Comp(comp) => comp,
            | EOut::Value(value) => COut::from_legacy_comp(
                &Comp::Force(Rc::new({
                    let readback_value = value.readback_value()?;
                    core::convert::identity(readback_value)
                })),
                OriginNode::new(entry(target_node, Some(ElabKind::ForceSugar)), vec![
                    value.origin,
                ]),
            )?,
        };
        let body = COut::from_legacy_comp(
            &Comp::Prj(
                side,
                Rc::new({
                    let readback_comp = target.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            ),
            OriginNode::new(entry(node, None), vec![target.origin]),
        )?;
        Ok(EOut::Comp({
            let wrapped = Self::wrap_hoists(hoists, body, node)?;
            core::convert::identity(wrapped)
        }))
    }

    /// Lowers `l ⊕ r` to `(force ⊕̂) l r` where `⊕̂` is the prelude operator
    /// of [`node_kinds::BINARY_OPERATORS`] ([`ElabKind::OperatorElab`]).
    fn binary(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let (operator_node, operator_name) = binary_operator(node)?;
        let mut hoists = Vec::new();
        let left_node = required_field(node, node_kinds::FIELD_LEFT)?;
        let left = self.value_expr(left_node, &mut hoists)?;
        let right_node = required_field(node, node_kinds::FIELD_RIGHT)?;
        let right = self.value_expr(right_node, &mut hoists)?;
        let elab = Some(ElabKind::OperatorElab);
        let var = VOut::from_legacy_value(
            &Value::var(operator_name.0),
            OriginNode::leaf(entry(operator_node, elab)),
        )?;
        let head = COut::from_legacy_comp(
            &Comp::Force(Rc::new({
                let readback_value = var.readback_value()?;
                core::convert::identity(readback_value)
            })),
            OriginNode::new(entry(operator_node, elab), vec![var.origin]),
        )?;
        let partial = COut::from_legacy_comp(
            &Comp::App(
                Rc::new({
                    let readback_comp = head.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                Rc::new({
                    let readback_value = left.readback_value()?;
                    core::convert::identity(readback_value)
                }),
            ),
            OriginNode::new(entry(node, elab), vec![head.origin, left.origin]),
        )?;
        let body = COut::from_legacy_comp(
            &Comp::App(
                Rc::new({
                    let readback_comp = partial.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                Rc::new({
                    let readback_value = right.readback_value()?;
                    core::convert::identity(readback_value)
                }),
            ),
            OriginNode::new(entry(node, elab), vec![partial.origin, right.origin]),
        )?;
        Self::wrap_hoists(hoists, body, node)
    }

    /// Lowers `-v` to `(force neg) v` ([`ElabKind::OperatorElab`]).
    fn unary(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        recursive::unary(self, node)
    }

    /// Lowers the bootstrap `#!{ … }` shell surface to host-effect
    /// operations. This intentionally accepts only flat simple commands
    /// separated by `;`/newlines/`&`; fuller shell control (pipes, `&&`, `||`,
    /// subshells, redirects, command substitutions, variable expansion, job
    /// control) remains out of fragment. A standalone `$(gandr-expression)`
    /// argument is evaluated before its containing command and contributes one
    /// typed `String` argv element; adjacent literal/escape fragments are one
    /// lexical word and are rejected until a typed concat operation lands.
    fn shell_block(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        let mut commands = Vec::new();
        self.collect_shell_commands(node, &mut commands)?;
        let Some(last) = commands.pop()
        else {
            return COut::from_legacy_comp(
                &Comp::ret(Value::Unit),
                OriginNode::leaf(entry(node, None)),
            );
        };

        let result_name = self.fresh_name();
        let result = Value::var(&result_name);
        let last_exec = Self::wrap_shell_command_hoists(
            last.hoists,
            &Self::exec_command(last.command),
            last.node,
            last.origin,
        )?;
        let result_origin = OriginNode::leaf(entry(last.node, None));
        let ret_origin = OriginNode::new(entry(last.node, None), vec![result_origin]);
        let mut acc = COut::from_legacy_comp(
            &Comp::bind(
                {
                    let readback_comp = last_exec.readback_comp()?;
                    core::convert::identity(readback_comp)
                },
                &result_name,
                Comp::ret(result),
            ),
            OriginNode::new(entry(last.node, None), vec![last_exec.origin, ret_origin]),
        )?;
        while let Some(command) = commands.pop() {
            let command_exec = Self::wrap_shell_command_hoists(
                command.hoists,
                &Self::exec_command(command.command),
                command.node,
                command.origin,
            )?;
            let comp = Comp::Bind(
                Rc::new({
                    let readback_comp = command_exec.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
                node_kinds::DISCARD_BINDER.to_owned(),
                Rc::new({
                    let readback_comp = acc.readback_comp()?;
                    core::convert::identity(readback_comp)
                }),
            );
            acc = COut::from_legacy_comp(
                &comp,
                OriginNode::new(entry(command.node, Some(ElabKind::SeqDiscard)), vec![
                    command_exec.origin,
                    acc.origin,
                ]),
            )?;
        }
        Ok(acc)
    }

    /// Collects the flat simple commands accepted in a shell block.
    fn collect_shell_commands<'tree>(
        &mut self,
        node: SynNode<'tree>,
        commands: &mut Vec<ShellCommand<'tree>>,
    ) -> LowerResult<()>
    {
        let mut pending = vec![node];
        while let Some(current) = pending.pop() {
            match current.kind() {
                | node_kinds::SHELL_BLOCK | node_kinds::SHELL_LIST => {
                    let children = named_non_extra_children(current);
                    pending.extend(children.into_iter().rev());
                },
                | node_kinds::LIST_OPERATOR => {},
                | node_kinds::COMMAND => commands.push({
                    let shell_command = self.shell_command(current)?;
                    core::convert::identity(shell_command)
                }),
                | node_kinds::PIPELINE
                | node_kinds::AND_EXPRESSION
                | node_kinds::OR_EXPRESSION
                | node_kinds::SUBSHELL
                | node_kinds::HOST_ESCAPE
                | node_kinds::COMMAND_SUBSTITUTION
                | node_kinds::VARIABLE_EXPANSION
                | node_kinds::REDIRECTION
                | node_kinds::ENVIRONMENT_ASSIGNMENT
                | node_kinds::NEGATION => {
                    return Err(LowerError::Unsupported {
                        kind: current.kind(),
                        byte_range: current.byte_range(),
                    });
                },
                | kind => {
                    return Err(LowerError::MalformedNode {
                        kind,
                        byte_range: current.byte_range(),
                    });
                },
            }
        }
        Ok(())
    }

    /// Lowers one simple shell command into an `Exec::exec` payload.
    fn shell_command<'tree>(
        &mut self,
        node: SynNode<'tree>,
    ) -> LowerResult<ShellCommand<'tree>>
    {
        let mut parts = Vec::new();
        let mut hoists = Vec::new();
        for child in named_non_extra_children(node) {
            match child.kind() {
                | node_kinds::COMMAND_NAME
                | node_kinds::SINGLE_QUOTED_STRING
                | node_kinds::DOUBLE_QUOTED_STRING => {
                    parts.push(ShellPart::literal(
                        {
                            let shell_atom = self.shell_atom(child)?;
                            core::convert::identity(shell_atom)
                        },
                        child,
                    ));
                },
                | node_kinds::ARGUMENT => {
                    parts.push({
                        let shell_argument = self.shell_argument(child, &mut hoists)?;
                        core::convert::identity(shell_argument)
                    });
                },
                | node_kinds::HOST_ESCAPE => {
                    parts.push({
                        let shell_host_escape = self.shell_host_escape(child, &mut hoists)?;
                        core::convert::identity(shell_host_escape)
                    });
                },
                | node_kinds::REDIRECTION
                | node_kinds::COMMAND_SUBSTITUTION
                | node_kinds::VARIABLE_EXPANSION
                | node_kinds::ENVIRONMENT_ASSIGNMENT
                | node_kinds::NEGATION => {
                    return Err(LowerError::Unsupported {
                        kind: child.kind(),
                        byte_range: child.byte_range(),
                    });
                },
                | kind => {
                    return Err(LowerError::MalformedNode {
                        kind,
                        byte_range: child.byte_range(),
                    });
                },
            }
        }
        let mut words = self.shell_words(parts).into_iter();
        let Some(program_word) = words.next()
        else {
            return Err(LowerError::MalformedNode {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        };
        let (program, program_origin) = Self::shell_program_word(program_word, node)?;
        let mut args = Vec::new();
        let mut arg_origins = Vec::new();
        for word in words {
            let (arg, origin) = self.shell_arg_word(word, node, &mut hoists)?;
            args.push(arg);
            arg_origins.push(origin);
        }
        let origin = Self::exec_command_origin(node, program_origin, arg_origins);
        let command = HostCommand { program, args };
        Ok(ShellCommand {
            command,
            hoists,
            node,
            origin,
        })
    }

    /// Regroups adjacent command fragments into lexical shell words.
    fn shell_words(
        &self,
        parts: Vec<ShellPart>,
    ) -> Vec<ShellWord>
    {
        let mut words: Vec<ShellWord> = Vec::new();
        for part in parts {
            let append = words.last().is_some_and(|word| {
                bool::from(self.same_shell_word(word.range.end.into(), part.range.start.into()))
            });
            if append {
                if let Some(word) = words.last_mut() {
                    word.push(part);
                }
            }
            else {
                words.push(ShellWord::new(part));
            }
        }
        words
    }

    /// Whether two adjacent fragments are part of the same shell word.
    fn same_shell_word(
        &self,
        left_end: SourceOffset,
        right_start: SourceOffset,
    ) -> ShellWordContinuation
    {
        let left_end = usize::from(left_end);
        let right_start = usize::from(right_start);
        if left_end > right_start {
            return false.into();
        }
        self.source
            .get(left_end .. right_start)
            .is_some_and(|gap| !gap.chars().any(char::is_whitespace))
            .into()
    }

    /// Lowers the program word; host escapes are never valid in `argv[0]`.
    fn shell_program_word(
        word: ShellWord,
        command: SynNode<'_>,
    ) -> LowerResult<(String, OriginNode)>
    {
        if let Some(part) = word
            .parts
            .iter()
            .find(|part| bool::from(part.is_host_escape()))
        {
            return Err(LowerError::Unsupported {
                kind: node_kinds::HOST_ESCAPE,
                byte_range: part.range.clone(),
            });
        }
        Self::literal_shell_word(word, command)
    }

    /// Lowers one non-program shell word to exactly one argv value.
    fn shell_arg_word(
        &mut self,
        word: ShellWord,
        command: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<(Value, OriginNode)>
    {
        if word.parts.iter().all(|part| !part.is_host_escape()) {
            let (text, origin) = Self::literal_shell_word(word, command)?;
            return Ok((Value::string(&text), origin));
        }
        if word.parts.len() != 1 {
            return Err(LowerError::Unsupported {
                kind: node_kinds::HOST_ESCAPE,
                byte_range: word.range,
            });
        }

        let range = word.range;
        let mut parts = word.parts.into_iter();
        let Some(part) = parts.next()
        else {
            return Err(LowerError::MalformedNode {
                kind: command.kind(),
                byte_range: range,
            });
        };
        let ShellPartValue::HostEscape(value) = part.value
        else {
            return Err(LowerError::MalformedNode {
                kind: command.kind(),
                byte_range: part.range,
            });
        };
        let bound = COut::from_legacy_comp(
            &Comp::ret(value),
            OriginNode::new(
                Self::shell_word_entry(command, range.clone(), Some(ElabKind::RetCoercion)),
                vec![part.origin],
            ),
        )?;
        let name = self.fresh_name();
        hoists.push(Hoist {
            name: name.clone(),
            bound,
        });
        let var_origin = OriginNode::leaf(Self::shell_word_entry(
            command,
            range.clone(),
            Some(ElabKind::BindHoist),
        ));
        Ok((
            Value::annot(Value::var(&name), ValueType::string()),
            OriginNode::new(
                Self::shell_word_entry(command, range, Some(ElabKind::BindHoist)),
                vec![var_origin],
            ),
        ))
    }

    /// Concatenates a literal-only shell word.
    fn literal_shell_word(
        word: ShellWord,
        command: SynNode<'_>,
    ) -> LowerResult<(String, OriginNode)>
    {
        let mut text = String::new();
        let range = word.range;
        for part in word.parts {
            match part.value {
                | ShellPartValue::Literal(fragment) => text.push_str(&fragment),
                | ShellPartValue::HostEscape(_) => {
                    return Err(LowerError::Unsupported {
                        kind: node_kinds::HOST_ESCAPE,
                        byte_range: part.range,
                    });
                },
            }
        }
        Ok((
            text,
            OriginNode::leaf(Self::shell_word_entry(command, range, None)),
        ))
    }

    /// Builds an origin entry for a synthetic shell word node.
    fn shell_word_entry(
        command: SynNode<'_>,
        range: SourceRange,
        elaboration: Option<ElabKind>,
    ) -> OriginEntry
    {
        let mut origin = entry(command, elaboration);
        origin.byte_range = range;
        origin
    }

    /// Decodes one shell argument wrapper.
    fn shell_argument(
        &mut self,
        node: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<ShellPart>
    {
        let Some(child) = named_non_extra_children(node).into_iter().next()
        else {
            return Err(LowerError::MalformedNode {
                kind: node.kind(),
                byte_range: node.byte_range(),
            });
        };
        match child.kind() {
            | node_kinds::SHELL_WORD
            | node_kinds::SINGLE_QUOTED_STRING
            | node_kinds::DOUBLE_QUOTED_STRING => Ok(ShellPart::literal(
                {
                    let shell_atom = self.shell_atom(child)?;
                    core::convert::identity(shell_atom)
                },
                child,
            )),
            | node_kinds::HOST_ESCAPE => self.shell_host_escape(child, hoists),
            | node_kinds::COMMAND_SUBSTITUTION | node_kinds::VARIABLE_EXPANSION => {
                Err(LowerError::Unsupported {
                    kind: child.kind(),
                    byte_range: child.byte_range(),
                })
            },
            | kind => Err(LowerError::MalformedNode {
                kind,
                byte_range: child.byte_range(),
            }),
        }
    }

    /// Lowers `$(gandr-expression)` to one checked `String` fragment.
    ///
    /// # Contract
    /// - requires: `node` is a `host_escape` CST node in shell argument
    ///   position.
    /// - ensures: lowers the interior expression in value position, preserving
    ///   left-to-right expression hoists in `hoists`.
    /// - provides: an annotated `String` fragment for its containing shell
    ///   word.
    /// - fails: returns the structured lowering error for a malformed escape or
    ///   unsupported interior expression.
    /// - panics: none.
    fn shell_host_escape(
        &mut self,
        node: SynNode<'_>,
        hoists: &mut Vec<Hoist>,
    ) -> LowerResult<ShellPart>
    {
        let expr_node = Self::host_escape_expression(node)?;
        let lowered = self.value_expr(expr_node, hoists)?;
        let origin = OriginNode::new(entry(node, None), vec![lowered.origin.clone()]);
        let value = Value::annot(
            {
                let readback_value = lowered.readback_value()?;
                core::convert::identity(readback_value)
            },
            ValueType::string(),
        );
        Ok(ShellPart {
            value: ShellPartValue::HostEscape(value),
            origin,
            range: node.byte_range(),
        })
    }

    /// Returns the interior expression of a shell host escape.
    ///
    /// # Contract
    /// - requires: `node` is a `host_escape` CST node.
    /// - ensures: uses the front-end's total `expression` field projection,
    ///   which rejects host escapes with extra significant body elements.
    /// - fails: returns [`LowerError::MalformedNode`] when the escape has no
    ///   resolvable single expression child.
    /// - panics: none.
    fn host_escape_expression(node: SynNode<'_>) -> LowerResult<SynNode<'_>>
    {
        node.child_by_field_name(node_kinds::FIELD_EXPRESSION)
            .ok_or_else(|| LowerError::MalformedNode {
                kind: node.kind(),
                byte_range: node.byte_range(),
            })
    }

    /// Decodes a command name, bare word, or quoted string token.
    fn shell_atom(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<String>
    {
        let text = self.text(node)?;
        match node.kind() {
            | node_kinds::COMMAND_NAME | node_kinds::SHELL_WORD => Ok(text.to_owned()),
            | node_kinds::SINGLE_QUOTED_STRING => {
                let Some(inner) = text
                    .strip_prefix('\'')
                    .and_then(|rest| rest.strip_suffix('\''))
                else {
                    return Err(LowerError::MalformedNode {
                        kind: node.kind(),
                        byte_range: node.byte_range(),
                    });
                };
                Ok(inner.to_owned())
            },
            | node_kinds::DOUBLE_QUOTED_STRING => {
                // A double-quoted argument carrying an escape sequence (a
                // backslash) is out of the no-interpolation fragment. The
                // melder inlines the escape as a flat `escape_sequence` tile
                // (not a named child), so the region is detected by its text —
                // any backslash — rather than by sub-node presence.
                if !named_non_extra_children(node).is_empty() || text.contains('\\') {
                    return Err(LowerError::Unsupported {
                        kind: node.kind(),
                        byte_range: node.byte_range(),
                    });
                }
                let Some(inner) = text
                    .strip_prefix('"')
                    .and_then(|rest| rest.strip_suffix('"'))
                else {
                    return Err(LowerError::MalformedNode {
                        kind: node.kind(),
                        byte_range: node.byte_range(),
                    });
                };
                Ok(Self::decode_string_escapes(inner.into()))
            },
            | kind => Err(LowerError::MalformedNode {
                kind,
                byte_range: node.byte_range(),
            }),
        }
    }

    /// Builds `perform Exec::exec {program, args, mode = "captured"}`.
    ///
    /// The lowerer always emits [`MODE_CAPTURED`](crate::host::MODE_CAPTURED):
    /// every `#!{ … }` block produces a consumed result (a bound value), so the
    /// corpus is unchanged (the lowering contract). The inherit mode is a
    /// driver-level decision for a discarded REPL command line, never a
    /// lowering of source.
    fn exec_command(command: HostCommand) -> Comp
    {
        let payload = Value::record([
            (
                crate::host::FIELD_PROGRAM.to_owned(),
                Value::string(&command.program),
            ),
            (
                crate::host::FIELD_ARGS.to_owned(),
                Value::list(command.args),
            ),
            (
                crate::host::FIELD_MODE.to_owned(),
                Value::string(crate::host::MODE_CAPTURED),
            ),
        ]);
        Comp::perform(crate::host::exec(), crate::host::EXEC_RUN, payload)
    }

    /// Builds the origin shadow for the `Exec::exec` perform and payload.
    ///
    /// # Contract
    /// - requires: `program_origin` belongs to `command`'s program token, and
    ///   `arg_origins` are in argv order.
    /// - ensures: record-field origins follow canonical field order (`args`,
    ///   `mode`, `program`) and argv origins follow source order.
    /// - provides: precise paths for type diagnostics inside computed argv.
    /// - panics: none.
    fn exec_command_origin(
        command: SynNode<'_>,
        program_origin: OriginNode,
        arg_origins: Vec<OriginNode>,
    ) -> OriginNode
    {
        let args_origin = OriginNode::new(entry(command, None), arg_origins);
        let mode_origin = OriginNode::leaf(entry(command, None));
        let payload_origin = OriginNode::new(entry(command, None), vec![
            args_origin,
            mode_origin,
            program_origin,
        ]);
        OriginNode::new(entry(command, None), vec![payload_origin])
    }

    /// Wraps a shell command's `Exec::exec` in host-expression hoists.
    ///
    /// # Contract
    /// - requires: `origin` describes the command whose `exec` is being
    ///   wrapped.
    /// - ensures: hoists run in source order before `exec`.
    /// - provides: `exec` unchanged when the command has no host escapes.
    /// - fails: propagates arena bridge failures while assembling the bind
    ///   chain.
    /// - panics: none.
    fn wrap_shell_command_hoists(
        hoists: Vec<Hoist>,
        exec: &Comp,
        node: SynNode<'_>,
        origin: OriginNode,
    ) -> LowerResult<COut>
    {
        let host = entry(node, Some(ElabKind::BindHoist));
        let body = COut::from_legacy_comp(exec, origin)?;
        Self::wrap_hoists_entry(hoists, body, &host)
    }

    // --- Blocks and statements -------------------------------------------------

    /// Lowers a block `{ s; …; t }`: statements desugar onto the bind-chain
    /// spine and the tail is the block's computation.
    fn block(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<COut>
    {
        recursive::block(self, node)
    }

    /// The binder of an identifier or wildcard pattern (wildcards bind the
    /// `_` discard name); other pattern forms are out of fragment.
    ///
    /// [SPECULATIVE DECISION] Nested tuple and constructor sub-patterns are
    /// out of the covered fragment (the design names only "tuple pattern" and
    /// "identifier pattern").
    fn pattern_binder(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<String>
    {
        match node.kind() {
            | node_kinds::IDENTIFIER => Ok({
                let text = self.text(node)?;
                core::convert::identity(text)
            }
            .to_owned()),
            | node_kinds::WILDCARD => Ok(node_kinds::DISCARD_BINDER.to_owned()),
            | kind => Err(LowerError::Unsupported {
                kind,
                byte_range: node.byte_range(),
            }),
        }
    }

    /// Parses a `parameters` node into `(name, annotation)` pairs.
    fn parameters(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<Vec<(String, Option<ValueType>)>>
    {
        let mut params = Vec::new();
        for parameter in named_non_extra_children(node) {
            if parameter.kind() != node_kinds::PARAMETER {
                continue;
            }
            let name_node = required_field(parameter, node_kinds::FIELD_NAME)?;
            let name = {
                let text = self.text(name_node)?;
                core::convert::identity(text)
            }
            .to_owned();
            let annotation = match parameter.child_by_field_name(node_kinds::FIELD_TYPE) {
                // Route through `lower_value_type_node` so a declared-datatype
                // parameter annotation `f(m: Maybe(a))` sees the nominal handle
                // (the lowering contract).
                | Some(ty_node) => Some({
                    let lower_value_type_node = self.lower_value_type_node(ty_node)?;
                    core::convert::identity(lower_value_type_node)
                }),
                | None => None,
            };
            params.push((name, annotation));
        }
        Ok(params)
    }

    // --- Items ---------------------------------------------------------------

    /// Lowers the `source_file` node: items in order, with `def_signature`
    /// ascription matching and origin-map assembly. In total mode, recovery
    /// is source-form-local: a failed import becomes an unnamed hole item; a
    /// failed item becomes a hole item with a best-effort name so signature
    /// attachment still works; an `ERROR` root becomes one hole item; and each
    /// dangling signature becomes an item whose hole term is the missing
    /// definition — the signature is its recorded goal.
    fn source_file(
        &mut self,
        root: SynNode<'_>,
    ) -> LowerResult<Lowered>
    {
        let mut sigs: Vec<PendingSig> = Vec::new();
        let mut items: Vec<Item> = Vec::new();
        let mut origins: Vec<OriginNode> = Vec::new();
        let mut attributes: Vec<RawAttr> = Vec::new();
        let mut foreign: Vec<ForeignModule> = Vec::new();
        let mut codata: BTreeMap<String, codata::CodataDecl> = BTreeMap::new();
        let mut imports: Vec<ImportDeclaration> = Vec::new();
        let mut namespace_handler = ImportNamespaceHandler;

        // The whole file failed to parse as items when it yields no item node
        // yet buffered parse obligations (garbage like `}{` / `@@@`): the melder
        // root is always a well-formed `source_file` node, so — unlike the
        // retired tree-sitter `ERROR` root — the signal is "no items, but
        // errors". One hole item covers the file. (Strict mode reports the
        // obligation as a `Syntax` error before reaching here; the branch keeps
        // this total on any input.)
        if named_non_extra_children(root).is_empty() && !self.obligations.is_empty() {
            let error = LowerError::Syntax {
                byte_range: root.byte_range(),
            };
            if !bool::from(self.total()) {
                return Err(error);
            }
            let hole = self.value_hole(root, &error)?;
            items.push(Item {
                name: None,
                ascription: None,
                term: Term::Value({
                    let readback_value = hole.readback_value()?;
                    core::convert::identity(readback_value)
                }),
            });
            origins.push(hole.origin);
        }
        else {
            // Pre-pass: collect every `extern` block into the foreign registry
            // before lowering any item, so a call `m.op(args)` elaborates
            // against its module regardless of source order (proposal-ffi.md
            // §2/§3.1). `extern` blocks are declarations, not runnable items —
            // they contribute no [`Item`].
            for child in named_non_extra_children(root) {
                if child.kind() != node_kinds::EXTERN_BLOCK {
                    continue;
                }
                match self.extern_block(child) {
                    | Ok(module) => {
                        self.foreign.insert(module.name.clone(), module.clone());
                        foreign.push(module);
                    },
                    // A malformed `extern` block declares nothing runnable: strict
                    // mode propagates, total mode drops it (no hole item — an
                    // extern block is not a term).
                    | Err(error) if !bool::from(self.total()) => return Err(error),
                    | Err(_dropped) => {},
                }
            }
            // Pre-pass: register every `codata` block into the codata registry
            // (the codata-declaration contract), before lowering any item, so a copattern
            // definition `def rec f() -> C { … }` is coverage-checked against
            // `C`'s observations regardless of source order. Like `extern`, a
            // `codata` block is a declaration, not a runnable item — it yields
            // no [`Item`].
            for child in named_non_extra_children(root) {
                if child.kind() != node_kinds::CODATA_DECLARATION {
                    continue;
                }
                match self.collect_codata(child, &mut codata) {
                    | Ok(()) => {},
                    | Err(error) if !bool::from(self.total()) => return Err(error),
                    | Err(_dropped) => {},
                }
            }
            // Pre-pass: register every `data D(ā) { … }` block into the data
            // registry (the data-declaration contract), before lowering any item, so a
            // constructor application `C(v̄)` or `case v { … }` resolves against
            // the datatype regardless of source order. Like `codata`, a `data`
            // block is a declaration, not a runnable item — it yields no
            // [`Item`]. The pass reads the stage-0 descriptions (the
            // `NominalId → DataId` seam), so it is infallible here.
            self.collect_data();
            for child in named_non_extra_children(root) {
                if child.kind() == node_kinds::IMPORT_DECLARATION {
                    let lowered = self
                        .import_index_base
                        .0
                        .checked_add(imports.len())
                        .map(ImportIndex)
                        .ok_or_else(|| LowerError::ImportIndexOverflow {
                            byte_range: child.byte_range(),
                        })
                        .and_then(|index| {
                            let (declaration, qualified) =
                                self.lower_import(child, index, &mut namespace_handler)?;
                            self.import_scope
                                .import_subtree(
                                    &NamePath::root(),
                                    qualified,
                                    &mut namespace_handler,
                                )
                                .map_err(|error| LowerError::Namespace {
                                    error,
                                    byte_range: child.byte_range(),
                                })?;
                            Ok(declaration)
                        });
                    match lowered {
                        | Ok(declaration) => imports.push(declaration),
                        | Err(ref error) if bool::from(self.total()) => {
                            let hole = self.value_hole(child, error)?;
                            items.push(Item {
                                name: None,
                                ascription: None,
                                term: Term::Value({
                                    let readback_value = hole.readback_value()?;
                                    core::convert::identity(readback_value)
                                }),
                            });
                            origins.push(hole.origin);
                        },
                        | Err(error) => return Err(error),
                    }
                    continue;
                }
                if matches!(
                    child.kind(),
                    node_kinds::EXTERN_BLOCK
                        | node_kinds::CODATA_DECLARATION
                        | node_kinds::DATA_DECLARATION
                        | node_kinds::SIGN_DECLARATION
                        | node_kinds::CIRCUIT_DECLARATION
                ) {
                    // Consumed by the pre-passes above (a `data` block is a
                    // declaration, not a runnable item; the lowering contract).
                    // A `sign` block and a top-level circuit declaration are
                    // declarations in the same sense: their route is the
                    // description one ([`crate::circuit::desc`]), which reads
                    // them into the declaration table, so term lowering sees a
                    // declaration rather than an unsupported expression.
                    continue;
                }
                if child.kind() == node_kinds::DEF_SIGNATURE {
                    match self.def_signature(child) {
                        | Ok(sig) => sigs.push(sig),
                        // A damaged signature item becomes a hole item.
                        | Err(ref error) if bool::from(self.total()) => {
                            let hole = self.value_hole(child, error)?;
                            items.push(Item {
                                name: None,
                                ascription: None,
                                term: Term::Value({
                                    let readback_value = hole.readback_value()?;
                                    core::convert::identity(readback_value)
                                }),
                            });
                            origins.push(hole.origin);
                        },
                        | Err(error) => return Err(error),
                    }
                    continue;
                }
                let item_index = items.len();
                match self.item(child) {
                    | Ok((mut item, origin)) => {
                        // An explicit signature wins over any sugar-derived
                        // ascription (it is the user's stated contract).
                        if let Some(name) = item.name.as_deref()
                            && let Some(sig_ty) = take_sig(&mut sigs, name.into())
                        {
                            item.ascription = Some(sig_ty);
                        }
                        items.push(item);
                        origins.push(origin);
                    },
                    // Item-level recovery: the item's term is a hole; a
                    // best-effort name keeps signature attachment working.
                    | Err(ref error) if bool::from(self.total()) => {
                        let name = child
                            .child_by_field_name(node_kinds::FIELD_NAME)
                            .and_then(|name_node| self.text(name_node).ok())
                            .map(NodeText::to_owned);
                        let ascription = name
                            .as_deref()
                            .and_then(|item_name| take_sig(&mut sigs, item_name.into()));
                        let hole = self.value_hole(child, error)?;
                        items.push(Item {
                            name,
                            ascription,
                            term: Term::Value({
                                let readback_value = hole.readback_value()?;
                                core::convert::identity(readback_value)
                            }),
                        });
                        origins.push(hole.origin);
                    },
                    | Err(error) => return Err(error),
                }
                // Attributes ride the item at `item_index` (both the success
                // and the recovery branch push exactly one item there); a
                // child with no `@[…]` blocks contributes none.
                self.item_attributes(child, item_index.into(), &mut attributes)?;
            }
        }

        if bool::from(self.total()) {
            // Dangling signatures: the hole is the missing definition; the
            // signature is its goal (appended after the real items, in
            // signature order).
            for sig in sigs.into_iter().filter(|sig| !sig.used) {
                let error = LowerError::DanglingSignature {
                    name: sig.name.clone(),
                    byte_range: sig.byte_range.clone(),
                };
                let hole_id = self.fresh_hole();
                items.push(Item {
                    name: Some(sig.name),
                    ascription: Some(sig.ty),
                    term: Term::Value(Value::Hole(hole_id.into())),
                });
                origins.push(OriginNode::leaf(OriginEntry {
                    cst_node: sig.node,
                    cst_hash: sig.hash,
                    byte_range: sig.byte_range,
                    elaboration: None,
                    note: Some(note_of(&error)),
                }));
            }
        }
        else if let Some(dangling) = sigs.iter().find(|sig| !sig.used) {
            return Err(LowerError::DanglingSignature {
                name: dangling.name.clone(),
                byte_range: dangling.byte_range.clone(),
            });
        }

        let mut origin_map = OriginMap::default();
        for (index, origin) in origins.into_iter().enumerate() {
            origin_map.insert_root(index, origin);
        }
        Ok(Lowered {
            items,
            foreign,
            imports,
            codata,
            data: core::mem::take(&mut self.data),
            origin: origin_map,
            attributes,
            import_scope: core::mem::take(&mut self.import_scope),
        })
    }

    /// Lowers one `import "URI" as name ;` declaration through the namespace
    /// engine, retaining its surface operands and rejecting an alias already
    /// bound by an earlier import.
    fn lower_import(
        &self,
        node: SynNode<'_>,
        index: ImportIndex,
        handler: &mut ImportNamespaceHandler,
    ) -> LowerResult<(ImportDeclaration, Trie<ImportIndex, SourceRange>)>
    {
        let uri = self.quoted_string_field(node, node_kinds::FIELD_URI)?;
        let alias_node = required_field(node, node_kinds::FIELD_ALIAS)?;
        let alias = Segment::from(self.text(alias_node)?.to_owned());
        let imported: Trie<ImportIndex, SourceRange> =
            core::iter::once((NamePath::root(), Binding::new(index, node.byte_range()))).collect();
        let qualified = Modifier::alias_as(alias.clone())
            .apply(imported, handler)
            .map_err(|error| LowerError::Namespace {
                error: ScopeError::from(error),
                byte_range: node.byte_range(),
            })?;
        Ok((ImportDeclaration { uri, alias }, qualified))
    }

    /// Collects the leading `@[…]` attribute blocks of one item node into
    /// [`RawAttr`]s keyed on `item_index` (proposal-attributes.md §2). A node
    /// with no `attribute` field (an expression item, or any non-`def` form)
    /// contributes nothing.
    ///
    /// Name-and-payload only: the schema lookup and typing are the
    /// [`crate::attributes`] pass. The payload lowers to its value fragment
    /// here so that pass can type it with the ordinary checker.
    fn item_attributes(
        &mut self,
        node: SynNode<'_>,
        item_index: ItemIndex,
        out: &mut Vec<RawAttr>,
    ) -> LowerResult<()>
    {
        let blocks: Vec<SynNode<'_>> = node.children_by_field_name(node_kinds::FIELD_ATTRIBUTE);
        for block in blocks {
            for attr in named_non_extra_children(block) {
                if attr.kind() != node_kinds::ATTRIBUTE {
                    continue;
                }
                let name_node = required_field(attr, node_kinds::FIELD_NAME)?;
                let payload = match attr.child_by_field_name(node_kinds::FIELD_PAYLOAD) {
                    | Some(payload_node) => Some({
                        let attribute_payload = self.attribute_payload(payload_node)?;
                        core::convert::identity(attribute_payload)
                    }),
                    | None => None,
                };
                out.push(RawAttr {
                    item: usize::from(item_index),
                    name: {
                        let text = self.text(name_node)?;
                        core::convert::identity(text)
                    }
                    .to_owned(),
                    name_range: name_node.byte_range(),
                    block_range: block.byte_range(),
                    payload,
                });
            }
        }
        Ok(())
    }

    /// Lowers one attribute payload to its value fragment (proposal §3.3 — a
    /// payload is data, never an `F`-computation). A pure value is the value
    /// fragment; a computation payload (or a value that hoisted one) is flagged
    /// non-value for the attribute pass to reject. In total mode an
    /// out-of-fragment payload becomes a value hole (a holey payload is
    /// honest); in strict mode it errs.
    fn attribute_payload(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<RawPayload>
    {
        let range = node.byte_range();
        let mut hoists = Vec::new();
        let lowered = match self.expr(node, &mut hoists) {
            | Err(ref error) if bool::from(self.total()) => {
                let hole = self.value_hole(node, error)?;
                return Ok(RawPayload {
                    value: hole.readback_value()?,
                    is_value_fragment: true,
                    range,
                });
            },
            | other => other?,
        };
        match lowered {
            | EOut::Value(value) if hoists.is_empty() => Ok(RawPayload {
                value: value.readback_value()?,
                is_value_fragment: true,
                range,
            }),
            // A computation payload, or a value that hoisted one, is outside
            // the value fragment: carry a placeholder the attribute pass never
            // reads (it rejects on `is_value_fragment`).
            | _ => Ok(RawPayload {
                value: Value::Unit,
                is_value_fragment: false,
                range,
            }),
        }
    }

    /// Lowers one `extern "abi" from "library" { … }` block to a
    /// [`ForeignModule`] (proposal-ffi.md §2). The block's `library` string is
    /// the module namespace; each `type Name;` is an opaque handle type (§4.4)
    /// and each bodyless `def name(params) -> T;` is a foreign function whose
    /// members bind as the lowering contract module members.
    ///
    /// # Contract
    /// - ensures: on success, a [`ForeignModule`] named by the `library`
    ///   string, carrying the declared opaque types and foreign functions with
    ///   their [`CType`] boundary mapping (§4).
    /// - fails: a malformed block node, or (strict mode) an out-of-boundary
    ///   member type — the six atoms, `CStr`, `Unit`, and declared opaque
    ///   handles are the MVP boundary (§4); anything else is
    ///   [`LowerError::Unsupported`].
    /// - panics: none.
    fn extern_block(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<ForeignModule>
    {
        let abi = self.quoted_string_field(node, node_kinds::FIELD_ABI)?;
        let name = self.quoted_string_field(node, node_kinds::FIELD_LIBRARY)?;
        // First collect the opaque handle types, so a later member signature
        // referencing one resolves it to a `Ptr` rather than a rejected atom.
        let mut types: Vec<String> = Vec::new();
        for member in named_non_extra_children(node) {
            if member.kind() == node_kinds::EXTERN_TYPE {
                let name_node = required_field(member, node_kinds::FIELD_NAME)?;
                types.push(
                    {
                        let text = self.text(name_node)?;
                        core::convert::identity(text)
                    }
                    .to_owned(),
                );
            }
        }
        let mut functions: Vec<ForeignFn> = Vec::new();
        for member in named_non_extra_children(node) {
            if member.kind() == node_kinds::EXTERN_FUNCTION {
                functions.push({
                    let extern_function = self.extern_function(member, &types)?;
                    core::convert::identity(extern_function)
                });
            }
        }
        Ok(ForeignModule {
            name,
            abi,
            library: self.quoted_string_field(node, node_kinds::FIELD_LIBRARY)?,
            types,
            functions,
        })
    }

    /// Reads a quoted string field, stripping the delimiting quotes.
    fn quoted_string_field(
        &self,
        node: SynNode<'_>,
        field: SyntaxField,
    ) -> LowerResult<String>
    {
        let string_node = required_field(node, field)?;
        let text = self.text(string_node)?;
        let inner = text
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(text.0);
        Ok(inner.to_owned())
    }

    /// Lowers one `extern` foreign-function signature `def name(params) -> T;`
    /// to a [`ForeignFn`], mapping each parameter and the result through the
    /// MVP boundary (proposal-ffi.md §4). `opaque` is the set of opaque
    /// handle types the enclosing block declares.
    fn extern_function(
        &self,
        node: SynNode<'_>,
        opaque: &[String],
    ) -> LowerResult<ForeignFn>
    {
        let name_node = required_field(node, node_kinds::FIELD_NAME)?;
        let op = {
            let text = self.text(name_node)?;
            core::convert::identity(text)
        }
        .to_owned();
        let params_node = required_field(node, node_kinds::FIELD_PARAMETERS)?;
        let mut params: Vec<ForeignParam> = Vec::new();
        for parameter in named_non_extra_children(params_node) {
            if parameter.kind() != node_kinds::PARAMETER {
                continue;
            }
            let param_name = {
                let param_name_node = required_field(parameter, node_kinds::FIELD_NAME)?;
                let text = self.text(param_name_node)?;
                core::convert::identity(text)
            }
            .to_owned();
            let type_node = parameter
                .child_by_field_name(node_kinds::FIELD_TYPE)
                .ok_or_else(|| {
                    // A boundary parameter must state its C type: an untyped extern
                    // parameter has no boundary mapping.
                    LowerError::Unsupported {
                        kind: parameter.kind(),
                        byte_range: parameter.byte_range(),
                    }
                })?;
            params.push(ForeignParam {
                name: param_name,
                c_type: self.boundary_ctype(type_node, opaque)?,
            });
        }
        let result = match node.child_by_field_name(node_kinds::FIELD_RESULT) {
            | Some(result_node) => self.boundary_ctype(result_node, opaque)?,
            // A result-less signature is a `void` foreign call.
            | None => CType::Void,
        };
        Ok(ForeignFn { op, params, result })
    }

    /// Maps an `extern`-boundary type node to its [`CType`] (proposal-ffi.md
    /// §4): the six numeric atoms by identity (§4.1), `CStr` to a string copy
    /// (§4.2), a declared opaque handle to `Ptr` (§4.4), and `Unit` to a `void`
    /// slot. Everything else is outside the MVP boundary.
    fn boundary_ctype(
        &self,
        node: SynNode<'_>,
        opaque: &[String],
    ) -> LowerResult<CType>
    {
        let unsupported = || LowerError::Unsupported {
            kind: node.kind(),
            byte_range: node.byte_range(),
        };
        let name = match node.kind() {
            | node_kinds::PRIMITIVE_TYPE | node_kinds::TYPE_IDENTIFIER => self.text(node)?,
            // A composite boundary type (functions, products, sums, records,
            // struct-by-value) is a growth item (§4.3), not the MVP boundary.
            | _ => return Err(unsupported()),
        };
        match name.0 {
            | "u32" => Ok(CType::U32),
            | "u64" => Ok(CType::U64),
            | "i32" => Ok(CType::I32),
            | "i64" => Ok(CType::I64),
            | "f32" => Ok(CType::F32),
            | "f64" => Ok(CType::F64),
            | node_kinds::NAME_CSTR_TYPE => Ok(CType::CStr),
            | node_kinds::NAME_UNIT_TYPE => Ok(CType::Void),
            // A declared opaque handle type (`type Db;`) crosses as a pointer.
            | other if opaque.contains(&other.to_owned()) => Ok(CType::Ptr),
            // `Integer`, `String`, sub-word integers, `bool`/`c_char`, and
            // undeclared type identifiers are outside the MVP boundary (§4.1).
            | _ => Err(unsupported()),
        }
    }

    /// Lowers a checked `module M (: #{ … })? { … }` declaration to one named
    /// item whose term is an ordinary record value, or a bind-chain returning
    /// that record when member definitions must be sequenced.
    ///
    /// # Contract
    /// - requires: `node` is a [`node_kinds::MODULE_DECLARATION`].
    /// - ensures: member definitions are evaluated exactly once in source
    ///   order; each non-duplicate definition contributes one binding and one
    ///   candidate record field; earlier binders scope over later definitions
    ///   and the final record; explicit matching filters candidate fields only.
    /// - fails: [`LowerError::DuplicateModuleMember`] for duplicate member
    ///   definitions, [`LowerError::DanglingSignature`] for unmatched member
    ///   signatures in strict mode, plus ordinary member/type lowering errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: bind sequencing plus terminal repacking realizes ordered
    ///   module evaluation without a core module constructor.
    /// - mutants: filter bindings with exports; reverse body iteration.
    /// - witnesses: `module_signature_repacking_hides_extra_members` and
    ///   `nested_modules_lower_as_parent_members_and_project`.
    fn module_declaration(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<(Item, OriginNode)>
    {
        let (name, ascription) = self.module_header(node)?;
        let mut body = ModuleBody::default();
        for member in node.children_by_field_name(node_kinds::FIELD_MEMBER) {
            let Some(plan) = self.module_member_plan(&mut body, member)?
            else {
                continue;
            };
            let recovery_ascription = plan.explicit.clone();
            let lowered = if member.kind() == node_kinds::MODULE_DECLARATION {
                self.nested_module_member_definition(member, plan.name, plan.explicit)
            }
            else {
                self.module_member_definition(member, plan.name, plan.explicit)
            };
            self.push_module_member_result(
                &mut body,
                member,
                recovery_ascription.as_ref(),
                lowered,
            )?;
        }
        let (term, origin) = self.finish_module_body(node, ascription.as_ref(), body)?;
        Ok((
            Item {
                name: Some(name),
                ascription,
                term,
            },
            origin,
        ))
    }

    /// Reads one module declaration's name and optional structural ascription.
    ///
    /// # Contract
    /// - requires: `node` is a [`node_kinds::MODULE_DECLARATION`].
    /// - ensures: the returned ascription is record-shaped whenever present.
    /// - fails: ordinary required-field or type-lowering errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: the declaration name and ascription are independent header
    ///   fields and can be lowered without inspecting the body.
    /// - mutants: read the first body identifier; accept a non-record type.
    /// - witnesses: `recognizes_module_declaration` and
    ///   `nonempty_module_ascription_checks_returned_record`.
    fn module_header(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<(String, Option<Ty>)>
    {
        let name_node = required_field(node, node_kinds::FIELD_NAME)?;
        let name = {
            let text = self.text(name_node)?;
            core::convert::identity(text)
        }
        .to_owned();
        Ok((name, self.module_ascription(node)?))
    }

    /// Lowers one module declaration's optional structural ascription.
    ///
    /// # Contract
    /// - ensures: the returned ascription is record-shaped whenever present.
    /// - fails: ordinary type-lowering and record-shape errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: nested lowering can reuse an already-owned name and lower
    ///   only the remaining header field.
    /// - mutants: reread the member name; skip record-shape validation.
    /// - witnesses: `nested_modules_lower_as_parent_members_and_project`.
    fn module_ascription(
        &self,
        node: SynNode<'_>,
    ) -> LowerResult<Option<Ty>>
    {
        match node.child_by_field_name(node_kinds::FIELD_ASCRIPTION) {
            | Some(ascription_node) => {
                let ty = self.lower_type_node(ascription_node)?;
                let ty = self.module_record_ascription(ty, ascription_node)?;
                Ok(Some(ty))
            },
            | None => Ok(None),
        }
    }

    /// Validates one module-body member and reserves its definition name.
    ///
    /// # Contract
    /// - ensures: signatures are retained for a later definition; a returned
    ///   plan owns a unique name and consumes its matching signature.
    /// - fails: duplicate-member, malformed-node, signature, and ordinary
    ///   recovery-construction errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: reserving names before lowering makes duplicate recovery
    ///   deterministic and gives every definition at most one signature.
    /// - mutants: reserve after lowering; leave a duplicate's signature unused.
    /// - witnesses: `duplicate_module_member_definition_is_rejected` and
    ///   `dangling_member_signature_is_strict_error_and_total_hole`.
    fn module_member_plan(
        &mut self,
        body: &mut ModuleBody,
        member: SynNode<'_>,
    ) -> LowerResult<Option<ModuleMemberPlan>>
    {
        if member.kind() == node_kinds::DEF_SIGNATURE {
            match self.def_signature(member) {
                | Ok(sig) => body.sigs.push(sig),
                | Err(ref error) if bool::from(self.total()) => {
                    let bound = self.comp_hole(member, error)?;
                    body.bindings.push(ModuleBinding {
                        binder: node_kinds::DISCARD_BINDER.to_owned(),
                        bound,
                    });
                },
                | Err(error) => return Err(error),
            }
            return Ok(None);
        }

        let member_name = match self.module_member_name(member) {
            | Ok(found_name) => found_name,
            | Err(ref error) if bool::from(self.total()) => {
                let bound = self.comp_hole(member, error)?;
                body.bindings.push(ModuleBinding {
                    binder: node_kinds::DISCARD_BINDER.to_owned(),
                    bound,
                });
                return Ok(None);
            },
            | Err(error) => return Err(error),
        };
        let Some(member_name) = member_name
        else {
            let error = LowerError::Unsupported {
                kind: member.kind(),
                byte_range: member.byte_range(),
            };
            if bool::from(self.total()) {
                let bound = self.comp_hole(member, &error)?;
                body.bindings.push(ModuleBinding {
                    binder: node_kinds::DISCARD_BINDER.to_owned(),
                    bound,
                });
                return Ok(None);
            }
            return Err(error);
        };

        if body.definitions.contains_key(&member_name) {
            drop(take_sig(&mut body.sigs, member_name.as_str().into()));
            let error = LowerError::DuplicateModuleMember {
                name: member_name,
                byte_range: member.byte_range(),
            };
            if bool::from(self.total()) {
                let bound = self.comp_hole(member, &error)?;
                body.bindings.push(ModuleBinding {
                    binder: node_kinds::DISCARD_BINDER.to_owned(),
                    bound,
                });
                return Ok(None);
            }
            return Err(error);
        }
        body.definitions
            .insert(member_name.clone(), member.byte_range());
        let explicit = take_sig(&mut body.sigs, member_name.as_str().into());
        Ok(Some(ModuleMemberPlan {
            name: member_name,
            explicit,
        }))
    }

    /// Records one lowered module member or its total-mode recovery.
    ///
    /// # Contract
    /// - ensures: success appends one binding and field; total-mode failure
    ///   appends the richest recoverable replacement in the same source slot.
    /// - fails: strict mode returns `lowered`'s error; total mode returns only
    ///   recovery-construction errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: one append point preserves the failed member's source slot
    ///   in both strict and total modes.
    /// - mutants: append recovered fields without bindings; swallow strict
    ///   errors.
    /// - witnesses: `malformed_module_member_recovers_in_total_mode`.
    fn push_module_member_result(
        &mut self,
        body: &mut ModuleBody,
        member: SynNode<'_>,
        explicit: Option<&Ty>,
        lowered: LowerResult<(ModuleBinding, ModuleField)>,
    ) -> LowerResult<()>
    {
        match lowered {
            | Ok((binding, field)) => {
                body.bindings.push(binding);
                body.fields.push(field);
            },
            | Err(ref error) if bool::from(self.total()) => {
                let recovered = self.recover_module_member(member, error, explicit)?;
                if let Some((binding, field)) = recovered {
                    body.bindings.push(binding);
                    body.fields.push(field);
                }
                else {
                    let bound = self.comp_hole(member, error)?;
                    body.bindings.push(ModuleBinding {
                        binder: node_kinds::DISCARD_BINDER.to_owned(),
                        bound,
                    });
                }
            },
            | Err(error) => return Err(error),
        }
        Ok(())
    }

    /// Finishes one accumulated module body as its core term.
    ///
    /// # Contract
    /// - ensures: total mode materializes every dangling signature as a
    ///   hole-producing binding plus candidate field, so export filtering
    ///   cannot erase its goal; strict mode rejects the first dangling
    ///   signature.
    /// - fails: [`LowerError::DanglingSignature`] in strict mode, or ordinary
    ///   final record construction errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: a recovery binding keeps missing-definition evidence
    ///   reachable even when structural matching hides its candidate field.
    /// - mutants: create only a field hole; drop unused signatures.
    /// - witnesses: `dangling_member_signature_is_strict_error_and_total_hole`.
    fn finish_module_body(
        &mut self,
        node: SynNode<'_>,
        ascription: Option<&Ty>,
        body: ModuleBody,
    ) -> LowerResult<(Term, OriginNode)>
    {
        let ModuleBody {
            sigs,
            mut bindings,
            mut fields,
            ..
        } = body;
        if bool::from(self.total()) {
            for sig in sigs.into_iter().filter(|sig| !sig.used) {
                let (binding, field) = self.dangling_module_member(sig)?;
                bindings.push(binding);
                fields.push(field);
            }
        }
        else if let Some(dangling) = sigs.iter().find(|sig| !sig.used) {
            return Err(LowerError::DanglingSignature {
                name: dangling.name.clone(),
                byte_range: dangling.byte_range.clone(),
            });
        }

        let record_ascription = Self::value_ascription(ascription);
        Self::module_term(node, bindings, fields, record_ascription)
    }

    /// Lowers the definition-only body of a one-level nested module.
    ///
    /// # Contract
    /// - requires: `node` is the nested [`node_kinds::MODULE_DECLARATION`].
    /// - ensures: the returned term obeys the same ordering and
    ///   record-repacking contract as a top-level module without constructing
    ///   an unused item name; `explicit` takes precedence over the inline
    ///   ascription.
    /// - fails: ordinary module ascription, member, and finalization errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: a definition-only leaf helper enforces the grammar's
    ///   one-level nesting bound without native recursion.
    /// - mutants: call `module_declaration`; scan nested module members.
    /// - witnesses: `nested_modules_lower_as_parent_members_and_project` and
    ///   `contracts::deeper_nested_module_uses_ordinary_recovery`.
    fn leaf_module_declaration(
        &mut self,
        node: SynNode<'_>,
        explicit: Option<Ty>,
    ) -> LowerResult<(Option<Ty>, Term, OriginNode)>
    {
        let inline = self.module_ascription(node)?;
        let effective = match explicit {
            | Some(ty) => Some(self.module_record_ascription(ty, node)?),
            | None => inline,
        };
        let mut body = ModuleBody::default();
        for member in node.children_by_field_name(node_kinds::FIELD_MEMBER) {
            let Some(plan) = self.module_member_plan(&mut body, member)?
            else {
                continue;
            };
            let recovery_ascription = plan.explicit.clone();
            let lowered = self.module_member_definition(member, plan.name, plan.explicit);
            self.push_module_member_result(
                &mut body,
                member,
                recovery_ascription.as_ref(),
                lowered,
            )?;
        }
        let (term, origin) = self.finish_module_body(node, effective.as_ref(), body)?;
        Ok((effective, term, origin))
    }

    /// Validates the optional module ascription's record shape without
    /// duplicating the checker.
    ///
    /// # Contract
    /// - ensures: strict mode accepts only record value types; total mode also
    ///   permits `Unknown`, the existing type-position recovery sentinel.
    /// - fails: [`LowerError::TypeSortMismatch`] when the ascription does not
    ///   lower to a record-shaped value type.
    /// - panics: none.
    fn module_record_ascription(
        &self,
        ty: Ty,
        node: SynNode<'_>,
    ) -> LowerResult<Ty>
    {
        match ty {
            | Ty::Value(ValueType::Record(_)) => Ok(ty),
            | Ty::Value(ValueType::Unknown) if bool::from(self.total()) => Ok(ty),
            | _ => Err(LowerError::TypeSortMismatch {
                expected: "a record value type",
                kind: node.kind(),
                byte_range: node.byte_range(),
            }),
        }
    }

    /// The definition name of a module member, if the member is a definition.
    ///
    /// # Contract
    /// - ensures: returns `None` for non-definition module members.
    /// - fails: [`LowerError::MalformedNode`] when a definition member lacks
    ///   its required name field.
    /// - panics: none.
    fn module_member_name(
        &self,
        member: SynNode<'_>,
    ) -> LowerResult<Option<String>>
    {
        match member.kind() {
            | node_kinds::DEF_VALUE | node_kinds::DEF_FUNCTION | node_kinds::MODULE_DECLARATION => {
                let name_node = required_field(member, node_kinds::FIELD_NAME)?;
                Ok(Some(
                    {
                        let text = self.text(name_node)?;
                        core::convert::identity(text)
                    }
                    .to_owned(),
                ))
            },
            | _ => Ok(None),
        }
    }

    /// Lowers one value or function module definition to its source-ordered
    /// binding and final record field.
    ///
    /// # Contract
    /// - requires: `name` is the already-read member name.
    /// - ensures: the returned binding evaluates the member once; the returned
    ///   field projects that binder into the module record.
    /// - fails: ordinary expression/function lowering errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: a generated binder separates one-time evaluation from the
    ///   candidate field used by terminal repacking.
    /// - mutants: inline the expression into the field; ignore `explicit`.
    /// - witnesses: `module_members_bind_in_source_order_and_return_record` and
    ///   `module_signature_repacking_hides_extra_members`.
    fn module_member_definition(
        &mut self,
        member: SynNode<'_>,
        name: String,
        explicit: Option<Ty>,
    ) -> LowerResult<(ModuleBinding, ModuleField)>
    {
        let (bound, effective) = match member.kind() {
            | node_kinds::DEF_VALUE => {
                let value_node = required_field(member, node_kinds::FIELD_VALUE)?;
                (
                    {
                        let bound = self.module_value_binding(value_node, explicit.as_ref())?;
                        core::convert::identity(bound)
                    },
                    explicit,
                )
            },
            | node_kinds::DEF_FUNCTION => {
                let (item, origin) = self.def_function(member)?;
                let derived = item.ascription;
                let effective = explicit.or(derived);
                let Term::Value(value) = item.term
                else {
                    return Err(LowerError::Unsupported {
                        kind: member.kind(),
                        byte_range: member.byte_range(),
                    });
                };
                (
                    {
                        let bound =
                            Self::module_ret_binding(member, value, origin, effective.as_ref())?;
                        core::convert::identity(bound)
                    },
                    effective,
                )
            },
            | kind => {
                return Err(LowerError::Unsupported {
                    kind,
                    byte_range: member.byte_range(),
                });
            },
        };
        let field = Self::module_field(name.as_str().into(), member, effective.as_ref());
        Ok((
            ModuleBinding {
                binder: name,
                bound,
            },
            field,
        ))
    }

    /// Lowers one nested module as a single parent binding and candidate field.
    ///
    /// # Contract
    /// - requires: `member` is a one-level nested module declaration and `name`
    ///   is its already-read member name.
    /// - ensures: the nested body is lowered without native recursion; a
    ///   preceding member signature takes the same precedence over the inline
    ///   declaration signature as for ordinary definition sugar.
    /// - fails: ordinary nested module lowering errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: passing the consumed signature into leaf finalization
    ///   makes it constrain both the nested binding and parent field.
    /// - mutants: discard `explicit`; allocate the declaration name again.
    /// - witnesses: `nested_member_signature_constrains_the_parent_binding`.
    fn nested_module_member_definition(
        &mut self,
        member: SynNode<'_>,
        name: String,
        explicit: Option<Ty>,
    ) -> LowerResult<(ModuleBinding, ModuleField)>
    {
        let (effective, term, origin) = self.leaf_module_declaration(member, explicit)?;
        let bound = match term {
            | Term::Value(value) => {
                let ret = Comp::Ret(Rc::new(value));
                let origin =
                    OriginNode::new(entry(member, Some(ElabKind::LetValueBind)), vec![origin]);
                COut::from_legacy_comp(&ret, origin)?
            },
            | Term::Comp(comp) => COut::from_legacy_comp(&comp, origin)?,
        };
        let field = Self::module_field(name.as_str().into(), member, effective.as_ref());
        Ok((
            ModuleBinding {
                binder: name,
                bound,
            },
            field,
        ))
    }

    /// Builds the computation that obtains a `def name = expr;` module member.
    ///
    /// # Contract
    /// - ensures: value-sorted expressions lower to `Ret`, computation-sorted
    ///   expressions bind directly, and pending hoists wrap the member exactly
    ///   once in source order.
    /// - fails: ordinary expression lowering errors.
    /// - panics: none.
    fn module_value_binding(
        &mut self,
        value_node: SynNode<'_>,
        ascription: Option<&Ty>,
    ) -> LowerResult<COut>
    {
        let mut hoists = Vec::new();
        let lowered = self.expr(value_node, &mut hoists)?;
        match lowered {
            | EOut::Value(value) => {
                let (value, origin) = Self::annotate_value_output(
                    {
                        let readback_value = value.readback_value()?;
                        core::convert::identity(readback_value)
                    },
                    value.origin,
                    Self::value_ascription(ascription),
                    value_node,
                );
                let ret = COut::from_legacy_comp(
                    &Comp::Ret(Rc::new(value)),
                    OriginNode::new(entry(value_node, Some(ElabKind::LetValueBind)), vec![
                        origin,
                    ]),
                )?;
                Self::wrap_hoists(hoists, ret, value_node)
            },
            | EOut::Comp(comp) => {
                let wrapped = Self::wrap_hoists(hoists, comp, value_node)?;
                match Self::comp_ascription(ascription) {
                    | Some(comp_ty) => Self::comp_ascribed_binding(value_node, wrapped, comp_ty),
                    | None => Ok(wrapped),
                }
            },
        }
    }

    /// Builds the `Ret` computation for an already-lowered value member.
    ///
    /// # Contract
    /// - ensures: value-sorted ascriptions annotate the value before it is
    ///   bound, so derived or explicit function signatures participate in
    ///   ordinary inference.
    /// - fails: arena readback/allocation failures only.
    /// - panics: none.
    fn module_ret_binding(
        member: SynNode<'_>,
        value: Value,
        origin: OriginNode,
        ascription: Option<&Ty>,
    ) -> LowerResult<COut>
    {
        let (value, origin) =
            Self::annotate_value_output(value, origin, Self::value_ascription(ascription), member);
        COut::from_legacy_comp(
            &Comp::Ret(Rc::new(value)),
            OriginNode::new(entry(member, Some(ElabKind::LetValueBind)), vec![origin]),
        )
    }

    /// Builds the shadow tree for the computation-ascription encoding.
    ///
    /// The core shape is `Force(Annot(Thunk(body), U_ω B))`, so the origin
    /// tree must mirror the same child chain rather than attaching `body`
    /// directly under the `Force` node.
    ///
    /// # Contract
    /// - ensures: returns `Force -> Annot -> Thunk -> body` provenance, with
    ///   every synthesized layer tagged [`ElabKind::CompAscription`].
    /// - panics: none.
    fn comp_ascription_origin(
        node: SynNode<'_>,
        body: OriginNode,
    ) -> OriginNode
    {
        let elab = Some(ElabKind::CompAscription);
        let thunk = OriginNode::new(entry(node, elab), vec![body]);
        let annot = OriginNode::new(entry(node, elab), vec![thunk]);
        OriginNode::new(entry(node, elab), vec![annot])
    }

    /// Encodes a computation member's explicit computation signature using the
    /// existing computation-ascription sugar.
    ///
    /// # Contract
    /// - ensures: the returned computation synthesizes the supplied computation
    ///   type via `force ((thunk body) : U_ω ty)`.
    /// - fails: arena allocation/readback failures only.
    /// - panics: none.
    fn comp_ascribed_binding(
        node: SynNode<'_>,
        body: COut,
        ascription: CompType,
    ) -> LowerResult<COut>
    {
        let annotated = Value::annot(
            Value::thunk(Grade::OMEGA, {
                let readback_comp = body.readback_comp()?;
                core::convert::identity(readback_comp)
            }),
            ValueType::thunk(Grade::OMEGA, ascription),
        );
        COut::from_legacy_comp(
            &Comp::Force(Rc::new(annotated)),
            Self::comp_ascription_origin(node, body.origin),
        )
    }

    /// Recovers a malformed module definition as a value-hole binding when its
    /// name is still available.
    ///
    /// # Contract
    /// - ensures: named damaged definitions still bind their name and
    ///   contribute one field in total mode; nameless damaged members return
    ///   `None` for the caller's discarded-hole path.
    /// - fails: arena allocation/readback failures only.
    /// - panics: none.
    fn recover_module_member(
        &mut self,
        member: SynNode<'_>,
        error: &LowerError,
        ascription: Option<&Ty>,
    ) -> LowerResult<Option<(ModuleBinding, ModuleField)>>
    {
        let Some(name) = member
            .child_by_field_name(node_kinds::FIELD_NAME)
            .and_then(|name_node| self.text(name_node).ok())
            .map(NodeText::to_owned)
        else {
            return Ok(None);
        };
        let hole = self.value_hole(member, error)?;
        let (value, origin) = Self::annotate_value_output(
            {
                let readback_value = hole.readback_value()?;
                core::convert::identity(readback_value)
            },
            hole.origin,
            Self::value_ascription(ascription),
            member,
        );
        let bound = COut::from_legacy_comp(
            &Comp::Ret(Rc::new(value)),
            OriginNode::new(entry(member, Some(ElabKind::LetValueBind)), vec![origin]),
        )?;
        let field = Self::module_field(name.as_str().into(), member, ascription);
        Ok(Some((
            ModuleBinding {
                binder: name,
                bound,
            },
            field,
        )))
    }

    /// Builds total-mode recovery for an unmatched member signature.
    ///
    /// # Contract
    /// - ensures: the binding evaluates a fresh hole carrying
    ///   [`HoleNote::MissingDefinition`] at the signature's source range; the
    ///   candidate field selects that binder and may be hidden by repacking
    ///   without erasing the hole.
    /// - fails: arena allocation/readback failures only.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: putting recovery evidence in the source-ordered binding,
    ///   not only the candidate field, makes export filtering lossless.
    /// - mutants: return only a field hole; annotate only the field projection.
    /// - witnesses: `dangling_member_signature_is_strict_error_and_total_hole`.
    fn dangling_module_member(
        &mut self,
        sig: PendingSig,
    ) -> LowerResult<(ModuleBinding, ModuleField)>
    {
        let error = LowerError::DanglingSignature {
            name: sig.name.clone(),
            byte_range: sig.byte_range.clone(),
        };
        let hole = Value::Hole(self.fresh_hole().into());
        let hole_origin = OriginNode::leaf(OriginEntry {
            cst_node: sig.node,
            cst_hash: sig.hash,
            byte_range: sig.byte_range.clone(),
            elaboration: None,
            note: Some(note_of(&error)),
        });
        let (value, origin) = match Self::value_ascription(Some(&sig.ty)) {
            | Some(value_ty) => (
                Value::annot(hole, value_ty),
                OriginNode::new(
                    OriginEntry {
                        cst_node: sig.node,
                        cst_hash: sig.hash,
                        byte_range: sig.byte_range.clone(),
                        elaboration: None,
                        note: None,
                    },
                    vec![hole_origin],
                ),
            ),
            | None => (hole, hole_origin),
        };
        let bound = COut::from_legacy_comp(
            &Comp::Ret(Rc::new(value)),
            OriginNode::new(
                OriginEntry {
                    cst_node: sig.node,
                    cst_hash: sig.hash,
                    byte_range: sig.byte_range.clone(),
                    elaboration: Some(ElabKind::LetValueBind),
                    note: None,
                },
                vec![origin],
            ),
        )?;
        let field = ModuleField {
            label: sig.name.clone(),
            value: Value::var(&sig.name),
            origin: OriginNode::leaf(OriginEntry {
                cst_node: sig.node,
                cst_hash: sig.hash,
                byte_range: sig.byte_range,
                elaboration: Some(ElabKind::LetValueBind),
                note: None,
            }),
        };
        Ok((
            ModuleBinding {
                binder: sig.name,
                bound,
            },
            field,
        ))
    }

    /// Constructs the generated final field selecting a member binder.
    ///
    /// # Contract
    /// - ensures: value-sorted member ascriptions are preserved as annotations
    ///   on the field value.
    /// - panics: none.
    fn module_field(
        name: DefinitionName<'_>,
        node: SynNode<'_>,
        ascription: Option<&Ty>,
    ) -> ModuleField
    {
        let value = Value::var(name.as_ref());
        let origin = OriginNode::leaf(entry(node, Some(ElabKind::LetValueBind)));
        let (value, origin) =
            Self::annotate_value_output(value, origin, Self::value_ascription(ascription), node);
        ModuleField {
            label: name.as_ref().to_owned(),
            value,
            origin,
        }
    }

    /// Wraps a value and origin in an annotation when a value type is present.
    ///
    /// # Contract
    /// - ensures: the returned origin mirrors the returned value's shape.
    /// - panics: none.
    fn annotate_value_output(
        value: Value,
        origin: OriginNode,
        ascription: Option<ValueType>,
        node: SynNode<'_>,
    ) -> (Value, OriginNode)
    {
        match ascription {
            | Some(value_ty) => (
                Value::annot(value, value_ty),
                OriginNode::new(entry(node, None), vec![origin]),
            ),
            | None => (value, origin),
        }
    }

    /// Extracts a value ascription from a lowered type.
    ///
    /// # Contract
    /// - ensures: returns `Some` only for value-sorted types.
    /// - panics: none.
    fn value_ascription(ascription: Option<&Ty>) -> Option<ValueType>
    {
        match ascription {
            | Some(&Ty::Value(ref value_ty)) => Some(value_ty.clone()),
            | _ => None,
        }
    }

    /// Extracts a computation ascription from a lowered type.
    ///
    /// # Contract
    /// - ensures: returns `Some` only for computation-sorted types.
    /// - panics: none.
    fn comp_ascription(ascription: Option<&Ty>) -> Option<CompType>
    {
        match ascription {
            | Some(&Ty::Comp(ref comp_ty)) => Some(comp_ty.clone()),
            | _ => None,
        }
    }

    /// Builds the module item's final record term, adding generated binds only
    /// when there are member definitions to sequence.
    ///
    /// An inline record ascription is an explicit structural matching
    /// coercion: every body member still evaluates in source order, while the
    /// returned record is rebuilt from only the fields named by the signature.
    ///
    /// # Contract
    /// - ensures: bindings are nested in source order and the final record is
    ///   canonical in label order; an ascribed record exposes exactly the
    ///   signature's fields.
    /// - fails: arena allocation/readback failures only.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: filtering candidate fields before the final record and
    ///   folding all bindings afterward separates visibility from evaluation.
    /// - mutants: filter bindings with fields; fold bindings in source order.
    /// - witnesses: `module_signature_repacking_hides_extra_members`,
    ///   `dangling_member_signature_is_strict_error_and_total_hole`, and the
    ///   `29-modules.gandr` model witness.
    fn module_term(
        node: SynNode<'_>,
        bindings: Vec<ModuleBinding>,
        fields: Vec<ModuleField>,
        ascription: Option<ValueType>,
    ) -> LowerResult<(Term, OriginNode)>
    {
        let exported = match ascription.as_ref() {
            | Some(&ValueType::Record(ref fields)) => Some(fields),
            | Some(_) | None => None,
        };
        let mut fields_by_label: BTreeMap<String, (Value, OriginNode)> = BTreeMap::new();
        for field in fields {
            if exported.is_none_or(|signature| signature.contains_key(&field.label)) {
                fields_by_label.insert(field.label, (field.value, field.origin));
            }
        }
        let mut values = Vec::with_capacity(fields_by_label.len());
        let mut origins = Vec::with_capacity(fields_by_label.len());
        for (label, (value, origin)) in fields_by_label {
            values.push((label, value));
            origins.push(origin);
        }
        let module_elab = Some(ElabKind::ModuleDeclaration);
        let record = Value::record(values);
        let record_origin = OriginNode::new(entry(node, module_elab), origins);
        let (record, record_origin) =
            Self::annotate_value_output(record, record_origin, ascription, node);
        if bindings.is_empty() {
            return Ok((Term::Value(record), record_origin));
        }
        let mut acc = COut::from_legacy_comp(
            &Comp::Ret(Rc::new(record)),
            OriginNode::new(entry(node, module_elab), vec![record_origin]),
        )?;
        for binding in bindings.into_iter().rev() {
            acc = COut::from_legacy_comp(
                &Comp::Bind(
                    Rc::new({
                        let readback_comp = binding.bound.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                    binding.binder,
                    Rc::new({
                        let readback_comp = acc.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                ),
                OriginNode::new(entry(node, module_elab), vec![
                    binding.bound.origin,
                    acc.origin,
                ]),
            )?;
        }
        Ok((
            Term::Comp({
                let readback_comp = acc.readback_comp()?;
                core::convert::identity(readback_comp)
            }),
            acc.origin,
        ))
    }

    /// Lowers one `def_signature` item to a [`PendingSig`].
    fn def_signature(
        &self,
        child: SynNode<'_>,
    ) -> LowerResult<PendingSig>
    {
        let name_node = required_field(child, node_kinds::FIELD_NAME)?;
        let ty_node = required_field(child, node_kinds::FIELD_TYPE)?;
        Ok(PendingSig {
            name: {
                let text = self.text(name_node)?;
                core::convert::identity(text)
            }
            .to_owned(),
            // Route through `lower_type_node` so a declared-datatype signature
            // `def x : Maybe(Integer);` sees the nominal handle (the lowering contract).
            ty: self.lower_type_node(ty_node)?,
            byte_range: child.byte_range(),
            node: child.cst_node(),
            hash: child.cst_hash(),
            used: false,
        })
    }

    /// Lowers one non-signature top-level item (without signature
    /// attachment, which [`Self::source_file`] performs).
    fn item(
        &mut self,
        child: SynNode<'_>,
    ) -> LowerResult<(Item, OriginNode)>
    {
        recursion_surface::validate_item(child)?;
        match child.kind() {
            | node_kinds::DEF_VALUE => {
                let name_node = required_field(child, node_kinds::FIELD_NAME)?;
                let name = {
                    let text = self.text(name_node)?;
                    core::convert::identity(text)
                }
                .to_owned();
                let value_node = required_field(child, node_kinds::FIELD_VALUE)?;
                let (term, origin) = self.finalize_term(value_node)?;
                Ok((
                    Item {
                        name: Some(name),
                        ascription: None,
                        term,
                    },
                    origin,
                ))
            },
            | node_kinds::MODULE_DECLARATION => self.module_declaration(child),
            | node_kinds::DEF_FUNCTION => self.def_function(child),
            // A `def rec` with a copattern body is a codata introduction:
            // elaborate its clauses to the `Cosplit` record-of-thunks. A
            // `def rec` with a *statement* body is user recursion — out of the
            // codata MVP; it declines through the fall-through below (a hole
            // term in total mode), the same disposition it had before
            // `def rec` was classified.
            | node_kinds::DEF_REC if child.def_rec_has_copattern_body().0 => {
                self.lower_copattern_def(child)
            },
            | node_kinds::EXPRESSION_STATEMENT => {
                let inner = sole_inner_expression(child)?;
                let (term, origin) = self.finalize_term(inner)?;
                Ok((
                    Item {
                        name: None,
                        ascription: None,
                        term,
                    },
                    origin,
                ))
            },
            // The trailing expression (script result) is an unnamed item;
            // ERROR/MISSING items land here too ([`Self::finalize_term`]
            // surfaces them as `Syntax`, which total mode holes).
            | _ => {
                let (term, origin) = self.finalize_term(child)?;
                Ok((
                    Item {
                        name: None,
                        ascription: None,
                        term,
                    },
                    origin,
                ))
            },
        }
    }

    /// Lowers an item-level expression to a [`Term`]: a value with pending
    /// hoists is wrapped into a computation (`Bind … Ret`), since there is
    /// no enclosing computation context at item level.
    fn finalize_term(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<(Term, OriginNode)>
    {
        let mut hoists = Vec::new();
        match self.expr(node, &mut hoists)? {
            | EOut::Comp(comp) => {
                let wrapped = Self::wrap_hoists(hoists, comp, node)?;
                Ok((
                    Term::Comp({
                        let readback_comp = wrapped.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                    wrapped.origin,
                ))
            },
            | EOut::Value(value) => {
                if hoists.is_empty() {
                    return Ok((
                        Term::Value({
                            let readback_value = value.readback_value()?;
                            core::convert::identity(readback_value)
                        }),
                        value.origin,
                    ));
                }
                let ret = COut::from_legacy_comp(
                    &Comp::Ret(Rc::new({
                        let readback_value = value.readback_value()?;
                        core::convert::identity(readback_value)
                    })),
                    OriginNode::new(entry(node, Some(ElabKind::RetCoercion)), vec![value.origin]),
                )?;
                let wrapped = Self::wrap_hoists(hoists, ret, node)?;
                Ok((
                    Term::Comp({
                        let readback_comp = wrapped.readback_comp()?;
                        core::convert::identity(readback_comp)
                    }),
                    wrapped.origin,
                ))
            },
        }
    }

    /// Lowers `def f(x: A, …) -> B { t }` via the recorded sugar: term
    /// `thunk { fn(x: A) { … t } }` with [`ElabKind::DefFunctionSugar`], and
    /// derived ascription `U_ω (A → … → B)` when every parameter is
    /// annotated and the result type is present.
    fn def_function(
        &mut self,
        node: SynNode<'_>,
    ) -> LowerResult<(Item, OriginNode)>
    {
        let name_node = required_field(node, node_kinds::FIELD_NAME)?;
        let name = {
            let text = self.text(name_node)?;
            core::convert::identity(text)
        }
        .to_owned();
        let params_node = required_field(node, node_kinds::FIELD_PARAMETERS)?;
        let params = self.parameters(params_node)?;
        let result_ty = match node.child_by_field_name(node_kinds::FIELD_RESULT) {
            // Declared-data-aware so a result type mentioning a declared
            // datatype (`-> F Maybe(Integer)`) sees the nominal handle at any
            // depth (the lowering contract).
            | Some(result_node) => Some({
                let lower_comp_type_node = self.lower_comp_type_node(result_node)?;
                core::convert::identity(lower_comp_type_node)
            }),
            | None => None,
        };
        let body_node = required_field(node, node_kinds::FIELD_BODY)?;
        let body = self.block(body_node)?;
        let sugar_entry = entry(node, Some(ElabKind::DefFunctionSugar));

        let derived = Self::derived_ascription(&params, result_ty);

        let nested = if params.is_empty() {
            // [SPECULATIVE DECISION] `def f() -> B { t }` ⇒ `thunk { t }`:
            // the sugar's degenerate case binds nothing.
            body
        }
        else {
            let abs_params = if derived.is_some() {
                params
                    .into_iter()
                    .map(|(param_name, _annot)| (param_name, None))
                    .collect()
            }
            else {
                params
            };
            Self::curry_abs(
                abs_params,
                body,
                &sugar_entry,
                Some(ElabKind::DefFunctionSugar),
            )?
        };
        let thunk = VOut::from_legacy_value(
            &Value::thunk(Grade::OMEGA, {
                let readback_comp = nested.readback_comp()?;
                core::convert::identity(readback_comp)
            }),
            OriginNode::new(sugar_entry, vec![nested.origin]),
        )?;
        Ok((
            Item {
                name: Some(name),
                ascription: derived,
                term: Term::Value({
                    let readback_value = thunk.readback_value()?;
                    core::convert::identity(readback_value)
                }),
            },
            thunk.origin,
        ))
    }

    /// The sugar-derived ascription `U_ω (A₁ → … → Aₙ → B)`, when derivable.
    fn derived_ascription(
        params: &[(String, Option<ValueType>)],
        result_ty: Option<CompType>,
    ) -> Option<Ty>
    {
        let mut comp = result_ty?;
        for param in params.iter().rev() {
            let arg = param.1.clone()?;
            comp = CompType::arrow(arg, comp);
        }
        Some(Ty::Value(ValueType::thunk(Grade::OMEGA, comp)))
    }
}

// --- Total-mode recovery helpers
// -------------------------------------------------------------

/// The byte range an error points at, when it carries one (every
/// input-shaped constructor does; the two infrastructure failures do not).
fn error_byte_range(error: &LowerError) -> Option<SourceRange>
{
    match *error {
        | LowerError::Syntax { ref byte_range }
        | LowerError::Unsupported { ref byte_range, .. }
        | LowerError::UnmarkedRecursiveReference { ref byte_range, .. }
        | LowerError::MarkedReferenceOutsideRecursiveScope { ref byte_range, .. }
        | LowerError::ReservedNamedMeasure { ref byte_range, .. }
        | LowerError::ReservedExplicitInstantiation { ref byte_range, .. }
        | LowerError::ReservedExplicitSize { ref byte_range, .. }
        | LowerError::ReservedCostBound { ref byte_range, .. }
        | LowerError::ReservedTailAssertion { ref byte_range }
        | LowerError::InvalidIntegerLiteral { ref byte_range, .. }
        | LowerError::InvalidGrade { ref byte_range, .. }
        | LowerError::MissingCaseArm { ref byte_range, .. }
        | LowerError::EmptyBlock { ref byte_range }
        | LowerError::DanglingSignature { ref byte_range, .. }
        | LowerError::DuplicateModuleMember { ref byte_range, .. }
        | LowerError::TypeSortMismatch { ref byte_range, .. }
        | LowerError::UnknownObservation { ref byte_range, .. }
        | LowerError::DuplicateObservation { ref byte_range, .. }
        | LowerError::MissingObservation { ref byte_range, .. }
        | LowerError::Namespace { ref byte_range, .. }
        | LowerError::ImportIndexOverflow { ref byte_range }
        | LowerError::MalformedNode { ref byte_range, .. } => Some(byte_range.clone()),
        | LowerError::ParserUnavailable { .. }
        | LowerError::ParseFailed
        | LowerError::ArenaBridge { .. } => None,
    }
}

/// Maps an input-shaped [`LowerError`] to the [`HoleNote`] its recovery
/// hole carries (the module doc's conversion table).
///
/// Total over the whole enum for totality's sake; the infrastructure
/// failures and `TypeSortMismatch` are unreachable here in total mode (the
/// former abort before walking, the latter is absorbed by total type
/// lowering) and map to conservative notes.
fn note_of(error: &LowerError) -> HoleNote
{
    match *error {
        | LowerError::Syntax { .. } => HoleNote::SyntaxError,
        | LowerError::Unsupported { kind, .. } | LowerError::TypeSortMismatch { kind, .. } => {
            HoleNote::UnsupportedForm { kind }
        },
        | LowerError::UnmarkedRecursiveReference { .. } => HoleNote::UnsupportedForm {
            kind: node_kinds::IDENTIFIER,
        },
        | LowerError::MarkedReferenceOutsideRecursiveScope { .. }
        | LowerError::ReservedNamedMeasure { .. }
        | LowerError::ReservedExplicitInstantiation { .. }
        | LowerError::ReservedExplicitSize { .. }
        | LowerError::ReservedCostBound { .. }
        | LowerError::ReservedTailAssertion { .. } => HoleNote::UnsupportedForm {
            kind: node_kinds::INSTANTIATION_EXPRESSION,
        },
        | LowerError::InvalidIntegerLiteral { ref text, .. } => {
            HoleNote::InvalidIntegerLiteral { text: text.clone() }
        },
        | LowerError::InvalidGrade { ref text, .. } => {
            HoleNote::InvalidGrade { text: text.clone() }
        },
        | LowerError::MissingCaseArm { constructor, .. } => {
            HoleNote::MissingCaseArm { constructor }
        },
        | LowerError::EmptyBlock { .. } => HoleNote::EmptyBlock,
        | LowerError::DanglingSignature { ref name, .. } => {
            HoleNote::MissingDefinition { name: name.clone() }
        },
        | LowerError::DuplicateModuleMember { .. } => HoleNote::UnsupportedForm {
            kind: node_kinds::MODULE_DECLARATION,
        },
        | LowerError::MalformedNode { kind, .. } => HoleNote::MalformedNode { kind },
        // Coverage gaps are handled in-band by the copattern elaborator (strict
        // propagates; total degrades — drop / last-wins / carrier-ascription
        // mismatch); this fallback keeps `note_of` total.
        | LowerError::UnknownObservation { .. }
        | LowerError::DuplicateObservation { .. }
        | LowerError::MissingObservation { .. } => HoleNote::UnsupportedForm {
            kind: node_kinds::COPATTERN_CLAUSE,
        },
        | LowerError::Namespace { .. } | LowerError::ImportIndexOverflow { .. } => {
            HoleNote::UnsupportedForm {
                kind: node_kinds::IMPORT_DECLARATION,
            }
        },
        | LowerError::ParserUnavailable { .. }
        | LowerError::ParseFailed
        | LowerError::ArenaBridge { .. } => HoleNote::MalformedNode {
            kind: node_kinds::SOURCE_FILE,
        },
    }
}

// --- Tree helpers
// -------------------------------------------------------------

/// Builds an [`OriginEntry`] for a CST node.
fn entry(
    node: SynNode<'_>,
    elaboration: Option<ElabKind>,
) -> OriginEntry
{
    OriginEntry {
        cst_node: node.cst_node(),
        cst_hash: node.cst_hash(),
        byte_range: node.byte_range(),
        elaboration,
        note: None,
    }
}

/// Builds the [`OriginEntry`] for a user-written hole at `node`, carrying its
/// [`HoleNote::UserHole`] `note`. Unlike [`Lowerer::hole_entry`] there is no
/// elided sub-range to take from an error: a user hole's range *is* the `?`
/// node's range.
fn user_hole_entry(
    node: SynNode<'_>,
    hole_note: HoleNote,
) -> OriginEntry
{
    OriginEntry {
        cst_node: node.cst_node(),
        cst_hash: node.cst_hash(),
        byte_range: node.byte_range(),
        elaboration: None,
        note: Some(hole_note),
    }
}

/// Copies an [`OriginEntry`] with a different elaboration tag.
fn with_elab(
    entry: &OriginEntry,
    elaboration: Option<ElabKind>,
) -> OriginEntry
{
    OriginEntry {
        cst_node: entry.cst_node,
        cst_hash: entry.cst_hash,
        byte_range: entry.byte_range.clone(),
        elaboration,
        note: None,
    }
}

/// The text of a CST node, total over any `(source, node)` pair.
pub(crate) fn node_text<'src>(
    source: PipelineSource<'src>,
    node: SynNode<'_>,
) -> LowerResult<NodeText<'src>>
{
    source
        .0
        .get(node.start_byte().0 .. node.end_byte().0)
        .map(NodeText::from)
        .ok_or_else(|| LowerError::MalformedNode {
            kind: node.kind(),
            byte_range: node.byte_range(),
        })
}

/// A grammar-required field, as a structured error rather than a panic.
pub(crate) fn required_field(
    node: SynNode<'_>,
    field: SyntaxField,
) -> LowerResult<SynNode<'_>>
{
    node.child_by_field_name(field)
        .ok_or_else(|| LowerError::MalformedNode {
            kind: node.kind(),
            byte_range: node.byte_range(),
        })
}

/// The single non-extra named child of a wrapper node (parenthesized
/// expressions, expression statements).
fn sole_inner_expression(node: SynNode<'_>) -> LowerResult<SynNode<'_>>
{
    named_non_extra_children(node)
        .into_iter()
        .next()
        .ok_or_else(|| LowerError::MalformedNode {
            kind: node.kind(),
            byte_range: node.byte_range(),
        })
}

/// The named children of a node, minus extras (comments, shebangs). The
/// `SynNode` adapter already grout-unwraps and space-skips, and classifies
/// comments/shebangs to no dispatched kind, so the `EXTRAS` filter is a
/// defensive no-op the melder view never triggers.
fn named_non_extra_children(node: SynNode<'_>) -> Vec<SynNode<'_>>
{
    node.named_children()
        .into_iter()
        .filter(|child| !node_kinds::EXTRAS.contains(&child.kind().as_ref()))
        .collect()
}

/// Finds a binary expression's operator tile and its prelude operator name
/// from the [`node_kinds::BINARY_OPERATORS`] table. The operator tile
/// classifies to no named kind, so the adapter recovers it
/// ([`SynNode::binary_operator`]) and its spelling keys the table.
fn binary_operator(node: SynNode<'_>) -> LowerResult<(SynNode<'_>, OperatorText<'static>)>
{
    let malformed = || LowerError::MalformedNode {
        kind: node.kind(),
        byte_range: node.byte_range(),
    };
    let operator = node.binary_operator().ok_or_else(malformed)?;
    let spelling = operator.text();
    for &(kind, name) in &node_kinds::BINARY_OPERATORS {
        if spelling.as_ref() == kind {
            return Ok((operator, name.into()));
        }
    }
    Err(malformed())
}

#[cfg(test)]
mod tests
{
    use super::*;
    #[test]
    fn thunk_carrier_is_arena_rooted_and_reads_back_to_public_term() -> Result<(), String>
    {
        let source = "thunk { ret 1 }";
        let tree = parse_tree(source.into())?;
        let root = tree.root();
        let Some(statement) = named_non_extra_children(root).into_iter().next()
        else {
            return Err(format!(
                "source must have one top-level expression: {source}"
            ));
        };
        let expr = expression_node(statement).map_err(|error| error.to_string())?;

        let mut direct = lowerer(source.into());
        let mut hoists = Vec::new();
        let lowered_expr = direct
            .expr(expr, &mut hoists)
            .map_err(|error| error.to_string())?;
        assert!(
            hoists.is_empty(),
            "thunk literal must not leave value hoists"
        );
        let EOut::Value(carrier) = lowered_expr
        else {
            return Err("thunk must lower as a value".to_owned());
        };
        let private_readback = carrier
            .readback_value()
            .map_err(|error| error.to_string())?;
        assert_eq!(
            carrier.arena.value(carrier.root),
            Ok(private_readback.clone())
        );
        assert!(
            matches!(private_readback, Value::Thunk(Grade::OMEGA, _)),
            "thunk expression must read back as a thunk value"
        );

        let mut public = lowerer(source.into());
        let (term, _) = public
            .finalize_term(expr)
            .map_err(|error| error.to_string())?;
        assert_eq!(term, Term::Value(private_readback));
        Ok(())
    }

    /// Parse `source` into a borrowable melder tree (the `SynNode` front-end),
    /// mapping a commit failure to a message.
    fn parse_tree(source: PipelineSource<'_>) -> Result<SynTree, String>
    {
        SynTree::parse(source.as_ref()).map_err(|error| format!("{source:?} must parse: {error:?}"))
    }

    fn expression_node(statement: SynNode<'_>) -> LowerResult<SynNode<'_>>
    {
        if statement.kind() == node_kinds::EXPRESSION_STATEMENT {
            sole_inner_expression(statement)
        }
        else {
            Ok(statement)
        }
    }
    #[test]
    fn curried_lambda_carrier_is_arena_rooted_and_reads_back_to_public_term() -> Result<(), String>
    {
        let source = "fn(x: Integer, y: Integer) { ret x }";
        let tree = parse_tree(source.into())?;
        let root = tree.root();
        let Some(statement) = named_non_extra_children(root).into_iter().next()
        else {
            return Err(format!(
                "source must have one top-level expression: {source}"
            ));
        };
        let expr = expression_node(statement).map_err(|error| error.to_string())?;

        let mut direct = lowerer(source.into());
        let mut hoists = Vec::new();
        let lowered_expr = direct
            .expr(expr, &mut hoists)
            .map_err(|error| error.to_string())?;
        assert!(
            hoists.is_empty(),
            "lambda expression must not leave value hoists"
        );
        let EOut::Comp(carrier) = lowered_expr
        else {
            return Err("lambda must lower as a computation".to_owned());
        };
        let private_readback = carrier.readback_comp().map_err(|error| error.to_string())?;
        assert_eq!(
            carrier.arena.comp(carrier.root),
            Ok(private_readback.clone())
        );
        let Comp::Abs(_, _, ref outer_body) = private_readback
        else {
            return Err("curried lambda must start with Abs".to_owned());
        };
        assert!(
            matches!(outer_body.as_ref(), Comp::Abs(_, _, _)),
            "curried lambda must nest an inner Abs"
        );

        let mut public = lowerer(source.into());
        let (term, _) = public
            .finalize_term(expr)
            .map_err(|error| error.to_string())?;
        assert_eq!(term, Term::Comp(private_readback));
        Ok(())
    }

    fn lowerer(source: PipelineSource<'_>) -> Lowerer<'_>
    {
        Lowerer {
            source,
            hoist: Gensym::new(GandrSort::TmpHoist),
            holes: Gensym::new(GandrSort::HoleAddr),
            strictness: Strictness::Strict,
            foreign: BTreeMap::new(),
            import_scope: Scope::new(),
            import_index_base: ImportIndex(0),
            codata: BTreeMap::new(),
            observations: BTreeSet::new(),
            data: BTreeMap::new(),
            constructors: BTreeMap::new(),
            obligations: Vec::new(),
        }
    }

    #[test]
    fn origin_map_uses_stable_ids_for_primary_lookup() -> Result<(), String>
    {
        let lowered = lower_source("ret 1".into()).map_err(|error| error.to_string())?;
        let Some(root_id) = lowered.origin.id_for_path(&[0])
        else {
            return Err("root compatibility path must have an origin id".to_owned());
        };
        let Some(child_id) = lowered.origin.id_for_path(&[0, 0])
        else {
            return Err("child compatibility path must have an origin id".to_owned());
        };
        assert_ne!(root_id, child_id);
        assert_eq!(0, root_id.raw().0);
        assert_eq!(1, child_id.raw().0);
        let Some(root_by_id) = lowered.origin.get(root_id)
        else {
            return Err("stable root id must resolve".to_owned());
        };
        let Some(root_by_path) = lowered.origin.get_path(&[0])
        else {
            return Err("root compatibility path must resolve".to_owned());
        };
        assert_eq!(root_by_id, root_by_path);
        Ok(())
    }

    #[test]
    fn origin_snapshot_is_legacy_path_readback() -> Result<(), String>
    {
        let lowered = lower_source("ret 1".into()).map_err(|error| error.to_string())?;
        // Each line carries the legacy path, the byte range, and the
        // reproducible merkle `cst_hash` column; the positional arena slot is
        // omitted. Assert the path/range prefixes, then that a
        // well-formed 16-hex `#hash` column follows each.
        let snapshot = lowered.origin.snapshot();
        let lines: Vec<&str> = snapshot.lines().collect();
        let prefixes = ["[0] => 0..5 #", "[0, 0] => 4..5 #"];
        assert_eq!(
            lines.len(),
            prefixes.len(),
            "one line per entry: {snapshot:?}"
        );
        for (line, prefix) in lines.iter().zip(prefixes) {
            let hash = line.strip_prefix(prefix).ok_or_else(|| {
                format!("snapshot line {line:?} lacks path/range prefix {prefix:?}")
            })?;
            assert_eq!(16, hash.len(), "cst_hash is a 16-hex column: {line:?}");
            u64::from_str_radix(hash, 16)
                .map_err(|error| format!("cst_hash {hash:?} must be hex: {error}"))?;
        }
        let paths: Vec<Vec<u32>> = lowered
            .origin
            .iter_paths()
            .map(|(path, _id, _entry)| Vec::<u32>::from(path.clone()))
            .collect();
        assert_eq!(paths, vec![vec![0], vec![0, 0]]);
        Ok(())
    }

    // --- FFI extern-block elaboration (proposal-ffi.md §2, §3.1, §4) ----------

    #[test]
    fn extern_block_registers_a_foreign_module_and_no_runnable_item() -> Result<(), String>
    {
        let source = "extern \"c\" from \"m\" {\n  def cos(x: f64) -> f64;\n  def pow(base: \
                      f64, exp: f64) -> f64;\n}";
        let lowered = lower_source(source.into()).map_err(|error| error.to_string())?;
        assert!(
            lowered.items.is_empty(),
            "an extern block is a declaration, not a runnable item"
        );
        assert_eq!(1, lowered.foreign.len(), "one foreign module registered");
        let module = lowered
            .foreign
            .first()
            .ok_or_else(|| "one module was just asserted".to_owned())?;
        assert_eq!("m", module.name, "the namespace is the library string");
        assert_eq!("c", module.abi);
        assert_eq!("m", module.library);
        assert_eq!(
            vec!["cos", "pow"],
            module
                .functions
                .iter()
                .map(|function| function.op.as_str())
                .collect::<Vec<_>>()
        );
        let cos = module
            .function("cos")
            .ok_or_else(|| "cos must be declared".to_owned())?;
        assert_eq!(cos.params, vec![ForeignParam {
            name: "x".to_owned(),
            c_type: CType::F64,
        }]);
        assert_eq!(CType::F64, cos.result);
        Ok(())
    }

    #[test]
    fn foreign_call_elaborates_to_a_perform_against_the_library_signature() -> Result<(), String>
    {
        let source = "extern \"c\" from \"m\" {\n  def cos(x: f64) -> f64;\n}\nm.cos(2.0f64)";
        let lowered = lower_source(source.into()).map_err(|error| error.to_string())?;
        assert_eq!(1, lowered.items.len(), "only the call is a runnable item");
        let item = lowered
            .items
            .first()
            .ok_or_else(|| "one item was just asserted".to_owned())?;
        let Term::Comp(Comp::Perform(ref sig, ref op, ref arg)) = item.term
        else {
            return Err(format!(
                "the foreign call must elaborate to a perform, got {:?}",
                item.term
            ));
        };
        assert_eq!(
            "m",
            sig.name().as_ref(),
            "the perform carries the per-library signature"
        );
        assert_eq!("cos", op);
        // The op resolves in the inline-carried signature (row ⟨m⟩ is honest).
        let op_def = sig
            .op("cos".into())
            .ok_or_else(|| "the signature must resolve the op".to_owned())?;
        assert_eq!(op_def.reply(), &ValueType::f64());
        // The payload is the argument record keyed by parameter name.
        let Value::Record(ref fields) = **arg
        else {
            return Err(format!(
                "the payload must be an argument record, got {arg:?}"
            ));
        };
        let x = fields
            .get("x")
            .ok_or_else(|| "the payload must carry the `x` argument".to_owned())?;
        assert_eq!(x.as_ref(), &Value::f64(2.0_f64));
        Ok(())
    }

    #[test]
    fn cstr_and_opaque_handle_boundary_types_map_per_section_4() -> Result<(), String>
    {
        let source = "extern \"c\" from \"lib\" {\n  type Db;\n  def open(path: CStr) -> \
                      Db;\n  def size(db: Db) -> i64;\n}";
        let lowered = lower_source(source.into()).map_err(|error| error.to_string())?;
        let module = lowered
            .foreign
            .first()
            .ok_or_else(|| "the module must register".to_owned())?;
        assert_eq!(
            module.types,
            vec!["Db".to_owned()],
            "the opaque handle type"
        );
        let open = module
            .function("open")
            .ok_or_else(|| "open must be declared".to_owned())?;
        let path = open
            .params
            .first()
            .ok_or_else(|| "open takes a path parameter".to_owned())?;
        assert_eq!(CType::CStr, path.c_type, "CStr crosses as char*");
        assert_eq!(
            CType::Ptr,
            open.result,
            "an opaque handle crosses as a pointer"
        );
        let open_op = open.effect_op();
        assert_eq!(
            open_op.payload(),
            &ValueType::record([("path".to_owned(), ValueType::string(),)])
        );
        assert_eq!(
            open_op.reply(),
            &ValueType::u64(),
            "the MVP handle carrier is u64"
        );
        Ok(())
    }

    #[test]
    fn foreign_call_to_an_unknown_member_is_unsupported() -> Result<(), String>
    {
        let source = "extern \"c\" from \"m\" {\n  def cos(x: f64) -> f64;\n}\nm.nope(1.0f64)";
        let error = lower_source(source.into())
            .err()
            .ok_or_else(|| "an unknown foreign member must be rejected".to_owned())?;
        assert!(
            matches!(error, LowerError::Unsupported { .. }),
            "a known module with an unknown member is Unsupported, got {error:?}"
        );
        Ok(())
    }

    #[test]
    fn foreign_call_arity_mismatch_is_unsupported() -> Result<(), String>
    {
        let source = "extern \"c\" from \"m\" {\n  def pow(base: f64, exp: f64) -> \
                      f64;\n}\nm.pow(2.0f64)";
        let error = lower_source(source.into())
            .err()
            .ok_or_else(|| "an arity mismatch must be rejected".to_owned())?;
        assert!(
            matches!(error, LowerError::Unsupported { .. }),
            "a foreign call with the wrong arity is Unsupported, got {error:?}"
        );
        Ok(())
    }

    #[test]
    fn host_call_elaborates_to_a_perform_against_the_host_signature() -> Result<(), String>
    {
        let source = "fs.read(\"/etc/hosts\")";
        let lowered = lower_source(source.into()).map_err(|error| error.to_string())?;
        assert_eq!(1, lowered.items.len(), "only the call is a runnable item");
        let item = lowered
            .items
            .first()
            .ok_or_else(|| "one item was just asserted".to_owned())?;
        let Term::Comp(Comp::Perform(ref sig, ref op, ref arg)) = item.term
        else {
            return Err(format!(
                "the host call must elaborate to a perform, got {:?}",
                item.term
            ));
        };
        assert_eq!(
            "Fs",
            sig.name().as_ref(),
            "the perform carries the host signature"
        );
        assert_eq!("read", op);
        // A one-parameter member performs the bare argument value (the
        // signature's payload type is `String`, not a record).
        assert_eq!(arg.as_ref(), &Value::string("/etc/hosts"));
        let op_def = sig
            .op("read".into())
            .ok_or_else(|| "the signature must resolve the op".to_owned())?;
        assert_eq!(op_def.reply(), &ValueType::string());
        Ok(())
    }

    #[test]
    fn host_call_packs_multi_parameter_payloads_as_a_record() -> Result<(), String>
    {
        let source = "fs.write(\"/tmp/out\", \"hello\")";
        let lowered = lower_source(source.into()).map_err(|error| error.to_string())?;
        let item = lowered
            .items
            .first()
            .ok_or_else(|| "the call must lower to an item".to_owned())?;
        let Term::Comp(Comp::Perform(ref sig, ref op, ref arg)) = item.term
        else {
            return Err(format!(
                "the host call must elaborate to a perform, got {:?}",
                item.term
            ));
        };
        assert_eq!("Fs", sig.name().as_ref());
        assert_eq!("write", op);
        assert_eq!(
            arg.as_ref(),
            &Value::record([
                ("path".to_owned(), Value::string("/tmp/out")),
                ("contents".to_owned(), Value::string("hello")),
            ]),
            "a several-parameter member performs the parameter-keyed record"
        );
        Ok(())
    }

    #[test]
    fn host_call_zero_parameter_payload_is_unit() -> Result<(), String>
    {
        for (source, sig_name, op_name) in [
            ("fs.tempdir()", "Fs", "tempdir"),
            ("fs.cwd()", "Fs", "cwd"),
            ("env.path()", "Env", "path"),
        ] {
            let lowered = lower_source(source.into()).map_err(|error| error.to_string())?;
            let item = lowered
                .items
                .first()
                .ok_or_else(|| "the call must lower to an item".to_owned())?;
            let Term::Comp(Comp::Perform(ref sig, ref op, ref arg)) = item.term
            else {
                return Err(format!(
                    "`{source}` must elaborate to a perform, got {:?}",
                    item.term
                ));
            };
            assert_eq!(sig.name().as_ref(), sig_name, "`{source}`");
            assert_eq!(op, op_name, "`{source}`");
            assert_eq!(
                &Value::Unit,
                arg.as_ref(),
                "`{source}`: a zero-parameter member performs `()`"
            );
        }
        Ok(())
    }

    #[test]
    fn host_call_covers_env_get() -> Result<(), String>
    {
        let lowered =
            lower_source("env.get(\"HOME\")".into()).map_err(|error| error.to_string())?;
        let item = lowered
            .items
            .first()
            .ok_or_else(|| "the call must lower to an item".to_owned())?;
        let Term::Comp(Comp::Perform(ref sig, ref op, ref arg)) = item.term
        else {
            return Err(format!("env.get must perform, got {:?}", item.term));
        };
        assert_eq!(("Env", "get"), (sig.name().as_ref(), op.as_str()));
        assert_eq!(arg.as_ref(), &Value::string("HOME"));
        Ok(())
    }

    #[test]
    fn host_call_covers_proc_exit() -> Result<(), String>
    {
        let lowered = lower_source("proc.exit(3)".into()).map_err(|error| error.to_string())?;
        let item = lowered
            .items
            .first()
            .ok_or_else(|| "the call must lower to an item".to_owned())?;
        let Term::Comp(Comp::Perform(ref sig, ref op, ref arg)) = item.term
        else {
            return Err(format!("proc.exit must perform, got {:?}", item.term));
        };
        assert_eq!(("Proc", "exit"), (sig.name().as_ref(), op.as_str()));
        assert_eq!(arg.as_ref(), &Value::int(3));
        Ok(())
    }

    #[test]
    fn host_call_hoists_computation_arguments() -> Result<(), String>
    {
        // The inner `fs.cwd()` is a computation in value position: it hoists
        // to a fresh variable bound around the outer perform.
        let source = "fs.read(fs.cwd())";
        let lowered = lower_source(source.into()).map_err(|error| error.to_string())?;
        let item = lowered
            .items
            .first()
            .ok_or_else(|| "the call must lower to an item".to_owned())?;
        let Term::Comp(Comp::Bind(ref bound, _, ref body)) = item.term
        else {
            return Err(format!(
                "the computation argument must hoist to a bind, got {:?}",
                item.term
            ));
        };
        let Comp::Perform(ref inner_sig, ref inner_op, _) = **bound
        else {
            return Err(format!(
                "the bound side must be the inner perform, got {bound:?}"
            ));
        };
        assert_eq!(
            ("Fs", "cwd"),
            (inner_sig.name().as_ref(), inner_op.as_str())
        );
        let Comp::Perform(ref outer_sig, ref outer_op, ref payload) = **body
        else {
            return Err(format!("the body must be the outer perform, got {body:?}"));
        };
        assert_eq!(
            ("Fs", "read"),
            (outer_sig.name().as_ref(), outer_op.as_str())
        );
        assert!(
            matches!(**payload, Value::Var(_)),
            "the outer payload is the hoisted variable, got {payload:?}"
        );
        Ok(())
    }

    #[test]
    fn host_call_to_an_unknown_member_is_unsupported() -> Result<(), String>
    {
        let error = lower_source("fs.nope(\"x\")".into())
            .err()
            .ok_or_else(|| "an unknown host member must be rejected".to_owned())?;
        assert!(
            matches!(error, LowerError::Unsupported { .. }),
            "a host module with an unknown member is Unsupported, got {error:?}"
        );
        Ok(())
    }

    #[test]
    fn host_call_arity_mismatch_is_unsupported() -> Result<(), String>
    {
        let error = lower_source("fs.write(\"/tmp/out\")".into())
            .err()
            .ok_or_else(|| "an arity mismatch must be rejected".to_owned())?;
        assert!(
            matches!(error, LowerError::Unsupported { .. }),
            "a host call with the wrong arity is Unsupported, got {error:?}"
        );
        Ok(())
    }

    #[test]
    fn bare_host_module_selection_is_unsupported() -> Result<(), String>
    {
        // A host module has no value-level members: uncalled `fs.read` (and
        // any other bare selection) takes the declined path.
        for source in ["fs.read", "fs.nope", "proc.exit"] {
            let error = lower_source(source.into())
                .err()
                .ok_or_else(|| format!("`{source}` must be rejected"))?;
            assert!(
                matches!(error, LowerError::Unsupported { .. }),
                "`{source}`: a bare host-module selection is Unsupported, got {error:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn extern_module_shadows_a_host_module() -> Result<(), String>
    {
        // A user `extern` declaration named `fs` claims the namespace: the
        // call takes the foreign path (record payload keyed by the declared
        // parameter), not the host path (bare payload).
        let source = "extern \"c\" from \"fs\" {\n  def read(x: f64) -> f64;\n}\nfs.read(2.0f64)";
        let lowered = lower_source(source.into()).map_err(|error| error.to_string())?;
        let item = lowered
            .items
            .first()
            .ok_or_else(|| "the call must lower to an item".to_owned())?;
        let Term::Comp(Comp::Perform(ref sig, ref op, ref arg)) = item.term
        else {
            return Err(format!("the call must perform, got {:?}", item.term));
        };
        assert_eq!(
            "fs",
            sig.name().as_ref(),
            "the perform carries the extern signature"
        );
        assert_eq!("read", op);
        assert!(
            matches!(**arg, Value::Record(_)),
            "the foreign convention packs an argument record, got {arg:?}"
        );
        Ok(())
    }

    #[test]
    fn host_call_errors_become_holes_in_total_mode() -> Result<(), String>
    {
        // The strict rejections above elide to holes in total mode, so a
        // damaged host call never poisons the rest of the file.
        for source in ["fs.nope(\"x\")", "fs.write(\"/tmp/out\")", "fs.read"] {
            let lowered = lower_source_total(source.into()).map_err(|error| error.to_string())?;
            assert_eq!(1, lowered.items.len(), "`{source}` still lowers to an item");
        }
        Ok(())
    }

    #[test]
    fn undeclared_boundary_type_is_rejected() -> Result<(), String>
    {
        // `Integer` is not a boundary atom — the boundary demands a concrete
        // fixed-width atom (proposal-ffi.md §4.1).
        let source = "extern \"c\" from \"m\" {\n  def f(x: Integer) -> f64;\n}";
        let error = lower_source(source.into())
            .err()
            .ok_or_else(|| "a non-boundary parameter type must be rejected".to_owned())?;
        assert!(
            matches!(error, LowerError::Unsupported { .. }),
            "an out-of-boundary parameter type is Unsupported, got {error:?}"
        );
        Ok(())
    }
}
