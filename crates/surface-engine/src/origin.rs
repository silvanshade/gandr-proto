//! Source identity for lowered terms: the `OriginMap` side table.
//!
//! `gandr-core-checker` syntax stays span-free and parser-free by decision;
//! this module is where positions live instead. It maps stable origin node
//! IDs to CST node IDs and byte ranges, plus an elaboration tag for
//! synthesized nodes (the `def` sugar, operator elaboration, `if` desugaring,
//! …) so elaborations can be un-sugared on demand.
//!
//! # Stable IDs and legacy paths
//!
//! [`OriginMap`] stores entries by [`OriginNodeId`], a local, deterministic
//! identity minted as lowered origin roots are flattened. Structural
//! child-index paths are a compatibility/readback boundary only: diagnostics,
//! goals, and existing goldens can still ask for path-ordered snapshots through
//! [`OriginMap::get_path`] and [`OriginMap::iter_paths`], but paths are not the
//! provenance table's primary key.

use alloc::collections::BTreeMap;

use gandr_core_term::boundary::PathIndex;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::syntax::WalkBase;
use gandr_surface_syntax::NodeId;
use gandr_surface_syntax::StableHash;

use crate::boundary::ItemIndex;
use crate::boundary::OriginEntryCount;
use crate::boundary::OriginMapEmpty;
use crate::boundary::OriginNodeOrdinal;
use crate::boundary::RecoveredSpanFlag;
use crate::boundary::SourceRange;
use crate::boundary::SyntaxKind;

/// Stable identity for a lowered origin node within one [`OriginMap`].
///
/// IDs are minted in deterministic root/preorder during lowerer finalization.
/// They are independent of the compatibility path index used for legacy
/// diagnostics and golden snapshots.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OriginNodeId
{
    /// Raw deterministic preorder ordinal local to this origin map.
    raw: u64,
}

impl OriginNodeId
{
    /// Builds an origin node ID from its raw local ordinal.
    #[inline]
    #[must_use]
    pub fn new<O>(raw: O) -> Self
    where
        O: Into<OriginNodeOrdinal>,
    {
        Self { raw: raw.into().0 }
    }

    /// Returns this ID's raw local ordinal.
    ///
    /// The ordinal is deterministic within one flattened [`OriginMap`] but is
    /// not a cross-map or cross-run stable identifier. Prefer carrying the
    /// typed [`OriginNodeId`] except at serialization/debugging boundaries.
    #[inline]
    #[must_use]
    pub const fn raw(self) -> OriginNodeOrdinal
    {
        OriginNodeOrdinal(self.raw)
    }
}
/// Compatibility path: item index followed by term child indices.
///
/// This is retained only as a snapshot/readback boundary for path-oriented
/// diagnostics and tests. [`OriginMap`] itself is keyed by [`OriginNodeId`].
#[repr(transparent)]
#[derive(Clone, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct OriginPath(pub Vec<u32>);

impl From<Vec<u32>> for OriginPath
{
    #[inline]
    fn from(value: Vec<u32>) -> Self
    {
        Self(value)
    }
}

impl core::fmt::Debug for OriginPath
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        self.0.fmt(f)
    }
}

impl From<OriginPath> for Vec<u32>
{
    #[inline]
    fn from(value: OriginPath) -> Self
    {
        value.0
    }
}

impl core::ops::Deref for OriginPath
{
    type Target = Vec<u32>;

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl core::ops::DerefMut for OriginPath
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.0
    }
}

impl core::borrow::Borrow<[u32]> for OriginPath
{
    #[inline]
    fn borrow(&self) -> &[u32]
    {
        &self.0
    }
}

/// Borrowed compatibility path through a lowered origin tree.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct OriginPathRef<'path>(pub &'path [u32]);

impl<'path> From<&'path OriginPath> for OriginPathRef<'path>
{
    #[inline]
    fn from(value: &'path OriginPath) -> Self
    {
        Self(&value.0)
    }
}

impl<'path> From<&'path Vec<u32>> for OriginPathRef<'path>
{
    #[inline]
    fn from(value: &'path Vec<u32>) -> Self
    {
        Self(value.as_slice())
    }
}

impl<'path> From<&'path [u32]> for OriginPathRef<'path>
{
    #[inline]
    fn from(value: &'path [u32]) -> Self
    {
        Self(value)
    }
}

impl<'path, const N: usize> From<&'path [u32; N]> for OriginPathRef<'path>
{
    #[inline]
    fn from(value: &'path [u32; N]) -> Self
    {
        Self(value.as_slice())
    }
}

impl core::ops::Deref for OriginPathRef<'_>
{
    type Target = [u32];

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        self.0
    }
}

/// Why a lowered term node does not correspond one-to-one to a CST node.
///
/// Every synthesized node carries one of these in its [`OriginEntry`], per
/// the proposal §5.2 requirement that elaborations be recorded so they can be
/// un-sugared on demand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElabKind
{
    /// `def f(x: A) -> B { t }` ⇒ ascription `U_ω (A → B)`, term
    /// `thunk { fn(x: A) { t } }` (the `def` sugar).
    DefFunctionSugar,
    /// `if c { t } else { u }` ⇒ `case` on `1 + 1`.
    IfSugar,
    /// `true` / `false` ⇒ an annotated injection into `1 + 1`.
    BoolLiteral,
    /// A binary/unary operator ⇒ an application of a forced prelude operator
    /// (`x * y` ⇒ `(force mul) x y`).
    OperatorElab,
    /// The syntax-directed force sugar: a call head (or projection target)
    /// that lowers to a *value* is wrapped in `Force`.
    ForceSugar,
    /// A value in computation position (block tail, arm body, …) is wrapped
    /// in `Ret`.
    RetCoercion,
    /// A computation in value position is hoisted: `ret (x * x)` ⇒
    /// `(x * x) >>= %tmp. ret %tmp` (the synthesized `Bind` and the variable
    /// occurrence both carry this tag).
    BindHoist,
    /// `let x = v; t` ⇒ `Bind(Ret v, x, t)` (the recorded `let`-value
    /// elaboration of the design).
    LetValueBind,
    /// `t;` ⇒ `Bind(t, "_", …)` (statement sequencing sugar).
    SeqDiscard,
    /// A computation-sorted ascription `(t : B)` ⇒ `force ((thunk t) : U_ω B)`
    /// — core has no computation-annotation node, so the expected type rides
    /// the thunk annotation and `force` synthesizes it back.
    CompAscription,
    /// Multi-parameter/multi-argument currying: the *inner* nodes of
    /// `fn(x, y) { t }` ⇒ `fn(x) { fn(y) { t } }` and `f(v, w)` ⇒ `f(v)(w)`.
    CurrySugar,
    /// The inner synthesized pairs of an n-ary tuple `(a, b, c)` ⇒
    /// `(a, (b, c))`.
    TupleNest,
    /// The inner synthesized splits (and fresh scrutinee variables) of an
    /// n-ary tuple pattern `let (x, y, z) = v;`.
    SplitNest,
    /// A nested constructor pattern's own eliminator: `case m { Just(Just(x))
    /// => t }` ⇒ an inner `DataCase` on the outer constructor's payload, with
    /// every tag the nested pattern does not name a missing-arm hole.
    PatternNest,
    /// A pattern binder the arm body sees under a nested test, or an
    /// as-binder: `p as x` ⇒ `Bind(Ret v, x, …)`. The name is bound rather
    /// than substituted so the binder keeps one occurrence and cannot capture.
    PatternAlias,
    /// The join point a compiled match binds when one arm body is reached
    /// from more than one branch: `run %j <- ret ((thunk { fn(%p) { body } })
    /// : U_ω (? → ?))`, with each reaching branch a `force %j %x`. The core
    /// has no join-point former because a join point is a compilation device
    /// rather than a term someone writes.
    PatternJoin,
    /// The occurrence a compiled match binds its scrutinee to, so every test
    /// beneath reads one variable rather than re-evaluating the scrutinee
    /// expression.
    PatternScrutinee,
    /// A module-select `M.l` whose value is a known module name ⇒ the flat
    /// qualified `Var("M.l")` (the module layer's namespace half — pure
    /// elaboration, unlike the structural `t.fst` / `t.snd` projection the
    /// same `projection_expression` otherwise lowers to).
    ModuleSelect,
    /// `module M (: #{ … })? { members… }` ⇒ one named record item with
    /// source-ordered generated binds for member definitions and a final
    /// returned record (the checked-module record lowering; no core module
    /// node).
    /// This tag marks the synthesized record and the module-only sequencing
    /// envelope so diagnostics can distinguish module sugar from an ordinary
    /// record literal or user-written `let`.
    ModuleDeclaration,
    /// Functional record update `#{ r | ℓ = v, … }` ⇒ a fresh-record rebuild
    /// `recordupdate r #{ℓ = v, …}` over
    /// [`gandr_core_term::prim::NativePrim::RecordUpdate`] (value-semantics
    /// MVP, `spec:surface-language/value-semantics.md`). The tag
    /// marks the synthesized application so a diagnostic can un-sugar the
    /// update back to its base-and-overrides, and the surface unparser can
    /// re-sugar it.
    RecordUpdate,
    /// A foreign call `m.op(args)` whose `m` is an `extern`-declared module ⇒
    /// `perform m.op {payload}` against the module's per-library effect
    /// signature (`spec:implementation/foreign-interface.md`). The synthesized
    /// `Perform` carries this tag; the argument record retains its own
    /// origin.
    ForeignPerform,
    /// A copattern definition's `Cosplit` case-tree node lowered to the
    /// record-of-thunks carrier `#{ πᵢ = thunk_ω tᵢ }` (codata design §4.2
    /// route (a), `codata MVP`): the synthesized record and each
    /// observation's delayed thunk carry this tag, so a diagnostic can
    /// re-sugar the record back to its copattern clauses. Zero frozen-core
    /// spend — the carrier is the existing record former over `U_ω` thunks.
    Cosplit,
    /// A codata observation `s.π` lowered to `let t <- RecordProj(s, π);
    /// force t` (codata design §3.1): the projection is the record field read
    /// and `force` performs the observation. The synthesized `Bind` /
    /// `Force` carry this tag; the observed target retains its own origin.
    Observe,
    /// A host-module call `fs.read(v)` / `env.get(v)` / `proc.exit(v)` whose
    /// head is a reserved host module ⇒ `perform` against the corresponding
    /// host effect signature ([`crate::host`], `host-module surface` — the same
    /// module-select ⇒ perform elaboration as [`Self::ForeignPerform`], with
    /// the payload shaped by the member's declared parameters). The
    /// synthesized `Perform` carries this tag; the arguments retain their own
    /// origins.
    HostPerform,
}

/// What a hole elides.
///
/// The structured note carried by every hole the lowerer synthesizes in
/// total mode ("with a `HoleNote::{SyntaxError, UnsupportedForm}` in the
/// origin map"); the catalogue mirrors the input-shaped
/// [`crate::lower::LowerError`] constructors, which are what total lowering
/// converts. The note plus the entry's byte range answer "what was elided,
/// and where" — together with the expected type from the goals report, this
/// is the seed of the hole-goal surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HoleNote
{
    /// An `ERROR` or `MISSING` CST region (the melder-CST switch retired this
    /// in favor of the per-obligation-class
    /// notes below; the variant is retained for compatibility but has no
    /// producer).
    SyntaxError,
    /// A returner / force / operator operand the source elides — no term where
    /// one was expected ([`gandr_surface_parser::Oblig::MissingMeld`]).
    MissingOperand,
    /// A delimiter the source never closes
    /// ([`gandr_surface_parser::Oblig::MissingTile`]); `opened_at` localizes
    /// the unclosed opener.
    MissingDelimiter
    {
        /// The byte range of the unclosed opener the obligation points at.
        opened_at: SourceRange,
    },
    /// A partially typed keyword
    /// ([`gandr_surface_parser::Oblig::IncompleteTile`]); `typed` is the
    /// prefix the source wrote.
    IncompleteKeyword
    {
        /// The partial keyword text the source typed.
        typed: String,
    },
    /// Two adjacent terms where the grammar expects an operator between them
    /// ([`gandr_surface_parser::Oblig::ExtraMeld`]).
    AdjacentTerms,
    /// A token the grammar has no tile for
    /// ([`gandr_surface_parser::Oblig::UnmoldedTok`]); the entry's byte range
    /// carries the unrecognized token.
    UnrecognizedToken,
    /// A reserved keyword used as an ordinary name
    /// ([`gandr_surface_parser::Oblig::ReservedKeyword`]).
    ReservedKeyword,
    /// Two precedence-incomparable operators with no disambiguating grouping
    /// ([`gandr_surface_parser::Oblig::AmbiguousPrec`]), at maximum severity.
    AmbiguousOperatorPrecedence,
    /// A construct outside the Stage-1-typeable fragment (sessions, sharing,
    /// worlds, shell blocks, …).
    UnsupportedForm
    {
        /// The elided construct's node kind.
        kind: SyntaxKind,
    },
    /// A module declaration whose lowercase-initial name could not be molded
    /// into the grammar's uppercase-only module-name tile.
    LowercaseModuleName
    {
        /// The source spelling the declaration could not use.
        name: String,
    },
    /// A `case` arm the source does not supply; the hole *is* the missing
    /// arm's body.
    MissingCaseArm
    {
        /// The constructor whose arm is missing (`Inl` or `Inr`).
        constructor: &'static str,
    },
    /// A block with no tail expression; the hole is the missing tail.
    EmptyBlock,
    /// A `number` token that is not an `i64` integer literal.
    InvalidIntegerLiteral
    {
        /// The literal's source text.
        text: String,
    },
    /// A grade annotation that is not a `u64` numeral or `ω`; the whole
    /// graded construct is elided (grades have no unknown representative).
    InvalidGrade
    {
        /// The grade's source text.
        text: String,
    },
    /// A `def name : T;` signature with no matching definition; the hole is
    /// the missing definition and the signature is its recorded goal.
    MissingDefinition
    {
        /// The signature's name.
        name: String,
    },
    /// A node missing grammar-guaranteed structure (only reachable on
    /// damaged trees).
    MalformedNode
    {
        /// The malformed node's kind.
        kind: SyntaxKind,
    },
    /// A `case` arm the *user* left indeterminate: a pattern hole stands
    /// where a test would, so the constructor tags that arm shadows can be
    /// neither taken nor passed over and the match stops here.
    ///
    /// **Distinct from [`Self::MissingCaseArm`], and the distinction is the
    /// point.** A missing arm is a match the source never wrote a branch for;
    /// an indeterminate one is a branch the source wrote and did not finish.
    /// Both reach the same runtime outcome, so the note is the only place the
    /// two stay apart — and filling the hole discharges this one while nothing
    /// the author writes inside the arm discharges the other. The optional
    /// `name` is the `?name` identifier, carried to address the hole and
    /// ignored by typing exactly as [`Self::UserHole`]'s is.
    IndeterminatePattern
    {
        /// The `?name` identifier, when the user named the hole.
        name: Option<String>,
    },
    /// A hole the *user wrote* (`?` or `?name`), not one recovered from a
    /// `LowerError`. Unlike every other variant it is created in both strict
    /// and total mode — a user hole is a legitimate term (an axiom), never a
    /// recovery artifact. The optional `name` is the Hazelnut hole *name* `u`:
    /// ignored by typing (two holes with different names type identically),
    /// carried only to address the hole in the goal stream.
    UserHole
    {
        /// The `?name` identifier, when the user named the hole.
        name: Option<String>,
    },
}

/// What one lowered term node originated from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginEntry
{
    /// The originating CST node's stable identity in the melder arena
    /// ([`gandr_surface_syntax::NodeId`], behind
    /// [`crate::synnode::SynNode::cst_node`]). A dense arena slot:
    /// deterministic within one parse but *positional* — a within-tree
    /// address (the substrate the structural diff aligns on), never
    /// a cross-run identity. [`OriginMap::snapshot`] keys on [`Self::cst_hash`]
    /// instead.
    pub cst_node: NodeId,
    /// The originating CST node's per-node merkle hash
    /// ([`gandr_surface_syntax::Cst::hash`], behind
    /// [`crate::synnode::SynNode::cst_hash`]): a content fingerprint over the
    /// node's significant structure. Reproducible across runs *and* processes —
    /// the property the freed tree-sitter subtree address it superseded
    /// provably lacked — so [`OriginMap::snapshot`] includes it as a sound
    /// provenance golden key.
    pub cst_hash: StableHash,
    /// The originating CST node's byte range in the source.
    pub byte_range: SourceRange,
    /// The elaboration tag, present exactly on synthesized nodes.
    pub elaboration: Option<ElabKind>,
    /// The hole note, present exactly on holes synthesized by total-mode
    /// lowering: what was elided at this position.
    pub note: Option<HoleNote>,
}

/// A semantically meaningful source component of one lowered term node.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OriginFacetKind
{
    /// An explicit grade written on a graded term former.
    Grade,
}

/// Exact source provenance for one non-child component of a lowered term.
///
/// Facets retain information such as a grade token that belongs to a semantic
/// node but is not itself a term child. Diagnostics can therefore select the
/// precise component named by an error without distorting the shadow tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginFacet
{
    /// The component's semantic role.
    pub kind: OriginFacetKind,
    /// Its exact half-open source range.
    pub byte_range: SourceRange,
}

/// The origin side table: stable origin node ID → [`OriginEntry`].
///
/// Backed by `BTreeMap`s so iteration order and compatibility snapshots are
/// deterministic. The path index maps legacy structural paths to stable IDs;
/// it is not the primary provenance storage.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OriginMap
{
    /// The stable-ID-indexed entries.
    entries: BTreeMap<OriginNodeId, OriginEntry>,
    /// Compatibility readback from path-oriented callers to stable IDs.
    path_index: BTreeMap<OriginPath, OriginNodeId>,
    /// Non-child semantic components retained for each origin node.
    facets: BTreeMap<OriginNodeId, Vec<OriginFacet>>,
}

impl OriginMap
{
    /// Looks up the entry for a stable origin node ID.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        id: OriginNodeId,
    ) -> Option<&OriginEntry>
    {
        self.entries.get(&id)
    }

    /// Looks up the entry for a legacy structural path.
    ///
    /// This is the explicit compatibility boundary for path-oriented
    /// diagnostics/readbacks. New provenance storage should carry
    /// [`OriginNodeId`] instead.
    #[inline]
    #[must_use]
    pub fn get_path<'path, P>(
        &self,
        path: P,
    ) -> Option<&OriginEntry>
    where
        P: Into<OriginPathRef<'path>>,
    {
        let path = path.into();
        let id = self.path_index.get(path.0)?;
        self.entries.get(id)
    }

    /// Returns the stable ID currently associated with a legacy path.
    #[inline]
    #[must_use]
    pub fn id_for_path<'path, P>(
        &self,
        path: P,
    ) -> Option<OriginNodeId>
    where
        P: Into<OriginPathRef<'path>>,
    {
        self.path_index.get(path.into().0).copied()
    }

    /// Iterates entries in stable-ID order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (OriginNodeId, &OriginEntry)>
    {
        self.entries.iter().map(|(&id, entry)| (id, entry))
    }

    /// Iterates entries in legacy path order for compatibility renderers.
    #[inline]
    pub fn iter_paths(&self) -> impl Iterator<Item = (&OriginPath, OriginNodeId, &OriginEntry)>
    {
        self.path_index
            .iter()
            .filter_map(|(path, id)| self.entries.get(id).map(|entry| (path, *id, entry)))
    }

    /// Returns the ordered semantic components retained for `id`.
    #[inline]
    #[must_use]
    pub fn facets(
        &self,
        id: OriginNodeId,
    ) -> &[OriginFacet]
    {
        self.facets.get(&id).map_or(&[], Vec::as_slice)
    }

    /// The number of recorded term nodes.
    #[inline]
    #[must_use]
    pub fn len(&self) -> OriginEntryCount
    {
        OriginEntryCount(self.entries.len())
    }

    /// Whether the map is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> OriginMapEmpty
    {
        OriginMapEmpty(self.entries.is_empty())
    }

    /// A deterministic, line-oriented compatibility rendering for golden tests:
    /// one `path => byte_range #cst_hash [elaboration] (note)` line per entry
    /// (elaboration and note only when present), in legacy path order.
    ///
    /// [`OriginEntry::cst_hash`] is **included**: the per-node merkle hash is
    /// a content fingerprint, reproducible across runs *and* processes, so it
    /// is a sound golden key over provenance.
    /// The positional [`OriginEntry::cst_node`] (the dense arena slot) is
    /// deliberately omitted — like the tree-sitter node address it
    /// superseded, it is a within-tree position, not a reproducible
    /// identity, and would make snapshots nondeterministic.
    #[inline]
    #[must_use]
    pub fn snapshot(&self) -> String
    {
        let mut lines: Vec<String> = Vec::new();
        for (path, _id, entry) in self.iter_paths() {
            let elab = entry
                .elaboration
                .map_or_else(String::new, |elab| format!(" [{elab:?}]"));
            let note = entry
                .note
                .as_ref()
                .map_or_else(String::new, |note| format!(" ({note:?})"));
            lines.push(format!(
                "{:?} => {:?} #{:016x}{elab}{note}\n",
                path.as_slice(),
                entry.byte_range,
                entry.cst_hash
            ));
        }
        lines.concat()
    }

    /// Records `entry` at stable `id`, with `path` as its compatibility
    /// readback address.
    fn insert(
        &mut self,
        id: OriginNodeId,
        path: OriginPath,
        entry: OriginEntry,
        facets: Vec<OriginFacet>,
    )
    {
        // Duplicate IDs/paths cannot arise from the shadow-tree flattening
        // (each origin node is visited once); plain inserts keep this total
        // without an unreachable error path.
        let _previous_entry = self.entries.insert(id, entry);
        let _previous_path = self.path_index.insert(path, id);
        if !facets.is_empty() {
            let _previous_facets = self.facets.insert(id, facets);
        }
    }

    /// Flattens one item-root origin tree into this map.
    pub(crate) fn insert_root(
        &mut self,
        item_index: impl Into<ItemIndex>,
        root: OriginNode,
    )
    {
        let item_index = item_index.into().0;
        let mut next_id = OriginNodeOrdinal(
            self.entries
                .keys()
                .next_back()
                .map_or(0, |id| id.raw().0.saturating_add(1)),
        );
        let item_component = match u32::try_from(item_index) {
            | Ok(component) => component,
            | Err(_) => u32::MAX,
        };
        let root_path = OriginPath(vec![item_component]);
        root.flatten_into(self, &mut next_id, &root_path);
    }
}

/// The shadow tree built during lowering: it mirrors the lowered term's
/// shape so stable origin IDs and compatibility paths can be assigned in one
/// flattening pass after the term is fully assembled.
#[derive(Clone, Debug)]
pub(crate) struct OriginNode
{
    /// This node's origin.
    entry: OriginEntry,
    /// The origins of the node's term children, in [`resolve`] order.
    children: Vec<Self>,
    /// Non-child source components owned by this semantic node.
    facets: Vec<OriginFacet>,
}

impl OriginNode
{
    /// Builds a shadow node from an entry and its term children.
    pub(crate) fn new(
        entry: OriginEntry,
        children: Vec<Self>,
    ) -> Self
    {
        Self {
            entry,
            children,
            facets: Vec::new(),
        }
    }

    /// Builds a childless shadow node.
    pub(crate) fn leaf(entry: OriginEntry) -> Self
    {
        Self::new(entry, Vec::new())
    }

    /// Rebinds this semantic root to the surface wrapper that denotes it,
    /// preserving the child origins that mirror the term's shape.
    pub(crate) fn with_root_entry(
        mut self,
        entry: OriginEntry,
    ) -> Self
    {
        self.entry = entry;
        self
    }

    /// Attaches one non-child semantic source component to this root.
    pub(crate) fn with_facet(
        mut self,
        facet: OriginFacet,
    ) -> Self
    {
        self.facets.push(facet);
        self
    }

    /// Whether this shadow tree already records a recovery over `span`: an
    /// entry carrying a [`HoleNote`] whose byte range overlaps or touches it.
    ///
    /// A recovery hole's entry is the only origin total-mode lowering leaves at
    /// a damaged region, so a caller holding a freshly lowered subtree reads
    /// this as *some consuming position reached that damage* — which is what
    /// [`crate::lower`] needs in order to tell an already-recovered region from
    /// one the walk never reached. The overlap-or-touch join is the one
    /// `Lowerer::responsible_obligation` uses, for the same reason: the
    /// melder's span conventions differ by obligation class.
    pub(crate) fn recovers(
        &self,
        span: &SourceRange,
    ) -> RecoveredSpanFlag
    {
        let mut pending = vec![self];
        while let Some(node) = pending.pop() {
            if node.entry.note.is_some()
                && node.entry.byte_range.0.start <= span.0.end
                && span.0.start <= node.entry.byte_range.0.end
            {
                return RecoveredSpanFlag(true);
            }
            pending.extend(node.children.iter());
        }
        RecoveredSpanFlag(false)
    }

    /// Flattens this shadow tree into `map`, assigning a fresh stable ID to
    /// each node while also recording the legacy compatibility `path`.
    fn flatten_into(
        self,
        map: &mut OriginMap,
        next_id: &mut OriginNodeOrdinal,
        path: &OriginPath,
    )
    {
        let mut pending = vec![(self, path.clone())];
        while let Some((node, node_path)) = pending.pop() {
            let id = OriginNodeId::new(*next_id);
            next_id.0 = next_id.0.saturating_add(1);
            map.insert(id, node_path.clone(), node.entry, node.facets);
            for (index, child) in node.children.into_iter().enumerate().rev() {
                // Most constructors have at most three children; a list
                // literal is n-ary (list-former design), so the index can be arbitrary.
                // The `u32::MAX` fallback keeps the traversal total if that
                // theoretical bound is ever exceeded.
                let component = u32::try_from(index).unwrap_or(u32::MAX);
                let mut child_path = node_path.clone();
                child_path.push(component);
                pending.push((child, child_path));
            }
        }
    }
}

/// A borrowed term node of either sort, as produced by [`resolve`].
#[derive(Clone, Copy, Debug)]
pub enum TermRef<'term>
{
    /// A value node.
    Value(
        /// The borrowed value.
        &'term Value,
    ),
    /// A computation node.
    Comp(
        /// The borrowed computation.
        &'term Comp,
    ),
}

/// Resolves a term-child path (the components *after* the item index)
/// against an item's term. Returns [`None`] when the path walks off the
/// term.
///
/// This function is the normative child-index order for [`OriginMap`] paths:
/// the value/computation children of each constructor in declaration order
/// (types and binder names are not children).
///
/// # Contract
/// - requires: `path` is a term-child path (the components after the item
///   index) for `term`.
/// - ensures: returns the borrowed `TermRef` the path addresses.
/// - provides: the normative `OriginMap` child-index order.
/// - fails: returns `None` when the path walks off the term.
/// - panics: none.
#[inline]
#[must_use]
pub fn resolve<'term, 'path, P>(
    term: &'term Term,
    path: P,
) -> Option<TermRef<'term>>
where
    P: Into<OriginPathRef<'path>>,
{
    let path = path.into();
    let start = match *term {
        | Term::Value(ref value) => TermRef::Value(value),
        | Term::Comp(ref comp) => TermRef::Comp(comp),
    };
    path.0
        .iter()
        .try_fold(start, |node, &component| step(node, component))
}

/// Steps from a term node to its `component`-th term child.
fn step(
    node: TermRef<'_>,
    component: impl Into<PathIndex>,
) -> Option<TermRef<'_>>
{
    let component = component.into();
    match node {
        | TermRef::Value(value) => step_value(value, component),
        | TermRef::Comp(comp) => step_comp(comp, component),
    }
}

/// Steps into a value's term children: `Pair` has two, `Inj`/`Annot` have
/// one value child, `Thunk` has one computation child, leaves have none.
fn step_value(
    value: &Value,
    component: impl Into<PathIndex>,
) -> Option<TermRef<'_>>
{
    let component = u32::from(component.into());
    match (value, component) {
        | (
            &(Value::Pair(ref child, _)
            | Value::Inj(_, ref child)
            | Value::Annot(ref child, _)
            // A reflexivity proof's witness is its single value child `0`
            // (identity-eliminator design).
            | Value::Here(ref child)
            // A packed module's payload is its single value child `0`; the
            // witness types are attributes, not children, exactly as an
            // ascription's type is.
            | Value::Pack { payload: ref child, .. }),
            0,
        )
        | (&Value::Pair(_, ref child), 1) => Some(TermRef::Value(child)),
        | (&Value::Thunk(_, ref child), 0) => Some(TermRef::Comp(child)),
        // A list literal's elements are its value children `0, 1, …` (list-former design).
        | (&Value::List(ref elements), index) => usize::try_from(index)
            .ok()
            .and_then(|element_index| elements.get(element_index))
            .map(|element| TermRef::Value(element)),
        // A record literal's field values are its value children `0, 1, …` in
        // canonical (sorted-label) order (record-former design), matching the order the
        // checker / machine / mark and the lowerer's origin assign.
        | (&Value::Record(ref fields), index) => usize::try_from(index)
            .ok()
            .and_then(|field_index| fields.values().nth(field_index))
            .map(|field| TermRef::Value(field)),
        | _ => None,
    }
}

/// Steps into a computation's term children, in constructor declaration
/// order: e.g. `App` is `0 = head, 1 = argument`; `Case` is `0 = scrutinee,
/// 1 = first-arm body, 2 = second-arm body`.
fn step_comp(
    comp: &Comp,
    component: impl Into<PathIndex>,
) -> Option<TermRef<'_>>
{
    let component = u32::from(component.into());
    match (comp, component) {
        // Computation children at index 0. `Reset`/`Shift` each carry a single
        // computation body; `Shift`'s continuation binder `k` is an attribute,
        // not a child (`deep edit descent`).
        | (
            &(Comp::Abs(_, _, ref child)
            | Comp::App(ref child, _)
            | Comp::Bind(ref child, ..)
            | Comp::With(ref child, _)
            | Comp::Prj(_, ref child)
            | Comp::Reset(ref child)
            | Comp::Shift(_, ref child)),
            0,
        )
        // Computation children at index 1. `Resume`'s computation argument is
        // its second child (its reified stack is the value child at 0).
        | (
            &(Comp::Bind(_, _, ref child)
            | Comp::Case(_, (_, ref child), _)
            | Comp::ListCase { nil: ref child, .. }
            | Comp::Split { body: ref child, .. }
            | Comp::With(_, ref child)
            | Comp::Resume(_, ref child)
            // The identity eliminator's base body is its computation child `1`
            // (its scrutinee is the value child `0`; the motive is a type —
            // an attribute, not a child; identity-eliminator design).
            | Comp::Walk { base: WalkBase { body: ref child, .. }, .. }
            // An unpack's body is its computation child `1` (its package value
            // is the value child `0`; the ascribed signature, the minted atoms
            // and the module binder are attributes, not children).
            | Comp::Unpack { body: ref child, .. }),
            1,
        )
        // A list-case's `nil` body is child 1, its `cons` body child 2 (the
        // `head`/`tail` binders are attributes, not children; list-former design D4).
        | (
            &(Comp::Case(_, _, (_, ref child)) | Comp::ListCase { cons: ref child, .. }),
            2,
        ) => Some(TermRef::Comp(child)),
        // A declared-data case's children: scrutinee (0) then one arm body per
        // constructor tag (1..), the order `lower::data` builds its origin
        // children in. Without this arm no origin path reaches inside a
        // `DataCase`, so every hole in one — a missing arm, an unfinished
        // pattern test — is invisible to the goal stream, to hole
        // localization, and to edit descent (`gandr-l1cw`). The payload
        // binders are attributes, not children.
        | (&Comp::DataCase(_, ref arms), index) if index > 0 => index
            .checked_sub(1)
            .and_then(|arm| usize::try_from(arm).ok())
            .and_then(|arm| arms.get(arm))
            .map(|arm| TermRef::Comp(&arm.1)),
        // A handler's computation children: scrutinee (0), return body (1), and
        // operation-clause bodies (2..), the order `edit::diff`/`edit::rebuild`
        // also use (`deep edit descent`). The signature and the clause binders are
        // attributes, not children.
        | (&Comp::Handle { ref scrutinee, .. }, 0) => Some(TermRef::Comp(scrutinee)),
        | (&Comp::Handle { ref ret, .. }, 1) => Some(TermRef::Comp(&ret.1)),
        | (&Comp::Handle { ref ops, .. }, index) => index
            .checked_sub(2)
            .and_then(|clause| usize::try_from(clause).ok())
            .and_then(|clause| ops.get(clause))
            .map(|clause| TermRef::Comp(&clause.body)),
        // Value children at index 0. `Dup`/`Drop` take a thunk value; `Perform`
        // takes its payload value (its signature and op name are attributes);
        // `Resume` takes its reified-stack value.
        | (
            &(Comp::Ret(ref child)
            | Comp::Force(ref child)
            | Comp::Case(ref child, ..)
            | Comp::ListCase { scrut: ref child, .. }
            // A declared-data case's scrutinee is its value child `0`; its arm
            // bodies are the computation children `1..` handled above.
            | Comp::DataCase(ref child, _)
            | Comp::Split { scrut: ref child, .. }
            | Comp::Dup(ref child)
            | Comp::Drop(ref child)
            | Comp::Perform(_, _, ref child)
            | Comp::Resume(ref child, _)
            // A record projection's record value is its single value child `0`
            // (record-former design D4; the `label` is an attribute, not a child).
            | Comp::RecordProj { record: ref child, .. }
            // The identity eliminator's scrutinee is its value child `0`
            // (identity-eliminator design).
            | Comp::Walk { scrut: ref child, .. }
            // An unpack's package value is its value child `0`.
            | Comp::Unpack { scrut: ref child, .. }),
            0,
        )
        | (&Comp::App(_, ref child), 1) => Some(TermRef::Value(child)),
        | _ => None,
    }
}
