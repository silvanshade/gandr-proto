//! Mold-driven syntax highlighting: the normative highlighter that replaces the
//! tree-sitter `highlights.scm` query engine.
//!
//! The tree-sitter highlighter is a pure kind→role function over the grammar's
//! `highlights.scm` (131 lines, zero query predicates). This module reproduces
//! that classification directly from the mold CST the batch parser commits, so
//! the Rust front-end needs neither a query engine nor a `regex` dependency. A
//! mold carries the grammar zipper (its rule `provenance`, sort, precedence),
//! and the CST carries the melded structure, so the classifier has strictly
//! more information than a flat capture query: it can distinguish tile
//! occurrences the `.scm` cannot express.
//!
//! The role vocabulary is [`gandr_surface_render_remote::present::HlRole`],
//! shared with the tree-sitter highlighter, so the TUI theme and the LSP token
//! legend project one vocabulary regardless of which highlighter produced the
//! spans.
//!
//! # Parity
//!
//! The byte-for-byte differential against the tree-sitter highlighter is the
//! designed parity lane, deferred with the tree-sitter reference; the
//! named-kind inventory ([`crate::parity`]) is the substrate table it will
//! consume. Where the PBG is structurally coarser than the committed
//! tree-sitter grammar (it molds every shell word as the single `shell_word`
//! atom, so it cannot split the leading command from its arguments), the
//! divergence is enumerated with a one-line justification rather than papered
//! over.

use alloc::vec::Vec;

use gandr_surface_render_remote::present::ByteOffset;
use gandr_surface_render_remote::present::HlRole;
use gandr_surface_render_remote::present::HlSpan;
use gandr_surface_syntax::Cst;
use gandr_surface_syntax::Material;
use gandr_surface_syntax::MoldPayload;
use gandr_surface_syntax::NodeId;
use gandr_surface_syntax::NodeKind;
use gandr_surface_syntax::NodeView;
use gandr_surface_syntax::SourceSlice;
use gandr_surface_syntax::TextOffset;

use crate::model::Pbg;
use crate::model::Provenance;
use crate::model::Regex;
use crate::model::Sym;
use crate::model::TileLabel;

/// Recover the host-language role of a shell-word token inside `$(` … `)`.
///
/// Shell lexing deliberately keeps the interior token as one `shell_word`; the
/// syntax view reinterprets the same token in the structurally-delimited host
/// expression slot. Mirror only the literal roles captured by
/// `highlights.scm`: booleans and unsuffixed numbers. Identifiers and typed
/// numbers remain uncaptured, exactly as on the tree-sitter side.
///
/// # Contract
/// - requires: `text` is the exact source text of a `shell_word` inside a
///   structurally recognized host escape.
/// - ensures: returns a host literal role only when tree-sitter captures the
///   corresponding host atom; never classifies an ordinary shell argument.
/// - panics: none.
fn host_escape_word_role(text: SourceSlice<'_>) -> Option<HlRole>
{
    let text = text.as_ref();
    if matches!(text, "true" | "false") {
        return Some(HlRole::Boolean);
    }
    let typed_number = ["u32", "u64", "i32", "i64", "f32", "f64"]
        .iter()
        .any(|suffix| {
            text.strip_suffix(suffix)
                .is_some_and(|digits| !digits.is_empty())
        });
    if text.as_bytes().first().is_some_and(u8::is_ascii_digit) && !typed_number {
        return Some(HlRole::Number);
    }
    None
}

/// Compute highlight spans for a committed mold CST.
///
/// One structural pass over the tree classifies every significant tile and
/// trivia leaf into a [`HlRole`] (context-free tiles via [`role_of`],
/// structurally-ambiguous `identifier`/`command_name` tiles via their melded
/// neighbours), then merges contiguous equal-role leaves into sorted, disjoint
/// spans — byte-for-byte the shape `gandr_surface_tree_sitter::highlight`
/// produces.
///
/// # Contract
/// - requires: `cst` was committed by `gandr_surface_parser::parse` over `pbg`
///   (its grammar fingerprint matches `pbg`).
/// - ensures: returns sorted, disjoint spans covering exactly the classified
///   bytes; total over any committed CST.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L4 — keywords, operators, literals, types, structural
///   identifiers, strings and shell leaves each drive a distinct classification
///   branch, and the E2 corpus witnesses byte-identical output.
/// - witness: `gandr_surface_grammar::highlight_parity` (integration test)
#[inline]
#[must_use]
pub fn highlight(
    pbg: &Pbg,
    cst: &Cst,
) -> Vec<HlSpan>
{
    let provs = mold_provenances(pbg);
    let mut leaves: Vec<Leaf> = Vec::new();
    walk(pbg, cst, &provs, cst.root(), &mut leaves);
    resolve_shell(&mut leaves);
    compress(&leaves)
}

/// The keyword tile forms — the `highlights.scm` keyword list plus the type
/// keywords `F`/`U`/`mu` and the `end` session terminator, and the
/// syntax-growth keywords that the live highlighter must keep honest on the
/// surface corpus.
const KEYWORDS: &[&str] = &[
    "def", "val", "run", "leta", "as", "extern", "from", "type", "fn", "ret", "thunk", "force",
    "case", "if", "else", "co", "hold", "dup", "drop", "send", "recv", "close", "select", "offer",
    "fork", "acquire", "release", "migrate", "at", "forall", "F", "U", "mu", "end",
    // Syntax-growth keywords; `module` is mirrored by tree-sitter,
    // while the rest remain PBG-only/parity-exempt (`crate::PBG_ONLY_KINDS`).
    "data", "codata", "for", "while", "loop", "break", "continue", "import", "module", "in", "rec",
    "op", "rule", "with", "infix", "infixl", "infixr", "prefix", "postfix",
    // The ruled circuit block form's leads (`crate::surface::circuit`). `sign`
    // and `oper` are reserved words; `sort`, `node`, and `feed` are contextual
    // keywords, so they highlight as keywords exactly where the circuit forms
    // mold them and stay ordinary names everywhere else.
    "sign", "sort", "oper", "node", "feed",
];

/// The term operator tile forms captured by `highlights.scm`'s operator list,
/// plus the ruled circuit arrow grid (`-->` / `<->` / `==>` / `<=>`), which
/// highlights as an operator like the term arrow it is disjoint from.
const OPERATORS: &[&str] = &[
    "->", "<-", "=>", "==", "!=", "<", "<=", ">", ">=", "+", "-", "*", "&&", "||", "|", "&", "/\\",
    "-->", "<->", "==>", "<=>",
];

/// The primitive-type tile forms captured as `type.builtin`.
const PRIMITIVE_TYPES: &[&str] = &[
    "Any", "Boolean", "Char", "Integer", "Never", "String", "Symbol", "Unit", "Unknown", "Void",
    "f32", "f64", "i32", "i64", "u32", "u64",
];

/// Number of tile symbols in a regex.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
struct TileSymCount(usize);

/// Compact leaf byte offset while highlights are coalesced.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
struct LeafByteOffset(u32);

impl From<TextOffset> for LeafByteOffset
{
    #[inline]
    fn from(value: TextOffset) -> Self
    {
        Self(value.into())
    }
}

/// Boolean marker for a definition form whose identifier names a function.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DefShapeIsFunction(bool);

/// Boolean marker for a tile inside a shell host escape.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct HostEscape(bool);

/// Reconstruct the per-[`gandr_surface_syntax::MoldId`] rule provenance (the
/// tree-sitter named kind of the producing rule).
///
/// The mold table assigns ids by walking `pbg.rules()` in order, each rule's
/// regex left to right, one id per static tile symbol
/// (`crate::mold::MoldTable::build`). Re-walking the rules in the same order
/// and counting tile symbols reproduces that alignment exactly, so `out[id]` is
/// the provenance of the rule that contributed mold `id`. The alignment is
/// witnessed against `MoldDef::sort`/`prec` by
/// `contracts::mold_provenance_alignment`.
///
/// # Contract
/// - requires: `pbg` is a checked PBG.
/// - ensures: returns one provenance per mold, indexed by
///   [`gandr_surface_syntax::MoldId`].
/// - panics: none.
#[must_use]
fn mold_provenances(pbg: &Pbg) -> Vec<Provenance>
{
    let mut out = Vec::with_capacity(pbg.mold_count().0);
    for rule in pbg.rules() {
        let count = count_tile_syms(rule.regex());
        for _ in 0 .. count.0 {
            out.push(rule.provenance());
        }
    }
    out
}

/// Count the static tile symbols in a rule regex, in the pre-order the mold
/// table assigns [`gandr_surface_syntax::MoldId`]s (one mold per static tile
/// symbol, regardless of `Optional`/`Repeat`/`Alt` nesting;
/// `crate::mold::walk_regex`).
fn count_tile_syms(regex: &Regex) -> TileSymCount
{
    let mut count = 0usize;
    let mut stack = vec![regex];
    while let Some(current) = stack.pop() {
        match *current {
            | Regex::Empty | Regex::Sym(Sym::Sort(_)) => {},
            | Regex::Sym(Sym::Tile(_)) => count = count.saturating_add(1),
            | Regex::Seq(ref items) | Regex::Alt(ref items) => {
                stack.extend(items.iter().rev());
            },
            | Regex::Optional(ref inner) | Regex::Repeat(ref inner) => stack.push(inner),
        }
    }
    TileSymCount(count)
}

/// A leaf's shell role in the source-order command pass.
#[derive(Clone, Copy, Eq, PartialEq)]
enum ShellKind
{
    /// Not a shell command word or separator.
    Plain,
    /// A `command_name` word, resolved positionally (leading word →
    /// `FunctionCall`, later word / redirection target → `Path`).
    CommandWord,
    /// A word that carries its own fixed role and is transparent to the command
    /// head: the shell negation `!` (uncaptured), so `! echo` still highlights
    /// `echo` as the command. An environment assignment is transparent too, but
    /// it reaches that by being its own `environment_assignment` tile — a
    /// [`ShellKind::Plain`] leaf with a `VariableParam` role — rather than a
    /// command word.
    Transparent,
    /// A command-list separator (`;`, `&&`, `||`, `|`, `|&`, the block opener,
    /// a subshell / command-substitution opener): the next `command_name`
    /// is a fresh command head.
    Separator,
}

/// One classified leaf awaiting the shell pass and compression.
struct Leaf
{
    /// Inclusive start byte.
    start: LeafByteOffset,
    /// Exclusive end byte.
    end: LeafByteOffset,
    /// The fixed role, or [`None`] until the shell pass resolves a command
    /// word.
    role: Option<HlRole>,
    /// The leaf's role in the shell command pass.
    shell: ShellKind,
}

/// The enclosing-bracket class the flat melded form places a tile in.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Bracket
{
    /// A `( … )` parameter list opened right after a `def`/`extern` name or
    /// `fn`: its `identifier` tiles are parameter binders.
    Params,
    /// A `( … )` tuple pattern opened immediately after `val`: its `identifier`
    /// tiles are value binders, not expression references.
    Pattern,
    /// An `@[ … ]` attribute block: its `identifier` tiles are attribute names.
    Attr,
    /// A `#{ … }` record: its field-name tiles (including a `_`) are
    /// uncaptured.
    Record,
    /// A `$(` … `)` shell host escape: its interior is a gandr expression,
    /// never a positional shell command word.
    HostEscape,
    /// Any other bracket.
    Plain,
}

/// The command-list separator tile labels (see [`ShellKind::Separator`]).
const SHELL_SEPARATORS: &[&str] = &[
    "#!{",
    ";",
    "&&",
    "||",
    "|",
    "|&",
    "subshell_open",
    "command_substitution_start",
];

/// Walk interior nodes iteratively, classify their tiles with sibling +
/// enclosing-bracket context, and append every classified leaf in source order.
fn walk(
    pbg: &Pbg,
    cst: &Cst,
    provs: &[Provenance],
    node: NodeId,
    out: &mut Vec<Leaf>,
)
{
    enum WalkFrame
    {
        Visit
        {
            node: NodeId, inherited: Bracket
        },
        Emit
        {
            node: NodeId, ctx: TileCtx
        },
    }

    let mut frames = vec![WalkFrame::Visit {
        node,
        inherited: Bracket::Plain,
    }];
    while let Some(frame) = frames.pop() {
        match frame {
            | WalkFrame::Visit {
                node: current,
                inherited,
            } => {
                let Ok(view) = cst.node(current)
                else {
                    continue;
                };
                if view.kind() == NodeKind::Token {
                    // A top-level token with no interior parent (e.g. the whole
                    // buffer is trivia); emit with empty context.
                    emit_leaf(pbg, provs, view, TileCtx::plain(), out);
                    continue;
                }
                let Ok(child_ids) = view.children()
                else {
                    continue;
                };
                // Gather direct significant children in source order (trivia
                // are emitted with no sibling role).
                let mut children: Vec<(NodeId, bool, Option<TileLabel>, LeafByteOffset)> =
                    Vec::new();
                let mut child_frames = Vec::new();
                for &child_id in child_ids {
                    let Ok(cview) = cst.node(child_id)
                    else {
                        continue;
                    };
                    if cview.material() == Material::Space {
                        child_frames.push(WalkFrame::Emit {
                            node: child_id,
                            ctx: TileCtx::plain(),
                        });
                        continue;
                    }
                    let is_tile = cview.kind() == NodeKind::Token;
                    let label = if is_tile {
                        tile_facts(pbg, provs, cview).map(|(label, _provenance)| label)
                    }
                    else {
                        None
                    };
                    children.push((
                        child_id,
                        is_tile,
                        label,
                        LeafByteOffset::from(cview.range().start()),
                    ));
                }
                children.sort_by_key(|&(_, _, _, start)| start);

                // A `def` name is a function definition only when the name is
                // immediately followed by a parameter list. Looking for any `{`
                // in the enclosing meld over-classifies value/signature members
                // inside `module M { … }` because the module body's brace is a
                // direct child of that melded form.
                let mut bracket_stack: Vec<Bracket> = if inherited == Bracket::Plain {
                    Vec::new()
                }
                else {
                    vec![inherited]
                };
                for (index, &(child_id, is_tile, label, _start)) in children.iter().enumerate() {
                    let prev = index
                        .checked_sub(1)
                        .and_then(|i| children.get(i))
                        .and_then(|&(_, _, neighbor_label, _)| neighbor_label);
                    let prev2 = index
                        .checked_sub(2)
                        .and_then(|i| children.get(i))
                        .and_then(|&(_, _, neighbor_label, _)| neighbor_label);
                    let next = children
                        .get(index.saturating_add(1))
                        .and_then(|&(_, _, neighbor_label, _)| neighbor_label);
                    let enclosing = bracket_stack.last().copied().unwrap_or(Bracket::Plain);

                    if is_tile {
                        let ctx = TileCtx {
                            prev,
                            next,
                            enclosing,
                            in_host_escape: HostEscape(
                                bracket_stack.contains(&Bracket::HostEscape),
                            ),
                            def_shape_is_function: DefShapeIsFunction(next == Some(TileLabel("("))),
                        };
                        child_frames.push(WalkFrame::Emit {
                            node: child_id,
                            ctx,
                        });
                    }
                    else {
                        let pattern_child = enclosing == Bracket::Pattern
                            || prev == Some(TileLabel("val"))
                            || next == Some(TileLabel("=>"));
                        let child_inherited = if pattern_child {
                            Bracket::Pattern
                        }
                        else if enclosing == Bracket::Record {
                            Bracket::Plain
                        }
                        else {
                            enclosing
                        };
                        child_frames.push(WalkFrame::Visit {
                            node: child_id,
                            inherited: child_inherited,
                        });
                    }

                    // Maintain the enclosing-bracket stack over this form's
                    // direct tiles.
                    if let Some(lbl) = label {
                        if let Some(tag) = open_bracket(lbl, prev, prev2) {
                            let tag = if enclosing == Bracket::Pattern
                                && tag == Bracket::Plain
                                && matches!(lbl.as_ref(), "(" | "[")
                            {
                                Bracket::Pattern
                            }
                            else {
                                tag
                            };
                            bracket_stack.push(tag);
                        }
                        else if matches!(lbl.as_ref(), ")" | "]" | "}" | "subshell_close") {
                            bracket_stack.pop();
                        }
                    }
                }
                frames.extend(child_frames.into_iter().rev());
            },
            | WalkFrame::Emit {
                node: emit_node,
                ctx,
            } => {
                if let Ok(view) = cst.node(emit_node) {
                    emit_leaf(pbg, provs, view, ctx, out);
                }
            },
        }
    }
}

/// Classify and record one leaf (tile or trivia) with its sibling +
/// enclosing-bracket context.
fn emit_leaf(
    pbg: &Pbg,
    provs: &[Provenance],
    view: NodeView<'_>,
    ctx: TileCtx,
    out: &mut Vec<Leaf>,
)
{
    let range = view.range();
    if view.material() == Material::Space {
        if let Ok(text) = view.text()
            && let Some(role) = trivia_role(text)
        {
            out.push(Leaf {
                start: LeafByteOffset::from(range.start()),
                end: LeafByteOffset::from(range.end()),
                role: Some(role),
                shell: ShellKind::Plain,
            });
        }
        return;
    }
    let Some((label, provenance)) = tile_facts(pbg, provs, view)
    else {
        return;
    };
    let (role, shell) = match label.as_ref() {
        | "identifier" => (
            identifier_role(
                provenance,
                ctx.prev,
                ctx.next,
                ctx.enclosing,
                ctx.def_shape_is_function,
            ),
            ShellKind::Plain,
        ),
        // An attribute-block close bracket is `@punctuation.special`, folded to
        // `Other` like its `@[` partner (`role_of` handles `@[`).
        | "]" if ctx.enclosing == Bracket::Attr => (Some(HlRole::Other), ShellKind::Plain),
        // `_` is `(wildcard) @variable.builtin` in a pattern, but a record field
        // name (`#{ _ = … }`) is uncaptured.
        | "_" if ctx.enclosing == Bracket::Record => (None, ShellKind::Plain),
        | "_" => (Some(HlRole::Variable), ShellKind::Plain),
        | "command_name" | "shell_word" if ctx.in_host_escape.0 => (
            view.text().ok().and_then(host_escape_word_role),
            ShellKind::Plain,
        ),
        | "command_name" | "shell_word" => classify_command_word(view),
        | _ => {
            let shell = if SHELL_SEPARATORS.contains(&label.as_ref()) {
                ShellKind::Separator
            }
            else {
                ShellKind::Plain
            };
            (role_of(label, provenance), shell)
        },
    };
    out.push(Leaf {
        start: LeafByteOffset::from(range.start()),
        end: LeafByteOffset::from(range.end()),
        role,
        shell,
    });
}

/// Classify a trivia (space-material) leaf by its text prefix.
///
/// The labeler records comments, block comments and shebangs as
/// [`Material::Space`] trivia (they carry no mold), so the mold is unavailable
/// here; `highlights.scm` captures `(line_comment)`/`(block_comment)` as
/// `@comment` and `(shebang)` as `@keyword.directive`. Ordinary whitespace is
/// uncaptured.
fn trivia_role(text: SourceSlice<'_>) -> Option<HlRole>
{
    let text = text.as_ref();
    if text.starts_with("//") || text.starts_with("/*") {
        Some(HlRole::Comment)
    }
    else if text.starts_with("#!") {
        Some(HlRole::Directive)
    }
    else {
        None
    }
}

/// The melded context a tile is classified in: its significant neighbours, its
/// enclosing bracket, and whether the enclosing form has a function shape.
#[derive(Clone, Copy)]
struct TileCtx
{
    /// The previous significant sibling's tile label.
    prev: Option<TileLabel>,
    /// The next significant sibling's tile label.
    next: Option<TileLabel>,
    /// The tile's enclosing bracket class.
    enclosing: Bracket,
    /// Whether this tile is inside a shell host escape's gandr-expression slot.
    in_host_escape: HostEscape,
    /// Whether the enclosing form carries a function shape (a `def` name
    /// split).
    def_shape_is_function: DefShapeIsFunction,
}

impl TileCtx
{
    /// The empty context (a trivia leaf or the whole-buffer token).
    const fn plain() -> Self
    {
        Self {
            prev: None,
            next: None,
            enclosing: Bracket::Plain,
            in_host_escape: HostEscape(false),
            def_shape_is_function: DefShapeIsFunction(false),
        }
    }
}

/// Resolve a tile leaf's mold to its label and reconstructed provenance.
fn tile_facts(
    pbg: &Pbg,
    provs: &[Provenance],
    view: NodeView<'_>,
) -> Option<(TileLabel, Provenance)>
{
    if let MoldPayload::Tile(mold) = view.payload() {
        let def = pbg.mold(mold).ok()?;
        let index = usize::try_from(u32::from(mold)).ok()?;
        let provenance = provs.get(index).copied()?;
        return Some((TileLabel(def.label), provenance));
    }
    None
}

/// Structurally classify an `identifier` tile from its melded neighbours and
/// its enclosing bracket, reproducing the parent-kind/field patterns of
/// `highlights.scm`.
fn identifier_role(
    provenance: Provenance,
    prev: Option<TileLabel>,
    next: Option<TileLabel>,
    enclosing: Bracket,
    def_shape_is_function: DefShapeIsFunction,
) -> Option<HlRole>
{
    // Enclosing-bracket forms: attribute name, parameter binder, pattern binder,
    // record field name (a `#{ name = … }` field is uncaptured).
    match enclosing {
        | Bracket::Attr => return Some(HlRole::Other),
        | Bracket::Params => return Some(HlRole::VariableParam),
        | Bracket::Pattern => return Some(HlRole::VariableDef),
        | Bracket::Record => return None,
        | Bracket::HostEscape | Bracket::Plain => {},
    }
    // Producing-rule forms the mold pins directly.
    match provenance.as_ref() {
        // A grade written as a word (`U[omega]`) parses as `(grade)`.
        | "u_type" => return Some(HlRole::Number),
        // A co-record field name (`co { fst = … }`) and a projection field
        // (`e.field`) are members; a record-expression field name (`#{ x = … }`)
        // is uncaptured.
        | "co_expression" | "projection_expression" => return Some(HlRole::Member),
        | "at_type" | "migrate_expression" | "select_session_type" | "offer_session_type" => {
            return Some(HlRole::Label);
        },
        | _ => {},
    }
    // Neighbour-driven binder / field / call / definition forms.
    match prev {
        | Some(TileLabel("?")) => Some(HlRole::Label),
        | Some(TileLabel(".")) => Some(HlRole::Member),
        | Some(TileLabel("def")) if def_shape_is_function.0 => Some(HlRole::FunctionDef),
        | Some(TileLabel("val" | "leta" | "as" | "def")) => Some(HlRole::VariableDef),
        | _ if next == Some(TileLabel("(")) => Some(HlRole::FunctionCall),
        | _ => None,
    }
}

/// The enclosing-bracket tag opened by a bracket tile, given its left context.
fn open_bracket(
    label: TileLabel,
    prev: Option<TileLabel>,
    prev2: Option<TileLabel>,
) -> Option<Bracket>
{
    match label.as_ref() {
        | "@[" => Some(Bracket::Attr),
        | "(" if prev == Some(TileLabel("$")) => Some(Bracket::HostEscape),
        | "(" => {
            let params = prev == Some(TileLabel("fn"))
                || (prev == Some(TileLabel("identifier"))
                    && matches!(prev2, Some(TileLabel("def" | "extern"))));
            let pattern = prev == Some(TileLabel("val"));
            Some(if params {
                Bracket::Params
            }
            else if pattern {
                Bracket::Pattern
            }
            else {
                Bracket::Plain
            })
        },
        | "#{" => Some(Bracket::Record),
        | "[" | "{" | "subshell_open" | "command_substitution_start" => Some(Bracket::Plain),
        | _ => None,
    }
}

/// Classify a shell command-word tile (`shell_word`, formerly `command_name`).
///
/// The PBG molds every shell word as the single `shell_word` atom class
/// (`command_name` folded onto it as an adaptation); the leading
/// word vs argument split is resolved positionally by [`resolve_shell`]. One
/// form is command-head-transparent — a `!` negation, which carries no role and
/// does not consume the following command.
///
/// A `NAME=value` environment assignment is NO LONGER handled here: the labeler
/// munches it into its own `environment_assignment` tile, which
/// takes its `VariableParam` role straight from [`role_of`] — mirroring
/// `highlights.scm`'s `(environment_assignment) @variable.parameter` over the
/// whole node, rather than recovering it from a command word's text.
fn classify_command_word(view: NodeView<'_>) -> (Option<HlRole>, ShellKind)
{
    if view.text().is_ok_and(|text| text.as_ref() == "!") {
        return (None, ShellKind::Transparent);
    }
    (None, ShellKind::CommandWord)
}

/// Classify a tile occurrence into a highlight role from its mold label and
/// producing-rule provenance, mirroring the grammar's `highlights.scm`.
///
/// This is the context-free core: it resolves every tile whose role is fixed by
/// its label (keywords, operators, primitive types, literals, string pieces,
/// constructors, type identifiers, shell leaves) plus the provenance-scoped
/// forms (`?` as a hole vs a session receive, attribute-block punctuation). The
/// structurally-ambiguous forms the PBG shares one mold for — a bare
/// `identifier` (variable reference, call target, binder, field, world) and the
/// shell `shell_word` (the single word class that absorbed `command_name`) —
/// return [`None`] here. `None` means "no span" (an uncaptured token) **only**
/// when [`highlight`](fn@crate::highlight) also declines the structural pass; a
/// captured-but-unmapped token (attribute-block punctuation, an `@attribute`
/// name) returns `Some(HlRole::Other)`, which is a real span, distinct from the
/// absent-span case.
///
/// # Contract
/// - requires: `label` and `provenance` come from a resolved
///   [`crate::MoldDef`].
/// - ensures: total — every input yields `Some(role)` or `None`; pure (no
///   allocation, no table lookup).
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — each role branch is a distinct label/provenance class
///   witnessed by the E2 corpus parity test.
/// - witness: `gandr_surface_grammar::highlight_parity` (integration test)
#[inline]
#[must_use]
pub fn role_of(
    label: TileLabel,
    provenance: Provenance,
) -> Option<HlRole>
{
    // Provenance-scoped forms first: the same label means different things.
    match (label.as_ref(), provenance.as_ref()) {
        // `?` is a typed hole only under the `hole` rule; a session receive
        // `?T.S` is not captured by `highlights.scm`.
        | ("?", "hole") => return Some(HlRole::Hole),
        // Attribute-block brackets are `@punctuation.special` — captured, but the
        // grammar's role table folds that to `Other` (a real span).
        | ("@[", _) => return Some(HlRole::Other),
        // Uncaptured tokens that share a label with a captured one: a session
        // receive `?` (vs a hole), and shell strings — a shell double-quoted
        // string (`"…"`/escape under `double_quoted_string`) and single-quoted
        // string (`double_string_fragment`/`single_quoted_content`/`'`), unlike a
        // host `(string)`.
        | ("?" | "double_string_fragment" | "single_quoted_content" | "'", _)
        | ("\"" | "escape_sequence", "double_quoted_string") => return None,
        | _ => {},
    }

    let label = label.as_ref();
    let provenance = provenance.as_ref();
    if KEYWORDS.contains(&label) {
        return Some(HlRole::Keyword);
    }
    if OPERATORS.contains(&label) || provenance == "redirection_operator" {
        return Some(HlRole::Operator);
    }
    if PRIMITIVE_TYPES.contains(&label) {
        return Some(HlRole::TypeBuiltin);
    }

    match label {
        | "true" | "false" => Some(HlRole::Boolean),
        | "character" => Some(HlRole::Character),
        // `(number)`/`(grade)` are `constant.numeric`; a `(typed_number)`
        // (`1.5f32`, `2u32`) is NOT captured by `highlights.scm`.
        | "number" | "ω" | "file_descriptor" => Some(HlRole::Number),
        // Host `(string)` pieces (shell strings are handled above).
        | "\"" | "string_fragment" => Some(HlRole::StringLit),
        | "escape_sequence" => Some(HlRole::Escape),
        | "constructor" => Some(HlRole::Constructor),
        | "type_identifier" => Some(HlRole::Type),
        | "type_variable" => Some(HlRole::TypeVariable),
        | "hole_name" => Some(HlRole::Label),
        | "variable_name" => Some(HlRole::Variable),
        | "argument" => Some(HlRole::Path),
        | "environment_assignment" => Some(HlRole::VariableParam),
        // Structurally-resolved `identifier`/`shell_word` (and the folded
        // `command_name` it absorbed) and every uncaptured token yield no span.
        | _ => None,
    }
}

/// Resolve `command_name` words positionally in source order: the first word
/// after a separator (or a block/subshell opener) is the command
/// ([`HlRole::FunctionCall`]); every later word — argument or redirection
/// target — is a path ([`HlRole::Path`]). A `!` negation is transparent to the
/// head.
fn resolve_shell(leaves: &mut [Leaf])
{
    leaves.sort_by_key(|leaf| leaf.start);
    let mut expect_command = true;
    for leaf in leaves.iter_mut() {
        match leaf.shell {
            | ShellKind::Separator => expect_command = true,
            | ShellKind::CommandWord => {
                if expect_command {
                    leaf.role = Some(HlRole::FunctionCall);
                    expect_command = false;
                }
                else {
                    leaf.role = Some(HlRole::Path);
                }
            },
            // A transparent word (`!` / `NAME=value`) keeps its own role and the
            // command head; a plain leaf is untouched.
            | ShellKind::Transparent | ShellKind::Plain => {},
        }
    }
}

/// Compress classified, source-ordered leaves into sorted, disjoint spans,
/// merging contiguous equal-role leaves (the tree-sitter `compress` semantics
/// over a disjoint leaf tiling). `leaves` must already be sorted by start.
fn compress(leaves: &[Leaf]) -> Vec<HlSpan>
{
    let mut spans: Vec<HlSpan> = Vec::new();
    let mut open: Option<(LeafByteOffset, LeafByteOffset, HlRole)> = None;
    for leaf in leaves {
        let Some(role) = leaf.role
        else {
            continue;
        };
        match open {
            | Some((open_start, open_end, open_role))
                if open_role == role && open_end == leaf.start =>
            {
                open = Some((open_start, leaf.end, open_role));
            },
            | Some((open_start, open_end, open_role)) => {
                push_span(&mut spans, open_start, open_end, open_role);
                open = Some((leaf.start, leaf.end, role));
            },
            | None => open = Some((leaf.start, leaf.end, role)),
        }
    }
    if let Some((start, end, role)) = open {
        push_span(&mut spans, start, end, role);
    }
    spans
}

/// Push one span, converting the compact byte offsets to a `usize` range.
fn push_span(
    spans: &mut Vec<HlSpan>,
    start: LeafByteOffset,
    end: LeafByteOffset,
    role: HlRole,
)
{
    let start = ByteOffset::from(usize::try_from(start.0).unwrap_or(usize::MAX));
    let end = ByteOffset::from(usize::try_from(end.0).unwrap_or(usize::MAX));
    spans.push(HlSpan::new(start .. end, role));
}

#[cfg(test)]
mod tests
{
    use gandr_surface_render_remote::present::HlRole;
    use gandr_surface_syntax::MoldId;

    use super::count_tile_syms;
    use super::mold_provenances;
    use super::role_of;
    use crate::model::Provenance;
    use crate::model::TileLabel;
    use crate::surface::built_in;

    /// The reconstructed provenance per mold aligns with the mold table: one
    /// provenance per mold, and each mold's producing rule (by reconstruction)
    /// has the mold's own sort and precedence. This is the load-bearing
    /// invariant the structural classifier rides — if the re-walk of the rules
    /// drifted from `MoldTable::build`'s id assignment, identifiers would be
    /// classified against the wrong rule.
    ///
    /// # Adequacy
    /// - hypothesis: L4 — the length equality plus the per-mold sort/prec
    ///   cross-check kill a reordered, over- or under-counted reconstruction.
    #[test]
    fn mold_provenance_alignment()
    {
        let pbg = built_in().expect("built-in grammar");
        let provs = mold_provenances(&pbg);
        assert_eq!(
            provs.len(),
            pbg.mold_count().0,
            "one reconstructed provenance per mold"
        );

        // Re-walk the rules in table order and cross-check every mold's
        // sort/prec against the rule the reconstruction attributes it to.
        let mut index = 0_u32;
        for rule in pbg.rules() {
            for _ in 0 .. count_tile_syms(rule.regex()).0 {
                let mold = pbg.mold(MoldId::from(index)).expect("mold in range");
                assert_eq!(mold.sort, rule.sort, "sort aligns at mold {index}");
                assert_eq!(mold.prec, rule.prec, "prec aligns at mold {index}");
                assert_eq!(
                    provs[usize::try_from(index).expect("index fits")],
                    rule.provenance(),
                    "provenance aligns at mold {index}"
                );
                index = index.checked_add(1).expect("mold count fits u32");
            }
        }
        assert_eq!(
            usize::try_from(index).expect("count fits"),
            pbg.mold_count().0,
            "reconstruction covers every mold"
        );
    }

    /// `role_of` pins the context-free tile classes `highlights.scm` fixes by
    /// label or provenance.
    #[test]
    fn role_of_pins_context_free_classes()
    {
        assert_eq!(
            Some(HlRole::Keyword),
            role_of(TileLabel("def"), Provenance("def_value"))
        );
        assert_eq!(
            Some(HlRole::Keyword),
            role_of(TileLabel("val"), Provenance("let_statement"))
        );
        assert_eq!(None, role_of(TileLabel("let"), Provenance("let_statement")));
        assert_eq!(
            Some(HlRole::Operator),
            role_of(TileLabel("->"), Provenance("function_type"))
        );
        assert_eq!(
            Some(HlRole::TypeBuiltin),
            role_of(TileLabel("String"), Provenance("primitive_type"))
        );
        assert_eq!(
            Some(HlRole::Constructor),
            role_of(TileLabel("constructor"), Provenance("constructor"))
        );
        assert_eq!(
            Some(HlRole::Number),
            role_of(TileLabel("number"), Provenance("number"))
        );
        assert_eq!(
            None,
            role_of(TileLabel("typed_number"), Provenance("typed_number"))
        );
        // A hole `?` vs an uncaptured session receive `?`, by provenance.
        assert_eq!(
            Some(HlRole::Hole),
            role_of(TileLabel("?"), Provenance("hole"))
        );
        assert_eq!(
            None,
            role_of(TileLabel("?"), Provenance("receive_session_type"))
        );
        // A host string is captured; a shell double-quoted string is not.
        assert_eq!(
            Some(HlRole::StringLit),
            role_of(TileLabel("\""), Provenance("string"))
        );
        assert_eq!(
            None,
            role_of(TileLabel("\""), Provenance("double_quoted_string"))
        );
        // A structural `identifier`/`command_name` defers to the walk.
        assert_eq!(
            None,
            role_of(TileLabel("identifier"), Provenance("def_value"))
        );
        assert_eq!(
            None,
            role_of(TileLabel("command_name"), Provenance("command"))
        );
    }
}
