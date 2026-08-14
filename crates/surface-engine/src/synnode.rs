//! The `SynNode` adapter: the melder CST (`gandr-surface-syntax`) presented
//! through the lowerer's named-node interface.
//!
//! `gandr_surface_syntax::Cst` is deliberately **form-name-free**: a node is
//! one of [`NodeKind::Cell`]/[`NodeKind::Meld`]/[`NodeKind::Wald`]/
//! [`NodeKind::Token`], a tile carries only a [`gandr_surface_syntax::MoldId`]
//! into the grammar's mold table, and grout carries a shape and a sort tag. The
//! lowerer ([`crate::lower`]), by contrast, is a rich named-AST consumer: it
//! dispatches on ~100 named kinds ([`crate::lower::node_kinds`]), recovers
//! grammar fields by name, and iterates *named* children.
//!
//! [`SynNode`] bridges the two. It is a borrowing view — a `&SynTree` paired
//! with an [`At`] locus, never an intermediate owned tree — and reconstructs
//! the named-AST surface on demand:
//!
//! - [`SynNode::kind`] classifies a Meld/Token by its **leading mold label**
//!   plus **tile shape** (and the Meld's own grammar sort) into the named kind
//!   the lowerer dispatches on. The melder preserves precedence/associativity
//!   in Meld nesting, so this recognizer never re-runs precedence.
//! - [`SynNode::child_by_field_name`] / [`SynNode::children_by_field_name`] do
//!   **positional tile recovery** within the recognized form.
//! - [`SynNode::named_children`] iterates the form's significant sub-nodes,
//!   grout-unwrapped and space-skipped — and, for the flat forms the melder
//!   does not group (blocks, `case`/`co`/record fields, parameters, arguments,
//!   `extern` members), **synthesizes** the intermediate named nodes the
//!   lowerer navigates ([`At::Run`]). The block statement segmentation (split
//!   on top-level `;`) is the highest-churn case.
//!
//! This module owns no grammar surface and no highlight logic; it is a pure
//! read adapter over the parser's committed tree.

use alloc::vec::Vec;
use std::sync::OnceLock;

use gandr_surface_grammar::Pbg;
use gandr_surface_grammar::Sort;
use gandr_surface_grammar::built_in;
use gandr_surface_parser::ObligationInstance;
use gandr_surface_parser::parse;
use gandr_surface_syntax::Cst;
use gandr_surface_syntax::Diff;
use gandr_surface_syntax::Material;
use gandr_surface_syntax::MoldPayload;
use gandr_surface_syntax::NodeId;
use gandr_surface_syntax::NodeKind;
use gandr_surface_syntax::NodeView;
use gandr_surface_syntax::SourceSlice;
use gandr_surface_syntax::StableHash;
use gandr_surface_syntax::diff;

use crate::boundary::CopatternBodyFlag;
use crate::boundary::ErrorPresence;
use crate::boundary::GroutLeafFlag;
use crate::boundary::GroutPresence;
use crate::boundary::HostEscapeFlag;
use crate::boundary::LowerIdentifierFlag;
use crate::boundary::MissingPresence;
use crate::boundary::NamedNodeFlag;
use crate::boundary::NamedTerminalFlag;
use crate::boundary::NumericSuffixFlag;
use crate::boundary::ReservedObservationFlag;
use crate::boundary::ShellRedirectionFlag;
use crate::boundary::SignificantIndex;
use crate::boundary::SourceOffset;
use crate::boundary::SourceRange;
use crate::boundary::StringRunIndex;
use crate::boundary::SyntaxField;
use crate::boundary::SyntaxKind;
use crate::boundary::TilePresence;
use crate::boundary::TileSpelling;
use crate::lower::LowerError;
use crate::lower::LowerResult;
use crate::lower::node_kinds;

/// Melder tile labels the recognizer dispatches on (raw tile spellings, not
/// named kinds — the named kinds live in [`node_kinds`]).
mod label
{
    use super::TileSpelling;
    /// Item-family lead tile.
    pub const DEF: TileSpelling = TileSpelling("def");
    /// Recursive-def tile (`def rec …`; W4d fold, recursive-codata design).
    pub const REC: TileSpelling = TileSpelling("rec");
    /// `codata` datatype-declaration lead tile (PBG surface; codata design).
    pub const CODATA: TileSpelling = TileSpelling("codata");
    /// Reserved 2-cell / rewrite-rule member lead tile (`rule lhs ==> rhs`).
    pub const RULE: TileSpelling = TileSpelling("rule");
    /// `extern` block lead tile.
    pub const EXTERN: TileSpelling = TileSpelling("extern");
    /// `data` datatype-declaration lead tile (PBG surface; declared-data
    /// design).
    pub const DATA: TileSpelling = TileSpelling("data");
    /// `sign` signature-block lead tile (the ruled circuit block form).
    pub const SIGN: TileSpelling = TileSpelling("sign");
    /// Circuit 1-cell member / top-level circuit declaration lead tile.
    pub const OPER: TileSpelling = TileSpelling("oper");
    /// `module` declaration lead tile (the checked-module surface).
    pub const MODULE: TileSpelling = TileSpelling("module");
    /// `import` declaration lead tile.
    pub const IMPORT: TileSpelling = TileSpelling("import");
    /// Attribute-block opener.
    pub const AT_BRACKET: TileSpelling = TileSpelling("@[");
    /// Attribute-block closer.
    pub const CLOSE_BRACKET: TileSpelling = TileSpelling("]");
    /// Value-statement lead tile.
    pub const VAL: TileSpelling = TileSpelling("val");
    /// Computation-binding statement lead tile.
    pub const RUN: TileSpelling = TileSpelling("run");
    /// Value/field binder tile.
    pub const EQUALS: TileSpelling = TileSpelling("=");
    /// Transparent signature ascription tile.
    pub const COLON: TileSpelling = TileSpelling(":");
    /// Opaque (sealing) signature ascription tile.
    pub const COLON_ANGLE: TileSpelling = TileSpelling(":>");
    /// Import-alias separator tile.
    pub const AS: TileSpelling = TileSpelling("as");
    /// Statement / item terminator.
    pub const SEMI: TileSpelling = TileSpelling(";");
    /// Comma separator.
    pub const COMMA: TileSpelling = TileSpelling(",");
    /// Bind-statement arrow.
    pub const LEFT_ARROW: TileSpelling = TileSpelling("<-");
    /// Case-arm arrow.
    pub const FAT_ARROW: TileSpelling = TileSpelling("=>");
    /// Projection dot.
    pub const DOT: TileSpelling = TileSpelling(".");
    /// Record-update pipe.
    pub const PIPE: TileSpelling = TileSpelling("|");
    /// Function-type / return arrow.
    pub const RIGHT_ARROW: TileSpelling = TileSpelling("->");
    /// `else` keyword.
    pub const ELSE: TileSpelling = TileSpelling("else");
    /// Grouping / call / tuple open paren.
    pub const LPAREN: TileSpelling = TileSpelling("(");
    /// Grouping close paren.
    pub const RPAREN: TileSpelling = TileSpelling(")");
    /// Block / body open brace.
    pub const LBRACE: TileSpelling = TileSpelling("{");
    /// Block / body close brace.
    pub const RBRACE: TileSpelling = TileSpelling("}");
    /// List open bracket.
    pub const LBRACKET: TileSpelling = TileSpelling("[");
    /// Record opener.
    pub const HASH_BRACE: TileSpelling = TileSpelling("#{");
    /// Shell-block opener.
    pub const SHELL_OPEN: TileSpelling = TileSpelling("#!{");
    /// Command-substitution opener.
    pub const COMMAND_SUBSTITUTION_START: TileSpelling = TileSpelling("command_substitution_start");
    /// Case lead tile.
    pub const CASE: TileSpelling = TileSpelling("case");
    /// If lead tile.
    pub const IF: TileSpelling = TileSpelling("if");
    /// Lambda lead tile.
    pub const FN: TileSpelling = TileSpelling("fn");
    /// Thunk lead tile.
    pub const THUNK: TileSpelling = TileSpelling("thunk");
    /// Force lead tile.
    pub const FORCE: TileSpelling = TileSpelling("force");
    /// Returner lead tile.
    pub const RET: TileSpelling = TileSpelling("ret");
    /// Lazy-pair lead tile.
    pub const CO: TileSpelling = TileSpelling("co");
    /// User-hole lead tile.
    pub const HOLE: TileSpelling = TileSpelling("?");
    /// String-literal delimiter tile.
    pub const DQUOTE: TileSpelling = TileSpelling("\"");
    /// Unary negation tile.
    pub const NEG: TileSpelling = TileSpelling("-");
    /// Returner type constructor.
    pub const F: TileSpelling = TileSpelling("F");
    /// Graded-thunk type constructor.
    pub const U: TileSpelling = TileSpelling("U");
    /// Product type operator.
    pub const STAR: TileSpelling = TileSpelling("*");
    /// Sum type operator.
    pub const PLUS: TileSpelling = TileSpelling("+");
    /// Lazy-product type operator.
    pub const AMP: TileSpelling = TileSpelling("&");
    /// The user-type identifier tile label.
    pub const TYPE_IDENTIFIER: TileSpelling = TileSpelling("type_identifier");
    /// The term identifier tile label.
    pub const IDENTIFIER: TileSpelling = TileSpelling("identifier");
    /// A numeric literal tile label.
    pub const NUMBER: TileSpelling = TileSpelling("number");
    /// A type-suffixed numeric literal tile label.
    pub const TYPED_NUMBER: TileSpelling = TileSpelling("typed_number");
    /// The constructor tile label.
    pub const CONSTRUCTOR: TileSpelling = TileSpelling("constructor");
    /// The boolean-true tile label.
    pub const TRUE: TileSpelling = TileSpelling("true");
    /// The boolean-false tile label.
    pub const FALSE: TileSpelling = TileSpelling("false");
    /// The wildcard pattern tile label.
    pub const WILDCARD: TileSpelling = TileSpelling("_");
    /// The unrestricted-grade tile label.
    pub const OMEGA: TileSpelling = TileSpelling("ω");
    /// The type-variable tile label.
    pub const TYPE_VARIABLE: TileSpelling = TileSpelling("type_variable");
    /// The `type` member keyword inside an `extern` block.
    pub const TYPE: TileSpelling = TileSpelling("type");
    /// Recursive-let statement lead (out of fragment).
    pub const LETA: TileSpelling = TileSpelling("leta");
    /// Session receive statement lead (out of fragment).
    pub const RECV: TileSpelling = TileSpelling("recv");
    /// Session acquire statement lead (out of fragment).
    pub const ACQUIRE: TileSpelling = TileSpelling("acquire");
    /// Session release statement lead (out of fragment).
    pub const RELEASE: TileSpelling = TileSpelling("release");
    /// Fork statement lead (out of fragment).
    pub const FORK: TileSpelling = TileSpelling("fork");
    /// String-literal single-quote delimiter (shell single-quoted strings).
    pub const SQUOTE: TileSpelling = TileSpelling("'");
    /// Grade-annotation open bracket (`thunk[r]` / `U[r]`).
    pub const LBRACKET_GRADE: TileSpelling = TileSpelling("[");
    /// Grade-annotation close bracket.
    pub const RBRACKET_GRADE: TileSpelling = TileSpelling("]");
    /// Shell pipeline operator.
    pub const SHELL_PIPE: TileSpelling = TileSpelling("|");
    /// Shell `&&` control operator.
    pub const SHELL_AND: TileSpelling = TileSpelling("&&");
    /// Shell `||` control operator.
    pub const SHELL_OR: TileSpelling = TileSpelling("||");
    /// Shell parameter / host-escape lead (`$name`, `${name}`, `$( … )`).
    pub const DOLLAR: TileSpelling = TileSpelling("$");
    /// Shell command negation (`! cmd`), tokenized as a bare shell word.
    pub const BANG: TileSpelling = TileSpelling("!");
    /// The bare-word command-token tile label inside a shell block (the single
    /// `shell_word` atom class; the W4e perf reshape folded the former
    /// `command_name` tile onto it, while the lowerer-facing KIND stays
    /// `command_name`).
    pub const COMMAND_NAME: TileSpelling = TileSpelling("shell_word");
    /// A command-local environment assignment `NAME=value`, molded as one tile
    /// from the labeler's whole-token munch.
    pub const ENVIRONMENT_ASSIGNMENT: TileSpelling = TileSpelling("environment_assignment");
}

/// Borrowed significant-child sequence consumed by recognizer helpers.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct SignificantChildren<'tree>(&'tree [NodeId]);

impl<'tree> From<&'tree [NodeId]> for SignificantChildren<'tree>
{
    #[inline]
    fn from(value: &'tree [NodeId]) -> Self
    {
        Self(value)
    }
}

impl<'tree> From<&'tree Vec<NodeId>> for SignificantChildren<'tree>
{
    #[inline]
    fn from(value: &'tree Vec<NodeId>) -> Self
    {
        Self(value.as_slice())
    }
}

impl core::ops::Deref for SignificantChildren<'_>
{
    type Target = [NodeId];

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        self.0
    }
}

/// Container and half-open significant-child window addressed by a view.
#[derive(Clone, Copy)]
struct SignificantWindow
{
    /// Node whose significant children form the window.
    container: NodeId,
    /// First significant-child index in the window.
    start: SignificantIndex,
    /// One past the last significant-child index in the window.
    limit: SignificantIndex,
}

/// The synthetic grade-annotation kind (the `[ … ]`-wrapped `number`/`ω` a
/// `u_type` / `thunk_expression` carries). Not a lowerer dispatch kind — the
/// grade reader ([`crate::lower::types::parse_grade`]) only reads its single
/// child token and its text.
const KIND_GRADE: SyntaxKind = SyntaxKind("grade");
/// Primitive type spellings the recognizer accepts in `u_type` positions.
const PRIMITIVE_TYPES: [&str; 16] = [
    "Any", "Unknown", "Never", "Boolean", "Integer", "u32", "u64", "i32", "i64", "f32", "f64",
    "Char", "String", "Symbol", "Unit", "Void",
];

/// The binary operator tile labels the recognizer classifies as
/// `binary_expression` (mirrors the grammar's `BINARY_OPERATORS`).
const BINARY_OPS: [&str; 12] = [
    "||", "&&", "==", "!=", "<=", ">=", "++", "<", ">", "+", "-", "*",
];

/// The synthetic argument-list kind (a call/type-application/constructor's
/// bracketed argument group). Not a lowerer dispatch kind — the lowerer only
/// iterates its named children.
const KIND_ARGUMENTS: SyntaxKind = SyntaxKind("arguments");
/// The synthetic parameter-list kind (a lambda/cases binder group). Not a
/// lowerer dispatch kind — the lowerer only iterates its named children.
const KIND_PARAMETERS: SyntaxKind = SyntaxKind("parameters");

/// The process-wide built-in grammar, checked and cached on first use.
fn grammar() -> &'static Pbg
{
    /// The process-wide cached grammar.
    static GRAMMAR: OnceLock<Pbg> = OnceLock::new();
    GRAMMAR.get_or_init(|| {
        #[expect(
            clippy::expect_used,
            reason = "the built-in grammar is a compile-pinned checked artifact (checked-PBG front-end design); a build failure is caught by gandr-surface-grammar's own contract tests, never at pipeline runtime"
        )]
        built_in().expect("the built-in grammar is checked")
    })
}

/// An owned parse result the [`SynNode`] views borrow from: the committed CST
/// plus the parse's severity-ordered obligations (the `obligation surface`
/// surface the total-mode hole enrichment consumes).
pub struct SynTree
{
    /// The committed concrete syntax tree.
    cst: Cst,
    /// The parse's obligations, severity-ordered (highest first).
    obligations: Vec<ObligationInstance>,
}

impl SynTree
{
    /// Parse `source` over the built-in grammar into a borrowable tree.
    ///
    /// # Contract
    /// - requires: `source` is any UTF-8 gandr source.
    /// - ensures: returns a [`SynTree`] whose [`SynTree::root`] is the file
    ///   root; the parse is total (any input yields a tree).
    /// - provides: the parse front-end the lowerer's CST walk consumes.
    /// - fails: [`LowerError::ParseFailed`] only for an arena-construction
    ///   failure at commit — never for ungrammatical input.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`LowerError::ParseFailed`] when the flat arena cannot be
    /// assembled at commit.
    #[inline]
    pub fn parse<'source, S>(source: S) -> LowerResult<Self>
    where
        S: Into<SourceSlice<'source>>,
    {
        let result = parse(grammar(), source.into()).map_err(|_error| LowerError::ParseFailed)?;
        let obligations = result.obligations().to_vec();
        Ok(Self {
            cst: result.into_cst(),
            obligations,
        })
    }

    /// The parse's obligations, severity-ordered (highest first).
    #[inline]
    #[must_use]
    pub fn obligations(&self) -> &[ObligationInstance]
    {
        &self.obligations
    }

    /// The committed concrete syntax tree this tree's views borrow from.
    ///
    /// The structural-diff seam ([`gandr_surface_syntax::diff`],
    /// `spec:implementation/incremental-pipeline.md` §"The
    /// structural diff") consumes two trees'
    /// [`Cst`]s directly; the merkle hashes it aligns on
    /// are the same the origin map records
    /// ([`crate::origin::OriginEntry::cst_hash`]).
    #[inline]
    #[must_use]
    pub fn cst(&self) -> &Cst
    {
        &self.cst
    }

    /// The structural CST diff against a re-parse
    /// (`spec:implementation/incremental-pipeline.md` §"The
    /// structural diff"): [`gandr_surface_syntax::diff`] over the two
    /// committed trees. Merkle-hash pruning matches every subtree whose
    /// significant content is unchanged, so an edit confined to one item leaves
    /// every *other* item's root in [`gandr_surface_syntax::Diff::matches`].
    ///
    /// # Contract
    /// - requires: `self` and `new` are parses over the same built-in grammar
    ///   (always true — [`SynTree::parse`] pins the checked-PBG front-end
    ///   design fingerprint).
    /// - ensures: returns the deterministic top-down diff; equal-`(kind,
    ///   payload, hash)` subtree roots are matched and pruned, differing
    ///   interiors are aligned by LCS over the same key.
    /// - provides: the §"The structural diff" AST-diff seam the incremental
    ///   pipeline consumes.
    /// - fails: never; malformed or unreadable nodes surface as unmatched
    ///   roots, not errors.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — an unchanged re-parse (every item root matched), a
    ///   single-item body edit (that item's root unmatched, its siblings still
    ///   matched), and an inserted item distinguish hash pruning from
    ///   re-alignment.
    /// - witness:
    ///   `origin_identity::tests::editing_one_item_leaves_every_other_matched`
    #[inline]
    #[must_use]
    pub fn diff(
        &self,
        new: &Self,
    ) -> Diff
    {
        diff(&self.cst, &new.cst)
    }

    /// The file-root node.
    #[inline]
    #[must_use]
    pub fn root(&self) -> SynNode<'_>
    {
        SynNode {
            tree: self,
            at: At::Node(self.cst.root()),
        }
    }

    /// The children of `id` with insignificant layout (space) removed, in
    /// source order — the "significant children" the recognizer scans.
    fn sig_children(
        &self,
        id: NodeId,
    ) -> Vec<NodeId>
    {
        let Ok(children) = self.cst.children(id)
        else {
            return Vec::new();
        };
        children
            .iter()
            .copied()
            .filter(|&child| !matches!(self.material(child), Some(Material::Space)))
            .collect()
    }

    /// The material class of `id`, or `None` when the node is out of range.
    fn material(
        &self,
        id: NodeId,
    ) -> Option<Material>
    {
        self.cst.node(id).ok().map(NodeView::material)
    }

    /// The `NodeKind` of `id`, or `None` when the node is out of range.
    fn node_kind(
        &self,
        id: NodeId,
    ) -> Option<NodeKind>
    {
        self.cst.node(id).ok().map(NodeView::kind)
    }

    /// The mold label of `id` when it is a tile, else `None` (grout and space
    /// carry no mold label).
    fn tile_label(
        &self,
        id: NodeId,
    ) -> Option<TileSpelling>
    {
        match {
            let present = self.cst.node(id).ok()?;
            core::convert::identity(present)
        }
        .payload()
        {
            | MoldPayload::Tile(mold) => {
                grammar().mold(mold).ok().map(|def| TileSpelling(def.label))
            },
            | MoldPayload::Grout { .. } | MoldPayload::Space => None,
        }
    }

    /// The grammar sort of `id`: a Meld/Wald carries its sort on its grout
    /// payload; a tile carries it on its mold.
    fn sort_of(
        &self,
        id: NodeId,
    ) -> Option<Sort>
    {
        match {
            let present = self.cst.node(id).ok()?;
            core::convert::identity(present)
        }
        .payload()
        {
            | MoldPayload::Tile(mold) => grammar().mold(mold).ok().map(|def| def.sort),
            | MoldPayload::Grout { sort, .. } => Sort::try_from_tag(sort).ok(),
            | MoldPayload::Space => None,
        }
    }

    /// The `[start, end)` byte range of `id`.
    fn range(
        &self,
        id: NodeId,
    ) -> SourceRange
    {
        SourceRange(self.cst.node(id).map_or(0 .. 0, |view| {
            let range = view.range();
            let start = usize::try_from(u32::from(range.start())).unwrap_or(0);
            let end = usize::try_from(u32::from(range.end())).unwrap_or(start);
            start .. end
        }))
    }

    /// The per-node merkle hash of `id` — a content fingerprint over `id`'s
    /// significant subtree ([`gandr_surface_syntax::Cst::hash`]). `0` when `id`
    /// is out of range, which is unreachable for a node this tree minted.
    fn hash(
        &self,
        id: NodeId,
    ) -> StableHash
    {
        self.cst.hash(id).unwrap_or(StableHash(0))
    }
}

/// The locus a [`SynNode`] presents: a real CST node, a real node forced to a
/// specific named kind, or a synthetic run of a container's significant
/// children (a statement, arm, field, parameter list, argument list, or virtual
/// block the melder does not group into a node of its own).
#[derive(Clone, Copy)]
enum At
{
    /// A real CST node, classified structurally.
    Node(NodeId),
    /// A real node presented with a fixed named kind (a bare `true`/`false`
    /// tile as the `boolean`'s child token).
    Forced
    {
        /// The backing node.
        id: NodeId,
        /// The forced named kind.
        kind: SyntaxKind,
    },
    /// A synthetic run `[lo, hi)` of `container`'s significant children,
    /// presented as a node of `kind`.
    Run
    {
        /// The container whose significant children the run indexes.
        container: NodeId,
        /// Inclusive start index into the container's significant children.
        lo: usize,
        /// Exclusive end index.
        hi: usize,
        /// The synthetic node's named kind.
        kind: SyntaxKind,
    },
}

/// A borrowing view over one node of a [`SynTree`], presenting the lowerer's
/// tree-sitter-shaped interface. `Copy`: it is `(&SynTree, At)`, never an owned
/// subtree.
#[derive(Clone, Copy)]
pub struct SynNode<'tree>
{
    /// The tree this view borrows from.
    tree: &'tree SynTree,
    /// The locus within the tree.
    at: At,
}

impl<'tree> SynNode<'tree>
{
    /// Wrap a real node.
    fn wrap(
        tree: &'tree SynTree,
        id: NodeId,
    ) -> Self
    {
        Self {
            tree,
            at: At::Node(id),
        }
    }

    /// The named kind this node presents to the lowerer's dispatch.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the [`node_kinds`] string the lowerer dispatches on,
    ///   classifying a Meld/Token by leading mold label plus tile shape and the
    ///   Meld's own grammar sort; a synthetic run returns its recorded kind.
    /// - provides: the recognizer entry (the lowerer's `.kind()`).
    /// - fails: never; an unrecognized node returns a stable non-dispatched
    ///   empty sentinel the lowerer treats as out-of-fragment.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn kind(self) -> SyntaxKind
    {
        match self.at {
            | At::Forced { kind, .. } | At::Run { kind, .. } => kind,
            | At::Node(id) => self.classify(id),
        }
    }

    /// The `[start, end)` byte range this node covers.
    #[inline]
    #[must_use]
    pub fn byte_range(self) -> SourceRange
    {
        match self.at {
            | At::Node(id) | At::Forced { id, .. } => self.tree.range(id),
            | At::Run {
                container, lo, hi, ..
            } => self.run_range(container, SignificantIndex(lo), SignificantIndex(hi)),
        }
    }

    /// The start byte of [`Self::byte_range`].
    #[inline]
    #[must_use]
    pub fn start_byte(self) -> SourceOffset
    {
        SourceOffset(self.byte_range().start)
    }

    /// The end byte of [`Self::byte_range`].
    #[inline]
    #[must_use]
    pub fn end_byte(self) -> SourceOffset
    {
        SourceOffset(self.byte_range().end)
    }

    /// The backing CST arena node this view's origin identity keys on: a real
    /// node's own [`NodeId`], or — for a synthetic run the melder does not
    /// group into a node of its own — its first significant child (falling
    /// back to the container when the run is empty).
    #[inline]
    #[must_use]
    fn backing_node(self) -> NodeId
    {
        match self.at {
            | At::Node(id) | At::Forced { id, .. } => id,
            | At::Run { container, lo, .. } => self
                .tree
                .sig_children(container)
                .get(lo)
                .copied()
                .unwrap_or(container),
        }
    }

    /// This node's stable CST identity for the origin map
    /// ([`gandr_surface_syntax::NodeId`]): a real node's dense arena slot, or a
    /// run's first significant child. The `NodeId`-typed identity that
    /// superseded the freed tree-sitter subtree address
    /// — positional within one parse (the substrate the structural diff
    /// aligns on), paired with the reproducible [`Self::cst_hash`] for
    /// provenance.
    #[inline]
    #[must_use]
    pub fn cst_node(self) -> NodeId
    {
        self.backing_node()
    }

    /// This node's per-node merkle hash ([`gandr_surface_syntax::Cst::hash`] of
    /// [`Self::cst_node`]): a content fingerprint over the node's significant
    /// structure, reproducible across runs and processes.
    #[inline]
    #[must_use]
    pub fn cst_hash(self) -> StableHash
    {
        self.tree.hash(self.backing_node())
    }

    /// The source text this node covers.
    #[inline]
    #[must_use]
    pub fn text(self) -> SourceSlice<'tree>
    {
        self.tree
            .cst
            .source()
            .as_ref()
            .get(self.byte_range().0)
            .map_or_else(|| SourceSlice::from(""), SourceSlice::from)
    }

    /// The significant children of this node's backing container,
    /// space-skipped.
    fn sig(self) -> Vec<NodeId>
    {
        match self.at {
            | At::Node(id) | At::Forced { id, .. } => self.tree.sig_children(id),
            | At::Run {
                container, lo, hi, ..
            } => self
                .tree
                .sig_children(container)
                .get(lo .. hi)
                .map(<[NodeId]>::to_vec)
                .unwrap_or_default(),
        }
    }

    /// The named (non-space, non-grout, non-punctuation) children of this node,
    /// with the flat forms the melder does not group synthesized into their
    /// intermediate named nodes.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the ordered named sub-nodes the lowerer iterates —
    ///   items under the file root; `;`-delimited statements (and the tail)
    ///   under a block; `,`-delimited arms/fields under `case`/`co`/records;
    ///   parameters, arguments, and members under their lists; and the sole
    ///   inner expression under a wrapper.
    /// - provides: the lowerer's `named_children` (minus extras) surface.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn named_children(self) -> Vec<Self>
    {
        match self.kind() {
            | node_kinds::SOURCE_FILE => self.items(),
            | node_kinds::BLOCK => self.block_statements(),
            | node_kinds::CASE_EXPRESSION => {
                self.comma_segments(self.brace_body(), node_kinds::ARM)
            },
            | node_kinds::CO_EXPRESSION => {
                self.comma_segments(self.brace_body(), node_kinds::CO_FIELD)
            },
            | node_kinds::RECORD_EXPRESSION | node_kinds::RECORD_UPDATE_EXPRESSION => {
                self.comma_segments(self.hash_brace_body(), node_kinds::RECORD_FIELD)
            },
            | node_kinds::RECORD_TYPE => {
                self.comma_segments(self.hash_brace_body(), node_kinds::RECORD_TYPE_FIELD)
            },
            | node_kinds::EXTERN_BLOCK => self.extern_members(),
            | node_kinds::CODATA_DECLARATION => {
                self.member_segments(self.brace_body(), node_kinds::CODATA_OBSERVATION)
            },
            | node_kinds::MODULE_DECLARATION => self.module_members(),
            | node_kinds::REC_BLOCK => self.rec_members(),
            | node_kinds::ATTRIBUTE_BLOCK => self.comma_segments(
                self.body_between(label::AT_BRACKET, label::CLOSE_BRACKET),
                node_kinds::ATTRIBUTE,
            ),
            | node_kinds::SHELL_BLOCK => self.shell_commands(),
            | node_kinds::COMMAND => self.command_parts(),
            | KIND_PARAMETERS => self.comma_segments(self.paren_body(), node_kinds::PARAMETER),
            | _ => self.plain_named_children(),
        }
    }

    /// The children matching a grammar field name, in source order.
    ///
    /// # Contract
    /// - requires: `field` names a repeated field the lowerer requests.
    /// - ensures: returns the positional tiles the field recovers (the
    ///   `argument` list of a call/type-application/constructor pattern, the
    ///   `member` list of an n-ary product/sum/lazy-product type, the
    ///   `attribute` blocks of an item), in order.
    /// - provides: the lowerer's `children_by_field_name` surface.
    /// - fails: never; an unknown field yields an empty vector.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn children_by_field_name<F>(
        self,
        field: F,
    ) -> Vec<Self>
    where
        F: Into<SyntaxField>,
    {
        match (self.kind(), field.into()) {
            | (node_kinds::INSTANTIATION_EXPRESSION, node_kinds::FIELD_INSTANTIATION) => {
                self.comma_segments(self.bracket_body(), node_kinds::INSTANTIATION_RESIDENT)
            },
            | (node_kinds::CALL_EXPRESSION, node_kinds::FIELD_ARGUMENT) => self
                .child_by_field_name(node_kinds::FIELD_ARGUMENTS)
                .map(Self::named_children)
                .unwrap_or_default(),
            | (
                node_kinds::TYPE_APPLICATION | node_kinds::CONSTRUCTOR_PATTERN,
                node_kinds::FIELD_ARGUMENT,
            ) => self
                .paren_group(KIND_ARGUMENTS)
                .map(Self::named_children)
                .unwrap_or_default(),
            | (
                node_kinds::PRODUCT_TYPE | node_kinds::SUM_TYPE | node_kinds::LAZY_PRODUCT_TYPE,
                node_kinds::FIELD_MEMBER,
            ) => self.type_members(),
            | (node_kinds::MODULE_DECLARATION, node_kinds::FIELD_MEMBER) => self.module_members(),
            | (node_kinds::REC_BLOCK, node_kinds::FIELD_MEMBER) => self.rec_members(),
            | (_, node_kinds::FIELD_ATTRIBUTE) => self.attribute_blocks(),
            // The copattern-clause list of a `def rec … { .π => e, … }` body
            // (codata design §5.1): the melder keeps the clauses flat, like a `case`'s
            // arms, so they segment on top-level `,` (a nested `,` inside a
            // clause body's call/tuple hides inside its own child meld).
            | (node_kinds::DEF_REC, node_kinds::FIELD_CLAUSE) => {
                self.comma_segments(self.brace_body(), node_kinds::COPATTERN_CLAUSE)
            },
            | _ => Vec::new(),
        }
    }

    /// The single child matching a grammar field name, if present.
    ///
    /// # Contract
    /// - requires: `field` names a field the lowerer requests on this form.
    /// - ensures: returns the positional tile the field recovers within the
    ///   recognized form — the def name / value, the binder pattern and its
    ///   right-hand side, a binary expression's operands, a projection's target
    ///   and field, an `if`'s condition and branches, and so on.
    /// - provides: the lowerer's `child_by_field_name` surface.
    /// - fails: never; an absent field yields `None`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn child_by_field_name<F>(
        self,
        field: F,
    ) -> Option<Self>
    where
        F: Into<SyntaxField>,
    {
        // Arms are grouped by identical body (the strict `match_same_arms`
        // wall): a `def`/`extern` name and a `let`/`hole` binder both recover
        // "the tile after a lead keyword", so they share an arm.
        match (self.kind(), field.into()) {
            // "the named child after the lead keyword".
            | (
                node_kinds::DEF_VALUE
                | node_kinds::DEF_SIGNATURE
                | node_kinds::DEF_FUNCTION
                | node_kinds::EXTERN_FUNCTION,
                node_kinds::FIELD_NAME,
            ) => self.after_lead(label::DEF),
            | (node_kinds::EXTERN_TYPE, node_kinds::FIELD_NAME) => self.after_lead(label::TYPE),
            | (node_kinds::CODATA_DECLARATION, node_kinds::FIELD_NAME) => {
                self.after_lead(label::CODATA)
            },
            | (node_kinds::MODULE_DECLARATION, node_kinds::FIELD_NAME) => {
                self.after_lead(label::MODULE)
            },
            // A `def rec` name is the ident after the `rec` tile (not after
            // `def`, which yields `rec`); its `->` result and paren params reuse
            // the shared separator recoveries below.
            | (node_kinds::DEF_REC, node_kinds::FIELD_NAME) => self.after(label::REC),
            | (node_kinds::LET_STATEMENT, node_kinds::FIELD_PATTERN) => self.after_lead(label::VAL),
            | (node_kinds::BIND_STATEMENT, node_kinds::FIELD_PATTERN) => {
                self.after_lead(label::RUN)
            },
            | (node_kinds::UNARY_EXPRESSION, node_kinds::FIELD_OPERAND) => {
                self.after_lead(label::NEG)
            },
            // The returner / force operand: the sole named child after the lead
            // keyword (`ret e` / `force e`).
            | (node_kinds::RET_EXPRESSION, node_kinds::FIELD_VALUE) => self.after_lead(label::RET),
            | (node_kinds::FORCE_EXPRESSION, node_kinds::FIELD_VALUE) => {
                self.after_lead(label::FORCE)
            },
            | (node_kinds::HOLE, node_kinds::FIELD_NAME) => self.after_lead(label::HOLE),
            // Parameter groups.
            | (
                node_kinds::DEF_FUNCTION
                | node_kinds::LAMBDA_EXPRESSION
                | node_kinds::EXTERN_FUNCTION
                | node_kinds::DEF_REC,
                node_kinds::FIELD_PARAMETERS,
            ) => self.paren_group(KIND_PARAMETERS),
            | (node_kinds::CALL_EXPRESSION, node_kinds::FIELD_ARGUMENTS) => {
                self.paren_group(KIND_ARGUMENTS)
            },
            | (
                node_kinds::DEF_FUNCTION
                | node_kinds::DEF_REC
                | node_kinds::LAMBDA_EXPRESSION
                | node_kinds::THUNK_EXPRESSION,
                node_kinds::FIELD_BODY,
            ) => self.brace_block(),
            | (node_kinds::IF_EXPRESSION, node_kinds::FIELD_CONSEQUENCE) => {
                self.nth_brace_block(SignificantIndex(0))
            },
            | (node_kinds::IF_EXPRESSION, node_kinds::FIELD_ALTERNATIVE) => self.if_alternative(),
            // "the named child after a separator tile".
            | (
                node_kinds::DEF_FUNCTION
                | node_kinds::FUNCTION_TYPE
                | node_kinds::EXTERN_FUNCTION
                | node_kinds::DEF_REC,
                node_kinds::FIELD_RESULT,
            ) => self.after(label::RIGHT_ARROW),
            // An observation's result type is the type after its (first,
            // top-level) `:`; the body of a copattern clause is the expression
            // after its `=>`, and its projected observation the ident after `.`
            // (absent for the reserved `_` default arm).
            | (
                node_kinds::PARAMETER
                | node_kinds::RECORD_TYPE_FIELD
                | node_kinds::CODATA_OBSERVATION,
                node_kinds::FIELD_TYPE,
            ) => self.after(label::COLON),
            | (
                node_kinds::LET_STATEMENT | node_kinds::CO_FIELD | node_kinds::RECORD_FIELD,
                node_kinds::FIELD_VALUE,
            ) => self.after(label::EQUALS),
            | (node_kinds::BIND_STATEMENT, node_kinds::FIELD_SOURCE) => {
                self.after(label::LEFT_ARROW)
            },
            // A projection's field and a copattern clause's observation are both
            // the ident after a `.`; a `case` arm's and a copattern clause's
            // body are both the expression after a `=>`.
            | (node_kinds::PROJECTION_EXPRESSION, node_kinds::FIELD_FIELD)
            | (node_kinds::COPATTERN_CLAUSE, node_kinds::FIELD_OBSERVATION) => {
                self.after(label::DOT)
            },
            | (node_kinds::ARM | node_kinds::COPATTERN_CLAUSE, node_kinds::FIELD_BODY) => {
                self.after(label::FAT_ARROW)
            },
            // "the named child before a separator tile".
            | (node_kinds::PROJECTION_EXPRESSION, node_kinds::FIELD_VALUE) => {
                self.before(label::DOT)
            },
            | (node_kinds::FUNCTION_TYPE, node_kinds::FIELD_PARAMETER) => {
                self.before(label::RIGHT_ARROW)
            },
            | (node_kinds::ARM, node_kinds::FIELD_PATTERN) => self.before(label::FAT_ARROW),
            // "the first named child".
            | (
                node_kinds::PARAMETER
                | node_kinds::CO_FIELD
                | node_kinds::RECORD_FIELD
                | node_kinds::RECORD_TYPE_FIELD
                | node_kinds::ATTRIBUTE,
                node_kinds::FIELD_NAME,
            ) => self.first_named(),
            // An observation's name is its first `identifier` tile — past any
            // reserved leading grade prefix (`1 step: …`), whose `number` / `ω`
            // tile `first_named` would otherwise return.
            | (node_kinds::CODATA_OBSERVATION, node_kinds::FIELD_NAME) => {
                self.first_tile(label::IDENTIFIER)
            },
            // An attribute payload or a parenthesized type: the sole
            // expression inside `( … )`.
            | (node_kinds::ATTRIBUTE, node_kinds::FIELD_PAYLOAD)
            | (node_kinds::PARENTHESIZED_TYPE, node_kinds::FIELD_TYPE) => {
                self.between(label::LPAREN, label::RPAREN)
            },
            // Shell host escapes are either a fielded node or a PBG-folded `$`
            // `(` expression `)` run. In shell mode a bare host atom such as
            // `1` may still carry the shell-word tile label; keep that
            // reinterpretation local to this host-expression slot.
            | (node_kinds::HOST_ESCAPE, node_kinds::FIELD_EXPRESSION) => {
                self.host_escape_expression_field()
            },
            // The import URI and `extern` ABI are each the first
            // quote-delimited run. An import's alias is the identifier after
            // the `as` tile. The melder inlines `"…"` as flat tiles, so each
            // string is a synthesized run rather than a node of its own.
            | (node_kinds::IMPORT_DECLARATION, node_kinds::FIELD_URI)
            | (node_kinds::EXTERN_BLOCK, node_kinds::FIELD_ABI) => {
                self.nth_string_run(StringRunIndex(0))
            },
            | (node_kinds::IMPORT_DECLARATION, node_kinds::FIELD_ALIAS) => self.after(label::AS),
            | (node_kinds::EXTERN_BLOCK, node_kinds::FIELD_LIBRARY) => {
                self.nth_string_run(StringRunIndex(1))
            },
            | (node_kinds::MODULE_DECLARATION, node_kinds::FIELD_ASCRIPTION) => {
                self.module_ascription()
            },
            // The optional `[ r ]` grade on a graded thunk / thunk type.
            | (node_kinds::THUNK_EXPRESSION | node_kinds::U_TYPE, node_kinds::FIELD_GRADE) => {
                self.grade_field()
            },
            // "the nth significant child".
            | (node_kinds::CALL_EXPRESSION, node_kinds::FIELD_FUNCTION)
            | (node_kinds::INSTANTIATION_EXPRESSION, node_kinds::FIELD_TARGET)
            | (
                node_kinds::CONSTRUCTOR_PATTERN | node_kinds::TYPE_APPLICATION,
                node_kinds::FIELD_CONSTRUCTOR,
            ) => self.nth_sig(SignificantIndex(0)),
            // `F` groups its argument in (`F A` melds as one node), so the
            // argument is the second significant child.
            | (node_kinds::F_TYPE, node_kinds::FIELD_ARGUMENT) => self.nth_sig(SignificantIndex(1)),
            // `U` / `U[r]` melds as a bare prefix: its argument is the
            // immediately-following significant *sibling* (the paren group or
            // atom), not a child — the grade bracket occupies the child slots.
            | (node_kinds::U_TYPE, node_kinds::FIELD_ARGUMENT) => self.u_type_argument(),
            // "the nth named child" (binary operands around the operator).
            | (node_kinds::BINARY_EXPRESSION, node_kinds::FIELD_LEFT) => {
                self.nth_binary_operand(SignificantIndex(0))
            },
            | (node_kinds::BINARY_EXPRESSION, node_kinds::FIELD_RIGHT) => {
                self.nth_binary_operand(SignificantIndex(1))
            },
            // "the named child between two delimiters".
            | (node_kinds::DEF_VALUE, node_kinds::FIELD_VALUE) => {
                self.between(label::EQUALS, label::SEMI)
            },
            | (node_kinds::DEF_SIGNATURE, node_kinds::FIELD_TYPE) => {
                self.between(label::COLON, label::SEMI)
            },
            | (node_kinds::RECORD_UPDATE_EXPRESSION, node_kinds::FIELD_BASE) => {
                self.between(label::HASH_BRACE, label::PIPE)
            },
            | (node_kinds::IF_EXPRESSION, node_kinds::FIELD_CONDITION) => {
                self.between(label::IF, label::LBRACE)
            },
            | (node_kinds::CASE_EXPRESSION, node_kinds::FIELD_VALUE) => {
                self.between(label::CASE, label::LBRACE)
            },
            | (node_kinds::ANNOTATION_EXPRESSION, node_kinds::FIELD_VALUE) => {
                self.between(label::LPAREN, label::COLON)
            },
            | (node_kinds::ANNOTATION_EXPRESSION, node_kinds::FIELD_TYPE) => {
                self.between(label::COLON, label::RPAREN)
            },
            | _ => None,
        }
    }

    /// The `index`-th significant child (used only for a boolean's `true`/
    /// `false` token).
    #[inline]
    #[must_use]
    pub fn child(
        self,
        index: SignificantIndex,
    ) -> Option<Self>
    {
        if self.kind() == node_kinds::BOOLEAN && index.0 == 0 {
            let id = self.backing_id()?;
            let forced = if self.text().as_ref() == label::TRUE.as_ref() {
                node_kinds::TRUE
            }
            else {
                node_kinds::FALSE
            };
            return Some(Self {
                tree: self.tree,
                at: At::Forced { id, kind: forced },
            });
        }
        self.sig().get(index.0).map(|&id| Self::wrap(self.tree, id))
    }

    /// The operator tile of a [`node_kinds::BINARY_EXPRESSION`]: the
    /// significant child whose label is one of the binary operators. Unlike
    /// the operands (recovered by `left`/`right`), the operator tile
    /// classifies to no named kind, so the lowerer recovers it here and
    /// maps its text to a prelude name.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns the operator tile of a binary expression (the first
    ///   top-level significant child whose label is a binary operator).
    /// - provides: the lowerer's operator-token recovery.
    /// - fails: `None` when no operator tile is present.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn binary_operator(self) -> Option<Self>
    {
        self.sig()
            .into_iter()
            .find(|&id| {
                self.tree
                    .tile_label(id)
                    .is_some_and(|label| BINARY_OPS.contains(&label.as_ref()))
            })
            .map(|id| Self::wrap(self.tree, id))
    }

    /// Whether this node is an `ERROR`/repair region: a grout leaf (a
    /// convex/ghost repair the melder inserted for a missing tile).
    #[inline]
    #[must_use]
    pub fn is_error(self) -> ErrorPresence
    {
        ErrorPresence(self.is_grout_leaf().0)
    }

    /// Whether this node stands in for a `MISSING` tile: the same grout-leaf
    /// signal as [`Self::is_error`] in the melder model (the obligation class
    /// distinguishes them; `melder-CST migration` milestone 3).
    #[inline]
    #[must_use]
    pub fn is_missing(self) -> MissingPresence
    {
        MissingPresence(self.is_grout_leaf().0)
    }

    /// Whether this subtree contains any repair region (a grout leaf anywhere).
    #[inline]
    #[must_use]
    pub fn has_error(self) -> ErrorPresence
    {
        ErrorPresence(
            self.backing_id()
                .is_some_and(|id| self.tree_has_grout(id).0),
        )
    }

    // --- Classification -------------------------------------------------------

    /// Classify a real node into the named kind the lowerer dispatches on.
    fn classify(
        self,
        id: NodeId,
    ) -> SyntaxKind
    {
        match self.tree.node_kind(id) {
            | Some(NodeKind::Wald) => node_kinds::SOURCE_FILE,
            | Some(NodeKind::Token) => self.classify_token(id),
            | Some(NodeKind::Meld | NodeKind::Cell) => self.classify_meld(id),
            | None => SyntaxKind(""),
        }
    }

    /// Classify a tile token by mold label and sort.
    fn classify_token(
        self,
        id: NodeId,
    ) -> SyntaxKind
    {
        let Some(text) = self.tree.tile_label(id)
        else {
            return SyntaxKind("");
        };
        let sort = self.tree.sort_of(id);
        match text {
            | label::IDENTIFIER => node_kinds::IDENTIFIER,
            | label::CONSTRUCTOR => node_kinds::CONSTRUCTOR,
            | label::NUMBER => node_kinds::NUMBER,
            | label::TYPED_NUMBER => node_kinds::TYPED_NUMBER,
            | label::TRUE | label::FALSE => node_kinds::BOOLEAN,
            | label::WILDCARD => node_kinds::WILDCARD,
            | label::OMEGA => node_kinds::OMEGA,
            | label::TYPE_IDENTIFIER | label::TYPE_VARIABLE => node_kinds::TYPE_IDENTIFIER,
            | other
                if matches!(sort, Some(Sort::Type))
                    && PRIMITIVE_TYPES.contains(&other.as_ref()) =>
            {
                node_kinds::PRIMITIVE_TYPE
            },
            | _ => SyntaxKind(""),
        }
    }

    /// Classify a Meld by its leading mold label, tile shape, and grammar sort.
    fn classify_meld(
        self,
        id: NodeId,
    ) -> SyntaxKind
    {
        let sig = self.tree.sig_children(id);
        let sort = self.tree.sort_of(id);
        let lead = sig.first().and_then(|&first| self.tree.tile_label(first));
        // An attributed item classifies as the def it decorates: skip the
        // leading `@[ … ]` attribute blocks first.
        if lead == Some(label::AT_BRACKET)
            && let Some(after) = self.after_attribute_blocks((&sig).into())
        {
            return self.classify_from_lead((&sig).into(), after, sort);
        }
        self.classify_from_lead((&sig).into(), SignificantIndex(0), sort)
    }

    /// Classify a Meld whose form starts at significant-child index `start`.
    fn classify_from_lead(
        self,
        sig: SignificantChildren<'_>,
        start: SignificantIndex,
        sort: Option<Sort>,
    ) -> SyntaxKind
    {
        let start = start.0;
        let lead_id = sig.get(start).copied();
        let lead = lead_id.and_then(|first| self.tree.tile_label(first));
        let second_label = sig
            .get(start.saturating_add(1))
            .and_then(|&node| self.tree.tile_label(node));
        match lead {
            | Some(label::DEF) => self.classify_def(sig, SignificantIndex(start)),
            | Some(label::EXTERN) => node_kinds::EXTERN_BLOCK,
            // The `data` / `codata` datatype-declaration leads. These classify
            // for the levitation stage-0 elaborator (`crate::desc_elab`):
            // the SynNode dispatch surface recognizes the
            // declaration kinds, though the lowerer's `item()` still firewalls
            // them from term lowering (their semantics graduate under the
            // pattern-matrix design).
            | Some(label::DATA) => node_kinds::DATA_DECLARATION,
            | Some(label::CODATA) => node_kinds::CODATA_DECLARATION,
            | Some(label::COMMAND_SUBSTITUTION_START) => node_kinds::COMMAND_SUBSTITUTION,
            // The ruled circuit block form's two item-position leads. They
            // classify so the circuit lowering (`crate::circuit_desc`) can find
            // them and so a form it does not carry names its construct in the
            // decline, instead of holing under an empty kind. A `rule` member
            // of a `data` block is a flat tile run rather than a Meld, so the
            // `rule` lead cannot be confused with one; the Item sort is
            // required anyway.
            | Some(label::SIGN) if matches!(sort, Some(Sort::Item)) => node_kinds::SIGN_DECLARATION,
            | Some(label::OPER | label::RULE) if matches!(sort, Some(Sort::Item)) => {
                node_kinds::CIRCUIT_DECLARATION
            },
            | Some(label::MODULE) => node_kinds::MODULE_DECLARATION,
            | Some(label::IMPORT) => node_kinds::IMPORT_DECLARATION,
            | Some(label::REC) => node_kinds::REC_BLOCK,
            | Some(label::VAL) => node_kinds::LET_STATEMENT,
            | Some(label::RUN) => node_kinds::BIND_STATEMENT,
            | Some(label::CASE) => node_kinds::CASE_EXPRESSION,
            | Some(label::IF) => node_kinds::IF_EXPRESSION,
            | Some(label::FN) => node_kinds::LAMBDA_EXPRESSION,
            | Some(label::THUNK) => node_kinds::THUNK_EXPRESSION,
            | Some(label::FORCE) => node_kinds::FORCE_EXPRESSION,
            | Some(label::RET) => node_kinds::RET_EXPRESSION,
            | Some(label::CO) => node_kinds::CO_EXPRESSION,
            | Some(label::LBRACE) => node_kinds::BLOCK,
            | Some(label::HASH_BRACE) if matches!(sort, Some(Sort::Type)) => {
                node_kinds::RECORD_TYPE
            },
            | Some(label::HASH_BRACE) if self.has_top_level(sig, label::PIPE).0 => {
                node_kinds::RECORD_UPDATE_EXPRESSION
            },
            | Some(label::HASH_BRACE) => node_kinds::RECORD_EXPRESSION,
            | Some(label::LBRACKET) => node_kinds::LIST_EXPRESSION,
            | Some(label::SHELL_OPEN) => node_kinds::SHELL_BLOCK,
            | Some(label::DQUOTE) => node_kinds::STRING,
            | Some(label::HOLE) => node_kinds::HOLE,
            | Some(label::NEG) => node_kinds::UNARY_EXPRESSION,
            | Some(label::F) if matches!(sort, Some(Sort::Type)) => node_kinds::F_TYPE,
            | Some(label::U) if matches!(sort, Some(Sort::Type)) => node_kinds::U_TYPE,
            | Some(label::LPAREN) => self.classify_paren(sig, SignificantIndex(start), sort),
            | Some(label::TYPE_IDENTIFIER) if second_label == Some(label::LPAREN) => {
                node_kinds::TYPE_APPLICATION
            },
            | _ => self.classify_infix(sort, lead_id, second_label),
        }
    }

    /// Classify a `def`-family Meld by the tile after the name (the factored
    /// grammar's discriminating tile). The `rec` tile immediately after `def`
    /// discriminates the recursive family ([`node_kinds::DEF_REC`], W4d fold);
    /// its interior body — copattern clauses (codata intro) or statements (user
    /// recursion) — is recovered by the lowerer, not by classification.
    fn classify_def(
        self,
        sig: SignificantChildren<'_>,
        start: SignificantIndex,
    ) -> SyntaxKind
    {
        let start = start.0;
        if sig
            .get(start.saturating_add(1))
            .and_then(|&node| self.tree.tile_label(node))
            == Some(label::REC)
        {
            return node_kinds::DEF_REC;
        }
        let disc = sig
            .get(start.saturating_add(2))
            .and_then(|&node| self.tree.tile_label(node));
        match disc {
            | Some(label::COLON) => node_kinds::DEF_SIGNATURE,
            | Some(label::LPAREN) => node_kinds::DEF_FUNCTION,
            | _ => node_kinds::DEF_VALUE,
        }
    }

    /// Classify a paren-led Meld (unit, tuple, annotation, parenthesized, or
    /// the type parenthesis).
    fn classify_paren(
        self,
        sig: SignificantChildren<'_>,
        start: SignificantIndex,
        sort: Option<Sort>,
    ) -> SyntaxKind
    {
        let start = start.0;
        if matches!(sort, Some(Sort::Type)) {
            return node_kinds::PARENTHESIZED_TYPE;
        }
        // A paren-led Meld in pattern position with a top-level `,` is a tuple
        // pattern (`(x, y)`), not a tuple expression — the sort discriminates.
        if matches!(sort, Some(Sort::Pattern)) && self.has_top_level(sig, label::COMMA).0 {
            return node_kinds::TUPLE_PATTERN;
        }
        // `( )` with nothing between the parens is the unit literal.
        let inner_empty = sig
            .get(start.saturating_add(1))
            .and_then(|&node| self.tree.tile_label(node))
            == Some(label::RPAREN);
        if inner_empty {
            return node_kinds::UNIT;
        }
        if self.has_top_level(sig, label::COMMA).0 {
            node_kinds::TUPLE_EXPRESSION
        }
        else if self.has_top_level(sig, label::COLON).0 {
            node_kinds::ANNOTATION_EXPRESSION
        }
        else {
            node_kinds::PARENTHESIZED_EXPRESSION
        }
    }

    /// Classify an operand-led Meld by its second significant tile: a
    /// projection, call, binary operator, or type operator; else a pattern
    /// grouping.
    fn classify_infix(
        self,
        sort: Option<Sort>,
        lead_id: Option<NodeId>,
        second_label: Option<TileSpelling>,
    ) -> SyntaxKind
    {
        let lead_label = lead_id.and_then(|node| self.tree.tile_label(node));
        if matches!(sort, Some(Sort::Pattern)) {
            if lead_label == Some(label::LPAREN) {
                return node_kinds::TUPLE_PATTERN;
            }
            if lead_label == Some(label::CONSTRUCTOR) {
                return node_kinds::CONSTRUCTOR_PATTERN;
            }
        }
        match second_label {
            | Some(label::DOT) => node_kinds::PROJECTION_EXPRESSION,
            | Some(label::LBRACKET_GRADE) if matches!(sort, Some(Sort::Expression)) => {
                node_kinds::INSTANTIATION_EXPRESSION
            },
            | Some(label::LPAREN) => node_kinds::CALL_EXPRESSION,
            | Some(label::RIGHT_ARROW) if matches!(sort, Some(Sort::Type)) => {
                node_kinds::FUNCTION_TYPE
            },
            | Some(label::STAR) if matches!(sort, Some(Sort::Type)) => node_kinds::PRODUCT_TYPE,
            | Some(label::PLUS) if matches!(sort, Some(Sort::Type)) => node_kinds::SUM_TYPE,
            | Some(label::AMP) if matches!(sort, Some(Sort::Type)) => node_kinds::LAZY_PRODUCT_TYPE,
            | Some(op) if BINARY_OPS.contains(&op.as_ref()) => node_kinds::BINARY_EXPRESSION,
            | _ => SyntaxKind(""),
        }
    }

    // --- Navigation helpers ---------------------------------------------------

    /// The backing real node id, if this view has one.
    fn backing_id(self) -> Option<NodeId>
    {
        match self.at {
            | At::Node(id) | At::Forced { id, .. } => Some(id),
            | At::Run { .. } => None,
        }
    }

    /// The `index`-th significant child, wrapped as a real node.
    fn nth_sig(
        self,
        index: SignificantIndex,
    ) -> Option<Self>
    {
        self.sig().get(index.0).map(|&id| Self::wrap(self.tree, id))
    }

    /// The `index`-th binary operand, reinterpreting shell host-escape word
    /// tokens as gandr atoms when a flat `$(...)` body is synthesized as an
    /// expression run.
    fn nth_binary_operand(
        self,
        index: SignificantIndex,
    ) -> Option<Self>
    {
        let mut seen = 0_usize;
        for id in self.sig() {
            let child = self.host_escape_projectable_element(id);
            if let Some(child) = child {
                if seen == index.0 {
                    return Some(child);
                }
                seen = seen.saturating_add(1);
            }
        }
        None
    }

    /// The argument of a graded-thunk type `U` / `U[r]`. The grade prefix melds
    /// as a bare node (its own children are the `U` tile and, when graded, the
    /// `[ r ]` bracket), so its type argument is the immediately-following
    /// significant *sibling* — unlike `F`, whose argument is a child. A `U`
    /// meld that does carry a non-grade child (a defensive, unobserved
    /// shape) prefers that child.
    fn u_type_argument(self) -> Option<Self>
    {
        let sig = self.sig();
        // sig[0] is the `U` tile; skip an optional `[ r ]` grade bracket.
        let mut index = 1_usize;
        if sig.get(index).and_then(|&node| self.tree.tile_label(node))
            == Some(label::LBRACKET_GRADE)
            && let Some(close) = self.matching_close(
                (&sig).into(),
                SignificantIndex(index),
                label::LBRACKET_GRADE,
                label::RBRACKET_GRADE,
            )
        {
            index = close.0.saturating_add(1);
        }
        if let Some(child) = sig.get(index) {
            return Some(Self::wrap(self.tree, *child));
        }
        self.next_significant_sibling()
    }

    /// The next significant sibling of this node's backing node, via the CST
    /// parent link (space-skipped). `None` for a synthetic run or the root.
    fn next_significant_sibling(self) -> Option<Self>
    {
        let id = self.backing_id()?;
        let parent = {
            let present = self.tree.cst.node(id).ok()?;
            core::convert::identity(present)
        }
        .parent()?;
        let siblings = self.tree.sig_children(parent);
        let position = siblings.iter().position(|&sibling| sibling == id)?;
        siblings
            .get(position.saturating_add(1))
            .map(|&next| Self::wrap(self.tree, next))
    }

    /// The first named child.
    fn first_named(self) -> Option<Self>
    {
        self.plain_named_children().into_iter().next()
    }

    /// The first significant child whose tile label is `wanted`.
    fn first_tile(
        self,
        wanted: TileSpelling,
    ) -> Option<Self>
    {
        self.sig()
            .into_iter()
            .find(|&id| self.tree.tile_label(id) == Some(wanted))
            .map(|id| Self::wrap(self.tree, id))
    }

    /// Whether a [`node_kinds::CODATA_OBSERVATION`] segment is a reserved
    /// parse-and-decline form (codata design §2): a `rule` 2-cell member, a
    /// graded observation (`1 step: …`), or a parameterized observation
    /// (`ap(x: a): b`, whose `(` precedes the result `:`). The MVP carrier
    /// (`codata MVP`) lowers only the plain `π : B` observation; a reserved
    /// member is registered but declined.
    #[inline]
    #[must_use]
    pub fn is_reserved_observation(self) -> ReservedObservationFlag
    {
        let sig = self.sig();
        let lead = sig.first().and_then(|&id| self.tree.tile_label(id));
        if lead == Some(label::RULE) {
            return ReservedObservationFlag(true);
        }
        // A leading grade prefix (`number` / `ω`) before the observation ident.
        if matches!(lead, Some(label::NUMBER | label::OMEGA)) {
            return ReservedObservationFlag(true);
        }
        // A parameter list: a `(` appearing before the result-type `:`.
        for id in &sig {
            match self.tree.tile_label(*id) {
                | Some(label::LPAREN) => return ReservedObservationFlag(true),
                | Some(label::COLON) => break,
                | _ => {},
            }
        }
        ReservedObservationFlag(false)
    }

    /// Whether a [`node_kinds::DEF_REC`] body is a copattern-clause list (its
    /// first body tile is a leading projection `.` or the default-arm `_`),
    /// rather than an ordinary statement body. The copattern body is the codata
    /// intro (codata design §5.1, `codata MVP`); a statement body is user
    /// recursion, which the codata MVP declines.
    #[inline]
    #[must_use]
    pub fn def_rec_has_copattern_body(self) -> CopatternBodyFlag
    {
        let Some((container, body)) = self.brace_body()
        else {
            return CopatternBodyFlag(false);
        };
        let sig = self.tree.sig_children(container);
        CopatternBodyFlag(matches!(
            sig.get(body.start).and_then(|&id| self.tree.tile_label(id)),
            Some(label::DOT | label::WILDCARD)
        ))
    }

    /// The named child immediately after the first tile labelled `lead`.
    fn after_lead(
        self,
        lead: TileSpelling,
    ) -> Option<Self>
    {
        let sig = self.sig();
        let pos = sig
            .iter()
            .position(|&node| self.tree.tile_label(node) == Some(lead))?;
        sig.get(pos.saturating_add(1))
            .filter(|&&id| self.is_named(id).0)
            .map(|&id| Self::wrap(self.tree, id))
    }

    /// The first named child after the first tile labelled `sep`.
    fn after(
        self,
        sep: TileSpelling,
    ) -> Option<Self>
    {
        let sig = self.sig();
        let pos = sig
            .iter()
            .position(|&node| self.tree.tile_label(node) == Some(sep))?;
        {
            let found = sig.get(pos.saturating_add(1) ..)?;
            core::convert::identity(found)
        }
        .iter()
        .find(|&&id| self.is_named(id).0)
        .map(|&id| Self::wrap(self.tree, id))
    }

    /// The last named child before the last tile labelled `sep`.
    fn before(
        self,
        sep: TileSpelling,
    ) -> Option<Self>
    {
        let sig = self.sig();
        let pos = sig
            .iter()
            .rposition(|&node| self.tree.tile_label(node) == Some(sep))?;
        {
            let found = sig.get(.. pos)?;
            core::convert::identity(found)
        }
        .iter()
        .rev()
        .find(|&&id| self.is_named(id).0)
        .map(|&id| Self::wrap(self.tree, id))
    }

    /// The first named child strictly between the first `open` and the next
    /// `close` tile.
    fn between(
        self,
        open: TileSpelling,
        close: TileSpelling,
    ) -> Option<Self>
    {
        let sig = self.sig();
        let open_pos = sig
            .iter()
            .position(|&node| self.tree.tile_label(node) == Some(open))?;
        for id in sig.get(open_pos.saturating_add(1) ..)? {
            if self.tree.tile_label(*id) == Some(close) {
                return None;
            }
            if self.is_named(*id).0 {
                return Some(Self::wrap(self.tree, *id));
            }
        }
        None
    }

    /// The expression inside a shell host escape.
    fn host_escape_expression_field(self) -> Option<Self>
    {
        let sig = self.sig();
        let open_pos = sig
            .iter()
            .position(|&node| self.tree.tile_label(node) == Some(label::LPAREN))?;
        let close_pos = self.matching_close(
            (&sig).into(),
            SignificantIndex(open_pos),
            label::LPAREN,
            label::RPAREN,
        )?;
        let body_start = open_pos.saturating_add(1);
        let body = sig.get(body_start .. close_pos.0)?;
        let window = self.container_window();
        let container = window.container;
        let base = window.start.0;
        if body.len() == 1 {
            let sole = *body.first()?;
            return self.host_escape_projectable_element(sole);
        }
        if body.len() == 3 {
            let left = *body.first()?;
            let operator = *body.get(1)?;
            let right = *body.get(2)?;
            if self
                .tree
                .tile_label(operator)
                .is_some_and(|label| BINARY_OPS.contains(&label.as_ref()))
                && self.host_escape_projectable_element(left).is_some()
                && self.host_escape_projectable_element(right).is_some()
            {
                return Some(self.run(
                    container,
                    SignificantIndex(base.saturating_add(body_start)),
                    SignificantIndex(base.saturating_add(close_pos.0)),
                    node_kinds::BINARY_EXPRESSION,
                ));
            }
        }
        None
    }

    /// Projects one significant host-escape body element as either a shell-word
    /// atom reinterpreted in gandr expression context or an already named gandr
    /// node. Punctuation/operators are not projectable operands by themselves.
    fn host_escape_projectable_element(
        self,
        id: NodeId,
    ) -> Option<Self>
    {
        if let Some(atom) = self.host_escape_shell_word_atom(id) {
            return Some(atom);
        }
        if !self.is_named(id).0 {
            return None;
        }
        let root = Self::wrap(self.tree, id);
        if root.kind() == node_kinds::BINARY_EXPRESSION
            && !root.is_host_escape_binary_expression().0
        {
            return None;
        }
        (!root.is_malformed_host_escape_block().0).then_some(root)
    }

    /// Whether every node in a binary-expression operand tree is projectable.
    fn host_escape_binary_tree_is_projectable(self) -> HostEscapeFlag
    {
        let mut pending = vec![self];
        while let Some(binary) = pending.pop() {
            let sig = binary.sig();
            let [left, operator, right] = *sig.as_slice()
            else {
                return HostEscapeFlag(false);
            };
            if !self
                .tree
                .tile_label(operator)
                .is_some_and(|label| BINARY_OPS.contains(&label.as_ref()))
            {
                return HostEscapeFlag(false);
            }
            for operand in [left, right] {
                if self.host_escape_shell_word_atom(operand).is_some() {
                    continue;
                }
                if !self.is_named(operand).0 {
                    return HostEscapeFlag(false);
                }
                let node = Self::wrap(self.tree, operand);
                if node.kind() == node_kinds::BINARY_EXPRESSION {
                    pending.push(node);
                }
                else if node.is_malformed_host_escape_block().0 {
                    return HostEscapeFlag(false);
                }
            }
        }
        HostEscapeFlag(true)
    }

    /// Whether a block-like host-escape operand lacks its required opening
    /// brace.
    fn is_malformed_host_escape_block(self) -> HostEscapeFlag
    {
        HostEscapeFlag(
            self.kind() == node_kinds::BLOCK
                && self
                    .sig()
                    .first()
                    .and_then(|&child| self.tree.tile_label(child))
                    != Some(label::LBRACE),
        )
    }

    /// Whether a binary expression exposes exactly
    /// operand/operator/operand significant children and both operands are
    /// projectable.
    fn is_host_escape_binary_expression(self) -> HostEscapeFlag
    {
        HostEscapeFlag(
            self.kind() == node_kinds::BINARY_EXPRESSION
                && self.host_escape_binary_tree_is_projectable().0,
        )
    }

    /// Re-presents a shell-word token that occupies a host-expression atom
    /// slot.
    fn host_escape_shell_word_atom(
        self,
        id: NodeId,
    ) -> Option<Self>
    {
        if self.tree.tile_label(id) != Some(label::COMMAND_NAME) {
            return None;
        }
        let text = self.tree.cst.source().as_ref().get(self.tree.range(id).0)?;
        let kind = host_escape_shell_word_kind(SourceSlice::from(text))?;
        Some(Self {
            tree: self.tree,
            at: At::Forced { id, kind },
        })
    }

    /// The named children with punctuation/keyword tiles and grout removed —
    /// the default `named_children` for leaf-ish and wrapper forms.
    fn plain_named_children(self) -> Vec<Self>
    {
        self.sig()
            .into_iter()
            .filter(|&id| self.is_named(id).0)
            .map(|id| Self::wrap(self.tree, id))
            .collect()
    }

    /// Whether `id` is significant named syntax (a Meld/Wald/Cell or a named
    /// terminal token) rather than punctuation or grout.
    fn is_named(
        self,
        id: NodeId,
    ) -> NamedNodeFlag
    {
        NamedNodeFlag(match self.tree.node_kind(id) {
            | Some(NodeKind::Meld | NodeKind::Wald | NodeKind::Cell) => true,
            | Some(NodeKind::Token) => self
                .tree
                .tile_label(id)
                .is_some_and(|label| is_named_terminal(label).0),
            | None => false,
        })
    }

    /// The named items under the file root (skipping trivia and grout). An
    /// item is any named node — a form Meld *or* a bare named terminal, since a
    /// top-level bare expression (`r`, `5`) is a single tile, not a Meld.
    fn items(self) -> Vec<Self>
    {
        let Some(id) = self.backing_id()
        else {
            return Vec::new();
        };
        self.tree
            .sig_children(id)
            .into_iter()
            .filter(|&child| self.is_named(child).0)
            .map(|child| Self::wrap(self.tree, child))
            .collect()
    }

    // --- Segmentation (the flat forms the melder does not group) --------------

    /// A block's statements and tail: split the body between the outermost
    /// `{`/`}` on top-level `;` tiles; each `;`-terminated run is a synthetic
    /// statement, and a final unterminated run is the tail (returned as its own
    /// single node when it is one).
    fn block_statements(self) -> Vec<Self>
    {
        let Some((container, body)) = self.brace_body()
        else {
            return Vec::new();
        };
        let sig = self.tree.sig_children(container);
        let mut out = Vec::new();
        let mut start = body.start;
        let mut index = body.start;
        while index < body.end {
            let terminated = sig
                .get(index)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::SEMI));
            if terminated {
                if index > start {
                    out.push(self.statement(
                        container,
                        (&sig).into(),
                        SignificantIndex(start),
                        SignificantIndex(index),
                    ));
                }
                start = index.saturating_add(1);
            }
            index = index.saturating_add(1);
        }
        // The trailing (unterminated) run is the tail: a single node is returned
        // directly; a multi-node run is a (recovered) statement.
        if start < body.end {
            if body.end.saturating_sub(start) == 1 {
                if let Some(id) = sig.get(start) {
                    out.push(Self::wrap(self.tree, *id));
                }
            }
            else {
                out.push(self.statement(
                    container,
                    (&sig).into(),
                    SignificantIndex(start),
                    SignificantIndex(body.end),
                ));
            }
        }
        out
    }

    /// Classify a `[lo, hi)` statement run and wrap it as a synthetic node.
    fn statement(
        self,
        container: NodeId,
        sig: SignificantChildren<'_>,
        lo: SignificantIndex,
        hi: SignificantIndex,
    ) -> Self
    {
        let lo = lo.0;
        let hi = hi.0;
        let lead = sig.get(lo).and_then(|&node| self.tree.tile_label(node));
        let kind = match lead {
            | Some(label::VAL) => node_kinds::LET_STATEMENT,
            | Some(label::RUN) => node_kinds::BIND_STATEMENT,
            // Out-of-fragment statement keywords keep their statement kind so
            // the lowerer rejects them as `Unsupported` rather than mis-lowering
            // their contents as a bare expression statement.
            | Some(other) => {
                unsupported_statement_kind(other).unwrap_or(node_kinds::EXPRESSION_STATEMENT)
            },
            | None => node_kinds::EXPRESSION_STATEMENT,
        };
        // An out-of-fragment statement's elided region is the whole statement
        // form including its terminating `;` (the grammar rule `leta … ;`); the
        // block splits the run just before that `;`, so extend the run to cover
        // it when present. In-fragment statements keep the terminator-free span.
        let end = if unsupported_statement_kind(lead.unwrap_or_default()).is_some()
            && sig
                .get(hi)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::SEMI))
        {
            hi.saturating_add(1)
        }
        else {
            hi
        };
        self.run(container, SignificantIndex(lo), SignificantIndex(end), kind)
    }

    /// The `,`-separated synthetic segments within `bounds`, each a node of
    /// `kind`.
    fn comma_segments(
        self,
        bounds: Option<(NodeId, Span)>,
        kind: SyntaxKind,
    ) -> Vec<Self>
    {
        let Some((container, body)) = bounds
        else {
            return Vec::new();
        };
        let sig = self.tree.sig_children(container);
        let mut out = Vec::new();
        let mut start = body.start;
        let mut index = body.start;
        while index < body.end {
            let is_comma = sig
                .get(index)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::COMMA));
            if is_comma {
                if index > start {
                    out.push(self.run(
                        container,
                        SignificantIndex(start),
                        SignificantIndex(index),
                        kind,
                    ));
                }
                start = index.saturating_add(1);
            }
            index = index.saturating_add(1);
        }
        if start < body.end {
            out.push(self.run(
                container,
                SignificantIndex(start),
                SignificantIndex(body.end),
                kind,
            ));
        }
        out
    }

    /// The member-separated synthetic segments within `bounds`, each a node
    /// of `kind`: members are terminated by the ruled `;` (the surface's
    /// declaration terminator), with the retired `,` separator admitted so a
    /// stale block still segments whole (the nested generator block's
    /// migration posture; `case` arms and records keep
    /// [`Self::comma_segments`]).
    fn member_segments(
        self,
        bounds: Option<(NodeId, Span)>,
        kind: SyntaxKind,
    ) -> Vec<Self>
    {
        let Some((container, body)) = bounds
        else {
            return Vec::new();
        };
        let sig = self.tree.sig_children(container);
        let mut out = Vec::new();
        let mut start = body.start;
        let mut index = body.start;
        while index < body.end {
            let is_separator = sig.get(index).is_some_and(|&node| {
                matches!(self.tree.tile_label(node), Some(label::SEMI | label::COMMA))
            });
            if is_separator {
                if index > start {
                    out.push(self.run(
                        container,
                        SignificantIndex(start),
                        SignificantIndex(index),
                        kind,
                    ));
                }
                start = index.saturating_add(1);
            }
            index = index.saturating_add(1);
        }
        if start < body.end {
            out.push(self.run(
                container,
                SignificantIndex(start),
                SignificantIndex(body.end),
                kind,
            ));
        }
        out
    }

    /// The `extern` block's `;`-separated members, classified `extern_type`
    /// (`type …`) or `extern_function` (`def …`).
    fn extern_members(self) -> Vec<Self>
    {
        let Some((container, body)) = self.brace_body()
        else {
            return Vec::new();
        };
        let sig = self.tree.sig_children(container);
        let mut out = Vec::new();
        let mut start = body.start;
        let mut index = body.start;
        while index < body.end {
            let is_semi = sig
                .get(index)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::SEMI));
            if is_semi {
                if index > start {
                    out.push(self.extern_member(
                        container,
                        (&sig).into(),
                        SignificantIndex(start),
                        SignificantIndex(index),
                    ));
                }
                start = index.saturating_add(1);
            }
            index = index.saturating_add(1);
        }
        if start < body.end {
            out.push(self.extern_member(
                container,
                (&sig).into(),
                SignificantIndex(start),
                SignificantIndex(body.end),
            ));
        }
        out
    }

    /// Classify one `extern` member run.
    fn extern_member(
        self,
        container: NodeId,
        sig: SignificantChildren<'_>,
        lo: SignificantIndex,
        hi: SignificantIndex,
    ) -> Self
    {
        let lo = lo.0;
        let hi = hi.0;
        let lead = sig.get(lo).and_then(|&node| self.tree.tile_label(node));
        let kind = if lead == Some(label::TYPE) {
            node_kinds::EXTERN_TYPE
        }
        else {
            node_kinds::EXTERN_FUNCTION
        };
        self.run(container, SignificantIndex(lo), SignificantIndex(hi), kind)
    }

    /// The `module` block's ordered definition/signature and nested-module
    /// members.
    ///
    /// # Contract
    /// - ensures: returns body members in source order, including one-level
    ///   nested modules, while excluding the header ascription.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: locating the module body before segmentation prevents its
    ///   inline record signature from becoming a body member.
    /// - mutants: segment the raw declaration window; use `brace_body`.
    /// - witnesses: `recognizes_one_level_nested_module_member`.
    fn module_members(self) -> Vec<Self>
    {
        self.definition_members(self.module_body())
    }

    /// The `rec` block's ordered recursive definitions.
    fn rec_members(self) -> Vec<Self>
    {
        self.definition_members(self.brace_body())
    }

    /// Segment definition or one-level nested-module members inside one
    /// already-located brace body.
    ///
    /// # Contract
    /// - ensures: each returned node spans one complete member in source order;
    ///   malformed suffixes stop segmentation rather than overlapping nodes.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: advancing to each delimiter-aware member end partitions
    ///   the body while preserving leading attributes.
    /// - mutants: advance from the lead rather than the run start; ignore
    ///   `end`.
    /// - witnesses: `recognizes_one_level_nested_module_member` and
    ///   `recognizes_module_declaration`.
    fn definition_members(
        self,
        region: Option<(NodeId, Span)>,
    ) -> Vec<Self>
    {
        let Some((container, body)) = region
        else {
            return Vec::new();
        };
        let sig = self.tree.sig_children(container);
        let mut out = Vec::new();
        let mut index = body.start;
        while index < body.end {
            let Some(lead_index) = self.module_member_lead_index(
                (&sig).into(),
                SignificantIndex(index),
                SignificantIndex(body.end),
            )
            else {
                break;
            };
            let kind = if sig
                .get(lead_index.0)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::MODULE))
            {
                node_kinds::MODULE_DECLARATION
            }
            else {
                self.classify_def((&sig).into(), lead_index)
            };
            let end = self
                .module_member_end((&sig).into(), lead_index, SignificantIndex(body.end), kind)
                .0;
            out.push(self.run(
                container,
                SignificantIndex(index),
                SignificantIndex(end),
                kind,
            ));
            if end <= index {
                break;
            }
            index = end;
        }
        out
    }

    /// The significant-child index of a module member's lead tile.
    ///
    /// # Contract
    /// - ensures: skips complete leading attribute blocks and returns only a
    ///   `def` or `module` tile within `[lo, hi)`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: delimiter-aware attribute skipping preserves the actual
    ///   member lead even when attribute interiors contain keywords.
    /// - mutants: skip one tile per attribute; accept arbitrary named tiles.
    /// - witnesses: `recognizes_one_level_nested_module_member`.
    fn module_member_lead_index(
        self,
        sig: SignificantChildren<'_>,
        lo: SignificantIndex,
        hi: SignificantIndex,
    ) -> Option<SignificantIndex>
    {
        let mut index = lo.0;
        while index < hi.0
            && sig
                .get(index)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::AT_BRACKET))
        {
            let close = self.matching_close(
                sig,
                SignificantIndex(index),
                label::AT_BRACKET,
                label::CLOSE_BRACKET,
            )?;
            index = close.0.saturating_add(1);
        }
        let node = sig.get(index)?;
        matches!(
            self.tree.tile_label(*node),
            Some(label::DEF | label::MODULE)
        )
        .then_some(SignificantIndex(index))
    }

    /// The end of one module member run.
    ///
    /// # Contract
    /// - ensures: functions and nested modules end after their matching body
    ///   brace; signature/value definitions end after their semicolon.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: member kind determines the only two valid terminator
    ///   families while shared brace matching handles inline record signatures.
    /// - mutants: stop at the first `}`; scan every member for `;`.
    /// - witnesses: `recognizes_one_level_nested_module_member`.
    fn module_member_end(
        self,
        sig: SignificantChildren<'_>,
        def_index: SignificantIndex,
        limit: SignificantIndex,
        kind: SyntaxKind,
    ) -> SignificantIndex
    {
        let def_index = def_index.0;
        let limit = limit.0;
        if matches!(
            kind,
            node_kinds::DEF_FUNCTION | node_kinds::MODULE_DECLARATION
        ) && let Some(open) = self.find_tile(
            sig,
            SignificantIndex(def_index),
            SignificantIndex(limit),
            label::LBRACE,
        ) {
            return SignificantIndex(
                self.matching_close(sig, open, label::LBRACE, label::RBRACE)
                    .map(|index| index.0)
                    .map_or(limit, |close| close.saturating_add(1)),
            );
        }
        SignificantIndex(
            self.find_tile(
                sig,
                SignificantIndex(def_index),
                SignificantIndex(limit),
                label::SEMI,
            )
            .map(|index| index.0)
            .map_or(limit, |semi| semi.saturating_add(1)),
        )
    }

    /// The optional signature ascription of a module declaration, transparent
    /// or opaque.
    ///
    /// # Contract
    /// - ensures: returns exactly the balanced `#{ ... }` after the module name
    ///   and colon, or `None` when no such header field exists.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: fixed header positions plus balanced delimiters
    ///   distinguish the ascription from records inside the body.
    /// - mutants: take the first `#{` anywhere; stop at the first `}`.
    /// - witnesses: `recognizes_module_declaration` and
    ///   `recognizes_one_level_nested_module_member`.
    fn module_ascription(self) -> Option<Self>
    {
        let window = self.raw_container_window();
        let container = window.container;
        let sig = self.tree.sig_children(container);
        let module_index =
            self.find_tile((&sig).into(), window.start, window.limit, label::MODULE)?;
        let after_name = module_index.0.saturating_add(2);
        if after_name >= window.limit.0 {
            return None;
        }
        // Either ascription tile opens the same signature: `:` transparent, `:>`
        // opaque. The tile is the *only* difference at this position, which is
        // what keeps the discrimination lookahead-free.
        let ascription_tile = sig
            .get(after_name)
            .and_then(|&node| self.tree.tile_label(node));
        if ascription_tile != Some(label::COLON) && ascription_tile != Some(label::COLON_ANGLE) {
            return None;
        }
        let open = self.find_tile(
            (&sig).into(),
            SignificantIndex(after_name.saturating_add(1)),
            window.limit,
            label::HASH_BRACE,
        )?;
        let close = self.matching_close((&sig).into(), open, label::HASH_BRACE, label::RBRACE)?;
        // The kind carries the ascription tile's meaning forward, so a consumer
        // cannot read a sealed signature as a transparent one by looking only at
        // the braces.
        let kind = if ascription_tile == Some(label::COLON_ANGLE) {
            node_kinds::OPAQUE_SIGNATURE
        }
        else {
            node_kinds::RECORD_TYPE
        };
        (close.0 < window.limit.0).then(|| {
            self.run(
                container,
                open,
                SignificantIndex(close.0.saturating_add(1)),
                kind,
            )
        })
    }

    /// The module body span, skipping an optional record-type ascription.
    ///
    /// # Contract
    /// - ensures: returns the interior of the balanced declaration body braces,
    ///   never the inline signature's record braces.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: consuming a balanced optional ascription before searching
    ///   for `{` isolates the body despite the shared `}` token.
    /// - mutants: search from the name; treat `#{` as the body opener.
    /// - witnesses: `recognizes_one_level_nested_module_member`.
    fn module_body(self) -> Option<(NodeId, Span)>
    {
        let window = self.raw_container_window();
        let container = window.container;
        let sig = self.tree.sig_children(container);
        let module_index =
            self.find_tile((&sig).into(), window.start, window.limit, label::MODULE)?;
        let after_name = module_index.0.saturating_add(2);
        let ascription_tile = sig
            .get(after_name)
            .and_then(|&node| self.tree.tile_label(node));
        let body_search_start = if after_name < window.limit.0
            && (ascription_tile == Some(label::COLON)
                || ascription_tile == Some(label::COLON_ANGLE))
        {
            let ascription_open = self.find_tile(
                (&sig).into(),
                SignificantIndex(after_name.saturating_add(1)),
                window.limit,
                label::HASH_BRACE,
            )?;
            let ascription_close = self.matching_close(
                (&sig).into(),
                ascription_open,
                label::HASH_BRACE,
                label::RBRACE,
            )?;
            ascription_close.0.saturating_add(1)
        }
        else {
            after_name
        };
        let open = self.find_tile(
            (&sig).into(),
            SignificantIndex(body_search_start),
            window.limit,
            label::LBRACE,
        )?;
        let close = self
            .matching_close((&sig).into(), open, label::LBRACE, label::RBRACE)
            .map_or_else(
                || window.limit.0.saturating_sub(1).max(open.0),
                |index| index.0.min(window.limit.0),
            );
        Some((container, Span {
            start: open.0.saturating_add(1),
            end: close,
        }))
    }

    /// The attribute blocks preceding an item/member (each `@[ … ]` run), as
    /// `attribute_block` synthetic nodes.
    fn attribute_blocks(self) -> Vec<Self>
    {
        let window = self.raw_container_window();
        let container = window.container;
        let start = window.start.0;
        let limit = window.limit.0;
        let sig = self.tree.sig_children(container);
        let mut out = Vec::new();
        let mut index = start;
        while index < limit
            && sig
                .get(index)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::AT_BRACKET))
        {
            let Some(close) = self
                .matching_close(
                    (&sig).into(),
                    SignificantIndex(index),
                    label::AT_BRACKET,
                    label::CLOSE_BRACKET,
                )
                .filter(|close| close.0 < limit)
            else {
                break;
            };
            out.push(self.run(
                container,
                SignificantIndex(index),
                SignificantIndex(close.0.saturating_add(1)),
                node_kinds::ATTRIBUTE_BLOCK,
            ));
            index = close.0.saturating_add(1);
        }
        out
    }

    /// The first / second (`n = 0` / `1`) quote-delimited run within this
    /// node's significant children, stopping at the block body `{`. The melder
    /// inlines an `extern`-block `"…"` as flat `"`/`string_fragment`/`"` tiles,
    /// so the ABI / library string is a synthesized run rather than a node.
    fn nth_string_run(
        self,
        n: StringRunIndex,
    ) -> Option<Self>
    {
        let n = n.0;
        let container = self.backing_id()?;
        let sig = self.tree.sig_children(container);
        let mut seen = 0_usize;
        let mut index = 0_usize;
        while index < sig.len() {
            let label = sig.get(index).and_then(|&node| self.tree.tile_label(node));
            if label == Some(label::LBRACE) {
                break;
            }
            if label == Some(label::DQUOTE) {
                let close = {
                    let found = sig.get(index.saturating_add(1) ..)?;
                    core::convert::identity(found)
                }
                .iter()
                .position(|&node| self.tree.tile_label(node) == Some(label::DQUOTE))
                .map(|offset| index.saturating_add(1).saturating_add(offset))?;
                if seen == n {
                    return Some(self.run(
                        container,
                        SignificantIndex(index),
                        SignificantIndex(close.saturating_add(1)),
                        node_kinds::STRING,
                    ));
                }
                seen = seen.saturating_add(1);
                index = close.saturating_add(1);
                continue;
            }
            index = index.saturating_add(1);
        }
        None
    }

    /// The `[ r ]` grade annotation of a `thunk[r]` / `U[r]`, as a single-tile
    /// run wrapping the grade `number`/`ω`. The melder inlines the grade as a
    /// flat tile between the brackets, so the grade reader's `child(0)` reads
    /// the run's sole child token and its text is the grade numeral.
    fn grade_field(self) -> Option<Self>
    {
        let (container, span) = self.body_between(label::LBRACKET_GRADE, label::RBRACKET_GRADE)?;
        (span.start < span.end).then(|| {
            self.run(
                container,
                SignificantIndex(span.start),
                SignificantIndex(span.end),
                KIND_GRADE,
            )
        })
    }

    /// A shell block's `;`-separated commands. Each segment is a simple
    /// [`node_kinds::COMMAND`] unless it carries a pipeline / control operator,
    /// in which case it presents as the operator's kind so the lowerer's shell
    /// path rejects it as unsupported (`host-module surface` boundary).
    fn shell_commands(self) -> Vec<Self>
    {
        let Some((container, body)) = self.body_between(label::SHELL_OPEN, label::RBRACE)
        else {
            return Vec::new();
        };
        let sig = self.tree.sig_children(container);
        let mut out = Vec::new();
        let mut start = body.start;
        let mut index = body.start;
        while index < body.end {
            let is_semi = sig
                .get(index)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::SEMI));
            if is_semi {
                if index > start {
                    out.push(self.run(
                        container,
                        SignificantIndex(start),
                        SignificantIndex(index),
                        self.shell_segment_kind(container, (&sig).into(), start, index),
                    ));
                }
                start = index.saturating_add(1);
            }
            index = index.saturating_add(1);
        }
        if start < body.end {
            out.push(self.run(
                container,
                SignificantIndex(start),
                SignificantIndex(body.end),
                self.shell_segment_kind(container, (&sig).into(), start, body.end),
            ));
        }
        out
    }

    /// Classify a shell `;`-segment: a pipeline / control operator at the
    /// command's lexical level (directly, or one level into a child meld the
    /// melder nested the operator into) makes it that operator's kind;
    /// operators inside a delimited `$(...)` host escape belong to the embedded
    /// gandr expression and are not shell control.
    fn shell_segment_kind(
        self,
        container: NodeId,
        sig: SignificantChildren<'_>,
        lo: impl Into<SignificantIndex>,
        hi: impl Into<SignificantIndex>,
    ) -> SyntaxKind
    {
        let lo = lo.into().0;
        let hi = hi.into().0;
        let Some(slice) = sig.get(lo .. hi)
        else {
            return node_kinds::COMMAND;
        };
        let mut offset = 0_usize;
        while offset < slice.len() {
            let absolute = lo.saturating_add(offset);
            if let Some((_part, next)) = self.flat_host_escape_part(container, sig, absolute, hi) {
                offset = next.0.saturating_sub(lo);
                continue;
            }
            let Some(&id) = slice.get(offset)
            else {
                break;
            };
            if let Some(kind) = shell_op_kind(self.tree.tile_label(id)) {
                return kind;
            }
            if matches!(
                self.tree.node_kind(id),
                Some(NodeKind::Meld | NodeKind::Cell)
            ) && self.shell_decoration_kind(id) != Some(node_kinds::HOST_ESCAPE)
            {
                for child in self.tree.sig_children(id) {
                    if let Some(kind) = shell_op_kind(self.tree.tile_label(child)) {
                        return kind;
                    }
                }
            }
            offset = offset.saturating_add(1);
        }
        node_kinds::COMMAND
    }

    /// The parts of a simple shell command: bare-word tokens forced to
    /// [`node_kinds::COMMAND_NAME`], flat `$(…)` runs grouped as
    /// [`node_kinds::HOST_ESCAPE`], and quoted strings forced to the
    /// single-/double-quoted kinds the lowerer's shell atom decoder dispatches
    /// on.
    fn command_parts(self) -> Vec<Self>
    {
        let window = self.container_window();
        let container = window.container;
        let lo = window.start.0;
        let hi = window.limit.0;
        let sig = self.tree.sig_children(container);
        let mut out = Vec::new();
        let mut index = lo;
        while index < hi {
            if let Some((part, next)) =
                self.flat_host_escape_part(container, (&sig).into(), index, hi)
            {
                out.push(part);
                index = next.0;
                continue;
            }
            if let Some(id) = sig.get(index) {
                out.push(self.shell_part(*id));
            }
            index = index.saturating_add(1);
        }
        out
    }

    /// Presents a flat `$` `(` expression `)` command-part run as one host
    /// escape, preserving the interior CST nodes for field projection.
    fn flat_host_escape_part(
        self,
        container: NodeId,
        sig: SignificantChildren<'_>,
        index: impl Into<SignificantIndex>,
        limit: impl Into<SignificantIndex>,
    ) -> Option<(Self, SignificantIndex)>
    {
        let index = index.into().0;
        let limit = limit.into().0;
        if sig.get(index).and_then(|&node| self.tree.tile_label(node)) != Some(label::DOLLAR)
            || sig
                .get(index.saturating_add(1))
                .and_then(|&node| self.tree.tile_label(node))
                != Some(label::LPAREN)
        {
            return None;
        }
        let close = self.matching_close(
            sig,
            SignificantIndex(index.saturating_add(1)),
            label::LPAREN,
            label::RPAREN,
        )?;
        if close.0 >= limit {
            return None;
        }
        Some((
            self.run(
                container,
                SignificantIndex(index),
                SignificantIndex(close.0.saturating_add(1)),
                node_kinds::HOST_ESCAPE,
            ),
            SignificantIndex(close.0.saturating_add(1)),
        ))
    }

    /// Present one shell-command child with its lowerer-facing kind forced. A
    /// bare word is a command word (or the `!` negation); a quoted run is a
    /// single/double-quoted string; and decorated runs force their kinds so the
    /// lowerer can either lower safe host escapes or reject still-unsupported
    /// variable expansions and redirections through the existing unsupported
    /// path.
    fn shell_part(
        self,
        id: NodeId,
    ) -> Self
    {
        let forced = match self.tree.node_kind(id) {
            | Some(NodeKind::Token) => match self.tree.tile_label(id) {
                | Some(label::COMMAND_NAME) => {
                    if self.tree.cst.source().as_ref().get(self.tree.range(id).0)
                        == Some(label::BANG.as_ref())
                    {
                        Some(node_kinds::NEGATION)
                    }
                    else {
                        Some(node_kinds::COMMAND_NAME)
                    }
                },
                | Some(label::ENVIRONMENT_ASSIGNMENT) => Some(node_kinds::ENVIRONMENT_ASSIGNMENT),
                | _ => None,
            },
            | Some(NodeKind::Meld | NodeKind::Cell) => self.shell_decoration_kind(id),
            | _ => None,
        };
        forced.map_or_else(
            || Self::wrap(self.tree, id),
            |kind| Self {
                tree: self.tree,
                at: At::Forced { id, kind },
            },
        )
    }

    /// The lowerer-facing kind of a shell-command child meld: a quoted string,
    /// a `$`-led variable expansion / host escape, or a redirection (any
    /// redirection operator among its children), else `None` for a plain word
    /// grouping.
    fn shell_decoration_kind(
        self,
        id: NodeId,
    ) -> Option<SyntaxKind>
    {
        let children = self.tree.sig_children(id);
        let first = children
            .first()
            .and_then(|&node| self.tree.tile_label(node));
        let second = children.get(1).and_then(|&node| self.tree.tile_label(node));
        match first {
            | Some(label::SQUOTE) => Some(node_kinds::SINGLE_QUOTED_STRING),
            | Some(label::DQUOTE) => Some(node_kinds::DOUBLE_QUOTED_STRING),
            | Some(label::DOLLAR) if second == Some(label::LPAREN) => Some(node_kinds::HOST_ESCAPE),
            | Some(label::DOLLAR) => Some(node_kinds::VARIABLE_EXPANSION),
            | _ if children
                .iter()
                .any(|&node| is_shell_redirection(self.tree.tile_label(node)).0) =>
            {
                Some(node_kinds::REDIRECTION)
            },
            | _ => None,
        }
    }

    // --- Field-recovery helpers -----------------------------------------------

    /// Flatten an n-ary product/sum/lazy-product type's right-nested operator
    /// chain back into its member list.
    fn type_members(self) -> Vec<Self>
    {
        let kind = self.kind();
        let op = match kind {
            | node_kinds::PRODUCT_TYPE => label::STAR,
            | node_kinds::SUM_TYPE => label::PLUS,
            | node_kinds::LAZY_PRODUCT_TYPE => label::AMP,
            | _ => return Vec::new(),
        };
        let mut out = Vec::new();
        let mut current = Some(self);
        while let Some(node) = current {
            if node.kind() == kind {
                if let Some(first) = node.sig().first() {
                    out.push(Self::wrap(self.tree, *first));
                }
                current = node.after(op);
            }
            else {
                out.push(node);
                break;
            }
        }
        out
    }

    /// A `( … )` group at this node's leading call/param paren, presented as a
    /// synthetic `kind` run spanning the parens inclusive.
    fn paren_group(
        self,
        kind: SyntaxKind,
    ) -> Option<Self>
    {
        let (container, span) = self.delimited_span(label::LPAREN, label::RPAREN)?;
        Some(self.run(
            container,
            SignificantIndex(span.start),
            SignificantIndex(span.end),
            kind,
        ))
    }

    /// The `n`-th `{ … }` brace block within this form, a synthetic `block` run
    /// spanning the braces inclusive.
    fn nth_brace_block(
        self,
        n: SignificantIndex,
    ) -> Option<Self>
    {
        // Window-aware so it also finds the brace blocks of a *synthetic* run
        // (e.g. the nested `if` of a flat `else if` chain), not only a real node.
        let window = self.container_window();
        let container = window.container;
        let base = window.start.0;
        let limit = window.limit.0;
        let n = n.0;
        let sig = self.tree.sig_children(container);
        let mut seen = 0_usize;
        let mut index = base;
        while index < limit {
            if sig
                .get(index)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::LBRACE))
            {
                // A matching `}`, or — when the melder ghost-closed an
                // unterminated block body — the window's last significant child,
                // so the body still recovers (`melder-CST migration`; a matched brace is
                // unaffected).
                let close = self
                    .matching_close(
                        (&sig).into(),
                        SignificantIndex(index),
                        label::LBRACE,
                        label::RBRACE,
                    )
                    .filter(|close| close.0 < limit)
                    .map_or_else(|| limit.saturating_sub(1).max(index), |index| index.0);
                if seen == n {
                    return Some(self.run(
                        container,
                        SignificantIndex(index),
                        SignificantIndex(close.saturating_add(1)),
                        node_kinds::BLOCK,
                    ));
                }
                seen = seen.saturating_add(1);
                index = close.saturating_add(1);
                continue;
            }
            index = index.saturating_add(1);
        }
        None
    }

    /// The single brace block of a lambda/thunk/def-function body.
    fn brace_block(self) -> Option<Self>
    {
        self.nth_brace_block(SignificantIndex(0))
    }

    /// The `if` alternative: an `else if …` chain (a nested `if_expression`),
    /// else the second `{ … }` brace block (the `else` branch). The melder
    /// keeps the whole `if … else if … else …` flat, so a nested `if` after
    /// the first `else` is synthesized as a run from that `if` to the
    /// form's end.
    fn if_alternative(self) -> Option<Self>
    {
        let sig = self.sig();
        let else_pos = sig
            .iter()
            .position(|&node| self.tree.tile_label(node) == Some(label::ELSE))?;
        let after = else_pos.saturating_add(1);
        if sig.get(after).and_then(|&node| self.tree.tile_label(node)) == Some(label::IF) {
            let window = self.container_window();
            return Some(self.run(
                window.container,
                SignificantIndex(window.start.0.saturating_add(after)),
                window.limit,
                node_kinds::IF_EXPRESSION,
            ));
        }
        self.nth_brace_block(SignificantIndex(1))
    }

    // --- Spans & bracket matching --------------------------------------------

    /// The container plus the `[lo, hi)` span strictly inside the outermost
    /// `{`/`}`.
    fn brace_body(self) -> Option<(NodeId, Span)>
    {
        self.body_between(label::LBRACE, label::RBRACE)
    }

    /// The container plus the `[lo, hi)` span strictly inside the outermost
    /// `(`/`)`.
    fn paren_body(self) -> Option<(NodeId, Span)>
    {
        self.body_between(label::LPAREN, label::RPAREN)
    }

    /// The container plus the `[lo, hi)` span strictly inside the outermost
    /// `[`/`]` instantiation slot.
    fn bracket_body(self) -> Option<(NodeId, Span)>
    {
        self.body_between(label::LBRACKET_GRADE, label::RBRACKET_GRADE)
    }

    /// The container plus the `[lo, hi)` span between the record `#{` and its
    /// matching `}`, skipping a leading `base |` update prefix.
    fn hash_brace_body(self) -> Option<(NodeId, Span)>
    {
        let (container, span) = self.body_between(label::HASH_BRACE, label::RBRACE)?;
        let sig = self.tree.sig_children(container);
        let start = sig
            .get(span.start .. span.end)
            .and_then(|slice| {
                slice
                    .iter()
                    .position(|&node| self.tree.tile_label(node) == Some(label::PIPE))
            })
            .map_or(span.start, |offset| {
                span.start.saturating_add(offset).saturating_add(1)
            });
        Some((container, Span {
            start,
            end: span.end,
        }))
    }

    /// The container plus the `[lo, hi)` span strictly inside the outermost
    /// `open`/`close` delimiters.
    fn body_between(
        self,
        open: TileSpelling,
        close: TileSpelling,
    ) -> Option<(NodeId, Span)>
    {
        let (container, span) = self.delimited_span(open, close)?;
        Some((container, Span {
            start: span.start.saturating_add(1),
            end: span.end.saturating_sub(1),
        }))
    }

    /// The container plus the `[open_index, close_index]`-inclusive span (as a
    /// `[start, end)` where `end` is one past the close) of the outermost
    /// `open`/`close` bracket pair. For a synthetic run the search is within
    /// the run's own slice.
    fn delimited_span(
        self,
        open: TileSpelling,
        close: TileSpelling,
    ) -> Option<(NodeId, Span)>
    {
        let window = self.container_window();
        let container = window.container;
        let base = window.start.0;
        let limit = window.limit.0;
        let sig = self.tree.sig_children(container);
        let open_offset = {
            let found = sig.get(base .. limit)?;
            core::convert::identity(found)
        }
        .iter()
        .position(|&node| self.tree.tile_label(node) == Some(open))?;
        let open_index = base.saturating_add(open_offset);
        // A matching close, or — when the melder ghost-closed an unterminated
        // delimiter (a `#!{ … ` / `( … ` / `{ … ` with no closer, whose repair
        // is a trailing grout leaf) — the window's last significant child, so
        // the body still recovers instead of collapsing to nothing. Only
        // triggers on genuinely-unclosed input; a matched delimiter is
        // unaffected.
        let close_index = self
            .matching_close((&sig).into(), SignificantIndex(open_index), open, close)
            .map_or_else(|| limit.saturating_sub(1).max(open_index), |index| index.0);
        Some((container, Span {
            start: open_index,
            end: close_index.saturating_add(1),
        }))
    }

    /// The backing container and the raw `[base, limit)` significant-child
    /// window this view addresses.
    fn raw_container_window(self) -> SignificantWindow
    {
        match self.at {
            | At::Node(id) | At::Forced { id, .. } => {
                let sig = self.tree.sig_children(id);
                SignificantWindow {
                    container: id,
                    start: SignificantIndex(0),
                    limit: SignificantIndex(sig.len()),
                }
            },
            | At::Run {
                container, lo, hi, ..
            } => SignificantWindow {
                container,
                start: SignificantIndex(lo),
                limit: SignificantIndex(hi),
            },
        }
    }

    /// The backing container and the `[base, limit)` significant-child window
    /// this view addresses. Item/member windows start *after* any leading
    /// `@[ … ]` attribute-block prefix, so a delimited-span scan (e.g. a
    /// `def`'s `parameters` paren) never matches inside an attribute
    /// payload's own brackets (`melder-CST migration` gap 6). Synthetic runs
    /// keep their absolute container indices, so module-member fields skip
    /// only attributes within that member's source span. An
    /// `attribute_block` run is itself the delimiter payload, so it keeps
    /// its opener visible.
    fn container_window(self) -> SignificantWindow
    {
        let window = self.raw_container_window();
        let container = window.container;
        let start = window.start.0;
        let limit = window.limit.0;
        let sig = self.tree.sig_children(container);
        let base = match self.at {
            | At::Run { kind, .. } if kind == node_kinds::ATTRIBUTE_BLOCK => window.start,
            | _ => self
                .after_attribute_blocks_between((&sig).into(), window.start, window.limit)
                .unwrap_or(window.start),
        };
        SignificantWindow {
            container,
            start: base,
            limit: SignificantIndex(limit.max(start)),
        }
    }

    /// The significant-child index of the `close` tile matching the `open` at
    /// `open_index`.
    ///
    /// Both `{` and `#{` share `}`. A scan for either opener therefore counts
    /// both spellings, so a record nested inside a block cannot close the block
    /// early (and conversely for a block nested inside a record).
    ///
    /// # Contract
    /// - requires: `open_index` names an `open` tile in `sig`.
    /// - ensures: returns the matching `close` index, counting both brace
    ///   opener spellings whenever they share `}`; returns `None` if
    ///   unbalanced.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: a single depth counter over every opener sharing the close
    ///   spelling prevents record and block interiors from closing each other.
    /// - mutants: count only `open`; return the first close.
    /// - witnesses: `recognizes_one_level_nested_module_member`.
    fn matching_close(
        self,
        sig: SignificantChildren<'_>,
        open_index: SignificantIndex,
        open: TileSpelling,
        close: TileSpelling,
    ) -> Option<SignificantIndex>
    {
        let mut depth = 0_usize;
        for (offset, node) in sig.iter().enumerate().skip(open_index.0) {
            let text = self.tree.tile_label(*node);
            let opens = text == Some(open)
                || (close == label::RBRACE
                    && matches!(text, Some(label::LBRACE | label::HASH_BRACE)));
            if opens {
                depth = depth.saturating_add(1);
            }
            else if text == Some(close) {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(SignificantIndex(offset));
                }
            }
        }
        None
    }

    /// Return the first significant-child index in `[start, end)` whose tile
    /// label is `needle`.
    fn find_tile(
        self,
        sig: SignificantChildren<'_>,
        start: SignificantIndex,
        end: SignificantIndex,
        needle: TileSpelling,
    ) -> Option<SignificantIndex>
    {
        let mut index = start.0;
        while index < end.0 {
            if sig
                .get(index)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(needle))
            {
                return Some(SignificantIndex(index));
            }
            index = index.saturating_add(1);
        }
        None
    }

    /// Whether a tile labelled `needle` appears at the top level of this node's
    /// significant children (depth `<= 1`, i.e. not nested inside an inner
    /// bracket pair).
    fn has_top_level(
        self,
        sig: SignificantChildren<'_>,
        needle: TileSpelling,
    ) -> TilePresence
    {
        let mut depth = 0_usize;
        for node in sig.iter() {
            match self.tree.tile_label(*node) {
                | Some(label::LPAREN | label::LBRACE | label::LBRACKET | label::HASH_BRACE) => {
                    depth = depth.saturating_add(1);
                },
                | Some(label::RPAREN | label::RBRACE) => depth = depth.saturating_sub(1),
                | Some(text) if depth <= 1 && text == needle => {
                    return TilePresence(true);
                },
                | _ => {},
            }
        }
        TilePresence(false)
    }

    /// The index just past a leading `@[ … ]` attribute-block prefix, or `None`
    /// when there is none.
    fn after_attribute_blocks(
        self,
        sig: SignificantChildren<'_>,
    ) -> Option<SignificantIndex>
    {
        self.after_attribute_blocks_between(sig, SignificantIndex(0), SignificantIndex(sig.len()))
    }

    /// The absolute significant-child index just past a leading `@[ … ]`
    /// prefix in `[start, end)`, or `None` when there is none.
    fn after_attribute_blocks_between(
        self,
        sig: SignificantChildren<'_>,
        start: SignificantIndex,
        end: SignificantIndex,
    ) -> Option<SignificantIndex>
    {
        let mut index = start.0;
        let mut advanced = false;
        while index < end.0
            && sig
                .get(index)
                .is_some_and(|&node| self.tree.tile_label(node) == Some(label::AT_BRACKET))
        {
            let close = self
                .matching_close(
                    sig,
                    SignificantIndex(index),
                    label::AT_BRACKET,
                    label::CLOSE_BRACKET,
                )
                .filter(|close| close.0 < end.0)?;
            index = close.0.saturating_add(1);
            advanced = true;
        }
        advanced.then_some(SignificantIndex(index))
    }

    /// The `[start, end)` byte range of a run.
    fn run_range(
        self,
        container: NodeId,
        lo: SignificantIndex,
        hi: SignificantIndex,
    ) -> SourceRange
    {
        let sig = self.tree.sig_children(container);
        let start = sig.get(lo.0).map_or(0, |&id| self.tree.range(id).start);
        let end =
            hi.0.checked_sub(1)
                .and_then(|last| sig.get(last))
                .map_or(start, |&id| self.tree.range(id).end);
        SourceRange(start .. end)
    }

    /// Whether this node is a grout leaf (the melder's convex/ghost repair for
    /// a missing tile).
    fn is_grout_leaf(self) -> GroutLeafFlag
    {
        GroutLeafFlag(self.backing_id().is_some_and(|id| {
            matches!(self.tree.material(id), Some(Material::Grout))
                && matches!(self.tree.node_kind(id), Some(NodeKind::Token))
        }))
    }

    /// Whether `id`'s subtree contains any grout leaf.
    fn tree_has_grout(
        self,
        id: NodeId,
    ) -> GroutPresence
    {
        if matches!(self.tree.material(id), Some(Material::Grout))
            && matches!(self.tree.node_kind(id), Some(NodeKind::Token))
        {
            return GroutPresence(true);
        }
        GroutPresence(
            self.tree
                .cst
                .children(id)
                .is_ok_and(|children| children.iter().any(|&child| self.tree_has_grout(child).0)),
        )
    }

    /// Construct a synthetic run node.
    fn run(
        self,
        container: NodeId,
        lo: SignificantIndex,
        hi: SignificantIndex,
        kind: SyntaxKind,
    ) -> Self
    {
        Self {
            tree: self.tree,
            at: At::Run {
                container,
                lo: lo.0,
                hi: hi.0,
                kind,
            },
        }
    }
}

/// A `[start, end)` half-open significant-child span.
#[derive(Clone, Copy)]
struct Span
{
    /// Inclusive start index.
    start: usize,
    /// Exclusive end index.
    end: usize,
}

/// The shell-block kind a pipeline / control operator tile induces on the
/// enclosing `;`-segment, or `None` for a non-operator tile. The kinds route
/// the segment to the lowerer's unsupported-shell path (`host-module surface`).
fn shell_op_kind(tile: Option<TileSpelling>) -> Option<SyntaxKind>
{
    match tile {
        | Some(label::SHELL_PIPE) => Some(node_kinds::PIPELINE),
        | Some(label::SHELL_AND) => Some(node_kinds::AND_EXPRESSION),
        | Some(label::SHELL_OR) => Some(node_kinds::OR_EXPRESSION),
        | _ => None,
    }
}

/// Whether a tile label is a shell redirection operator (`> >> < <& >& <>`),
/// which makes its enclosing command-part meld an out-of-fragment redirection.
fn is_shell_redirection(tile: Option<TileSpelling>) -> ShellRedirectionFlag
{
    ShellRedirectionFlag(matches!(
        tile,
        Some(TileSpelling(">" | ">>" | "<" | "<&" | ">&" | "<>" | "&>"))
    ))
}

/// The out-of-fragment statement kind a lead keyword induces, or `None` for a
/// keyword that does not lead one (so the run stays an expression statement).
/// These statement forms are flat runs in the block body, so the lead keyword
/// is the only discriminator; classifying them keeps the lowerer's
/// `Unsupported` rejection instead of a silent mis-lowering (`melder-CST
/// migration`).
fn unsupported_statement_kind(lead: TileSpelling) -> Option<SyntaxKind>
{
    match lead {
        | label::LETA => Some(node_kinds::LETA_STATEMENT),
        | label::RECV => Some(node_kinds::RECV_STATEMENT),
        | label::ACQUIRE => Some(node_kinds::ACQUIRE_STATEMENT),
        | label::RELEASE => Some(node_kinds::RELEASE_STATEMENT),
        | label::FORK => Some(node_kinds::FORK_STATEMENT),
        | _ => None,
    }
}

/// Whether a tile label names a "named terminal" — an identifier, literal, or
/// atom the lowerer treats as its own node — rather than punctuation, a
/// keyword, or an operator.
fn is_named_terminal(label: TileSpelling) -> NamedTerminalFlag
{
    NamedTerminalFlag(
        matches!(
            label,
            TileSpelling(
                "identifier"
                    | "number"
                    | "typed_number"
                    | "constructor"
                    | "type_identifier"
                    | "type_variable"
                    | "character"
                    | "hole_name"
                    | "_"
                    | "true"
                    | "false"
                    | "ω"
            )
        ) || PRIMITIVE_TYPES.contains(&label.as_ref()),
    )
}

/// The host-expression kind of a shell word that appeared between `$(` and `)`.
fn host_escape_shell_word_kind(text: SourceSlice<'_>) -> Option<SyntaxKind>
{
    if text.as_ref() == "true" || text.as_ref() == "false" {
        Some(node_kinds::BOOLEAN)
    }
    else if text
        .as_ref()
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_digit)
    {
        if has_primitive_numeric_suffix(text).0 {
            Some(node_kinds::TYPED_NUMBER)
        }
        else {
            Some(node_kinds::NUMBER)
        }
    }
    else if is_lower_identifier_word(text).0 {
        Some(node_kinds::IDENTIFIER)
    }
    else {
        None
    }
}

/// Whether a shell-word numeric spelling ends in one primitive type suffix.
fn has_primitive_numeric_suffix(text: SourceSlice<'_>) -> NumericSuffixFlag
{
    NumericSuffixFlag(
        ["u32", "u64", "i32", "i64", "f32", "f64"]
            .iter()
            .any(|suffix| {
                text.as_ref()
                    .strip_suffix(suffix)
                    .is_some_and(|digits| !digits.is_empty())
            }),
    )
}

/// Whether `text` is a lowercase host identifier shell-word spelling.
fn is_lower_identifier_word(text: SourceSlice<'_>) -> LowerIdentifierFlag
{
    let mut bytes = text.as_ref().bytes();
    let Some(first) = bytes.next()
    else {
        return LowerIdentifierFlag(false);
    };
    LowerIdentifierFlag(
        (first.is_ascii_lowercase() || first == b'_')
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
    )
}

#[cfg(test)]
mod tests
{
    use super::*;
    #[test]
    fn recognizes_case_and_constructor_pattern() -> Result<(), String>
    {
        let source = tree("def k = case v { Inl(x) => ret x, Inr(y) => ret y };")?;
        let case = field(
            {
                let item0 = item0(&source)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::CASE_EXPRESSION, case.kind(), "case");
        assert_eq!(
            "v",
            {
                let field = field(case, "value")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "scrutinee"
        );
        let arms = case.named_children();
        assert_eq!(2, arms.len(), "two arms");
        let Some(first_arm) = arms.first()
        else {
            return Err("first arm present".to_owned());
        };
        assert_eq!(node_kinds::ARM, first_arm.kind(), "arm kind");
        let pattern = field(*first_arm, "pattern")?;
        assert_eq!(
            node_kinds::CONSTRUCTOR_PATTERN,
            pattern.kind(),
            "arm pattern is a constructor"
        );
        assert_eq!(
            "Inl",
            {
                let field = field(pattern, "constructor")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "constructor name"
        );
        assert_eq!(
            1,
            pattern.children_by_field_name("argument").len(),
            "one constructor argument"
        );
        assert_eq!(
            node_kinds::RET_EXPRESSION,
            {
                let field = field(*first_arm, "body")?;
                core::convert::identity(field)
            }
            .kind(),
            "arm body"
        );
        Ok(())
    }
    #[test]
    fn recognizes_atoms_and_wrappers() -> Result<(), String>
    {
        def_value(
            &{
                let tree = tree("def b = true;")?;
                core::convert::identity(tree)
            },
            node_kinds::BOOLEAN,
        )?;
        def_value(
            &{
                let tree = tree("def u = ();")?;
                core::convert::identity(tree)
            },
            node_kinds::UNIT,
        )?;
        def_value(
            &{
                let tree = tree("def s = \"hi\";")?;
                core::convert::identity(tree)
            },
            node_kinds::STRING,
        )?;
        def_value(
            &{
                let tree = tree("def tp = (a, b, c);")?;
                core::convert::identity(tree)
            },
            node_kinds::TUPLE_EXPRESSION,
        )?;
        def_value(
            &{
                let tree = tree("def ls = [1, 2, 3];")?;
                core::convert::identity(tree)
            },
            node_kinds::LIST_EXPRESSION,
        )?;
        def_value(
            &{
                let tree = tree("def tn = 8080u32;")?;
                core::convert::identity(tree)
            },
            node_kinds::TYPED_NUMBER,
        )?;
        def_value(
            &{
                let lifted_value = tree("def lm = fn(x: Integer) { ret x };")?;
                core::convert::identity(lifted_value)
            },
            node_kinds::LAMBDA_EXPRESSION,
        )?;
        def_value(
            &{
                let tree = tree("def th = thunk { ret 1 };")?;
                core::convert::identity(tree)
            },
            node_kinds::THUNK_EXPRESSION,
        )?;
        def_value(
            &{
                let tree = tree("def forced = force t;")?;
                core::convert::identity(tree)
            },
            node_kinds::FORCE_EXPRESSION,
        )?;

        let boolean_owned = tree("def b = true;")?;
        let boolean = field(
            {
                let item0 = item0(&boolean_owned)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        let Some(token) = boolean.child(SignificantIndex(0))
        else {
            return Err("boolean has a true/false child".to_owned());
        };
        assert_eq!(
            node_kinds::TRUE,
            token.kind(),
            "boolean child is the true token"
        );
        Ok(())
    }
    #[test]
    fn recognizes_type_forms() -> Result<(), String>
    {
        /// The `type` of a `def id : T;` signature.
        fn sig_type<'source>(
            src: impl Into<crate::boundary::PipelineSource<'source>>
        ) -> Result<(SynTree, SyntaxKind), String>
        {
            let owned = tree(src)?;
            let kind = {
                let item0 = item0(&owned)?;
                let item0 = field(item0, "type")?;
                core::convert::identity(item0)
            }
            .kind();
            Ok((owned, kind))
        }

        assert_eq!(
            node_kinds::PRIMITIVE_TYPE,
            {
                let sig_type = sig_type("def p : Integer;")?;
                core::convert::identity(sig_type)
            }
            .1,
            "primitive"
        );
        assert_eq!(
            node_kinds::TYPE_IDENTIFIER,
            {
                let sig_type = sig_type("def a : A;")?;
                core::convert::identity(sig_type)
            }
            .1,
            "type identifier"
        );
        assert_eq!(
            node_kinds::FUNCTION_TYPE,
            {
                let sig_type = sig_type("def f : A -> B;")?;
                core::convert::identity(sig_type)
            }
            .1,
            "arrow"
        );
        assert_eq!(
            node_kinds::PRODUCT_TYPE,
            {
                let sig_type = sig_type("def pr : A * B;")?;
                core::convert::identity(sig_type)
            }
            .1,
            "product"
        );
        assert_eq!(
            node_kinds::SUM_TYPE,
            {
                let sig_type = sig_type("def su : A + B;")?;
                core::convert::identity(sig_type)
            }
            .1,
            "sum"
        );
        assert_eq!(
            node_kinds::LAZY_PRODUCT_TYPE,
            {
                let sig_type = sig_type("def lp : A & B;")?;
                core::convert::identity(sig_type)
            }
            .1,
            "lazy product"
        );
        assert_eq!(
            node_kinds::F_TYPE,
            {
                let sig_type = sig_type("def ft : F Integer;")?;
                core::convert::identity(sig_type)
            }
            .1,
            "returner type"
        );
        assert_eq!(
            node_kinds::RECORD_TYPE,
            {
                let sig_type = sig_type("def rt : #{ a : Integer };")?;
                core::convert::identity(sig_type)
            }
            .1,
            "record type"
        );
        assert_eq!(
            node_kinds::TYPE_APPLICATION,
            {
                let list = sig_type("def ta : List(Integer);")?;
                core::convert::identity(list)
            }
            .1,
            "application"
        );

        let arrow_owned = tree("def f : A -> B;")?;
        let arrow = field(
            {
                let item0 = item0(&arrow_owned)?;
                core::convert::identity(item0)
            },
            "type",
        )?;
        assert_eq!(
            "A",
            {
                let field = field(arrow, "parameter")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "arrow parameter"
        );
        assert_eq!(
            "B",
            {
                let field = field(arrow, "result")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "arrow result"
        );

        // The non-associative type `*` yields a clean product only at two
        // members; `A * B * C` mis-parses (the melder buffers obligations), so
        // the flattening is exercised at the clean two-member shape.
        let product_owned = tree("def pr : A * B;")?;
        let product = field(
            {
                let item0 = item0(&product_owned)?;
                core::convert::identity(item0)
            },
            "type",
        )?;
        assert_eq!(
            2,
            product.children_by_field_name("member").len(),
            "the product flattens to its two members"
        );
        Ok(())
    }
    #[test]
    fn recognizes_multiple_items_and_recovery() -> Result<(), String>
    {
        let many = tree("def a = 1;\ndef b = 2;\ndef c = 3;")?;
        assert_eq!(3, many.root().named_children().len(), "three items");

        // A missing operand leaves a grout repair the adapter reports as an
        // error region under the enclosing expression.
        let damaged = tree("def x = ret (a + );")?;
        let value = field(
            {
                let item0 = item0(&damaged)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert!(
            value.has_error(),
            "the value subtree carries the missing-operand repair"
        );
        assert!(
            !damaged.obligations().is_empty(),
            "the parse buffered an obligation"
        );
        Ok(())
    }
    #[test]
    fn recognizes_nullary_and_saturated_constructor_patterns() -> Result<(), String>
    {
        // A bare nullary constructor pattern (`Nil`) and a saturated one
        // (`Cons(h, t)`) both classify as constructor patterns, recover the
        // constructor name, and count their arguments (`melder-CST migration` gap 4).
        let source = tree("def k = case v { Nil => ret 0, Cons(h, t) => ret 1 };")?;
        let arms = {
            let item0 = item0(&source)?;
            let item0 = field(item0, "value")?;
            core::convert::identity(item0)
        }
        .named_children();
        let Some(nil_arm) = arms.first()
        else {
            return Err("nil arm present".to_owned());
        };
        let nil = field(*nil_arm, "pattern")?;
        assert_eq!(
            node_kinds::CONSTRUCTOR_PATTERN,
            nil.kind(),
            "the nullary pattern is a constructor pattern"
        );
        assert_eq!(
            "Nil",
            {
                let field = field(nil, "constructor")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "nullary constructor"
        );
        assert_eq!(
            0,
            nil.children_by_field_name("argument").len(),
            "a nullary constructor binds nothing"
        );
        let Some(cons_arm) = arms.get(1)
        else {
            return Err("cons arm present".to_owned());
        };
        let cons = field(*cons_arm, "pattern")?;
        assert_eq!(
            "Cons",
            {
                let field = field(cons, "constructor")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "cons constructor"
        );
        assert_eq!(
            2,
            cons.children_by_field_name("argument").len(),
            "`Cons` binds two arguments"
        );
        Ok(())
    }
    #[test]
    fn shell_control_operators_route_to_unsupported_kinds() -> Result<(), String>
    {
        let pipeline = tree("#!{ printf 'x' | cat; }")?;
        let commands = {
            let item0 = item0(&pipeline)?;
            core::convert::identity(item0)
        }
        .named_children();
        assert_eq!(
            Some(node_kinds::PIPELINE),
            commands.first().copied().map(SynNode::kind),
            "a pipeline segment presents as PIPELINE so the lowerer rejects it"
        );
        let control = tree("#!{ true && false; }")?;
        let control_commands = {
            let item0 = item0(&control)?;
            core::convert::identity(item0)
        }
        .named_children();
        assert_eq!(
            Some(node_kinds::AND_EXPRESSION),
            control_commands.first().copied().map(SynNode::kind),
            "an `&&` segment presents as AND_EXPRESSION"
        );
        Ok(())
    }
    #[test]
    fn shell_host_escape_multiple_interiors_have_no_field() -> Result<(), String>
    {
        let source = tree("#!{ printf '%s' $(1 2); }")?;
        let commands = {
            let item0 = item0(&source)?;
            core::convert::identity(item0)
        }
        .named_children();
        let command = nth(&commands, 0)?;
        let parts = command.named_children();
        let host = parts
            .iter()
            .copied()
            .find(|part| part.kind() == node_kinds::HOST_ESCAPE)
            .ok_or_else(|| "host escape part is present".to_owned())?;
        assert!(
            host.child_by_field_name("expression").is_none(),
            "multiple interior expressions must not project one preferred field"
        );
        Ok(())
    }

    /// Parse `src` into a borrowable tree, mapping any commit failure to a
    /// message.
    fn tree<'source>(
        src: impl Into<crate::boundary::PipelineSource<'source>>
    ) -> Result<SynTree, String>
    {
        let src = src.into();
        SynTree::parse(src.0).map_err(|error| format!("{:?} must parse: {error:?}", src.0))
    }
    #[test]
    fn recognizes_codata_declaration() -> Result<(), String>
    {
        let t = tree("codata Point { x: Integer, y: Integer }")?;
        let item = item0(&t)?;
        assert_eq!(
            node_kinds::CODATA_DECLARATION,
            item.kind(),
            "codata block classifies"
        );
        assert_eq!(
            "Point",
            {
                let field = field(item, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "codata name"
        );
        let obs = item.named_children();
        assert_eq!(2, obs.len(), "two observation members");
        assert_eq!(
            node_kinds::CODATA_OBSERVATION,
            {
                let nth = nth(&obs, 0)?;
                core::convert::identity(nth)
            }
            .kind(),
            "observation classifies"
        );
        assert_eq!(
            "x",
            {
                let nth = nth(&obs, 0)?;
                let nth = field(nth, "name")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "first observation name"
        );
        assert_eq!(
            "Integer",
            {
                let nth = nth(&obs, 0)?;
                let nth = field(nth, "type")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "first observation result type"
        );
        assert!(
            !{
                let nth = nth(&obs, 0)?;
                core::convert::identity(nth)
            }
            .is_reserved_observation(),
            "a plain `π : B` observation is not reserved"
        );
        assert_eq!(
            "y",
            {
                let nth = nth(&obs, 1)?;
                let nth = field(nth, "name")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "second observation name"
        );

        let reserved = tree("codata Session { 1 step: F(Unit), rule tail(x) ==> x }")?;
        let members = {
            let item0 = item0(&reserved)?;
            core::convert::identity(item0)
        }
        .named_children();
        assert!(
            {
                let nth = nth(&members, 0)?;
                core::convert::identity(nth)
            }
            .is_reserved_observation(),
            "a graded observation is a reserved slot"
        );
        assert!(
            {
                let nth = nth(&members, 1)?;
                core::convert::identity(nth)
            }
            .is_reserved_observation(),
            "a rule 2-cell member is a reserved slot"
        );
        Ok(())
    }
    #[test]
    fn recognizes_def_rec_copattern_body() -> Result<(), String>
    {
        let t = tree("def rec origin() -> Point { .x => 0, .y => 0 }")?;
        let item = item0(&t)?;
        assert_eq!(node_kinds::DEF_REC, item.kind(), "def rec classifies");
        assert_eq!(
            "origin",
            {
                let field = field(item, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "def rec name"
        );
        assert_eq!(
            "Point",
            {
                let field = field(item, "result")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "def rec result type"
        );
        assert!(
            item.def_rec_has_copattern_body(),
            "a `.π =>` body is a copattern body"
        );
        let clauses = item.children_by_field_name(node_kinds::FIELD_CLAUSE);
        assert_eq!(2, clauses.len(), "two copattern clauses");
        assert_eq!(
            "x",
            {
                let nth = nth(&clauses, 0)?;
                let nth = field(nth, "observation")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "first clause observation"
        );
        assert_eq!(
            "0",
            {
                let nth = nth(&clauses, 0)?;
                let nth = field(nth, "body")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "first clause body"
        );
        assert_eq!(
            "y",
            {
                let nth = nth(&clauses, 1)?;
                let nth = field(nth, "observation")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "second clause observation"
        );

        let stmt = tree("def rec fact(n: Integer) -> F Integer { ret 1 }")?;
        assert!(
            !{
                let item0 = item0(&stmt)?;
                core::convert::identity(item0)
            }
            .def_rec_has_copattern_body(),
            "a statement body is not a copattern body"
        );

        let mutual = tree(concat!(
            "rec { ",
            "def even(n: Integer) -> F Integer { ret odd[<](n) } ",
            "def odd(n: Integer) -> F Integer { ret even[<](n) }",
            " }",
        ))?;
        let group = item0(&mutual)?;
        assert_eq!(node_kinds::REC_BLOCK, group.kind(), "rec block classifies");
        assert_eq!(
            2,
            group.children_by_field_name(node_kinds::FIELD_MEMBER).len(),
            "rec block exposes both definitions"
        );

        // A nested `,` inside a clause body's call hides in that call's own
        // meld, so the clause segmentation sees only the top-level separators.
        let nested = tree("def rec nats(n: Integer) -> S { .head => n, .tail => f(g(n, 1)) }")?;
        let deep = {
            let item0 = item0(&nested)?;
            core::convert::identity(item0)
        }
        .children_by_field_name(node_kinds::FIELD_CLAUSE);
        assert_eq!(2, deep.len(), "nested comma does not over-split clauses");
        assert_eq!(
            "f(g(n, 1))",
            {
                let nth = nth(&deep, 1)?;
                let nth = field(nth, "body")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "clause body meld"
        );
        Ok(())
    }

    /// The `value` field of a `def id = <expr>;` item's expression.
    fn def_value(
        tree: &SynTree,
        expected: SyntaxKind,
    ) -> Result<(), String>
    {
        let item = item0(tree)?;
        assert_eq!(
            node_kinds::DEF_VALUE,
            item.kind(),
            "leading item is a def_value"
        );
        let value = field(item, "value")?;
        assert_eq!(value.kind(), expected, "def value classifies");
        Ok(())
    }
    #[test]
    fn recognizes_block_statement_segmentation() -> Result<(), String>
    {
        let source = tree("def m = { val a = 1; run z <- act; a; ret a };")?;
        let block = field(
            {
                let item0 = item0(&source)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::BLOCK, block.kind(), "block value");
        let children = block.named_children();
        assert_eq!(4, children.len(), "three statements and a tail");
        let kinds: Vec<SyntaxKind> = children.iter().map(|child| child.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                node_kinds::LET_STATEMENT,
                node_kinds::BIND_STATEMENT,
                node_kinds::EXPRESSION_STATEMENT,
                node_kinds::RET_EXPRESSION,
            ],
            "statements segment on `;` and the tail is a bare expression"
        );
        let Some(value_stmt) = children.first()
        else {
            return Err("value statement present".to_owned());
        };
        assert_eq!(
            "a",
            {
                let field = field(*value_stmt, "pattern")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "value binder"
        );
        assert_eq!(
            node_kinds::NUMBER,
            {
                let field = field(*value_stmt, "value")?;
                core::convert::identity(field)
            }
            .kind(),
            "statement value"
        );
        let Some(bind_stmt) = children.get(1)
        else {
            return Err("bind statement present".to_owned());
        };
        assert_eq!(
            "z",
            {
                let field = field(*bind_stmt, "pattern")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "bind binder"
        );
        assert_eq!(
            "act",
            {
                let field = field(*bind_stmt, "source")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "bind source"
        );
        Ok(())
    }
    #[test]
    fn recognizes_binary_precedence_nesting() -> Result<(), String>
    {
        let source = tree("def s = a + b * c;")?;
        let sum = field(
            {
                let item0 = item0(&source)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::BINARY_EXPRESSION, sum.kind(), "outer is a sum");
        assert_eq!(
            "a",
            {
                let field = field(sum, "left")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "left operand"
        );
        let product = field(sum, "right")?;
        assert_eq!(
            node_kinds::BINARY_EXPRESSION,
            product.kind(),
            "the product binds tighter and nests on the right"
        );
        assert_eq!(
            "b",
            {
                let field = field(product, "left")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "product left"
        );
        assert_eq!(
            "c",
            {
                let field = field(product, "right")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "product right"
        );
        Ok(())
    }
    #[test]
    fn recognizes_projection_and_call() -> Result<(), String>
    {
        let proj = tree("def p = ret r.fst.snd;")?;
        // `r.fst.snd` nests left: (r.fst).snd.
        let ret = field(
            {
                let item0 = item0(&proj)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::RET_EXPRESSION, ret.kind(), "ret leads");

        let call = tree("def c = f(a, b);")?;
        let call_node = field(
            {
                let item0 = item0(&call)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::CALL_EXPRESSION, call_node.kind(), "call");
        assert_eq!(
            "f",
            {
                let field = field(call_node, "function")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "call head"
        );
        let args = call_node.children_by_field_name(node_kinds::FIELD_ARGUMENT);
        assert_eq!(2, args.len(), "two call arguments");
        Ok(())
    }
    #[test]
    fn recognizes_instantiation_residents() -> Result<(), String>
    {
        let marked = tree("def c = f[<, >](a);")?;
        let root_item = item0(&marked)?;
        let call = field(root_item, "value")?;
        let instantiation = field(call, "function")?;
        assert_eq!(
            node_kinds::INSTANTIATION_EXPRESSION,
            instantiation.kind(),
            "the bracketed postfix is an instantiation expression"
        );
        let target = field(instantiation, "target")?;
        assert_eq!(
            "f",
            target.text().as_ref(),
            "the instantiation preserves its target"
        );
        let residents = instantiation.children_by_field_name(node_kinds::FIELD_INSTANTIATION);
        assert_eq!(2, residents.len(), "the comma list exposes two residents");
        assert_eq!("<", residents[0].text().as_ref());
        assert_eq!(">", residents[1].text().as_ref());
        Ok(())
    }
    #[test]
    fn recognizes_if_branches() -> Result<(), String>
    {
        let source = tree("def i = if c { ret 1 } else { ret 2 };")?;
        let cond = field(
            {
                let item0 = item0(&source)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::IF_EXPRESSION, cond.kind(), "if");
        assert_eq!(
            "c",
            {
                let field = field(cond, "condition")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "condition"
        );
        assert_eq!(
            node_kinds::BLOCK,
            {
                let field = field(cond, "consequence")?;
                core::convert::identity(field)
            }
            .kind(),
            "consequence block"
        );
        assert_eq!(
            node_kinds::BLOCK,
            {
                let field = field(cond, "alternative")?;
                core::convert::identity(field)
            }
            .kind(),
            "alternative block"
        );
        Ok(())
    }
    #[test]
    fn recognizes_extern_strings_and_members() -> Result<(), String>
    {
        let source = tree("extern \"c\" from \"testlib\" { type Db; def cos(x: f64) -> f64; }")?;
        let item = item0(&source)?;
        assert_eq!(node_kinds::EXTERN_BLOCK, item.kind(), "extern block");
        assert_eq!(
            "\"c\"",
            {
                let field = field(item, "abi")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "abi string (quoted)"
        );
        assert_eq!(
            "\"testlib\"",
            {
                let field = field(item, "library")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "library string (quoted)"
        );
        let members = item.named_children();
        assert_eq!(2, members.len(), "one type and one function member");
        let kinds: Vec<SyntaxKind> = members.iter().map(|member| member.kind()).collect();
        assert_eq!(kinds, vec![
            node_kinds::EXTERN_TYPE,
            node_kinds::EXTERN_FUNCTION
        ]);
        Ok(())
    }
    #[test]
    fn recognizes_explicit_grades() -> Result<(), String>
    {
        // A thunk with an explicit grade groups cleanly (`thunk[r] { … }`).
        let thunk_owned = tree("def th = thunk[2] { ret 1 };")?;
        let thunk = field(
            {
                let item0 = item0(&thunk_owned)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::THUNK_EXPRESSION, thunk.kind(), "graded thunk");
        let grade = field(thunk, "grade")?;
        assert_eq!("2", grade.text().as_ref(), "the grade text survives");
        assert_eq!(
            Some(node_kinds::NUMBER),
            grade.child(SignificantIndex(0)).map(SynNode::kind),
            "the grade's sole child is the numeral"
        );
        // An ungraded thunk has no grade field (the reader defaults to ω).
        let plain_owned = tree("def th = thunk { ret 1 };")?;
        let plain = field(
            {
                let item0 = item0(&plain_owned)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert!(
            plain.child_by_field_name("grade").is_none(),
            "an ungraded thunk carries no grade"
        );
        // A `U[r]` thunk-type prefix carries the same grade annotation.
        let u_owned = tree("def u : U[1] Integer;")?;
        let u_ty = field(
            {
                let item0 = item0(&u_owned)?;
                core::convert::identity(item0)
            },
            "type",
        )?;
        assert_eq!(node_kinds::U_TYPE, u_ty.kind(), "graded thunk type");
        assert_eq!(
            "1",
            {
                let field = field(u_ty, "grade")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "U grade text survives"
        );
        Ok(())
    }
    #[test]
    fn recognizes_shell_commands_and_parts() -> Result<(), String>
    {
        let source = tree("#!{ echo hi 'a b'; printf \"x\"; }")?;
        let shell = item0(&source)?;
        assert_eq!(node_kinds::SHELL_BLOCK, shell.kind(), "shell block");
        let commands = shell.named_children();
        assert_eq!(2, commands.len(), "two `;`-separated commands");
        let Some(first) = commands.first()
        else {
            return Err("first command present".to_owned());
        };
        assert_eq!(node_kinds::COMMAND, first.kind(), "command kind");
        let parts = first.named_children();
        let kinds: Vec<SyntaxKind> = parts.iter().map(|part| part.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                node_kinds::COMMAND_NAME,
                node_kinds::COMMAND_NAME,
                node_kinds::SINGLE_QUOTED_STRING,
            ],
            "the program, a bare argument, and a single-quoted string"
        );
        assert_eq!(
            Some("echo".to_owned()),
            parts
                .first()
                .copied()
                .map(|part| part.text().as_ref().to_owned()),
            "the program token"
        );
        let Some(second) = commands.get(1)
        else {
            return Err("second command present".to_owned());
        };
        let second_kinds: Vec<SyntaxKind> =
            second.named_children().iter().map(|p| p.kind()).collect();
        assert_eq!(second_kinds, vec![
            node_kinds::COMMAND_NAME,
            node_kinds::DOUBLE_QUOTED_STRING,
        ]);
        Ok(())
    }
    #[test]
    fn recognizes_ret_and_force_operands() -> Result<(), String>
    {
        // The returner and force operands are recovered by the `value` field
        // (`melder-CST migration` gap 1) — the sole named child after the lead keyword.
        let ret = tree("def r = ret x;")?;
        let ret_value = field(
            {
                let item0 = item0(&ret)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::RET_EXPRESSION, ret_value.kind(), "ret leads");
        assert_eq!(
            "x",
            {
                let field = field(ret_value, "value")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "ret operand"
        );

        let force = tree("def forced = force t;")?;
        let force_value = field(
            {
                let item0 = item0(&force)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(
            node_kinds::FORCE_EXPRESSION,
            force_value.kind(),
            "force leads"
        );
        assert_eq!(
            "t",
            {
                let field = field(force_value, "value")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "force operand"
        );
        Ok(())
    }
    #[test]
    fn recognizes_extern_function_result() -> Result<(), String>
    {
        // An `extern` member function recovers its `-> result` boundary type
        // (`melder-CST migration` gap 2); a result-less member has none.
        let source =
            tree("extern \"c\" from \"lib\" { def cos(x: f64) -> f64; def sink(x: f64); }")?;
        let members = {
            let item0 = item0(&source)?;
            core::convert::identity(item0)
        }
        .named_children();
        let Some(cos) = members.first()
        else {
            return Err("first extern member present".to_owned());
        };
        assert_eq!(node_kinds::EXTERN_FUNCTION, cos.kind(), "extern function");
        assert_eq!(
            "f64",
            {
                let field = field(*cos, "result")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "result boundary type"
        );
        let Some(sink) = members.get(1)
        else {
            return Err("second extern member present".to_owned());
        };
        assert!(
            sink.child_by_field_name("result").is_none(),
            "a result-less extern member has no result field"
        );
        Ok(())
    }
    #[test]
    fn recognizes_tuple_pattern_in_value_statement() -> Result<(), String>
    {
        // A paren-led pattern-sorted Meld with a top-level `,` is a tuple
        // pattern, not a tuple expression (`melder-CST migration` gap 3).
        let source = tree("def m = { val (x, y) = p; ret x };")?;
        let block = field(
            {
                let item0 = item0(&source)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        let Some(let_stmt) = block
            .named_children()
            .into_iter()
            .find(|child| child.kind() == node_kinds::LET_STATEMENT)
        else {
            return Err("let statement present".to_owned());
        };
        let pattern = field(let_stmt, "pattern")?;
        assert_eq!(
            node_kinds::TUPLE_PATTERN,
            pattern.kind(),
            "the binder is a tuple pattern"
        );
        let elements: Vec<String> = pattern
            .named_children()
            .iter()
            .map(|element| element.text().as_ref().to_owned())
            .collect();
        assert_eq!(elements, vec!["x", "y"], "the tuple pattern's two binders");
        Ok(())
    }
    #[test]
    fn shell_host_escape_projects_token_interior() -> Result<(), String>
    {
        for (source_text, expected_kind, expected_text) in [
            ("#!{ printf '%s' $(1); }", node_kinds::NUMBER, "1"),
            ("#!{ printf '%s' $(1e4); }", node_kinds::NUMBER, "1e4"),
            (
                "#!{ printf '%s' $(1e4f64); }",
                node_kinds::TYPED_NUMBER,
                "1e4f64",
            ),
        ] {
            let source = tree(source_text)?;
            let commands = {
                let item0 = item0(&source)?;
                core::convert::identity(item0)
            }
            .named_children();
            let command = nth(&commands, 0)?;
            assert_eq!(node_kinds::COMMAND, command.kind(), "simple command");
            let parts = command.named_children();
            let host = parts
                .iter()
                .copied()
                .find(|part| part.kind() == node_kinds::HOST_ESCAPE)
                .ok_or_else(|| "host escape part is present".to_owned())?;
            let expression = field(host, "expression")?;
            assert_eq!(expression.kind(), expected_kind, "interior kind");
            assert_eq!(
                expression.text().as_ref(),
                expected_text,
                "token-shaped interior projects as the host expression"
            );
        }
        Ok(())
    }

    /// The first top-level item under the file root.
    fn item0(tree: &SynTree) -> Result<SynNode<'_>, String>
    {
        tree.root()
            .named_children()
            .into_iter()
            .next()
            .ok_or_else(|| "the source has one item".to_owned())
    }
    #[test]
    fn recognizes_def_family() -> Result<(), String>
    {
        let value = tree("def x = 5;")?;
        let item = item0(&value)?;
        assert_eq!(node_kinds::DEF_VALUE, item.kind(), "def value");
        assert_eq!(
            "x",
            {
                let field = field(item, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "def name recovered"
        );
        assert_eq!(
            node_kinds::NUMBER,
            {
                let field = field(item, "value")?;
                core::convert::identity(field)
            }
            .kind(),
            "value is a number"
        );

        let sig = tree("def x : Integer;")?;
        let sig_item = item0(&sig)?;
        assert_eq!(node_kinds::DEF_SIGNATURE, sig_item.kind(), "def signature");
        assert_eq!(
            "x",
            {
                let field = field(sig_item, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "signature name"
        );
        assert_eq!(
            node_kinds::PRIMITIVE_TYPE,
            {
                let field = field(sig_item, "type")?;
                core::convert::identity(field)
            }
            .kind(),
            "signature type"
        );

        let func = tree("def add(a: Integer, b: Integer) -> Integer { ret a }")?;
        let func_item = item0(&func)?;
        assert_eq!(node_kinds::DEF_FUNCTION, func_item.kind(), "def function");
        assert_eq!(
            "add",
            {
                let field = field(func_item, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "function name"
        );
        let params = {
            let field = field(func_item, "parameters")?;
            core::convert::identity(field)
        }
        .named_children();
        assert_eq!(2, params.len(), "two parameters");
        let Some(first_param) = params.first()
        else {
            return Err("first parameter present".to_owned());
        };
        assert_eq!(node_kinds::PARAMETER, first_param.kind(), "parameter kind");
        assert_eq!(
            "a",
            {
                let field = field(*first_param, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "parameter name"
        );
        assert_eq!(
            node_kinds::PRIMITIVE_TYPE,
            {
                let field = field(*first_param, "type")?;
                core::convert::identity(field)
            }
            .kind(),
            "parameter type"
        );
        assert_eq!(
            node_kinds::PRIMITIVE_TYPE,
            {
                let field = field(func_item, "result")?;
                core::convert::identity(field)
            }
            .kind(),
            "function result type"
        );
        assert_eq!(
            node_kinds::BLOCK,
            {
                let field = field(func_item, "body")?;
                core::convert::identity(field)
            }
            .kind(),
            "function body"
        );
        Ok(())
    }
    #[test]
    fn recognizes_co_and_record_forms() -> Result<(), String>
    {
        let co = tree("def c = co { fst = ret 1, snd = ret 2 };")?;
        let co_node = field(
            {
                let item0 = item0(&co)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::CO_EXPRESSION, co_node.kind(), "co");
        let fields = co_node.named_children();
        assert_eq!(2, fields.len(), "two co fields");
        let Some(first) = fields.first()
        else {
            return Err("first co field present".to_owned());
        };
        assert_eq!(node_kinds::CO_FIELD, first.kind(), "co field kind");
        assert_eq!(
            "fst",
            {
                let field = field(*first, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "co field name"
        );

        let record = tree("def r = #{ a = 1, b = 2 };")?;
        let record_node = field(
            {
                let item0 = item0(&record)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(node_kinds::RECORD_EXPRESSION, record_node.kind(), "record");
        assert_eq!(2, record_node.named_children().len(), "two record fields");

        let update = tree("def u = #{ base | a = 1 };")?;
        let update_node = field(
            {
                let item0 = item0(&update)?;
                core::convert::identity(item0)
            },
            "value",
        )?;
        assert_eq!(
            node_kinds::RECORD_UPDATE_EXPRESSION,
            update_node.kind(),
            "record update"
        );
        assert_eq!(
            "base",
            {
                let field = field(update_node, "base")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "update base"
        );
        assert_eq!(1, update_node.named_children().len(), "one override field");
        Ok(())
    }
    #[test]
    fn recognizes_attribute_internals() -> Result<(), String>
    {
        let source = tree("@[doc(\"hi\"), deprecated] def x = 5;")?;
        let item = item0(&source)?;
        assert_eq!(node_kinds::DEF_VALUE, item.kind(), "attributed def_value");
        let blocks = item.children_by_field_name(node_kinds::FIELD_ATTRIBUTE);
        assert_eq!(1, blocks.len(), "one attribute block");
        let Some(block) = blocks.first()
        else {
            return Err("attribute block present".to_owned());
        };
        assert_eq!(
            node_kinds::ATTRIBUTE_BLOCK,
            block.kind(),
            "block classifies"
        );
        let attrs = block.named_children();
        assert_eq!(2, attrs.len(), "two attributes in the block");
        let Some(doc) = attrs.first()
        else {
            return Err("first attribute present".to_owned());
        };
        assert_eq!(node_kinds::ATTRIBUTE, doc.kind(), "attribute kind");
        assert_eq!(
            "doc",
            {
                let field = field(*doc, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "attribute name"
        );
        assert_eq!(
            node_kinds::STRING,
            {
                let field = field(*doc, "payload")?;
                core::convert::identity(field)
            }
            .kind(),
            "attribute payload is a string expression"
        );
        let Some(deprecated) = attrs.get(1)
        else {
            return Err("second attribute present".to_owned());
        };
        assert_eq!(
            "deprecated",
            {
                let field = field(*deprecated, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "bare marker name"
        );
        assert!(
            deprecated.child_by_field_name("payload").is_none(),
            "a bare marker has no payload"
        );
        Ok(())
    }
    #[test]
    fn recognizes_graded_thunk_type_argument() -> Result<(), String>
    {
        // The graded-thunk prefix `U` / `U[r]` melds as a bare sibling of its
        // argument; the `argument` field recovers the following sibling across
        // signature, parameter, and annotation positions (`melder-CST migration` gap
        // 5).
        let signature = tree("def u : U[1] (Integer -> F Integer);")?;
        let u_ty = field(
            {
                let item0 = item0(&signature)?;
                core::convert::identity(item0)
            },
            "type",
        )?;
        assert_eq!(node_kinds::U_TYPE, u_ty.kind(), "graded thunk type");
        assert_eq!(
            "1",
            {
                let field = field(u_ty, "grade")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "grade survives"
        );
        let argument = field(u_ty, "argument")?;
        assert_eq!(
            node_kinds::PARENTHESIZED_TYPE,
            argument.kind(),
            "the argument is the parenthesized sibling, not the grade bracket"
        );
        assert_eq!(
            "(Integer -> F Integer)",
            argument.text().as_ref(),
            "argument text"
        );

        // An ungraded `U (…)` recovers the same sibling argument.
        let ungraded = tree("def uu : U (F Integer);")?;
        let ungraded_ty = field(
            {
                let item0 = item0(&ungraded)?;
                core::convert::identity(item0)
            },
            "type",
        )?;
        assert!(
            ungraded_ty.child_by_field_name("grade").is_none(),
            "an ungraded U carries no grade"
        );
        assert_eq!(
            "(F Integer)",
            {
                let field = field(ungraded_ty, "argument")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "ungraded U argument"
        );

        // In parameter position the argument recovers identically.
        let parameter = tree("def g(f: U (Integer -> F Integer)) { ret 0 }")?;
        let params = {
            let item0 = item0(&parameter)?;
            let item0 = field(item0, "parameters")?;
            core::convert::identity(item0)
        }
        .named_children();
        let Some(param) = params.first()
        else {
            return Err("parameter present".to_owned());
        };
        let param_ty = field(*param, "type")?;
        assert_eq!(node_kinds::U_TYPE, param_ty.kind(), "parameter U type");
        assert_eq!(
            "(Integer -> F Integer)",
            {
                let field = field(param_ty, "argument")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "parameter U argument"
        );
        Ok(())
    }
    #[test]
    fn recognizes_attributed_def_function_parameters() -> Result<(), String>
    {
        // An attribute payload's own parens must not shadow the `def`'s
        // parameter parens: the scan skips the `@[ … ]` prefix (`melder-CST migration`
        // gap 6).
        let source = tree("@[doc(\"hi\")] def f(a: Integer) { ret a }")?;
        let item = item0(&source)?;
        assert_eq!(
            node_kinds::DEF_FUNCTION,
            item.kind(),
            "attributed def function"
        );
        assert_eq!(
            "f",
            {
                let field = field(item, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "def name past the attribute"
        );
        let params = field(item, "parameters")?;
        assert_eq!(
            "(a: Integer)",
            params.text().as_ref(),
            "parameters are the def's, not the attribute payload's"
        );
        let named = params.named_children();
        assert_eq!(1, named.len(), "one parameter");
        let Some(first) = named.first()
        else {
            return Err("parameter present".to_owned());
        };
        assert_eq!(
            "a",
            {
                let field = field(*first, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "parameter name"
        );
        Ok(())
    }

    /// The named child at `field`, or an error message.
    fn field(
        node: SynNode<'_>,
        name: impl Into<SyntaxField>,
    ) -> Result<SynNode<'_>, String>
    {
        let name = name.into();
        node.child_by_field_name(name)
            .ok_or_else(|| format!("{:?} must recover field {name}", node.kind()))
    }
    #[test]
    fn recognizes_module_declaration() -> Result<(), String>
    {
        let bare = tree("module M { def x = 1; def y : Integer; }")?;
        assert!(
            bare.obligations().is_empty(),
            "bare module molds cleanly: {:?}",
            bare.obligations()
        );
        let module = item0(&bare)?;
        assert_eq!(
            node_kinds::MODULE_DECLARATION,
            module.kind(),
            "module classifies"
        );
        assert_eq!(
            "M",
            {
                let field = field(module, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "module name"
        );
        assert!(
            module.child_by_field_name("ascription").is_none(),
            "bare module has no ascription"
        );
        let members = module.named_children();
        assert_eq!(2, members.len(), "two bare module members");
        assert_eq!(
            "x",
            {
                let nth = nth(&members, 0)?;
                let nth = field(nth, "name")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "first member name"
        );
        assert_eq!(
            node_kinds::DEF_VALUE,
            {
                let nth = nth(&members, 0)?;
                core::convert::identity(nth)
            }
            .kind(),
            "first member is a value definition"
        );
        assert_eq!(
            node_kinds::DEF_SIGNATURE,
            {
                let nth = nth(&members, 1)?;
                core::convert::identity(nth)
            }
            .kind(),
            "second member is a signature"
        );
        let repeated_kinds = module
            .children_by_field_name(node_kinds::FIELD_MEMBER)
            .iter()
            .map(|member| member.kind())
            .collect::<Vec<_>>();
        assert_eq!(
            repeated_kinds,
            vec![node_kinds::DEF_VALUE, node_kinds::DEF_SIGNATURE],
            "member field preserves source order"
        );

        let ascribed = tree(
            "module M : #{ x: Integer, f: F Integer } { \
             def x = 1; \
             @[doc({ ret 0 })] def f(a: Integer) -> F Integer { ret a } \
             }",
        )?;
        assert!(
            ascribed.obligations().is_empty(),
            "ascribed module molds cleanly: {:?}",
            ascribed.obligations()
        );
        let ascribed_module = item0(&ascribed)?;
        let ascription = field(ascribed_module, "ascription")?;
        assert_eq!(
            node_kinds::RECORD_TYPE,
            ascription.kind(),
            "module ascription is a record type"
        );
        let fields = ascription.named_children();
        assert_eq!(2, fields.len(), "two ascription fields");
        assert_eq!(
            "x",
            {
                let nth = nth(&fields, 0)?;
                let nth = field(nth, "name")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "first ascription field name"
        );
        assert_eq!(
            "F Integer",
            {
                let nth = nth(&fields, 1)?;
                let nth = field(nth, "type")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "second ascription field type"
        );
        let ascribed_members = ascribed_module.named_children();
        assert_eq!(2, ascribed_members.len(), "two ascribed module members");
        assert_eq!(
            "x",
            {
                let nth = nth(&ascribed_members, 0)?;
                let nth = field(nth, "name")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "first ascribed member remains first"
        );
        let function_member = nth(&ascribed_members, 1)?;
        assert_eq!(
            node_kinds::DEF_FUNCTION,
            function_member.kind(),
            "function member preserves its def kind"
        );
        assert_eq!(
            "f",
            {
                let field = field(function_member, "name")?;
                core::convert::identity(field)
            }
            .text()
            .as_ref(),
            "attributed second member remains second"
        );
        let attribute_blocks = function_member.children_by_field_name(node_kinds::FIELD_ATTRIBUTE);
        assert_eq!(
            1,
            attribute_blocks.len(),
            "module member exposes attribute block"
        );
        let attributes = {
            let nth = nth(&attribute_blocks, 0)?;
            core::convert::identity(nth)
        }
        .named_children();
        assert_eq!(
            "doc",
            {
                let nth = nth(&attributes, 0)?;
                let nth = field(nth, "name")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "member attribute name"
        );
        assert_eq!(
            "{ ret 0 }",
            {
                let nth = nth(&attributes, 0)?;
                let nth = field(nth, "payload")?;
                core::convert::identity(nth)
            }
            .text()
            .as_ref(),
            "attribute payload keeps delimiter-like syntax"
        );
        let params = field(function_member, "parameters")?;
        assert_eq!(
            "(a: Integer)",
            params.text().as_ref(),
            "function parameters are not the attribute payload"
        );
        let body = field(function_member, "body")?;
        assert_eq!(
            "{ ret a }",
            body.text().as_ref(),
            "function body is not the attribute payload block"
        );
        assert_eq!(
            node_kinds::BLOCK,
            body.kind(),
            "function body adapts as a block"
        );
        Ok(())
    }

    #[test]
    fn recognizes_one_level_nested_module_member() -> Result<(), String>
    {
        let parsed = tree(
            "module Outer : #{ before: Integer, inner: #{ answer: Integer }, after: Integer } { \
             def before = 0; \
             module inner : #{ answer: Integer } { def answer = 42; } \
             def after = inner.answer; \
             }",
        )?;
        assert!(
            parsed.obligations().is_empty(),
            "nested module molds cleanly: {:?}",
            parsed.obligations()
        );
        let outer = item0(&parsed)?;
        let members = outer.children_by_field_name(node_kinds::FIELD_MEMBER);
        assert_eq!(3, members.len(), "outer module has three members");
        assert_eq!(
            members
                .iter()
                .map(|member| member.kind())
                .collect::<Vec<_>>(),
            vec![
                node_kinds::DEF_VALUE,
                node_kinds::MODULE_DECLARATION,
                node_kinds::DEF_VALUE,
            ],
            "nested module remains in source order"
        );
        let inner = nth(&members, 1)?;
        assert_eq!(
            "inner",
            field(inner, node_kinds::FIELD_NAME)?.text().as_ref(),
            "nested module exposes its lowercase component name"
        );
        let ascription = field(inner, node_kinds::FIELD_ASCRIPTION)?;
        assert_eq!(
            node_kinds::RECORD_TYPE,
            ascription.kind(),
            "nested module exposes its inline structural signature"
        );
        let signature_fields = ascription.named_children();
        assert_eq!(1, signature_fields.len(), "one nested signature field");
        assert_eq!(
            "answer",
            field(nth(&signature_fields, 0)?, node_kinds::FIELD_NAME)?
                .text()
                .as_ref(),
            "nested signature field name"
        );
        let inner_members = inner.children_by_field_name(node_kinds::FIELD_MEMBER);
        assert_eq!(1, inner_members.len(), "one nested definition");
        assert_eq!(
            "answer",
            field(nth(&inner_members, 0)?, node_kinds::FIELD_NAME)?
                .text()
                .as_ref(),
            "nested member name"
        );
        Ok(())
    }

    /// The `index`-th element of `items`, or an error message (avoids the
    /// clippy `indexing_slicing` wall in tests).
    fn nth<'tree>(
        items: &[SynNode<'tree>],
        index: impl Into<SignificantIndex>,
    ) -> Result<SynNode<'tree>, String>
    {
        let index = index.into();
        items
            .get(index.0)
            .copied()
            .ok_or_else(|| format!("element {} of {} is present", index.0, items.len()))
    }
}
