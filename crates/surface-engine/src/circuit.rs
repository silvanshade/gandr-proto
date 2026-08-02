//! The **circuit surface check**: arrow-kind confirmation and the reserved
//! reversible glyph's decline.
//!
//! The ruled circuit block form
//! (`docs/gandr/spec/surface-language/circuit-cells.md` §"The block form,
//! ruled") requires that **every arrow reports the kind of the thing it belongs
//! to**, and it is deliberately a *redundancy*: a declaration's kind keyword,
//! its arrow, and its body lines' arrows spell the block's dimension three
//! independent ways, so a disagreement in any one is a localized, nameable
//! error rather than a silent reinterpretation. This module is the checker half
//! of that redundancy.
//!
//! The confirmation cannot be the grammar's, and the reason is not economy. A
//! **body line's** arrow comes from the *applied head's* kind — `p(x) ==> (x′)`
//! because `p` is a rule, `add(x′, y′) --> (z)` because `add` is an oper — and
//! which names are rules is an environment fact, not a shape. A grammar
//! restricted to the matching glyph would also turn every disagreement into a
//! parse failure, which is the opposite of what the ruling asks for. So
//! `gandr_surface_grammar::surface::circuit` admits the whole grid at every
//! arrow position and the confirmation happens here, against the names in
//! scope.
//!
//! # The grid, and what each row expects
//!
//! | kind-class                    | directed | bidirectional |
//! | ----------------------------- | -------- | ------------- |
//! | circuit 1-cell former         | `-->`    | `<->`         |
//! | rewrite face, every dimension | `==>`    | `<=>`         |
//!
//! An arrow belongs to the **circuit** row when the thing it decorates is a
//! `data` constructor, an `oper`, or a `feed` wire, and to the **face** row
//! when it is a `rule` — a member, a top-level declaration, or a rewrite-sorted
//! binder in a parameter telescope. Dimension is read from the endpoints, never
//! from the arrow, so a 3-cell face writes `==>` exactly like a 2-cell one and
//! the check is per-row, never per-dimension.
//!
//! `<->` is **reserved**. It is in the circuit row, so it agrees with an `oper`
//! and still declines: a `<->` oper is a semantic claim that the function is a
//! data-level iso — distinct from a rule's bidirectionality, which `<=>`
//! carries as an engine licence — and the reversible-oper lane owes that
//! checking discipline before the form can be accepted.
//!
//! # What this pass does not do
//!
//! It is a surface check, not an elaboration: it reads names and arrows and
//! nothing else. In particular it does **not** fold the port/name sets (the
//! monogamy discipline and its non-disjointness diagnostic), sweep node-only
//! wiring for cycles that owe a `feed`, or lower anything — circuit members
//! stay parse-and-decline at lowering. An applied head the environment does not
//! know is skipped rather than guessed: an unresolved name is a name-resolution
//! question, and answering it here would report the wrong error.

use alloc::collections::BTreeMap;

use gandr_surface_syntax::NodeId;
use gandr_theory_levitation::SurfaceSpan;

use crate::boundary::CircuitName;
use crate::boundary::MatchDecision;
use crate::boundary::PipelineSource;
use crate::boundary::TileSpelling;
use crate::cst_read::BracketLabel;
use crate::cst_read::Cursor;
use crate::cst_read::Reader;
use crate::cst_read::grammar;
use crate::cst_read::is_closer;
use crate::cst_read::is_opener;
use crate::cst_read::split_at_top_level;
use crate::desc_elab::ElabDiagnostic;
use crate::synnode::SynTree;

/// The kind a circuit name is declared at — the description universe's own
/// structure, surfaced as the member keywords.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CircuitKind
{
    /// A colour: `sort Nat : Type`.
    Sort,
    /// A constructor: `data Zero : Nat`.
    Data,
    /// A circuit 1-cell: `oper add : (Nat, Nat) --> Nat`.
    Oper,
    /// A rewrite face: `rule cong2 : … ==> …`.
    Rule,
}

impl CircuitKind
{
    /// The member keyword this kind is spelled with.
    fn keyword(self) -> TileSpelling
    {
        TileSpelling(match self {
            | Self::Sort => "sort",
            | Self::Data => "data",
            | Self::Oper => "oper",
            | Self::Rule => "rule",
        })
    }

    /// The kind this member keyword declares, or [`None`] when the label is not
    /// a member keyword.
    fn from_keyword(label: TileSpelling) -> Option<Self>
    {
        match label.0 {
            | "sort" => Some(Self::Sort),
            | "data" => Some(Self::Data),
            | "oper" => Some(Self::Oper),
            | "rule" => Some(Self::Rule),
            | _ => None,
        }
    }

    /// The arrow row this kind's arrows must come from, or [`None`] for a kind
    /// that carries no arrow and licenses none.
    ///
    /// A `sort` declares a colour: it has no interface, so nothing it leads
    /// expects an arrow and an arrow near it is somebody else's error.
    fn row(self) -> Option<ArrowRow>
    {
        match self {
            | Self::Sort => None,
            | Self::Data | Self::Oper => Some(ArrowRow::Circuit),
            | Self::Rule => Some(ArrowRow::Face),
        }
    }
}

/// A row of the arrow grid: the kind-class the shaft carries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArrowRow
{
    /// The circuit 1-cell formers `-->` and `<->`.
    Circuit,
    /// The rewrite faces `==>` and `<=>`, at every dimension.
    Face,
}

impl ArrowRow
{
    /// How a diagnostic names this row.
    fn description(self) -> TileSpelling
    {
        TileSpelling(match self {
            | Self::Circuit => "a circuit 1-cell former",
            | Self::Face => "a rewrite face",
        })
    }

    /// The correction a diagnostic offers for this row.
    fn correction(self) -> TileSpelling
    {
        TileSpelling(match self {
            | Self::Circuit => "write `-->`",
            | Self::Face => "write `==>`, or `<=>` for the invertible face",
        })
    }
}

/// One glyph of the four-glyph arrow grid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Arrow
{
    /// `-->`, the directed circuit 1-cell former.
    DirectedCircuit,
    /// `<->`, reserved for reversible opers.
    ReversibleCircuit,
    /// `==>`, the directed rewrite face.
    DirectedFace,
    /// `<=>`, the invertible rewrite face.
    InvertibleFace,
}

impl Arrow
{
    /// The glyph a tile label spells, or [`None`] when the label is not a grid
    /// glyph.
    fn from_label(label: TileSpelling) -> Option<Self>
    {
        match label.0 {
            | "-->" => Some(Self::DirectedCircuit),
            | "<->" => Some(Self::ReversibleCircuit),
            | "==>" => Some(Self::DirectedFace),
            | "<=>" => Some(Self::InvertibleFace),
            | _ => None,
        }
    }

    /// The row this glyph's shaft carries.
    fn row(self) -> ArrowRow
    {
        match self {
            | Self::DirectedCircuit | Self::ReversibleCircuit => ArrowRow::Circuit,
            | Self::DirectedFace | Self::InvertibleFace => ArrowRow::Face,
        }
    }

    /// Whether this glyph is reserved — parses, and declines until its lane
    /// lands a checking discipline.
    fn is_reserved(self) -> MatchDecision
    {
        MatchDecision(matches!(self, Self::ReversibleCircuit))
    }

    /// How the glyph is written.
    fn spelling(self) -> TileSpelling
    {
        TileSpelling(match self {
            | Self::DirectedCircuit => "-->",
            | Self::ReversibleCircuit => "<->",
            | Self::DirectedFace => "==>",
            | Self::InvertibleFace => "<=>",
        })
    }
}

/// The circuit names in scope, each with the kind it was declared at.
///
/// Scopes nest by copy rather than by chain: a `sign` block extends the
/// file-level names with its members, and a block-bodied member extends that
/// with its own parameter binders. The counts are declaration-sized, so the
/// copy is cheaper than the indirection a chain would cost every lookup.
#[repr(transparent)]
#[derive(Clone, Debug, Default)]
struct KindEnv<'tree>
{
    /// Name to declared kind.
    entries: BTreeMap<CircuitName<'tree>, CircuitKind>,
}

impl<'tree> KindEnv<'tree>
{
    /// Record `name` as declared at `kind`, shadowing any earlier binding.
    fn bind(
        &mut self,
        name: CircuitName<'tree>,
        kind: CircuitKind,
    )
    {
        self.entries.insert(name, kind);
    }

    /// The kind `name` was declared at, or [`None`] when it is not in scope.
    fn kind_of(
        &self,
        name: CircuitName<'tree>,
    ) -> Option<CircuitKind>
    {
        self.entries.get(&name).copied()
    }
}

/// The circuit surface check's verdict: every arrow-kind disagreement and every
/// reserved-glyph decline the source earned, in source order.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CircuitSurface
{
    /// The declines, in source order.
    pub diagnostics: Vec<ElabDiagnostic>,
}

/// **Check** every circuit declaration in `source`, confirming each arrow
/// against the kind of the thing it belongs to and declining the reserved
/// reversible glyph.
///
/// # Contract
/// - requires: `source` parses (an unparseable source yields an empty verdict —
///   a repair obligation is the parser's report, not this pass's).
/// - ensures: returns one diagnostic per arrow whose row disagrees with its
///   subject's kind, plus one per `<->` occurrence that agrees; an arrow earns
///   at most one diagnostic, the disagreement taking precedence over the
///   reservation because a `<->` in the face row is in the wrong row before it
///   is reserved. Non-circuit items are ignored, and an applied head the
///   environment does not know is skipped rather than guessed.
/// - provides: the checker half of the ruling's three-way redundancy — the
///   declaration keyword, the signature arrow, and the body-line arrows.
/// - fails: never; total on any input.
/// - panics: none.
/// - intension: names are collected before arrows are confirmed, so a body may
///   apply a head its block declares later.
///
/// # Adequacy
/// - hypothesis: L4 — the four subjects that carry an arrow (a member
///   declaration, a top-level declaration, a rewrite-sorted binder, a body
///   line) are separated by the diagnostic each earns, and the agreeing,
///   disagreeing, and reserved glyphs are separated at one subject; the
///   unresolved-head path is observed by its silence.
/// - witness: `gandr-surface-engine` `tests/circuit.rs`
///   `the_ruled_worked_examples_confirm_every_arrow`
/// - witness: `gandr-surface-engine` `tests/circuit.rs`
///   `a_declaration_arrow_disagreeing_with_its_kind_is_named`
/// - witness: `gandr-surface-engine` `tests/circuit.rs`
///   `a_body_line_arrow_disagreeing_with_its_head_is_named`
/// - witness: `gandr-surface-engine` `tests/circuit.rs`
///   `the_reserved_reversible_glyph_declines_naming_its_lane`
#[inline]
#[must_use]
pub fn check_circuit_surface<'source, S>(source: S) -> CircuitSurface
where
    S: Into<PipelineSource<'source>>,
{
    let source = source.into();
    let Ok(tree) = SynTree::parse(source.0)
    else {
        return CircuitSurface::default();
    };
    let Some(pbg) = grammar()
    else {
        return CircuitSurface::default();
    };
    let reader = Reader::new(pbg, tree.cst());
    let items = reader.sig_children(tree.cst().root());
    let mut check = Check {
        reader: &reader,
        diagnostics: Vec::new(),
    };
    let file_scope = check.file_scope(&items);
    for &item in &items {
        check.item(item, &file_scope);
    }
    CircuitSurface {
        diagnostics: check.diagnostics,
    }
}

/// The member keywords a `sign` block's members are led by.
const MEMBER_LEADS: [TileSpelling; 4] = [
    TileSpelling("sort"),
    TileSpelling("data"),
    TileSpelling("oper"),
    TileSpelling("rule"),
];

/// One run of the circuit surface check over one parse.
struct Check<'run, 'tree>
{
    /// The reader resolving tile labels against the grammar.
    reader: &'run Reader<'tree>,
    /// The declines collected so far, in source order.
    diagnostics: Vec<ElabDiagnostic>,
}

impl<'tree> Check<'_, 'tree>
{
    /// The file-level scope: every top-level circuit declaration's name and
    /// kind, collected before any arrow is confirmed.
    fn file_scope(
        &self,
        items: &[NodeId],
    ) -> KindEnv<'tree>
    {
        let mut scope = KindEnv::default();
        for &item in items {
            let children = self.reader.sig_children(item);
            let Some((name, kind)) = self.judgment_head(&children)
            else {
                continue;
            };
            if matches!(kind, CircuitKind::Oper | CircuitKind::Rule) {
                scope.bind(name, kind);
            }
        }
        scope
    }

    /// The declared name and kind of a keyword-led judgment run
    /// (`kw name : …`), or [`None`] when the run is not one.
    fn judgment_head(
        &self,
        run: &[NodeId],
    ) -> Option<(CircuitName<'tree>, CircuitKind)>
    {
        let &lead = run.first()?;
        let label = self.reader.label(lead)?;
        let kind = CircuitKind::from_keyword(label)?;
        let &name = run.get(1)?;
        Some((CircuitName(self.reader.text(name).0), kind))
    }

    /// Check one top-level item: a `sign` block, a circuit declaration, or
    /// something this pass does not read.
    fn item(
        &mut self,
        item: NodeId,
        file_scope: &KindEnv<'tree>,
    )
    {
        let children = self.reader.sig_children(item);
        let lead = children.first().and_then(|&id| self.reader.label(id));
        match lead.map(|label| label.0) {
            | Some("sign") => self.sign_block(&children, file_scope),
            | Some("oper" | "rule") => self.judgment(&children, file_scope),
            | _ => {},
        }
    }

    /// Check one `sign` block: collect its members' kinds, then confirm every
    /// member against the scope they all share.
    fn sign_block(
        &mut self,
        children: &[NodeId],
        file_scope: &KindEnv<'tree>,
    )
    {
        let mut cursor = Cursor::new(self.reader, children);
        // `sign`, the block's name, then the opening brace.
        cursor.bump();
        cursor.bump();
        if !cursor.eat(TileSpelling("{")).0 {
            return;
        }
        let region = cursor.until_close_brace();
        let members = split_before_leads(self.reader, &region, &MEMBER_LEADS);
        let mut scope = file_scope.clone();
        for member in &members {
            if let Some((name, kind)) = self.judgment_head(member) {
                scope.bind(name, kind);
            }
        }
        for member in &members {
            self.judgment(member, &scope);
        }
    }

    /// Check one keyword-led judgment — a `sign` member or a top-level
    /// declaration — against `scope`.
    fn judgment(
        &mut self,
        run: &[NodeId],
        scope: &KindEnv<'tree>,
    )
    {
        let Some((name, kind)) = self.judgment_head(run)
        else {
            return;
        };
        let Some(tail) = self.after_colon(run)
        else {
            return;
        };
        let (signature, body) = self.split_body(tail);
        let subject = Subject::Declaration { name, kind };
        let binders = self.signature(signature, kind.row(), subject);
        let Some(body) = body
        else {
            return;
        };
        let mut inner = scope.clone();
        for &(binder, binder_kind) in &binders {
            inner.bind(binder, binder_kind);
        }
        for statement in split_at_top_level(self.reader, body, TileSpelling(";")) {
            self.statement(&statement, &inner);
        }
    }

    /// The part of a judgment run after its top-level `:`.
    fn after_colon<'run>(
        &self,
        run: &'run [NodeId],
    ) -> Option<&'run [NodeId]>
    {
        let colon = self.find_at_top_level(run, TileSpelling(":"))?;
        run.get(colon.0.saturating_add(1) ..)
    }

    /// Split a judgment's tail into its signature and its optional body
    /// interior, at the top-level `{` that opens the filler.
    ///
    /// # Contract
    /// - requires: `tail` is a judgment run past its `:`, from a **clean**
    ///   parse — the melder's commit then puts the body's matching `}` last,
    ///   which is what bounds the interior. A repaired parse can violate that;
    ///   the slice is taken with `get`, so the result is a shorter interior,
    ///   never a panic.
    /// - ensures: returns the signature before the `{`, and the interior
    ///   strictly between the `{` and the run's last node; a tail with no
    ///   top-level `{` is all signature and no body.
    /// - provides: the boundary between a declaration's sphere and its filler.
    /// - fails: never.
    /// - panics: none.
    fn split_body<'run>(
        &self,
        tail: &'run [NodeId],
    ) -> (&'run [NodeId], Option<&'run [NodeId]>)
    {
        let Some(brace) = self.find_at_top_level(tail, TileSpelling("{"))
        else {
            return (tail, None);
        };
        let signature = tail.get(.. brace.0).unwrap_or(tail);
        // The body interior runs from just past the `{` to just before the
        // matching `}`, which the melder's commit guarantees is the run's last
        // significant child.
        let interior = tail
            .get(brace.0.saturating_add(1) .. tail.len().saturating_sub(1))
            .unwrap_or(&[]);
        (signature, Some(interior))
    }

    /// Confirm a signature's arrows and collect its parameter binders.
    ///
    /// Two arrow positions live here and they take different expectations: the
    /// signature's own arrow, which comes from the declared kind, and a
    /// rewrite-sorted binder's arrow inside the parameter list, which is a
    /// rule's and therefore always a face.
    fn signature(
        &mut self,
        signature: &[NodeId],
        expected: Option<ArrowRow>,
        subject: Subject<'tree>,
    ) -> Vec<(CircuitName<'tree>, CircuitKind)>
    {
        let arrow = self.find_arrow_at_top_level(signature);
        if let (Some(arrow), Some(expected)) = (arrow, expected) {
            self.confirm(arrow.node, arrow.glyph, expected, subject);
        }
        let mut binders = Vec::new();
        let Some(parameters) = self.parameter_interior(signature, arrow.map(|arrow| arrow.index))
        else {
            return binders;
        };
        for entry in split_at_top_level(self.reader, parameters, TileSpelling(",")) {
            let Some((name, kind)) = self.judgment_head(&entry)
            else {
                continue;
            };
            binders.push((name, kind));
            if let Some(arrow) = self.find_arrow_at_top_level(&entry)
                && let Some(expected) = kind.row()
            {
                self.confirm(arrow.node, arrow.glyph, expected, Subject::Binder {
                    name,
                    kind,
                });
            }
        }
        binders
    }

    /// The interior of the signature's parameter list — the first top-level
    /// `( … )` group that sits before the signature's arrow.
    ///
    /// A bare-sort parameter side has no group, and a bare-sort *result* side
    /// must not be mistaken for one, which is what the `before` bound rules
    /// out.
    ///
    /// The closer is found by scanning from the **opener**, not from just past
    /// it. [`Self::scan_top_level`] offers both brackets at the outer depth, so
    /// a scan that started inside the group would return a *nested* group's
    /// closer and silently truncate the interior — dropping every entry after
    /// the nesting, and with it every binder those entries bind.
    fn parameter_interior<'run>(
        &self,
        signature: &'run [NodeId],
        before: Option<RunIndex>,
    ) -> Option<&'run [NodeId]>
    {
        let open = self.find_at_top_level(signature, TileSpelling("("))?;
        if before.is_some_and(|arrow| open.0 >= arrow.0) {
            return None;
        }
        let group = signature.get(open.0 ..)?;
        let close = self.find_at_top_level(group, TileSpelling(")"))?;
        group.get(1 .. close.0)
    }

    /// Confirm one body statement's arrow against the kind of the thing it
    /// belongs to.
    ///
    /// A `node` line's arrow comes from the applied head's kind — the redex /
    /// frame distinction made locally visible. A `feed` line applies nothing:
    /// it is a wire, so its arrow is a circuit 1-cell former.
    fn statement(
        &mut self,
        statement: &[NodeId],
        scope: &KindEnv<'tree>,
    )
    {
        let lead = statement.first().and_then(|&id| self.reader.label(id));
        let Some(arrow) = self.find_arrow_at_top_level(statement)
        else {
            return;
        };
        match lead.map(|label| label.0) {
            | Some("feed") => {
                self.confirm(arrow.node, arrow.glyph, ArrowRow::Circuit, Subject::Feed);
            },
            | Some("node") => {
                let Some(head) = self.applied_head(statement)
                else {
                    return;
                };
                // An unresolved head is a name-resolution question, not an
                // arrow-kind one: confirming against a guess would report an
                // error the program does not have.
                let Some(kind) = scope.kind_of(head)
                else {
                    return;
                };
                let Some(expected) = kind.row()
                else {
                    return;
                };
                self.confirm(arrow.node, arrow.glyph, expected, Subject::Application {
                    head,
                    kind,
                });
            },
            | _ => {},
        }
    }

    /// The head a `node` line applies: the name just past the statement's `:`.
    fn applied_head(
        &self,
        statement: &[NodeId],
    ) -> Option<CircuitName<'tree>>
    {
        let tail = self.after_colon(statement)?;
        let &head = tail.first()?;
        self.reader.label(head)?;
        Some(CircuitName(self.reader.text(head).0))
    }

    /// Record the verdict for one arrow: a row disagreement, a reserved-glyph
    /// decline, or nothing.
    fn confirm(
        &mut self,
        id: NodeId,
        glyph: Arrow,
        expected: ArrowRow,
        subject: Subject<'tree>,
    )
    {
        let span = self.reader.span(id);
        if glyph.row() != expected {
            self.diagnostics
                .push(mismatch_diagnostic(glyph, expected, subject, span));
        }
        else if glyph.is_reserved().0 {
            self.diagnostics.push(reserved_diagnostic(subject, span));
        }
    }

    /// The index of the first top-level occurrence of `label` in `run`.
    fn find_at_top_level(
        &self,
        run: &[NodeId],
        label: TileSpelling,
    ) -> Option<RunIndex>
    {
        self.scan_top_level(run, |actual| {
            (actual == label).then_some(TopLevelHit::Label)
        })
        .map(|(index, _hit)| index)
    }

    /// The first top-level arrow glyph in `run`, located.
    fn find_arrow_at_top_level(
        &self,
        run: &[NodeId],
    ) -> Option<FoundArrow>
    {
        let (index, hit) = self.scan_top_level(run, |actual| {
            Arrow::from_label(actual).map(TopLevelHit::Arrow)
        })?;
        let &node = run.get(index.0)?;
        match hit {
            | TopLevelHit::Arrow(glyph) => Some(FoundArrow { index, node, glyph }),
            | TopLevelHit::Label => None,
        }
    }

    /// Scan `run` at bracket depth zero, returning the first hit `probe`
    /// accepts.
    ///
    /// Depth opens on the node whose label opens a bracket and closes on its
    /// closer, and both brackets themselves are offered to `probe` at the
    /// *outer* depth — which is what lets a caller find the `(` that opens a
    /// parameter list and the `)` that closes it in the same scan.
    fn scan_top_level(
        &self,
        run: &[NodeId],
        probe: impl Fn(TileSpelling) -> Option<TopLevelHit>,
    ) -> Option<(RunIndex, TopLevelHit)>
    {
        let mut depth: u32 = 0;
        for (index, &id) in run.iter().enumerate() {
            let Some(label) = self.reader.label(id)
            else {
                continue;
            };
            if is_closer(BracketLabel(label.0)).0 {
                depth = depth.saturating_sub(1);
            }
            if depth == 0
                && let Some(hit) = probe(label)
            {
                return Some((RunIndex(index), hit));
            }
            if is_opener(BracketLabel(label.0)).0 {
                depth = depth.saturating_add(1);
            }
        }
        None
    }
}

/// A located arrow-grid glyph inside a run of significant children.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FoundArrow
{
    /// Where the glyph sits in the run.
    index: RunIndex,
    /// The node carrying the glyph.
    node: NodeId,
    /// The glyph itself.
    glyph: Arrow,
}

/// A position into a run of significant children.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RunIndex(usize);

/// What a top-level scan matched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TopLevelHit
{
    /// The probed label itself.
    Label,
    /// An arrow-grid glyph.
    Arrow(Arrow),
}

/// The thing an arrow belongs to, as a diagnostic names it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Subject<'tree>
{
    /// A `sign` member or a top-level circuit declaration.
    Declaration
    {
        /// The declared name.
        name: CircuitName<'tree>,
        /// The kind its keyword declares.
        kind: CircuitKind,
    },
    /// A binder in a parameter telescope.
    Binder
    {
        /// The bound name.
        name: CircuitName<'tree>,
        /// The kind its keyword binds at.
        kind: CircuitKind,
    },
    /// A `node` line applying a head.
    Application
    {
        /// The applied head's name.
        head: CircuitName<'tree>,
        /// The kind the head is declared at.
        kind: CircuitKind,
    },
    /// A `feed` line's back-edge wire.
    Feed,
}

impl Subject<'_>
{
    /// How a diagnostic opens when it is about this subject.
    fn describe(self) -> String
    {
        match self {
            | Self::Declaration { name, kind } => {
                format!("`{} {}`", kind.keyword().0, name.0)
            },
            | Self::Binder { name, kind } => {
                format!("the binder `{} {}`", kind.keyword().0, name.0)
            },
            | Self::Application { head, kind } => {
                format!(
                    "the node line applying `{}`, declared `{}`,",
                    head.0,
                    kind.keyword().0
                )
            },
            | Self::Feed => "the feed line's back-edge wire".to_owned(),
        }
    }
}

/// The diagnostic an arrow earns when its row disagrees with its subject's
/// kind.
fn mismatch_diagnostic(
    glyph: Arrow,
    expected: ArrowRow,
    subject: Subject<'_>,
    span: SurfaceSpan,
) -> ElabDiagnostic
{
    ElabDiagnostic::new(
        format!(
            "{} takes {}, but its arrow `{}` is {}; {}",
            subject.describe(),
            expected.description().0,
            glyph.spelling().0,
            glyph.row().description().0,
            expected.correction().0
        ),
        span,
    )
}

/// The diagnostic the reserved reversible glyph earns.
fn reserved_diagnostic(
    subject: Subject<'_>,
    span: SurfaceSpan,
) -> ElabDiagnostic
{
    ElabDiagnostic::new(
        format!(
            "{} writes the reserved arrow `<->`: a reversible circuit former is a semantic claim \
             that the thing it forms is a data-level iso — distinct from a rule's \
             bidirectionality, which `<=>` carries as an engine licence — so the form parses and \
             is declined until the reversible-oper lane lands its checking discipline",
            subject.describe()
        ),
        span,
    )
}

/// Split a member region into runs that each begin at a top-level occurrence of
/// one of `leads`.
///
/// The ruled `sign` block separates its members with nothing at all: each is
/// keyword-led, so the lead *is* the separator, and a lead deeper than the top
/// level belongs to a parameter binder rather than a new member. Material
/// before the first lead is dropped, because it belongs to no member.
///
/// # Contract
/// - requires: `region` is a `sign` block's member region, with balanced
///   brackets.
/// - ensures: returns the member runs in source order, each starting at its
///   lead keyword; a region with no lead yields no runs.
/// - provides: the keyword-led member split the separator-free block form
///   needs.
/// - fails: never.
/// - panics: none.
fn split_before_leads(
    reader: &Reader<'_>,
    region: &[NodeId],
    leads: &[TileSpelling],
) -> Vec<Vec<NodeId>>
{
    let mut members: Vec<Vec<NodeId>> = Vec::new();
    let mut current: Vec<NodeId> = Vec::new();
    let mut depth: u32 = 0;
    for &id in region {
        let label = reader.label(id);
        if let Some(label) = label {
            if is_closer(BracketLabel(label.0)).0 {
                depth = depth.saturating_sub(1);
            }
            if depth == 0 && leads.contains(&label) && !current.is_empty() {
                members.push(core::mem::take(&mut current));
            }
            if is_opener(BracketLabel(label.0)).0 {
                depth = depth.saturating_add(1);
            }
        }
        if !current.is_empty() || label.is_some_and(|label| leads.contains(&label)) {
            current.push(id);
        }
    }
    if !current.is_empty() {
        members.push(current);
    }
    members
}
