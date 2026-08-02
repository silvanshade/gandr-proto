//! The **circuit surface check**: arrow-kind confirmation, the reserved
//! reversible glyph's decline, and the port-name fold that binds a body's
//! internal wires.
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
//! # The name-set fold, and why it has exactly one failure mode
//!
//! The ruling's other half is the port discipline: "each wire name is produced
//! exactly once and consumed exactly once; internal wires are **implicit** —
//! the head declares the interface, and the body names not in it are internal,
//! computed and checked by the name-set fold". [`Check::ports`] is that fold.
//! It walks the head's two port lists and the body's statements in source
//! order, and each site contributes its **uses**: a name it produces, or a name
//! it consumes. The head's input list produces the wires the body consumes and
//! its output list consumes the wires the body produces, because polarity is
//! read from the body's side of the boundary.
//!
//! The fold's carrier is a set of **sorted port names**, and a name's identity
//! is its spelling. Merging a site into the accumulator therefore has one way
//! to fail — two distinct ports arriving under one spelling — and the three
//! checks the discipline names are three readings of that one failure:
//!
//! * **Port linearity.** Two uses at the same polarity are two distinct ports:
//!   the wire already has that endpoint. This is the published attachment
//!   discipline's target-face-must-be-unused side condition
//!   \[@curien-hothanh-mimram-2019-opetopes\].
//! * **Boundary agreement.** Two uses that declare the name at different sorts
//!   are two distinct ports sharing a spelling — the same discipline's
//!   boundaries-must-agree side condition. Agreement is compared on the sort's
//!   *spelling* with whitespace elided, which is what a pass over the parsed
//!   form can see; resolved sort equality is elaboration's.
//! * **Redex disjointness.** Two redexes in one body are disjoint exactly when
//!   they share no port name, which is the parser-level reading of the
//!   horizontal-composition fence. It needs no separate rule: a shared name is
//!   already a merge failure, and the diagnostic reads the two sites to name
//!   the fence when both of them are redex lines.
//!
//! What survives the fold is the **internal-wire set**: every body name outside
//! the head's interface that has both a producer and a consumer, recorded with
//! the sites at each end ([`InternalWire`]). That is the binder the ruling
//! calls implicit — `x′` in the congruence cell flows from a redex output to a
//! frame input and is never written down as a declaration.
//!
//! # What this pass does not do
//!
//! It is a surface check, not an elaboration: it reads names, arrows, and sort
//! spellings, and nothing else. The fold rejects **only** non-disjointness, so
//! the other half of "exactly once" — a name with one endpoint and not the
//! other — is classified rather than refused: an unpaired body name simply
//! fails to become an internal wire. That residue and the node-only cycle sweep
//! whose diagnostic is _this cycle has no `feed`; close loops with `feed`_
//! belong to the back-edge rung, which owes both. Nothing here lowers, either —
//! circuit members stay parse-and-decline at lowering. An applied head the
//! environment does not know is skipped rather than guessed: an unresolved name
//! is a name-resolution question, and answering it here would report the wrong
//! error.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;

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

/// A fixed phrase a diagnostic is composed from.
///
/// Distinct from [`TileSpelling`]: a tile spelling is source the user wrote,
/// while a phrase is prose this module supplies. Keeping them apart stops a
/// diagnostic fragment from being passed where a grammar label is expected.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DiagnosticPhrase(&'static str);

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
    fn description(self) -> DiagnosticPhrase
    {
        DiagnosticPhrase(match self {
            | Self::Circuit => "a circuit 1-cell former",
            | Self::Face => "a rewrite face",
        })
    }

    /// The correction a diagnostic offers for this row.
    fn correction(self) -> DiagnosticPhrase
    {
        DiagnosticPhrase(match self {
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

/// The circuit surface check's verdict: every decline the source earned, in
/// source order, and every internal wire the name-set fold bound.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CircuitSurface
{
    /// The declines — arrow-kind disagreements, reserved-glyph reservations,
    /// and shared port names — ordered by span.
    pub diagnostics: Vec<ElabDiagnostic>,
    /// The internal wires the fold bound, in the order their producing site
    /// appears.
    pub internal_wires: Vec<InternalWire>,
}

/// One **internal wire**: a body port name outside the head's interface that
/// the fold paired with a producer and a consumer.
///
/// The ruled form writes no declaration for these — "internal wires are
/// implicit; the head declares the interface, and the body names not in it are
/// internal" — so this record *is* the binding. A body name with only one
/// endpoint never becomes one: the fold classifies, and refuses only
/// non-disjointness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternalWire
{
    /// The declaration whose body binds the wire.
    pub declaration: String,
    /// The wire's port name.
    pub name: String,
    /// The site that puts a value on the wire.
    pub produced_by: WireEnd,
    /// The site that takes the value off it.
    pub consumed_by: WireEnd,
}

/// One end of an internal wire: the body statement that produces or consumes
/// it.
///
/// The head's port lists are deliberately absent. A wire touching the interface
/// is an interface port, not an internal one, so no internal wire can carry
/// that end.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WireEnd
{
    /// A `node` line whose head is a `rule` — a **redex**.
    Redex
    {
        /// The applied head's name.
        head: String,
        /// The occurrence label, when the line carries one.
        label: Option<String>,
    },
    /// A `node` line whose head is an `oper` or a `data` constructor — a
    /// **frame**.
    Frame
    {
        /// The applied head's name.
        head: String,
        /// The occurrence label, when the line carries one.
        label: Option<String>,
    },
    /// A `node` line applying a head the environment does not know, which is a
    /// name-resolution question rather than a wiring one.
    Unresolved
    {
        /// The applied head's name.
        head: String,
        /// The occurrence label, when the line carries one.
        label: Option<String>,
    },
    /// A `feed` line's back-edge — the only cycle-forming statement.
    Feed
    {
        /// The occurrence label, when the line carries one.
        label: Option<String>,
    },
}

/// **Check** every circuit declaration in `source`.
///
/// Confirms each arrow against the kind of the thing it belongs to, declines
/// the reserved reversible glyph, and folds each body's port names into its
/// internal wires.
///
/// # Contract
/// - requires: `source` parses (an unparseable source yields an empty verdict —
///   a repair obligation is the parser's report, not this pass's).
/// - ensures: returns one diagnostic per arrow whose row disagrees with its
///   subject's kind, plus one per `<->` occurrence that agrees, plus one per
///   port name two distinct ports share; an arrow earns at most one diagnostic,
///   the disagreement taking precedence over the reservation because a `<->` in
///   the face row is in the wrong row before it is reserved. Every body name
///   outside its head's interface that the fold paired is returned as an
///   [`InternalWire`]. Non-circuit items are ignored, a declaration without a
///   filler is folded not at all (it declares a boundary and wires nothing),
///   and an applied head the environment does not know is skipped rather than
///   guessed.
/// - provides: the checker half of the ruling's three-way arrow redundancy —
///   the declaration keyword, the signature arrow, and the body-line arrows —
///   together with the port discipline's implicit internal-wire binder.
/// - fails: never; total on any input.
/// - panics: none.
/// - intension: names are collected before arrows are confirmed, so a body may
///   apply a head its block declares later; the returned diagnostics are
///   ordered by span, so a binder's arrow (which sits before the signature
///   arrow it is confirmed after) reports where the reader will look for it.
///
/// # Adequacy
/// - hypothesis: L4 — the four subjects that carry an arrow (a member
///   declaration, a top-level declaration, a rewrite-sorted binder, a body
///   line) are separated by the diagnostic each earns, and the agreeing,
///   disagreeing, and reserved glyphs are separated at one subject; the
///   unresolved-head path is observed by its silence. For the fold: the bound
///   internal-wire set and its two endpoint classes separate the well-formed
///   body, and the three readings of non-disjointness (same polarity, sort
///   disagreement, two redexes) are separated by the diagnostic each earns.
/// - witness: `gandr-surface-engine` `tests/circuit.rs`
///   `the_ruled_worked_examples_confirm_every_arrow`
/// - witness: `gandr-surface-engine` `tests/circuit.rs`
///   `a_declaration_arrow_disagreeing_with_its_kind_is_named`
/// - witness: `gandr-surface-engine` `tests/circuit.rs`
///   `a_body_line_arrow_disagreeing_with_its_head_is_named`
/// - witness: `gandr-surface-engine` `tests/circuit.rs`
///   `the_reserved_reversible_glyph_declines_naming_its_lane`
/// - witness: `gandr-surface-engine` `tests/circuit_ports.rs`
///   `a_well_formed_block_binds_its_internal_wires`
/// - witness: `gandr-surface-engine` `tests/circuit_ports.rs`
///   `a_shared_port_name_is_refused_naming_the_name`
/// - witness: `gandr-surface-engine` `tests/circuit_ports.rs`
///   `two_redexes_sharing_a_port_name_are_not_disjoint`
/// - witness: `gandr-surface-engine` `tests/circuit_ports.rs`
///   `a_ports_uses_must_agree_in_sort`
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
        internal_wires: Vec::new(),
    };
    let file_scope = check.file_scope(&items);
    for &item in &items {
        check.item(item, &file_scope);
    }
    // Two checks share one traversal, and neither visits its subjects in span
    // order on its own: a rewrite binder's arrow sits inside the parameter list
    // it is confirmed after, and a body's fold runs once the whole body has been
    // walked. Sorting restores the source order the verdict promises.
    let mut diagnostics = check.diagnostics;
    diagnostics.sort_by_key(|diagnostic| (diagnostic.span.start, diagnostic.span.end));
    CircuitSurface {
        diagnostics,
        internal_wires: check.internal_wires,
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
    /// The declines collected so far, in traversal order.
    diagnostics: Vec<ElabDiagnostic>,
    /// The internal wires the fold has bound so far, in traversal order.
    internal_wires: Vec<InternalWire>,
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
        let mut inner = scope.clone();
        for &(binder, binder_kind) in &binders {
            inner.bind(binder, binder_kind);
        }
        // A declaration without a filler declares a boundary and wires nothing,
        // so it has no statements — but its interface still has port names, and
        // two of those sharing a spelling is the same failure whether or not a
        // body reads them.
        let statements = match body {
            | Some(body) => split_at_top_level(self.reader, body, TileSpelling(";")),
            | None => Vec::new(),
        };
        for statement in &statements {
            self.statement(statement, &inner);
        }
        self.ports(name, signature, &statements, &inner);
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
        self.group_interior(signature)
    }

    /// The interior of the first top-level `( … )` group in `run`.
    ///
    /// The closer is found by scanning from the **opener**, not from just past
    /// it. [`Self::scan_top_level`] offers both brackets at the outer depth, so
    /// a scan that started inside the group would return a *nested* group's
    /// closer and silently truncate the interior — dropping every entry after
    /// the nesting, and with it every port and binder those entries carry.
    fn group_interior<'run>(
        &self,
        run: &'run [NodeId],
    ) -> Option<&'run [NodeId]>
    {
        let open = self.find_at_top_level(run, TileSpelling("("))?;
        let group = run.get(open.0 ..)?;
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

    // --- The name-set fold ----------------------------------------------------

    /// Fold one declaration's port names: the head's two lists and every body
    /// statement, merged into a set of sorted port names.
    ///
    /// # Contract
    /// - requires: `signature` is the judgment's tail before its filler and
    ///   `statements` the filler's `;`-separated lines, both from the same
    ///   declaration — empty for a declaration that fills no boundary; `scope`
    ///   resolves the heads those lines apply.
    /// - ensures: pushes one diagnostic per merge that found a spelling shared
    ///   by two distinct ports, and one [`InternalWire`] per body name outside
    ///   the head's interface that acquired both a producer and a consumer.
    /// - provides: the ruling's implicit internal-wire binder, and the port
    ///   discipline's single rejection path.
    /// - fails: never; the fold is total and a malformed run simply contributes
    ///   fewer uses.
    /// - panics: none.
    /// - intension: sites are merged in source order — the input list, the
    ///   output list, then the body lines — so the *first* of two colliding
    ///   uses is the earlier one and the diagnostic is located at the later.
    fn ports(
        &mut self,
        declaration: CircuitName<'tree>,
        signature: &[NodeId],
        statements: &[Vec<NodeId>],
        scope: &KindEnv<'tree>,
    )
    {
        let mut fold = PortFold::default();
        let arrow = self.find_arrow_at_top_level(signature);
        if let Some(interior) = self.parameter_interior(signature, arrow.map(|found| found.index)) {
            self.declared_ports(
                &mut fold,
                interior,
                Polarity::Produced,
                InterfaceSide::Input,
            );
        }
        if let Some(found) = arrow
            && let Some(result) = signature.get(found.index.0.saturating_add(1) ..)
            && let Some(interior) = self.group_interior(result)
        {
            self.declared_ports(
                &mut fold,
                interior,
                Polarity::Consumed,
                InterfaceSide::Output,
            );
        }
        for statement in statements {
            self.statement_ports(&mut fold, statement, scope);
        }
        for shared in &fold.shared {
            self.diagnostics.push(shared_diagnostic(shared));
        }
        self.internal_wires.extend(fold.internal_wires(declaration));
    }

    /// Merge one side of the head's interface: its **declared** ports, each of
    /// which may carry a sort.
    ///
    /// Polarity is read from the body's side of the boundary, which is why the
    /// input list produces and the output list consumes.
    fn declared_ports(
        &self,
        fold: &mut PortFold<'tree>,
        interior: &[NodeId],
        polarity: Polarity,
        side: InterfaceSide,
    )
    {
        for entry in split_at_top_level(self.reader, interior, TileSpelling(",")) {
            let Some(port) = self.declared_port(&entry)
            else {
                continue;
            };
            fold.declare(port.name);
            fold.merge(PortUse {
                name: port.name,
                site: PortSite::Interface(side),
                polarity,
                sort: Some(port.sort),
                span: port.span,
            });
        }
    }

    /// Read one entry of a head's port list.
    ///
    /// Three of the four rungs bind no name the body can reach. A **`rule`
    /// binder** is a rewrite-sorted parameter — a head the body applies, which
    /// [`KindEnv`] already carries — and not a wire. A bare sort and a `_` port
    /// are the sugar ladder's fresh-name rungs: "unnamed tuple inputs mint
    /// fresh names in order", and a freshly minted name collides with
    /// nothing, so contributing no use is exactly right. Only a named port
    /// and a `data` binder name a wire.
    fn declared_port(
        &self,
        entry: &[NodeId],
    ) -> Option<DeclaredPort<'tree>>
    {
        if let Some((name, kind)) = self.judgment_head(entry) {
            if !matches!(kind, CircuitKind::Data) {
                return None;
            }
            let sort = self.sort_text(entry)?;
            let &at = entry.get(1)?;
            return Some(DeclaredPort {
                name,
                sort,
                span: self.reader.span(at),
            });
        }
        let colon = self.find_at_top_level(entry, TileSpelling(":"))?;
        if colon.0 != 1 {
            return None;
        }
        let &at = entry.first()?;
        let name = self.port_name(at)?;
        let sort = self.sort_text(entry)?;
        Some(DeclaredPort {
            name,
            sort,
            span: self.reader.span(at),
        })
    }

    /// The sort spelling an entry declares, with whitespace elided.
    ///
    /// Agreement is compared on this spelling because it is what a pass over
    /// the parsed form can see: a port type reaches the flat run either as
    /// tiles or as one Meld, and concatenating their texts reproduces the
    /// written type either way. Eliding whitespace keeps `Stream(Nat)` and
    /// `Stream( Nat )` the same sort; resolved equality is elaboration's,
    /// not this pass's.
    fn sort_text(
        &self,
        entry: &[NodeId],
    ) -> Option<PortSortText>
    {
        let tail = self.after_colon(entry)?;
        let mut spelling = String::new();
        for &id in tail {
            for character in self.reader.text(id).0.chars() {
                if !character.is_whitespace() {
                    spelling.push(character);
                }
            }
        }
        if spelling.is_empty() {
            return None;
        }
        Some(PortSortText(spelling))
    }

    /// The wire name a port-name node spells, or [`None`] for the `_` that
    /// mints a fresh one.
    fn port_name(
        &self,
        id: NodeId,
    ) -> Option<CircuitName<'tree>>
    {
        let label = self.reader.label(id)?;
        if label.0 != "identifier" {
            return None;
        }
        Some(CircuitName(self.reader.text(id).0))
    }

    /// Merge one body statement's port uses.
    ///
    /// A `node` line consumes its head's arguments and produces the tuple past
    /// the arrow; a `feed` line does the same with a tuple on each side.
    /// Neither the applied head nor the occurrence label is a wire.
    fn statement_ports(
        &self,
        fold: &mut PortFold<'tree>,
        statement: &[NodeId],
        scope: &KindEnv<'tree>,
    )
    {
        let lead = statement.first().and_then(|&id| self.reader.label(id));
        let label = self.occurrence_label(statement);
        let Some(tail) = self.after_colon(statement)
        else {
            return;
        };
        // The inputs a line consumes: a `node` line's argument list sits past
        // the applied head, a `feed` line's tuple sits immediately past the `:`.
        let (site, inputs) = match lead.map(|found| found.0) {
            | Some("node") => {
                let Some(head) = self.applied_head(statement)
                else {
                    return;
                };
                let role = NodeRole::of(scope.kind_of(head));
                let Some(arguments) = tail.get(1 ..)
                else {
                    return;
                };
                (PortSite::Node { label, head, role }, arguments)
            },
            | Some("feed") => (PortSite::Feed { label }, tail),
            | _ => return,
        };
        if let Some(interior) = self.group_interior(inputs) {
            self.tuple_ports(fold, interior, Polarity::Consumed, site);
        }
        // The outputs it produces: the tuple past the line's arrow.
        if let Some(found) = self.find_arrow_at_top_level(statement)
            && let Some(results) = statement.get(found.index.0.saturating_add(1) ..)
            && let Some(interior) = self.group_interior(results)
        {
            self.tuple_ports(fold, interior, Polarity::Produced, site);
        }
    }

    /// The occurrence label a body statement carries between its keyword and
    /// its `:`, when it carries one.
    fn occurrence_label(
        &self,
        statement: &[NodeId],
    ) -> Option<CircuitName<'tree>>
    {
        let colon = self.find_at_top_level(statement, TileSpelling(":"))?;
        if colon.0 != 2 {
            return None;
        }
        let &at = statement.get(1)?;
        self.reader.label(at)?;
        Some(CircuitName(self.reader.text(at).0))
    }

    /// Merge every name in a body port tuple `(a, b)`.
    ///
    /// Tuple entries are bare names: they are already bound, or being bound, by
    /// the wiring, so none of them declares a sort. A `_` mints a fresh wire
    /// and contributes nothing.
    fn tuple_ports(
        &self,
        fold: &mut PortFold<'tree>,
        interior: &[NodeId],
        polarity: Polarity,
        site: PortSite<'tree>,
    )
    {
        for entry in split_at_top_level(self.reader, interior, TileSpelling(",")) {
            let Some(&at) = entry.first()
            else {
                continue;
            };
            let Some(name) = self.port_name(at)
            else {
                continue;
            };
            fold.merge(PortUse {
                name,
                site,
                polarity,
                sort: None,
                span: self.reader.span(at),
            });
        }
    }
}

/// Which side of a wire an occurrence stands on.
///
/// The ruled form carries polarity by **position** rather than by a sigil, and
/// the interface's positions are read from the *body's* side of the boundary:
/// the head's input list hands wires to the body, so it produces them, and its
/// output list takes wires from the body, so it consumes them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Polarity
{
    /// The occurrence puts a value on the wire.
    Produced,
    /// The occurrence takes the value off it.
    Consumed,
}

impl Polarity
{
    /// How a diagnostic says a name was used at this polarity.
    fn participle(self) -> DiagnosticPhrase
    {
        DiagnosticPhrase(match self {
            | Self::Produced => "produced",
            | Self::Consumed => "consumed",
        })
    }
}

/// Which of a head's two port lists a use came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InterfaceSide
{
    /// Left of the signature's arrow.
    Input,
    /// Right of it.
    Output,
}

/// Whether a `node` line is a redex, a frame, or neither because its head is
/// unresolved.
///
/// The spec's own vocabulary: "a frame line carries `-->`, a redex line carries
/// `==>` (the applied head's kind)". The fold reuses it so a wire's ends read
/// as the diagram they describe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeRole
{
    /// The head is a `rule`.
    Redex,
    /// The head is an `oper` or a `data` constructor.
    Frame,
    /// The environment does not know the head.
    Unresolved,
}

impl NodeRole
{
    /// The role a head of this kind gives its line.
    fn of(kind: Option<CircuitKind>) -> Self
    {
        match kind {
            | Some(CircuitKind::Rule) => Self::Redex,
            | Some(CircuitKind::Data | CircuitKind::Oper) => Self::Frame,
            // A `sort` heads nothing: a line applying one is as unresolvable as
            // a line applying a name nobody declared.
            | Some(CircuitKind::Sort) | None => Self::Unresolved,
        }
    }

    /// How a diagnostic names a line in this role.
    fn description(self) -> DiagnosticPhrase
    {
        DiagnosticPhrase(match self {
            | Self::Redex => "redex",
            | Self::Frame => "frame",
            | Self::Unresolved => "node",
        })
    }
}

/// The site one port use came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PortSite<'tree>
{
    /// One side of the head's interface.
    Interface(InterfaceSide),
    /// A `node` line applying a head.
    Node
    {
        /// The occurrence label, when the line carries one.
        label: Option<CircuitName<'tree>>,
        /// The applied head's name.
        head: CircuitName<'tree>,
        /// Whether the line is a redex or a frame.
        role: NodeRole,
    },
    /// A `feed` line's back-edge.
    Feed
    {
        /// The occurrence label, when the line carries one.
        label: Option<CircuitName<'tree>>,
    },
}

impl PortSite<'_>
{
    /// How a diagnostic names this site.
    fn describe(self) -> String
    {
        match self {
            | Self::Interface(InterfaceSide::Input) => "the head's input list".to_owned(),
            | Self::Interface(InterfaceSide::Output) => "the head's output list".to_owned(),
            | Self::Node { label, head, role } => match label {
                | Some(label) => format!(
                    "the {} line `{}` applying `{}`",
                    role.description().0,
                    label.0,
                    head.0
                ),
                | None => format!("the {} line applying `{}`", role.description().0, head.0),
            },
            | Self::Feed { label } => match label {
                | Some(label) => format!("the feed line `{}`", label.0),
                | None => "the feed line".to_owned(),
            },
        }
    }

    /// This site as one end of an internal wire, or [`None`] for the interface
    /// — a wire the interface touches is a port, never an internal wire.
    fn body_end(self) -> Option<WireEnd>
    {
        match self {
            | Self::Interface(_) => None,
            | Self::Node { label, head, role } => {
                let head = head.0.to_owned();
                let label = label.map(|found| found.0.to_owned());
                Some(match role {
                    | NodeRole::Redex => WireEnd::Redex { head, label },
                    | NodeRole::Frame => WireEnd::Frame { head, label },
                    | NodeRole::Unresolved => WireEnd::Unresolved { head, label },
                })
            },
            | Self::Feed { label } => Some(WireEnd::Feed {
                label: label.map(|found| found.0.to_owned()),
            }),
        }
    }
}

/// A port's declared sort, spelled with whitespace elided.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct PortSortText(String);

/// One entry of a head's port list that names a wire.
#[derive(Clone, Debug)]
struct DeclaredPort<'tree>
{
    /// The wire's name.
    name: CircuitName<'tree>,
    /// The sort the entry declares it at.
    sort: PortSortText,
    /// Where the name is written.
    span: SurfaceSpan,
}

/// One occurrence of a port name.
#[derive(Clone, Debug)]
struct PortUse<'tree>
{
    /// The name used.
    name: CircuitName<'tree>,
    /// Where it was used.
    site: PortSite<'tree>,
    /// Which side of the wire the use stands on.
    polarity: Polarity,
    /// The sort the use declares, for the sites that declare one.
    sort: Option<PortSortText>,
    /// Where the occurrence is written.
    span: SurfaceSpan,
}

/// One name's two endpoints — at most one producer and at most one consumer,
/// which is what "produced exactly once and consumed exactly once" bounds.
#[derive(Clone, Debug, Default)]
struct Wire<'tree>
{
    /// The use that puts a value on the wire.
    produced: Option<PortUse<'tree>>,
    /// The use that takes it off.
    consumed: Option<PortUse<'tree>>,
}

/// Why two uses under one spelling are two **distinct** ports.
///
/// Both readings are the same failure — a spelling shared by two ports — and
/// this only chooses which sentence explains it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SharingReading
{
    /// They stand on the same side of the wire, so the wire already has that
    /// endpoint.
    SamePolarity,
    /// They declare the name at different sorts, so they cannot be one port.
    SortDisagreement,
}

/// One refused merge: a port name two distinct ports arrived under.
#[derive(Clone, Debug)]
struct Shared<'tree>
{
    /// The shared name — what the diagnostic must say.
    name: CircuitName<'tree>,
    /// The use already in the fold.
    first: PortUse<'tree>,
    /// The use that could not join it.
    second: PortUse<'tree>,
    /// Which sentence explains the sharing.
    reading: SharingReading,
}

/// The name-set fold's accumulator.
#[derive(Clone, Debug, Default)]
struct PortFold<'tree>
{
    /// Each port name's endpoints so far.
    wires: BTreeMap<CircuitName<'tree>, Wire<'tree>>,
    /// Every merge the fold refused, in merge order.
    shared: Vec<Shared<'tree>>,
    /// The names the head's interface declares, which are ports rather than
    /// internal wires.
    interface: BTreeSet<CircuitName<'tree>>,
}

impl<'tree> PortFold<'tree>
{
    /// Record `name` as an interface port, so the fold never counts it
    /// internal.
    fn declare(
        &mut self,
        name: CircuitName<'tree>,
    )
    {
        self.interface.insert(name);
    }

    /// Merge one use into the fold.
    ///
    /// # Contract
    /// - requires: uses arrive in source order, so the retained endpoint of a
    ///   refused merge is the earlier one.
    /// - ensures: fills the name's endpoint at `entry`'s polarity when that
    ///   endpoint is free and no sort already recorded for the name disagrees
    ///   with `entry`'s; otherwise records the sharing and leaves the fold
    ///   unchanged, so a third use collides against the same first one rather
    ///   than cascading.
    /// - provides: the fold's single rejection path.
    /// - fails: never; a refused merge is recorded, not returned.
    /// - panics: none.
    fn merge(
        &mut self,
        entry: PortUse<'tree>,
    )
    {
        let name = entry.name;
        let wire = self.wires.get(&name).cloned().unwrap_or_default();
        let (same, opposite) = match entry.polarity {
            | Polarity::Produced => (wire.produced, wire.consumed),
            | Polarity::Consumed => (wire.consumed, wire.produced),
        };
        if let Some(first) = same {
            self.shared.push(Shared {
                name,
                first,
                second: entry,
                reading: SharingReading::SamePolarity,
            });
            return;
        }
        if let Some(ref first) = opposite
            && let Some(ref standing) = first.sort
            && let Some(ref arriving) = entry.sort
            && standing != arriving
        {
            let first = first.clone();
            self.shared.push(Shared {
                name,
                first,
                second: entry,
                reading: SharingReading::SortDisagreement,
            });
            return;
        }
        let wire = self.wires.entry(name).or_default();
        match entry.polarity {
            | Polarity::Produced => wire.produced = Some(entry),
            | Polarity::Consumed => wire.consumed = Some(entry),
        }
    }

    /// The internal wires this fold bound: every name outside the interface
    /// that acquired both endpoints, ordered by where its producer is
    /// written.
    ///
    /// A name with one endpoint is left out rather than refused. Half of
    /// "exactly once" is disjointness, which the fold enforces; the other half
    /// is totality, which the back-edge rung owes together with its cycle
    /// sweep.
    fn internal_wires(
        &self,
        declaration: CircuitName<'_>,
    ) -> Vec<InternalWire>
    {
        let mut bound: Vec<(SurfaceSpan, InternalWire)> = Vec::new();
        for (name, wire) in &self.wires {
            if self.interface.contains(name) {
                continue;
            }
            let (Some(produced), Some(consumed)) = (wire.produced.as_ref(), wire.consumed.as_ref())
            else {
                continue;
            };
            let (Some(produced_by), Some(consumed_by)) =
                (produced.site.body_end(), consumed.site.body_end())
            else {
                continue;
            };
            bound.push((produced.span, InternalWire {
                declaration: declaration.0.to_owned(),
                name: name.0.to_owned(),
                produced_by,
                consumed_by,
            }));
        }
        bound.sort_by_key(|&(span, _)| (span.start, span.end));
        bound.into_iter().map(|(_span, wire)| wire).collect()
    }
}

/// The diagnostic a shared port name earns.
///
/// The name comes first in every sentence, because naming the shared name is
/// what the discipline promises a wiring error will do.
fn shared_diagnostic(shared: &Shared<'_>) -> ElabDiagnostic
{
    let message = match shared.reading {
        | SharingReading::SamePolarity => {
            let fence = redex_fence(shared);
            if shared.first.site == shared.second.site {
                format!(
                    "the port name `{}` is {} twice by {}: {}each wire name is produced exactly \
                     once and consumed exactly once",
                    shared.name.0,
                    shared.second.polarity.participle().0,
                    shared.second.site.describe(),
                    fence
                )
            }
            else {
                format!(
                    "the port name `{}` is {} by {} and again by {}: {}each wire name is produced \
                     exactly once and consumed exactly once",
                    shared.name.0,
                    shared.second.polarity.participle().0,
                    shared.first.site.describe(),
                    shared.second.site.describe(),
                    fence
                )
            }
        },
        | SharingReading::SortDisagreement => format!(
            "the port name `{}` is declared at `{}` by {} and at `{}` by {}: a port's uses must \
             agree in sort",
            shared.name.0,
            sort_spelling(shared.first.sort.as_ref()),
            shared.first.site.describe(),
            sort_spelling(shared.second.sort.as_ref()),
            shared.second.site.describe()
        ),
    };
    ElabDiagnostic::new(message, shared.second.span)
}

/// The horizontal-composition fence, stated when both sharing sites are redex
/// lines — the case the ruling singles out.
fn redex_fence(shared: &Shared<'_>) -> String
{
    let is_redex = |site: PortSite<'_>| {
        matches!(site, PortSite::Node {
            role: NodeRole::Redex,
            ..
        })
    };
    if is_redex(shared.first.site) && is_redex(shared.second.site) {
        "two redexes in one body are disjoint exactly when they share no port name, and ".to_owned()
    }
    else {
        String::new()
    }
}

/// A declared sort as a diagnostic spells it, or the empty spelling for a use
/// that declares none.
fn sort_spelling(sort: Option<&PortSortText>) -> String
{
    sort.map_or_else(String::new, |found| found.0.clone())
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
