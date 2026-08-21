//! The ruled circuit block form's **shared shape vocabulary**: the member
//! kinds, the arrow grid, the kind environment, and the run scanning every pass
//! over a circuit declaration does.
//!
//! Two passes read the same shape and must not drift: the surface check (the
//! parent module's arrow-kind confirmation and name-set fold) and the
//! description route ([`super::desc`], the lowering into
//! [`gandr_theory_levitation::SignDesc`]). A block's shape is one reading —
//! where its `:` sits, which run is its signature, which is its filler, what a
//! parameter entry declares — so that reading lives here and both passes take
//! it from one place rather than reimplementing it against the same tiles.
//!
//! This module knows tiles, runs, and the member grammar. It knows nothing
//! about diagnostics or about descriptions; each consumer supplies its own.

use gandr_surface_grammar::Sort;
use gandr_surface_syntax::NodeId;
use gandr_theory_levitation::SurfaceSpan;

use crate::boundary::CircuitName;
use crate::boundary::MatchDecision;
use crate::boundary::TileSpelling;
use crate::cst_read::BracketLabel;
use crate::cst_read::Reader;
use crate::cst_read::is_closer;
use crate::cst_read::is_opener;
use crate::cst_read::split_at_top_level;

/// A fixed phrase a diagnostic is composed from.
///
/// Distinct from [`TileSpelling`]: a tile spelling is source the user wrote,
/// while a phrase is prose a pass supplies. Keeping them apart stops a
/// diagnostic fragment from being passed where a grammar label is expected.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticPhrase(pub &'static str);

/// The kind a circuit name is declared at — the description universe's own
/// structure, surfaced as the member keywords.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CircuitKind
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
    pub fn keyword(self) -> TileSpelling
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
    pub fn from_keyword(label: TileSpelling) -> Option<Self>
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
    pub fn row(self) -> Option<ArrowRow>
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
pub enum ArrowRow
{
    /// The circuit 1-cell formers `-->` and `<->`.
    Circuit,
    /// The rewrite faces `==>` and `<=>`, at every dimension.
    Face,
}

impl ArrowRow
{
    /// How a diagnostic names this row.
    pub fn description(self) -> DiagnosticPhrase
    {
        DiagnosticPhrase(match self {
            | Self::Circuit => "a circuit 1-cell former",
            | Self::Face => "a rewrite face",
        })
    }

    /// The correction a diagnostic offers for this row.
    pub fn correction(self) -> DiagnosticPhrase
    {
        DiagnosticPhrase(match self {
            | Self::Circuit => "write `-->`",
            | Self::Face => "write `==>`, or `<=>` for the invertible face",
        })
    }
}

/// One glyph of the four-glyph arrow grid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arrow
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
    pub fn from_label(label: TileSpelling) -> Option<Self>
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
    pub fn row(self) -> ArrowRow
    {
        match self {
            | Self::DirectedCircuit | Self::ReversibleCircuit => ArrowRow::Circuit,
            | Self::DirectedFace | Self::InvertibleFace => ArrowRow::Face,
        }
    }

    /// Whether this glyph is reserved — parses, and declines until its lane
    /// lands a checking discipline.
    pub fn is_reserved(self) -> MatchDecision
    {
        MatchDecision(matches!(self, Self::ReversibleCircuit))
    }

    /// How the glyph is written.
    pub fn spelling(self) -> TileSpelling
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
pub struct KindEnv<'tree>
{
    /// Name to declared kind.
    entries: alloc::collections::BTreeMap<CircuitName<'tree>, CircuitKind>,
}

impl<'tree> KindEnv<'tree>
{
    /// Record `name` as declared at `kind`, shadowing any earlier binding.
    pub fn bind(
        &mut self,
        name: CircuitName<'tree>,
        kind: CircuitKind,
    )
    {
        self.entries.insert(name, kind);
    }

    /// The kind `name` was declared at, or [`None`] when it is not in scope.
    pub fn kind_of(
        &self,
        name: CircuitName<'tree>,
    ) -> Option<CircuitKind>
    {
        self.entries.get(&name).copied()
    }
}

/// The member keywords a `sign` block's members are led by.
pub const MEMBER_LEADS: [TileSpelling; 4] = [
    TileSpelling("sort"),
    TileSpelling("data"),
    TileSpelling("oper"),
    TileSpelling("rule"),
];

/// A position into a run of significant children.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunIndex(pub usize);

/// A located arrow-grid glyph inside a run of significant children.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FoundArrow
{
    /// Where the glyph sits in the run.
    pub index: RunIndex,
    /// The node carrying the glyph.
    pub node: NodeId,
    /// The glyph itself.
    pub glyph: Arrow,
}

/// What a top-level scan matched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TopLevelHit
{
    /// The probed label itself.
    Label,
    /// An arrow-grid glyph.
    Arrow(Arrow),
}

/// A port's declared sort, spelled with whitespace elided.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortSortText(pub String);

/// One entry of a head's port list that names a wire.
#[derive(Clone, Debug)]
pub struct DeclaredPort<'tree>
{
    /// The wire's name.
    pub name: CircuitName<'tree>,
    /// The sort the entry declares it at.
    pub sort: PortSortText,
    /// Where the name is written.
    pub span: SurfaceSpan,
}

/// The **shape reading** of a circuit declaration: where its `:` sits, which
/// run is its signature, which is its filler, and what each of its entries
/// declares.
///
/// Copyable and stateless — it is a reader plus the ruled form's grammar,
/// nothing else — so a pass holds one and asks it questions rather than
/// threading a cursor.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Shape<'run, 'tree>
{
    /// The reader resolving tile labels against the grammar.
    pub reader: &'run Reader<'tree>,
}

impl<'tree> Shape<'_, 'tree>
{
    /// The declared name and kind of a keyword-led judgment run
    /// (`kw name : …`), or [`None`] when the run is not one.
    pub fn judgment_head(
        self,
        run: &[NodeId],
    ) -> Option<(CircuitName<'tree>, CircuitKind)>
    {
        let &lead = run.first()?;
        let label = self.reader.label(lead)?;
        let kind = CircuitKind::from_keyword(label)?;
        let &name = run.get(1)?;
        Some((CircuitName(self.reader.text(name).0), kind))
    }

    /// The part of a judgment run after its top-level `:`.
    pub fn after_colon(
        self,
        run: &[NodeId],
    ) -> Option<&[NodeId]>
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
    pub fn split_body(
        self,
        tail: &[NodeId],
    ) -> (&[NodeId], Option<&[NodeId]>)
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

    /// The grammar sort of the first parenthesized parameter group before the
    /// signature arrow, or `None` when the side is bare or the group is after
    /// the arrow.
    ///
    /// # Contract
    /// - requires: `signature` is one declaration's signature run.
    /// - ensures: returns the opener's grammar sort without descending into the
    ///   group.
    /// - provides: the distinction between a circuit binder group (`Item`) and
    ///   a shared expression group (`Expression`).
    /// - fails: never; a missing node yields `None`.
    /// - panics: none.
    pub fn parameter_group_sort(
        self,
        signature: &[NodeId],
        before: Option<RunIndex>,
    ) -> Option<Sort>
    {
        for (index, &id) in signature.iter().enumerate() {
            let index = RunIndex(index);
            if before.is_some_and(|arrow| index.0 >= arrow.0) {
                break;
            }
            if self.reader.label(id) == Some(TileSpelling("(")) {
                return self.reader.sort(id);
            }
            if self.reader.is_meld(id).0 && self.reader.sort(id) == Some(Sort::Expression) {
                return Some(Sort::Expression);
            }
        }
        None
    }

    /// The interior of the signature's parameter list — the first top-level
    /// `( … )` group that sits before the signature's arrow.
    ///
    /// A bare-sort parameter side has no group, and a bare-sort *result* side
    /// must not be mistaken for one, which is what the `before` bound rules
    /// out.
    pub fn parameter_interior(
        self,
        signature: &[NodeId],
        before: Option<RunIndex>,
    ) -> Option<&[NodeId]>
    {
        let open = self.find_at_top_level(signature, TileSpelling("("))?;
        if before.is_some_and(|arrow| open.0 >= arrow.0) {
            return None;
        }
        self.group_interior(signature)
    }

    /// The interior of the signature's **result** list — the first top-level
    /// `( … )` group past its arrow.
    pub fn result_interior(
        self,
        signature: &[NodeId],
    ) -> Option<&[NodeId]>
    {
        let found = self.find_arrow_at_top_level(signature)?;
        let result = signature.get(found.index.0.saturating_add(1) ..)?;
        self.group_interior(result)
    }

    /// The interior of the first top-level `( … )` group in `run`.
    ///
    /// The closer is found by scanning from the **opener**, not from just past
    /// it. [`Self::scan_top_level`] offers both brackets at the outer depth, so
    /// a scan that started inside the group would return a *nested* group's
    /// closer and silently truncate the interior — dropping every entry after
    /// the nesting, and with it every port and binder those entries carry.
    pub fn group_interior(
        self,
        run: &[NodeId],
    ) -> Option<&[NodeId]>
    {
        let open = self.find_at_top_level(run, TileSpelling("("))?;
        let group = run.get(open.0 ..)?;
        let close = self.find_at_top_level(group, TileSpelling(")"))?;
        group.get(1 .. close.0)
    }

    /// The head a `node` line applies: the name just past the statement's `:`.
    pub fn applied_head(
        self,
        statement: &[NodeId],
    ) -> Option<CircuitName<'tree>>
    {
        let tail = self.after_colon(statement)?;
        let &head = tail.first()?;
        self.reader.label(head)?;
        Some(CircuitName(self.reader.text(head).0))
    }

    /// The occurrence label a body statement carries between its keyword and
    /// its `:`, when it carries one.
    pub fn occurrence_label(
        self,
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

    /// The wire name a port-name node spells, or [`None`] for the `_` that
    /// mints a fresh one.
    pub fn port_name(
        self,
        id: NodeId,
    ) -> Option<CircuitName<'tree>>
    {
        let label = self.reader.label(id)?;
        if label.0 != "identifier" {
            return None;
        }
        Some(CircuitName(self.reader.text(id).0))
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
    pub fn declared_port(
        self,
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
    pub fn sort_text(
        self,
        entry: &[NodeId],
    ) -> Option<PortSortText>
    {
        let tail = self.after_colon(entry)?;
        self.run_text(tail)
    }

    /// The spelling of a whole run, with whitespace elided, or [`None`] when it
    /// is empty.
    pub fn run_text(
        self,
        run: &[NodeId],
    ) -> Option<PortSortText>
    {
        let mut spelling = String::new();
        for &id in run {
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

    /// The index of the first top-level occurrence of `label` in `run`.
    pub fn find_at_top_level(
        self,
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
    pub fn find_arrow_at_top_level(
        self,
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
        self,
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

    /// Split a run into its comma-separated entries at bracket depth zero.
    pub fn comma_entries(
        self,
        run: &[NodeId],
    ) -> Vec<Vec<NodeId>>
    {
        split_at_top_level(self.reader, run, TileSpelling(","))
    }

    /// Split a filler interior into its `;`-separated statements.
    pub fn statements(
        self,
        interior: &[NodeId],
    ) -> Vec<Vec<NodeId>>
    {
        split_at_top_level(self.reader, interior, TileSpelling(";"))
    }
}

/// Split a member region into runs that each begin at a top-level occurrence
/// of one of `leads`.
///
/// The ruled `sign` block terminates every member with `;`, and the
/// terminator is load-bearing at the member level: an unterminated member
/// list would otherwise parse cleanly as the wrong tree. A lead
/// deeper than the top level belongs to a parameter binder rather than a new
/// member, and material before the first lead is dropped, because it belongs
/// to no member. Each run's trailing top-level `;` — the member's terminator
/// — is stripped, so the invariants the consumers were written against hold
/// unchanged: a body-carrying member's run ends at its body's `}`, and a
/// boundary-only member's run ends at its signature.
///
/// # Contract
/// - requires: `region` is a `sign` block's member region, with balanced
///   brackets.
/// - ensures: returns the member runs in source order, each starting at its
///   lead keyword and stripped of its trailing `;` terminator when one molds; a
///   region with no lead yields no runs.
/// - provides: the keyword-led member split the terminated block form needs.
/// - fails: never.
/// - panics: none.
pub fn split_before_leads(
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
    for member in &mut members {
        // The member's terminator is the run's last tile when one molded (a
        // repaired parse can lose it); no member form otherwise ends in `;`.
        if member
            .last()
            .and_then(|&id| reader.label(id))
            .is_some_and(|label| label.0 == ";")
        {
            member.pop();
        }
    }
    members
}
