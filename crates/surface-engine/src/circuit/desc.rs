//! The ruled circuit block form's **description route**: a `sign` block read
//! into the declaration table, with its block-bodied `rule` members carried as
//! [`CircuitRule`]s.
//!
//! This is the acceptance flip. A `sign` block used to reach the lowerer as an
//! unsupported form and hole; it now reads into a
//! [`gandr_theory_levitation::DataDesc`] whose `circuits` the declaration table
//! checks ([`gandr_theory_levitation::check_desc`]) and whose cells the cell
//! layer admits through its single admission seam
//! ([`gandr_theory_computads::elaborate_data_desc`]). Nothing here decides
//! admission: the gate is the cell layer's, and this module's job is to hand it
//! a description that says what the block said.
//!
//! # What a `sign` block becomes
//!
//! | member                   | becomes                                          |
//! | ------------------------ | ------------------------------------------------ |
//! | `sort S : Type`          | nothing — the colour is the block's own identity |
//! | `data C : sig`           | a constructor of the description                 |
//! | `oper f : sig`           | an operation, with its two port lists as arity   |
//! | `rule r : sphere { … }`  | a [`CircuitRule`] — the graduated form           |
//! | `rule r : sphere`        | nothing — see below                              |
//!
//! A **`rule` member with no filler** declares a rewrite face between *sorts*
//! (`rule involutive : (b : Bit) <=> (c : Bit)`), and the declaration table has
//! no slot for a term-free sphere: a [`gandr_theory_levitation::CellFace`]
//! carries two terms, and the sphere-typed representation `Φ ▸ x ⇴ y` is the
//! higher-cells lane's (`docs/gandr/spec/surface-language/higher-cells.md`,
//! section "Sphere-typed boundaries"). Such a member is therefore carried by no
//! description member rather than approximated by one; its arrow is still
//! confirmed by the surface check. An **`oper` member with a filler** is a
//! circuit 1-cell *definition*, which is a different graduation from this one,
//! and it is declined by name rather than silently dropped.
//!
//! # Where the sphere comes from, and what that costs
//!
//! **The ruled block form writes no sphere in terms.** Its signature fixes the
//! sphere's *sorts* and its interface; the endpoint terms would have to come
//! from the parameter telescope, and even a pinned binder `rule p : x ==> x′`
//! names only that port's own endpoints, never the whole diagram's
//! (`docs/gandr/spec/surface-language/circuit-cells.md`, section "The derived
//! pair meets the sphere by checking, not by synthesis": "Where every
//! rewrite-sorted binder is unpinned, the declaration writes no endpoint terms
//! at all … and the sphere is supplied from outside the block").
//!
//! So this route supplies [`CircuitRule::sphere`] as the pair the wiring
//! derives, and the consequence is stated rather than hidden: **the declaration
//! table's derived-boundary comparison is a re-derivation agreement for a
//! source-authored rule, not a constraint the declaration imposes.** The
//! direction the spec protects is intact — nothing writes a derived pair into a
//! sphere a declaration fixed — but there is no such sphere to protect yet. The
//! explicit two-sided boundary spelling, carried as an open question in the
//! same section, is what would make the comparison bite from source; until it
//! lands, the declaration-table refusals this route can earn are the ones that
//! read the declaration rather than the wiring: an out-of-signature frame head,
//! a redex applying a rewrite the telescope does not declare, and a wiring that
//! reaches a port from itself.
//!
//! # What is declined, and by which lane
//!
//! Three shapes the ruled grammar writes leave the term-shaped store, and each
//! is declined with the question it belongs to rather than admitted
//! speculatively:
//!
//! * a **many-out node** or a **many-out interface** — one output port per
//!   statement is what a single-rooted term can hold, and widening it is the
//!   cell-alphabet question (`gandr-ui9`);
//! * a **`feed` statement** — a wheel-bearing body derives no boundary pair,
//!   and separating a delay-guarded cycle from an unguarded one is owed with
//!   the `feed` statement's own elaboration;
//! * a body whose declared output port unfolds to **more than one redex
//!   occurrence** — horizontal composition, licensed only against an earned
//!   shift-equivalence witness, and sequential composition, which the boundary
//!   language spells `ρ then ρ′` and has no former for yet
//!   ([`gandr_theory_levitation::CircuitElaborationError::ManyRedexOccurrences`]).

use gandr_core_checker::boundary::NameRef;
use gandr_core_checker::grade::Grade;
use gandr_surface_syntax::NodeId;
use gandr_theory_levitation::Attrs;
use gandr_theory_levitation::CellFace;
use gandr_theory_levitation::CircuitBody;
use gandr_theory_levitation::CircuitDerivationError;
use gandr_theory_levitation::CircuitFrame;
use gandr_theory_levitation::CircuitNode;
use gandr_theory_levitation::CircuitRule;
use gandr_theory_levitation::Code;
use gandr_theory_levitation::CtorDesc;
use gandr_theory_levitation::DataDesc;
use gandr_theory_levitation::DeclPolarity;
use gandr_theory_levitation::FrameHead;
use gandr_theory_levitation::FreeTerm;
use gandr_theory_levitation::Name;
use gandr_theory_levitation::NominalId;
use gandr_theory_levitation::OpDesc;
use gandr_theory_levitation::PortInstantiationError;
use gandr_theory_levitation::PrimTy;
use gandr_theory_levitation::RewritePort;
use gandr_theory_levitation::SortDesc;
use gandr_theory_levitation::SortRef;
use gandr_theory_levitation::SurfaceSpan;
use gandr_theory_levitation::ValueTypeRef;
use gandr_theory_levitation::boundary::NominalSerial;
use gandr_theory_levitation::derive_boundaries;
use gandr_theory_levitation::wellformed::derive_cell_var_meta;

use crate::boundary::CircuitName;
use crate::boundary::MatchDecision;
use crate::boundary::TileSpelling;
use crate::boundary::TypeName;
use crate::circuit::shape::CircuitKind;
use crate::circuit::shape::KindEnv;
use crate::circuit::shape::MEMBER_LEADS;
use crate::circuit::shape::Shape;
use crate::circuit::shape::split_before_leads;
use crate::cst_read::Cursor;
use crate::desc_elab::ElabDiagnostic;

/// The tracker item the cell-alphabet question is owed to, named in the decline
/// so a reader can find the lane rather than guess at it.
const ALPHABET_QUESTION: &str = "gandr-ui9";

/// Read one `sign` block into a description, appending every decline.
///
/// # Contract
/// - requires: `node` is a `sign` block's Meld, from a parse `shape` reads.
/// - ensures: `Some(desc)` for a block whose header reads (`sign`, a name, and
///   an opening brace); the description carries one constructor per `data`
///   member, one operation per `oper` member, and one [`CircuitRule`] per
///   block-bodied `rule` member the route admits; every member it does not
///   carry appends a diagnostic naming the member and the lane that owes it.
/// - provides: the acceptance flip's surface half — the declaration table entry
///   a parsed circuit block denotes.
/// - fails: never; a member the route cannot read contributes a diagnostic and
///   no description member.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the four member families are separated by what each
///   contributes (a constructor, an operation, a circuit rule, nothing), and
///   the decline channels are separated by the member each names.
/// - witness: `gandr-surface-engine` `tests/circuit_desc.rs`
///   `a_ruled_sign_block_lowers_its_members_into_a_description`
/// - witness: `gandr-surface-engine` `tests/circuit_desc.rs`
///   `a_many_out_node_declines_naming_the_cell_alphabet_question`
/// - witness: `gandr-surface-engine` `tests/circuit_desc.rs`
///   `a_feed_statement_declines_naming_the_wheel_obligation`
pub fn sign_desc(
    shape: Shape<'_, '_>,
    node: NodeId,
    serial: NominalSerial,
    diagnostics: &mut Vec<ElabDiagnostic>,
) -> Option<DataDesc>
{
    let children = shape.reader.sig_children(node);
    let mut cursor = Cursor::new(shape.reader, &children);
    // `sign`, the block's name, then the opening brace.
    cursor.bump();
    let name_id = cursor.bump()?;
    let name = shape.reader.text(name_id).0.to_owned();
    if !cursor.eat(TileSpelling("{")).0 {
        return None;
    }
    let region = cursor.until_close_brace();
    let members = split_before_leads(shape.reader, &region, &MEMBER_LEADS);
    Some(block_desc(shape, &members, name, serial, diagnostics))
}

/// Read one top-level circuit declaration into a **singleton** description.
///
/// A top-level `oper` / `rule` stands outside any `sign` block, so it has no
/// sibling members and no block name: the description is named after the
/// declaration itself, and its signature is the only one in scope. That is
/// enough for the declaration table to check a `rule`'s wiring against its own
/// telescope; it is deliberately *not* enough for a frame head to resolve,
/// which is why a top-level rule applying an undeclared frame earns the
/// out-of-signature refusal rather than silence.
pub fn circuit_declaration_desc(
    shape: Shape<'_, '_>,
    node: NodeId,
    serial: NominalSerial,
    diagnostics: &mut Vec<ElabDiagnostic>,
) -> Option<DataDesc>
{
    let children = shape.reader.sig_children(node);
    let (name, _kind) = shape.judgment_head(&children)?;
    Some(block_desc(
        shape,
        core::slice::from_ref(&children),
        name.0.to_owned(),
        serial,
        diagnostics,
    ))
}

/// Assemble one description out of a run of keyword-led members.
fn block_desc(
    shape: Shape<'_, '_>,
    members: &[Vec<NodeId>],
    name: String,
    serial: NominalSerial,
    diagnostics: &mut Vec<ElabDiagnostic>,
) -> DataDesc
{
    // Every member's kind is collected before any body is read, so a body may
    // apply a head its block declares later (the surface check's own order).
    let mut kinds = KindEnv::default();
    for member in members {
        if let Some((member_name, kind)) = shape.judgment_head(member) {
            kinds.bind(member_name, kind);
        }
    }
    // The declared sort set: every `sort` member, in declaration order; a
    // block writing none declares the degenerate single sort named by the
    // block itself (the `DataDesc::new` default).
    let mut sorts: Vec<SortDesc> = Vec::new();
    for member in members {
        if let Some((member_name, CircuitKind::Sort)) = shape.judgment_head(member) {
            sorts.push(SortDesc::new(member_name.0, DeclPolarity::Data));
        }
    }
    let sort_names: Vec<String> = if sorts.is_empty() {
        vec![name.clone()]
    }
    else {
        sorts.iter().map(|sort| sort.name.to_string()).collect()
    };
    let mut ctors: Vec<CtorDesc> = Vec::new();
    let mut ops: Vec<OpDesc> = Vec::new();
    let mut circuits: Vec<CircuitRule> = Vec::new();
    for member in members {
        let Some((member_name, kind)) = shape.judgment_head(member)
        else {
            continue;
        };
        match kind {
            // The sort members were collected above.
            | CircuitKind::Sort => {},
            | CircuitKind::Data => {
                ctors.push(ctor_member(shape, member, member_name, &sort_names));
            },
            | CircuitKind::Oper => {
                if let Some(op) = oper_member(shape, member, member_name, diagnostics) {
                    ops.push(op);
                }
            },
            | CircuitKind::Rule => {
                if let Some(rule) = rule_member(shape, member, member_name, &kinds, diagnostics) {
                    circuits.push(rule);
                }
            },
        }
    }
    let desc = DataDesc::new(
        NominalId::new(serial, name),
        Vec::new(),
        ctors,
        ops,
        Vec::new(),
        DeclPolarity::Data,
        Attrs::empty(),
    )
    .with_circuits(circuits);
    if sorts.is_empty() {
        desc
    }
    else {
        desc.with_sorts(sorts)
    }
}

/// Read a `data C : sig` member into a constructor descriptor.
///
/// The sugar ladder is what fixes the field list: `data Zero : Nat` is `() -->
/// (_ : Nat)` and `data Succ : Nat --> Nat` is `(_ : Nat) --> (_ : Nat)`, so a
/// bare-sort side with no arrow declares the *result* and no fields, and the
/// parameter side's ports are the payload. A port at a **declared sort** is a
/// recursive occurrence ([`Code::Var`] at that sort — the sort-indexed
/// universe's reading); any other port sort stays a symbolic field. The
/// result-sort slot is the output port's sort, defaulting to the block's
/// first sort for a member writing no output.
fn ctor_member(
    shape: Shape<'_, '_>,
    run: &[NodeId],
    name: CircuitName<'_>,
    sort_names: &[String],
) -> CtorDesc
{
    let (inputs, output) = signature_ports(shape, run);
    let fields: Vec<Code> = inputs
        .iter()
        .map(|port| {
            if sort_names.contains(&port.sort) {
                Code::var(port.sort.as_str())
            }
            else {
                Code::field(
                    sort_ref(TypeName(port.sort.as_str())),
                    Grade::ONE,
                    Attrs::empty(),
                )
            }
        })
        .collect();
    let result: Name = output.map_or_else(
        || sort_names.first().map_or("", String::as_str).into(),
        |port| port.sort.into(),
    );
    CtorDesc::new(
        name.0.to_owned(),
        Code::product_of(fields),
        result,
        Attrs::empty(),
    )
}

/// Read an `oper f : sig` member into an operation descriptor, declining a
/// filler.
fn oper_member(
    shape: Shape<'_, '_>,
    run: &[NodeId],
    name: CircuitName<'_>,
    diagnostics: &mut Vec<ElabDiagnostic>,
) -> Option<OpDesc>
{
    if has_filler(shape, run).0 {
        diagnostics.push(ElabDiagnostic::new(
            format!(
                "the filler of `oper {}` defines a circuit 1-cell, which this route does not \
                 carry: the description layer holds an operation's arity, not its wiring, and the \
                 1-cell definition form graduates with its own rung",
                name.0
            ),
            run_span(shape, run),
        ));
        return None;
    }
    let (inputs, output) = signature_ports(shape, run);
    let inputs: Vec<SortRef> = inputs
        .into_iter()
        .map(|port| SortRef::new(port.name, port.sort))
        .collect();
    let outputs = output
        .into_iter()
        .map(|port| SortRef::new(port.name, port.sort))
        .collect::<Vec<SortRef>>();
    Some(OpDesc::new(
        name.0.to_owned(),
        crate::desc_elab::bridge_arity(inputs, outputs),
        Attrs::empty(),
    ))
}

/// Read a block-bodied `rule r : sphere { filler }` member into a circuit rule.
///
/// This is the graduated form. Everything the route cannot carry declines here
/// with the lane that owes it, and nothing is admitted speculatively.
fn rule_member(
    shape: Shape<'_, '_>,
    run: &[NodeId],
    name: CircuitName<'_>,
    kinds: &KindEnv<'_>,
    diagnostics: &mut Vec<ElabDiagnostic>,
) -> Option<CircuitRule>
{
    let span = run_span(shape, run);
    let tail = shape.after_colon(run)?;
    let (signature, interior) = shape.split_body(tail);
    // A `rule` member with no filler declares a face between sorts, which the
    // declaration table has no term-free slot for; the higher-cells lane owes
    // the sphere-typed representation, and the surface check has already
    // confirmed the member's arrow.
    let interior = interior?;
    let ports = telescope(shape, signature);
    let out = match declared_output(shape, signature) {
        | Ok(out) => out,
        | Err(decline) => {
            diagnostics.push(ElabDiagnostic::new(decline.message(name), span));
            return None;
        },
    };
    let mut nodes: Vec<CircuitNode> = Vec::new();
    let mut fresh = FreshWire::default();
    for statement in shape.statements(interior) {
        match body_node(shape, &statement, &ports, kinds, &mut fresh) {
            | Ok(Some(node)) => nodes.push(node),
            | Ok(None) => {},
            | Err(decline) => {
                diagnostics.push(ElabDiagnostic::new(
                    decline.message(name),
                    run_span(shape, &statement),
                ));
                return None;
            },
        }
    }
    let body = CircuitBody::new(nodes, out);
    // The sphere the surface supplies is the pair the wiring derives (see the
    // module documentation: the ruled form writes no sphere in terms). A wiring
    // that derives nothing keeps its declared output port on both sides, so the
    // declaration table reaches its own refusal rather than this route
    // inventing one.
    let sphere = match derive_boundaries(&body) {
        | Ok(derived) => face(derived.source, derived.target, span),
        | Err(CircuitDerivationError::CyclicWiring(ref port)) => {
            let placeholder = FreeTerm::Var(port.clone());
            face(placeholder.clone(), placeholder, span)
        },
        | Err(CircuitDerivationError::NodeBudget { budget }) => {
            diagnostics.push(ElabDiagnostic::new(
                format!(
                    "circuit rule `{}`'s wiring unfolds past the derivation's node budget of \
                     {budget}: a wire consumed twice is unfolded twice, so reconvergence is a \
                     shared subterm on the term-shaped store and a body of doubling frames \
                     derives an exponentially large boundary",
                    name.0
                ),
                span,
            ));
            return None;
        },
    };
    Some(CircuitRule::new(name.0.to_owned(), sphere, body).with_ports(ports))
}

/// The cell face a derived boundary pair makes, with its per-variable metadata
/// derived rather than declared.
fn face(
    source: FreeTerm,
    target: FreeTerm,
    span: SurfaceSpan,
) -> CellFace
{
    let vars = derive_cell_var_meta(&source);
    CellFace::new(source, target, vars, span)
}

/// Read the parameter telescope's **rewrite-sorted** binders, keyed by name.
///
/// A `data` binder and a plain port name wires rather than a head, so neither
/// is a rewrite port; the name-set fold binds those and the derivation resolves
/// them as boundary variables.
fn telescope(
    shape: Shape<'_, '_>,
    signature: &[NodeId],
) -> Vec<RewritePort>
{
    let mut ports = Vec::new();
    let arrow = shape.find_arrow_at_top_level(signature);
    let Some(interior) = shape.parameter_interior(signature, arrow.map(|found| found.index))
    else {
        return ports;
    };
    for entry in shape.comma_entries(interior) {
        let Some((name, CircuitKind::Rule)) = shape.judgment_head(&entry)
        else {
            continue;
        };
        let Some(found) = shape.find_arrow_at_top_level(&entry)
        else {
            continue;
        };
        let Some(colon) = shape.find_at_top_level(&entry, TileSpelling(":"))
        else {
            continue;
        };
        let Some(source) = entry.get(colon.0.saturating_add(1) .. found.index.0)
        else {
            continue;
        };
        let Some(target) = entry.get(found.index.0.saturating_add(1) ..)
        else {
            continue;
        };
        // Pinning is all-or-nothing: one arrow cannot carry a sort on one side
        // and a term on the other, so a binder is sorted exactly when both of
        // its sides are sorts.
        let port = if is_sort_run(shape, source).0 && is_sort_run(shape, target).0 {
            RewritePort::sorted(
                name.0,
                shape
                    .run_text(source)
                    .map_or_else(String::new, |text| text.0),
            )
        }
        else {
            RewritePort::pinned(
                name.0,
                shape.free_term_run(source),
                shape.free_term_run(target),
            )
        };
        ports.push(port);
    }
    ports
}

/// Whether a run spells a **sort** rather than a term: one type identifier, or
/// one Meld headed by one.
fn is_sort_run(
    shape: Shape<'_, '_>,
    run: &[NodeId],
) -> MatchDecision
{
    let [id] = *run
    else {
        return MatchDecision(false);
    };
    if let Some(label) = shape.reader.label(id) {
        return MatchDecision(label.0 == "type_identifier");
    }
    let inner = shape.reader.sig_children(id);
    MatchDecision(
        inner
            .first()
            .and_then(|&head| shape.reader.label(head))
            .is_some_and(|label| label.0 == "type_identifier"),
    )
}

/// The single output port a rule's declared interface names.
fn declared_output(
    shape: Shape<'_, '_>,
    signature: &[NodeId],
) -> Result<String, Decline>
{
    let Some(interior) = shape.result_interior(signature)
    else {
        return Err(Decline::UnnamedOutputPort);
    };
    let entries = shape.comma_entries(interior);
    let [ref entry] = *entries
    else {
        return Err(Decline::ManyOutInterface {
            ports: entries.len(),
        });
    };
    let Some(port) = shape.declared_port(entry)
    else {
        return Err(Decline::UnnamedOutputPort);
    };
    Ok(port.name.0.to_owned())
}

/// Read one body statement into a wiring node.
fn body_node(
    shape: Shape<'_, '_>,
    statement: &[NodeId],
    ports: &[RewritePort],
    kinds: &KindEnv<'_>,
    fresh: &mut FreshWire,
) -> Result<Option<CircuitNode>, Decline>
{
    let lead = statement
        .first()
        .and_then(|&id| shape.reader.label(id))
        .map(|label| label.0);
    match lead {
        | Some("feed") => Err(Decline::FeedStatement),
        | Some("node") => node_statement(shape, statement, ports, kinds, fresh).map(Some),
        | _ => Ok(None),
    }
}

/// Read a `node w? : head(args) <arrow> (out)` statement into a frame or a
/// redex.
fn node_statement(
    shape: Shape<'_, '_>,
    statement: &[NodeId],
    ports: &[RewritePort],
    kinds: &KindEnv<'_>,
    fresh: &mut FreshWire,
) -> Result<CircuitNode, Decline>
{
    let Some(head) = shape.applied_head(statement)
    else {
        return Err(Decline::MalformedStatement);
    };
    let Some(tail) = shape.after_colon(statement)
    else {
        return Err(Decline::MalformedStatement);
    };
    let arguments = tail
        .get(1 ..)
        .and_then(|rest| shape.group_interior(rest))
        .map(|interior| wire_names(shape, interior, fresh))
        .unwrap_or_default();
    let Some(interior) = shape.result_interior(statement)
    else {
        return Err(Decline::MalformedStatement);
    };
    let outs = wire_names(shape, interior, fresh);
    let [ref out] = *outs
    else {
        return Err(Decline::ManyOutNode {
            head: head.0.to_owned(),
            ports: outs.len(),
        });
    };
    if let Some(port) = ports.iter().find(|port| port.name.as_ref() == head.0) {
        let args: Vec<FreeTerm> = arguments
            .iter()
            .map(|name| FreeTerm::var(name.as_str()))
            .collect();
        let redex = port
            .interface()
            .instantiate(args, out.clone())
            .map_err(|error| Decline::Instantiation { error })?;
        return Ok(CircuitNode::Redex(redex));
    }
    // A head the block declares at `data` builds a constructor application and
    // one declared at `oper` an operation application. A head nothing declares
    // is still read as an operation: the declaration table's in-signature rule
    // is what refuses it, and refusing it here would report a name-resolution
    // failure as a wiring one.
    let frame_head = match kinds.kind_of(head) {
        | Some(CircuitKind::Data) => FrameHead::Ctor(head.0.into()),
        | _ => FrameHead::Op(head.0.into()),
    };
    let args: Vec<FreeTerm> = arguments
        .iter()
        .map(|name| FreeTerm::var(name.as_str()))
        .collect();
    Ok(CircuitNode::Frame(CircuitFrame::new(
        frame_head,
        args,
        out.clone(),
    )))
}

/// The wire names a body port tuple spells, with `_` minting a fresh one.
fn wire_names(
    shape: Shape<'_, '_>,
    interior: &[NodeId],
    fresh: &mut FreshWire,
) -> Vec<String>
{
    let mut names = Vec::new();
    for entry in shape.comma_entries(interior) {
        let Some(&at) = entry.first()
        else {
            continue;
        };
        match shape.port_name(at) {
            | Some(name) => names.push(name.0.to_owned()),
            | None => names.push(fresh.mint()),
        }
    }
    names
}

/// The fresh names the sugar ladder's `_` rung mints, in order.
///
/// The minted spelling carries mathematical angle brackets, which the surface's
/// identifier lexis cannot produce, so a minted wire can never be captured by a
/// body port written with the same letters — the same guarantee
/// [`gandr_theory_levitation::RewritePort::interface`] rests on for its
/// endpoint variables.
#[repr(transparent)]
#[derive(Debug, Default)]
struct FreshWire
{
    /// How many have been minted so far.
    minted: usize,
}

impl FreshWire
{
    /// The next fresh wire name.
    fn mint(&mut self) -> String
    {
        let name = format!("_\u{27e8}{}\u{27e9}", self.minted);
        self.minted = self.minted.saturating_add(1);
        name
    }
}

/// Why a circuit rule member does not reach the declaration table.
enum Decline
{
    /// The rule's interface declares several output ports.
    ManyOutInterface
    {
        /// How many it declares.
        ports: usize,
    },
    /// The rule's interface declares no *named* output port, so the derivation
    /// has nothing to resolve.
    UnnamedOutputPort,
    /// A body statement binds several output ports.
    ManyOutNode
    {
        /// The applied head.
        head: String,
        /// How many ports the line binds.
        ports: usize,
    },
    /// The body carries a `feed` back-edge.
    FeedStatement,
    /// A redex line does not instantiate the port it applies.
    Instantiation
    {
        /// The instantiation's own refusal.
        error: PortInstantiationError,
    },
    /// A statement the route could not read at all.
    MalformedStatement,
}

impl Decline
{
    /// How this decline reads, for the rule it declines.
    fn message(
        &self,
        rule: CircuitName<'_>,
    ) -> String
    {
        match *self {
            | Self::ManyOutInterface { ports } => format!(
                "circuit rule `{}` declares {ports} output ports: the derivation reads one \
                 declared output port, because the boundaries it produces land on the \
                 term-shaped store and a term has one root — a many-out interface is the \
                 cell-alphabet question ({ALPHABET_QUESTION}), declined here rather than omitted",
                rule.0
            ),
            | Self::UnnamedOutputPort => format!(
                "circuit rule `{}` names no output port, so the derivation has nothing to \
                 resolve; write the result side as a named port list `( name : Sort )`",
                rule.0
            ),
            | Self::ManyOutNode { ref head, ports } => format!(
                "circuit rule `{}`'s line applying `{head}` binds {ports} output ports: each body \
                 statement this route admits binds exactly one, and a many-out node is the \
                 cell-alphabet question ({ALPHABET_QUESTION}), declined here rather than omitted",
                rule.0
            ),
            | Self::FeedStatement => format!(
                "circuit rule `{}` carries a `feed` back-edge: a wheel-bearing body derives no \
                 boundary pair, so a delay-guarded cycle is refused by the same guard that \
                 refuses an unguarded one, and separating them is owed with the `feed` \
                 statement's own elaboration",
                rule.0
            ),
            | Self::Instantiation { ref error } => match *error {
                | PortInstantiationError::SourceArity {
                    ref rewrite,
                    expected,
                    supplied,
                } => format!(
                    "circuit rule `{}`'s redex applies `{rewrite}` to {supplied} argument(s), but \
                     its source interface has {expected} distinct pattern variable(s)",
                    rule.0
                ),
                | PortInstantiationError::UnboundTargetEndpoint {
                    ref rewrite,
                    ref endpoints,
                } => format!(
                    "circuit rule `{}`'s redex applies `{rewrite}`, whose declared target names \
                     the endpoint(s) {} that neither its source nor the line's output port \
                     supplies",
                    rule.0,
                    endpoints
                        .iter()
                        .map(|endpoint| format!("`{endpoint}`"))
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            },
            | Self::MalformedStatement => format!(
                "circuit rule `{}` carries a body statement this route cannot read",
                rule.0
            ),
        }
    }
}

/// The value-type reference a port's sort spelling denotes.
///
/// A circuit port's sort reaches this route as a spelling rather than as a
/// resolved type — resolved equality is elaboration's, not the surface's — so
/// the reference is the primitive the spelling names, or a nullary constructor
/// application over it.
fn sort_ref(sort: TypeName<'_>) -> ValueTypeRef
{
    PrimTy::from_label(NameRef::from(sort.0)).map_or_else(
        || ValueTypeRef::Ctor {
            head: sort.0.into(),
            args: Box::default(),
        },
        ValueTypeRef::Prim,
    )
}

/// One entry of a signature's port list, as the description layer needs it.
struct SignaturePort
{
    /// The port's name — the written one, or the sugar ladder's minted one.
    name: String,
    /// The sort spelling the entry declares.
    sort: String,
}

/// Read a member's signature into its input ports and its single output port.
///
/// The sugar ladder is applied here, which is what makes the named-port normal
/// form the only shape past this point: a bare-sort side is one unnamed port,
/// an unnamed port mints a fresh name in order, and a side with no arrow is the
/// result (`data Zero : Nat` is `() --> (_ : Nat)`).
fn signature_ports(
    shape: Shape<'_, '_>,
    run: &[NodeId],
) -> (Vec<SignaturePort>, Option<SignaturePort>)
{
    let Some(tail) = shape.after_colon(run)
    else {
        return (Vec::new(), None);
    };
    let (signature, _filler) = shape.split_body(tail);
    let mut fresh = FreshWire::default();
    let Some(found) = shape.find_arrow_at_top_level(signature)
    else {
        // No arrow: the whole side is the result, and the member takes no
        // inputs.
        return (Vec::new(), side_ports(shape, signature, &mut fresh).pop());
    };
    let inputs = signature
        .get(.. found.index.0)
        .map(|side| side_ports(shape, side, &mut fresh))
        .unwrap_or_default();
    let output = signature
        .get(found.index.0.saturating_add(1) ..)
        .map(|side| side_ports(shape, side, &mut fresh))
        .unwrap_or_default()
        .pop();
    (inputs, output)
}

/// Read one side of a signature — a parenthesized port list, or the bare sort
/// that is sugar for a one-port list.
fn side_ports(
    shape: Shape<'_, '_>,
    side: &[NodeId],
    fresh: &mut FreshWire,
) -> Vec<SignaturePort>
{
    let Some(interior) = shape.group_interior(side)
    else {
        return shape
            .run_text(side)
            .map(|sort| {
                vec![SignaturePort {
                    name: fresh.mint(),
                    sort: sort.0,
                }]
            })
            .unwrap_or_default();
    };
    let mut ports = Vec::new();
    for entry in shape.comma_entries(interior) {
        // A rewrite-sorted binder is a head, not a port: it carries no sort a
        // constructor field or an operation arity could hold.
        if matches!(
            shape.judgment_head(&entry),
            Some((_, CircuitKind::Rule | CircuitKind::Oper | CircuitKind::Sort))
        ) {
            continue;
        }
        match shape.declared_port(&entry) {
            | Some(port) => ports.push(SignaturePort {
                name: port.name.0.to_owned(),
                sort: port.sort.0,
            }),
            | None => {
                if let Some(sort) = shape.run_text(&entry) {
                    ports.push(SignaturePort {
                        name: fresh.mint(),
                        sort: sort.0,
                    });
                }
            },
        }
    }
    ports
}

/// Whether a member carries a filler.
fn has_filler(
    shape: Shape<'_, '_>,
    run: &[NodeId],
) -> MatchDecision
{
    MatchDecision(
        shape
            .after_colon(run)
            .and_then(|tail| shape.split_body(tail).1)
            .is_some(),
    )
}

/// The span a member run covers, from its lead to its last node.
fn run_span(
    shape: Shape<'_, '_>,
    run: &[NodeId],
) -> SurfaceSpan
{
    let Some(&lead) = run.first()
    else {
        return crate::cst_read::empty_surface_span();
    };
    let &last = run.last().unwrap_or(&lead);
    SurfaceSpan::new(shape.reader.span(lead).start, shape.reader.span(last).end)
}

impl Shape<'_, '_>
{
    /// Read a run of significant children as a boundary term.
    ///
    /// A pinned endpoint reaches the flat run either as one tile (`x`) or as
    /// one Meld (`Succ(x)`), so the single-node case is the term reader's; a
    /// longer run is spelled out and read as a variable, which is the honest
    /// reading of a shape this route does not otherwise parse.
    fn free_term_run(
        self,
        run: &[NodeId],
    ) -> FreeTerm
    {
        match *run {
            | [id] => self.reader.free_term(id),
            | _ => FreeTerm::var(
                self.run_text(run)
                    .map_or_else(String::new, |text| text.0)
                    .as_str(),
            ),
        }
    }
}
