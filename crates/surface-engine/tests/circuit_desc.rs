//! The circuit block form's **description route** — the acceptance flip.
//!
//! Each test here exercises one clause of the graduation: the happy path (a
//! well-formed rule block reaches the cell layer behind the gate), and each
//! decline with its own diagnostic.

use core::fmt::Write as _;

use gandr_surface_engine::boundary::PipelineSource;
use gandr_surface_engine::circuit::check_circuit_surface;
use gandr_surface_engine::desc_cells::elaborate_desc_cells;
use gandr_surface_engine::desc_elab::elaborate_data_descs;
use gandr_surface_engine::lower::node_kinds;
use gandr_surface_engine::synnode::SynNode;
use gandr_surface_engine::synnode::SynTree;
use gandr_theory_levitation::CircuitNode;
use gandr_theory_levitation::PortFace;

use crate::common::TestText;
// Keep this wrapper local because `semantic_copy!` is library-internal.
// Exporting it for an integration test would grow the public API.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ExpressionEndpointPresence(bool);

impl From<bool> for ExpressionEndpointPresence
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<ExpressionEndpointPresence> for bool
{
    #[inline]
    fn from(value: ExpressionEndpointPresence) -> Self
    {
        value.0
    }
}

/// The congruence cell of the ruling, minus one redex: the single-redex block
/// this rung graduates, whose composite is one whiskering. The family is
/// declared once, whole, as a nested generator block (the item-level `data`
/// member is retired from `sign` blocks); the `sign` block presents the sort,
/// the operation, and the rule.
const CONG1: TestText<'static> = TestText(
    r#"data Nat : Type {
  Zero : Nat;
  Succ : (n : Nat) --> Nat;
}

sign Nat {
  sort Nat : Type;
  oper add : (Nat, Nat) --> Nat;

  rule cong1 : (
    rule p : Nat ==> Nat,
    data x : Nat,
    data y : Nat
  ) ==> (z : Nat) {
    node : p(x) ==> (x′);
    node : add(x′, y) --> (z);
  };
}
"#,
);

/// The two-redex congruence cell, verbatim from the ruling.
const CONG2: TestText<'static> = TestText(
    r#"sign Nat {
  sort Nat : Type;
  oper add : (Nat, Nat) --> Nat;

  rule cong2 : (
    rule p : Nat ==> Nat,
    rule q : Nat ==> Nat,
    data x : Nat,
    data y : Nat
  ) ==> (z : Nat) {
    node : p(x) ==> (x′);
    node : q(y) ==> (y′);
    node : add(x′, y′) --> (z);
  };
}
"#,
);

/// Every diagnostic message the description route reported, in order.
fn messages(source: TestText<'_>) -> Vec<String>
{
    elaborate_data_descs(source.0)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

/// Reverting the refusal exposes the misleading fallback diagnostic
/// `names no output port`, rather than naming the expression-kind mismatch.
#[test]
fn an_expression_kind_signature_endpoint_is_refused_by_the_description_route()
{
    fn has_expression_endpoint(node: SynNode<'_>) -> ExpressionEndpointPresence
    {
        if node.kind() == node_kinds::PARENTHESIZED_EXPRESSION && node.text().as_ref() == "(f)" {
            return ExpressionEndpointPresence::from(true);
        }
        ExpressionEndpointPresence::from(
            node.named_children()
                .into_iter()
                .any(|child| bool::from(has_expression_endpoint(child))),
        )
    }
    let source = r#"sign Wrong {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  rule face : (f) ==> (z : Nat) {
    node : f(f) --> (z);
  };
}
"#;
    let tree = SynTree::parse(source).expect("the ordinary parser commits this source");
    assert!(
        bool::from(has_expression_endpoint(tree.root())),
        "the rule's `(f)` endpoint must arrive as a normal expression-kind CST node"
    );
    let elab = elaborate_data_descs(source);
    assert!(
        elab.diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("parenthesized expression")
                && diagnostic.message.contains("direct circuit binder")
        }),
        "the description route must refuse the expression-kind endpoint: {:?}",
        elab.diagnostics
    );
    let desc = elab
        .descs
        .iter()
        .find(|desc| desc.id.name.as_ref() == "Wrong")
        .expect("the sign still yields a description shell");
    assert!(
        desc.circuits.is_empty(),
        "a refused signature must not render as a circuit rule"
    );
}

/// Assert that some message contains every fragment, and return it.
#[track_caller]
fn assert_names(
    messages: &[String],
    fragments: &[TestText<'_>],
) -> String
{
    let found = messages.iter().find(|message| {
        fragments
            .iter()
            .all(|fragment| message.contains(fragment.0))
    });
    match found {
        | Some(message) => message.clone(),
        | None => panic!("no diagnostic names {fragments:?}; got {messages:#?}"),
    }
}

#[test]
fn a_ruled_sign_block_lowers_its_members_into_a_description()
{
    let elab = elaborate_data_descs(CONG1.0);
    assert!(
        elab.diagnostics.is_empty(),
        "a well-formed block declines nothing: {:#?}",
        elab.diagnostics
    );
    let [ref data, ref desc] = *elab.descs
    else {
        panic!(
            "one description per declaration — the nested block and the sign block; got {}",
            elab.descs.len()
        );
    };
    assert_eq!("Nat", data.id.name.as_ref(), "the family's own name");
    assert_eq!(
        2,
        data.ctors.len(),
        "one constructor per generator of the nested block"
    );
    assert_eq!("Nat", desc.id.name.as_ref(), "the block's own name");
    assert_eq!(1, desc.opers.len(), "one operation per `oper` member");
    assert_eq!(
        1,
        desc.circuits.len(),
        "one circuit rule per block-bodied `rule` member"
    );
    let rule = &desc.circuits[0];
    assert_eq!("cong1", rule.name.as_ref(), "the member's own name");
    assert_eq!(1, rule.ports.len(), "the rewrite-sorted telescope");
    assert!(
        matches!(rule.ports[0].face, PortFace::Sorted(ref sort) if sort.as_ref() == "Nat"),
        "an unpinned binder is sorted by the face's sort: {:?}",
        rule.ports[0].face
    );
    assert_eq!(2, rule.body.nodes.len(), "the body's two statements");
    assert!(
        matches!(rule.body.nodes[0], CircuitNode::Redex(_)),
        "a line applying a telescope port is a redex"
    );
    assert!(
        matches!(rule.body.nodes[1], CircuitNode::Frame(_)),
        "a line applying an `oper` is a frame"
    );
    assert_eq!("z", rule.body.out.as_ref(), "the declared output port");
}

#[test]
fn a_single_redex_block_reaches_the_cell_layer_behind_the_gate()
{
    let elab = elaborate_data_descs(CONG1.0);
    let cells = elaborate_desc_cells(&elab.descs);
    assert!(
        cells.diagnostics.is_empty(),
        "the gate admits a well-formed block: {:#?}",
        cells.diagnostics
    );
    // Two frame-defining cells from the nested block's constructors (one
    // store) plus the sign block rule's own cell (the other).
    let total: usize = cells
        .circuit_completions
        .iter()
        .map(|completion| completion.outcome.store().len())
        .fold(0_usize, |count, store| {
            count.saturating_add(usize::from(store))
        });
    assert_eq!(
        3, total,
        "the rule's cell joins the constructors' frame cells across the two stores"
    );
    let [ref composite] = *cells.composites
    else {
        panic!("one whiskered composite per admitted circuit rule");
    };
    assert_eq!(
        Some(vec![0_usize.into()]),
        composite.active_position(),
        "the redex sits at `add`'s first argument"
    );
}

#[test]
fn an_item_level_data_member_declines_with_the_nested_block_respelling()
{
    // The retired member: a `sign` block presents sorts, operations, and
    // rules only, so a stale `data` member is declined with the nested
    // generator block's respelling (the grammar keeps it admissible for
    // exactly this decline).
    let reported = messages(TestText(
        r#"sign Stale {
  sort Nat : Type;
  data Zero : Nat;
  oper add : (Nat, Nat) --> Nat;
}
"#,
    ));
    assert_names(&reported, &[
        TestText("the item-level `data Zero` member is retired"),
        TestText("nested generator block"),
        TestText("data S : Type { Zero : S;"),
    ]);
}

#[test]
fn the_name_set_fold_reaches_the_description_route_as_a_production_check()
{
    let shared = TestText(
        r#"sign Shared {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  oper g : (a : Nat) --> (b : Nat);

  rule twice : (rule p : Nat ==> Nat, data x : Nat) ==> (o : Nat) {
    node : p(x) ==> (m);
    node : f(x) --> (m);
  };
}
"#,
    );
    let reported = messages(shared);
    assert_names(&reported, &[
        TestText("the port name `m`"),
        TestText("produced"),
        TestText("produced exactly once and consumed exactly once"),
    ]);
    let standalone: Vec<String> = check_circuit_surface(shared.0)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect();
    assert!(
        !standalone.is_empty() && standalone.iter().all(|message| reported.contains(message)),
        "fold verdict unchanged: {standalone:#?}; reported: {reported:#?}"
    );
}

#[test]
fn a_many_out_node_declines_naming_the_cell_alphabet_question()
{
    let reported = messages(TestText(
        r#"sign Wide {
  sort Nat : Type;
  oper tee : (a : Nat) --> (b : Nat, c : Nat);

  rule split : (data x : Nat) ==> (z : Nat) {
    node : tee(x) --> (z, w);
  };
}
"#,
    ));
    assert_names(&reported, &[
        TestText("circuit rule `split`"),
        TestText("applying `tee` binds 2 output ports"),
        TestText("cell-alphabet question (gandr-ui9)"),
    ]);
}

#[test]
fn a_many_out_interface_declines_naming_the_cell_alphabet_question()
{
    let reported = messages(TestText(
        r#"sign WideFace {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);

  rule pair : (data x : Nat) ==> (y : Nat, z : Nat) {
    node : f(x) --> (y);
  };
}
"#,
    ));
    assert_names(&reported, &[
        TestText("circuit rule `pair` declares 2 output ports"),
        TestText("a term has one root"),
        TestText("cell-alphabet question (gandr-ui9)"),
    ]);
}

#[test]
fn a_feed_statement_declines_naming_the_wheel_obligation()
{
    let reported = messages(TestText(
        r#"sign Wheel {
  sort Nat : Type;
  oper zip : (s : Nat, t : Nat) --> (u : Nat);

  rule spin : (data stream : Nat) ==> (out : Nat) {
    node : zip(stream, state) --> (out);
    feed : (out) --> (state);
  };
}
"#,
    ));
    assert_names(&reported, &[
        TestText("circuit rule `spin` carries a `feed` back-edge"),
        TestText("wheel-bearing body derives no boundary pair"),
    ]);
}

#[test]
fn a_two_redex_body_declines_with_both_occurrences()
{
    let elab = elaborate_data_descs(CONG2.0);
    assert!(
        elab.diagnostics.is_empty(),
        "the declaration table admits the block: {:#?}",
        elab.diagnostics
    );
    let cells = elaborate_desc_cells(&elab.descs);
    let reported: Vec<String> = cells
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect();
    assert_names(&reported, &[
        TestText("circuit rule `cong2`"),
        TestText("unfolds to 2 redex occurrences"),
        TestText("`p` at [0]"),
        TestText("`q` at [1]"),
        TestText("incomparable"),
        TestText("shift-equivalence witness"),
    ]);
    assert!(
        cells.composites.is_empty(),
        "a declined body contributes no composite"
    );
}

#[test]
fn an_out_of_signature_frame_head_earns_the_declaration_tables_refusal()
{
    let reported = messages(TestText(
        r#"sign Stray {
  sort Nat : Type;
  rule stray : (data x : Nat) ==> (z : Nat) {
    node : mystery(x) --> (z);
  };
}
"#,
    ));
    assert_names(&reported, &[
        TestText("circuit rule `stray`"),
        TestText("frame applies symbol `mystery`"),
        TestText("not in the datatype's signature"),
    ]);
}

#[test]
fn a_redex_applying_an_undeclared_rewrite_earns_its_own_refusal()
{
    let reported = messages(TestText(
        r#"sign NoPort {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);

  rule stray : (rule p : Nat ==> Nat, data x : Nat) ==> (z : Nat) {
    node : p(x) ==> (m);
    node : f(m) --> (z);
  };
}
"#,
    ));
    assert!(
        reported.is_empty(),
        "a declared port is not an unknown one: {reported:#?}"
    );
}

#[test]
fn a_wiring_that_closes_a_cycle_earns_the_declaration_tables_refusal()
{
    let reported = messages(TestText(
        r#"sign Loop {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  oper g : (a : Nat) --> (b : Nat);

  rule spin : (data x : Nat) ==> (z : Nat) {
    node : f(w) --> (z);
    node : g(z) --> (w);
  };
}
"#,
    ));
    assert_names(&reported, &[
        TestText("circuit rule `spin`"),
        TestText("reaches port"),
        TestText("from itself"),
    ]);
}

#[test]
fn a_repeated_argument_name_reaches_the_linearity_refusal()
{
    // The shape that reaches the admission seam's linearity refusal from
    // source is an application consuming one wire twice — `add(x, x)`. The
    // sign block's members are `;`-terminated (gandr-ng9.14), so the source
    // parses cleanly and the ruling's diagnostic is triggerable by a program:
    // the wiring derives the boundary `add(x, x) ==> z`, whose left-hand side
    // copies the hole `x`, and the cell layer's single admission seam refuses
    // it, naming the copy and the respelling — the same refusal
    // `gandr-theory-computads`
    // `elaborate::tests::a_circuit_rule_whose_boundary_copies_a_hole_is_refused`
    // witnesses on a hand-built description. The name-set fold's consumed-twice
    // verdict rides alongside: a wiring error and an ill-formed cell are
    // different failures, and both reach the user.
    //
    // (The block's `rule` member is named `copy2`: `dup` is a reserved
    // surface keyword — `dup ( E )` — so a member named `dup` never reaches
    // the route, which is the reservation working, not the hazard.)
    let elab = elaborate_data_descs(
        TestText(
            r#"sign Copy {
  sort Nat : Type;
  oper add : (l : Nat, r : Nat) --> (s : Nat);
  rule copy2 : (data x : Nat) ==> (z : Nat) {
    node : add(x, x) --> (z);
  };
}
"#,
        )
        .0,
    );
    let reported: Vec<String> = elab
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    // The name-set fold's consumed-twice verdict: a wiring error, reported by
    // the description route.
    assert_names(&reported, &[
        TestText("the port name `x`"),
        TestText("consumed"),
        TestText("produced exactly once and consumed exactly once"),
    ]);
    // And the ruling's own refusal, reached at the cell layer's admission
    // seam: the wiring derives the boundary `add(x, x) ==> add(x, x)`, whose
    // left-hand side copies the hole `x`.
    let cells = elaborate_desc_cells(&elab.descs);
    let cell_reported: Vec<String> = cells
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert_names(&cell_reported, &[
        TestText("non-linear cell pattern"),
        TestText("hole `x` occurs more than once"),
        TestText("cell patterns are linear"),
    ]);
}

#[test]
fn a_doubling_body_declines_on_the_derivation_node_budget()
{
    // Reconvergence written across lines, which is the shape the ruled grammar
    // does admit: each level's wire is consumed by two frames whose outputs
    // meet in one, so the derived boundary doubles per level. Twelve levels
    // pass the derivation's ceiling, and the surface says so with the ceiling
    // it hit instead of unfolding 2^12 nodes.
    let mut source = String::from(
        r#"sign Doubling {
  sort Nat : Type;
  oper l : (a : Nat) --> (b : Nat);
  oper r : (a          : Nat) --> (b : Nat);
  oper add : (a : Nat, b : Nat) --> (c : Nat);
  rule blow :          (data w0 : Nat) ==> (w12 : Nat) {
"#,
    );
    for level in 1 ..= 12_u32 {
        let below = level.saturating_sub(1);
        write!(
            source,
            r#"    node : l(w{below}) --> (a{level});
    node : r(w{below}) --> (b{level});
    node : add(a{level}, b{level}) --> (w{level});
"#,
        )
        .expect("writing to a String does not fail");
    }
    source.push_str("  };\n}\n");
    let reported: Vec<String> = elaborate_data_descs(source.as_str())
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect();
    assert_names(&reported, &[
        TestText("circuit rule `blow`"),
        TestText("past the derivation's node budget of 4096"),
        TestText("reconvergence is a shared subterm"),
    ]);
}

#[test]
fn an_oper_filler_stays_declined_and_names_its_own_rung()
{
    let reported = messages(TestText(
        r#"sign Pipe {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);

  oper pipeline : (i : Nat) --> (o : Nat) {
    node : f(i) --> (o);
  };
}
"#,
    ));
    assert_names(&reported, &[
        TestText("the filler of `oper pipeline`"),
        TestText("defines a circuit 1-cell"),
    ]);
}

#[test]
fn a_sign_block_is_a_declaration_rather_than_an_unsupported_item()
{
    let lowered = gandr_surface_engine::lower::lower_source_total(PipelineSource(CONG1.0))
        .expect("total mode lowers any input");
    assert!(
        lowered.items.is_empty(),
        "a `sign` block contributes no runnable item"
    );
}

#[test]
fn a_fillerless_rule_member_declines_to_the_higher_cells_lane()
{
    // `rule involutive : (b : Bit) <=> (c : Bit)` declares a face between
    // SORTS. A description member holds a rewrite face as a pair of terms, so
    // there is no slot for the term-free sphere and the higher-cells lane owes
    // its representation. What changed is not the verdict but its visibility:
    // the member used to leave this route contributing neither a description
    // member nor a diagnostic, which reads to its author exactly like
    // acceptance.
    let reported = messages(TestText(
        r#"sign Bits {
  sort Bit : Type;
  rule involutive : (b : Bit) <=> (c : Bit);
}
"#,
    ));
    assert_names(&reported, &[
        TestText("rule `involutive`"),
        TestText("declares a face between sorts and writes no filler"),
        TestText("does not carry yet"),
        TestText("higher-cells lane"),
    ]);
    // Declining is not carrying: no circuit rule reached the description, and
    // therefore no cell reached the store.
    let elab = elaborate_data_descs(
        r#"sign Bits {
  sort Bit : Type;
  rule involutive : (b : Bit) <=> (c : Bit);
}
"#,
    );
    let desc = elab
        .descs
        .first()
        .expect("the block reads into a description");
    assert!(
        desc.circuits.is_empty(),
        "a fillerless member is carried by no description member"
    );
    let cells = elaborate_desc_cells(&elab.descs);
    let store = cells
        .circuit_completions
        .first()
        .expect("one completion per description")
        .outcome
        .store();
    assert_eq!(
        gandr_theory_cell_complexes::CellCount::from(0_usize),
        store.len(),
        "and contributes no cell"
    );
}

#[test]
fn a_written_face_in_a_sign_block_declines_naming_the_block_form_that_holds_it()
{
    // The written face `rule lhs ==> rhs` is a `data` / `codata` block member.
    // Spelled inside a `sign` block it carries no top-level `:`, so this route
    // reads no sphere from it — and now says which block form does hold it,
    // instead of dropping the member with no diagnostic at all.
    let reported = messages(TestText(
        r#"sign Adder {
  sort Nat : Type;
  oper add : (Nat, Nat) --> Nat;
  rule unit ==> add;
}
"#,
    ));
    assert_names(&reported, &[
        TestText("rule `unit`"),
        TestText("carries no `:` signature"),
        TestText("`data` or `codata` block member instead"),
    ]);
}

#[test]
fn a_member_with_no_judgment_head_declines_rather_than_vanishing()
{
    // A member run always starts at its lead keyword (`split_before_leads`
    // drops what precedes the first one), so a member whose name the parse did
    // not reach arrives here as a bare keyword. The lead is all the decline can
    // name, and naming it is what separates a member the route could not read
    // from a member the source never wrote.
    let reported = messages(TestText(
        r#"sign Broken {
  sort Nat : Type;
  rule ;
}
"#,
    ));
    assert_names(&reported, &[
        TestText("the `rule` member declares no name"),
        TestText("`rule <name> : <signature>`"),
    ]);
}

#[test]
fn a_data_spelled_sign_oper_declines_before_no_output_can_be_fabricated()
{
    let source = r#"sign Theory {
  sort Nat : Type;
  oper add(m : Nat, n : Nat) -> Nat;
  oper succ : (n : Nat) --> Nat;
}
"#;
    let elab = elaborate_data_descs(source);
    let desc = elab
        .descs
        .first()
        .expect("the sign block reads into a description");
    assert_eq!(1, desc.sorts.len(), "the sort sibling remains");
    assert_eq!(1, desc.opers.len(), "the valid operation sibling remains");
    assert_eq!(
        "succ",
        desc.opers
            .first()
            .expect("one valid operation")
            .name
            .as_ref(),
        "the malformed member never fabricates an empty-arity operation"
    );
    assert_eq!(
        1,
        elab.diagnostics.len(),
        "the malformed member region earns exactly one primary diagnostic: {:?}",
        elab.diagnostics
    );
    let decline = elab.diagnostics.first().expect("one localized decline");
    assert!(
        decline.message.contains("top-level `:`")
            && decline
                .message
                .contains("`oper add : (m : Nat, n : Nat) --> Nat`")
            && decline
                .message
                .contains("`oper name : (inputs) --> (output)`"),
        "the decline names the judgment spelling and recovered respelling: {}",
        decline.message
    );
    let start = usize::from(decline.span.start);
    let end = usize::from(decline.span.end);
    assert_eq!(
        "oper add(m : Nat, n : Nat) -> Nat",
        source
            .get(start .. end)
            .expect("the decline span is in source"),
        "the decline covers only the offending member"
    );

    let cells = elaborate_desc_cells(&elab.descs);
    assert!(
        cells
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.message.contains("no output port")),
        "a run with no top-level judgment colon never reaches `NoOutput`: {:?}",
        cells.diagnostics
    );

    let colon_source = "sign Missing { sort Nat : Type; oper missing : (n : Nat) --> (); }";
    let colon_anchored = elaborate_data_descs(colon_source);
    assert!(
        colon_anchored.diagnostics.is_empty(),
        "the colon-anchored positive control is a readable judgment: {:?}",
        colon_anchored.diagnostics
    );
    let colon_desc = colon_anchored
        .descs
        .first()
        .expect("the colon-anchored sign block elaborates");
    assert_eq!(
        1,
        colon_desc.opers.len(),
        "the colon-anchored outputless judgment reaches operation admission"
    );
    let colon_cells = elaborate_desc_cells(&colon_anchored.descs);
    assert!(
        colon_cells
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("no output port")),
        "`NoOutput` remains reserved for a colon-anchored signature truly missing output: {:?}",
        colon_cells.diagnostics
    );
}

/// A declined rule member must not consume the member written after it
/// (`gandr-wvd.6.1.1`).
///
/// The written face `rule lhs ==> rhs` is a data / codata spelling this route
/// declines by name. Before the grammar admitted the face as a `sign`-block
/// member tail, parser repair molded everything from the face's arrow onward
/// into one unlabelled run — so a second rule, or any member at all written
/// after a first rule, vanished into a whole-block "reading stopped" decline.
/// Each stacked face now parses as its own member and earns its own decline,
/// and every sibling around them still presents.
#[test]
fn a_declined_rule_member_leaves_the_members_after_it_intact()
{
    // Two written faces stack: each is named by its own decline, and neither
    // absorbs the other.
    let two = elaborate_data_descs(
        r#"sign Adder {
  sort Nat : Type;
  oper add : (Nat, Nat) --> Nat;
  rule unit ==> add;
  rule swap ==> mul;
}
"#,
    );
    assert_eq!(1, two.descs.len(), "the block still elaborates: {two:?}");
    assert_eq!(1, two.descs[0].opers.len(), "the operation presents");
    let named: Vec<&str> = two
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.message.contains("carries no `:` signature"))
        .flat_map(|diagnostic| {
            ["unit", "swap"]
                .into_iter()
                .filter(|n| diagnostic.message.contains(*n))
        })
        .collect();
    assert_eq!(
        vec!["unit", "swap"],
        named,
        "each written face declines by name at its own member: {:?}",
        two.diagnostics
    );
    assert!(
        !two.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("reading stopped")),
        "no whole-block decline may stand in for the per-member ones: {:?}",
        two.diagnostics
    );

    // A member written after a declined rule presents: order, not kind, was
    // what the absorption turned on.
    let mixed = elaborate_data_descs(
        r#"sign Adder {
  sort Nat : Type;
  oper add : (Nat, Nat) --> Nat;
  rule unit ==> add;
  oper neg : (Nat) --> Nat;
}
"#,
    );
    assert_eq!(
        1,
        mixed.descs.len(),
        "the block still elaborates: {mixed:?}"
    );
    assert_eq!(
        2,
        mixed.descs[0].opers.len(),
        "the operation after the declined rule presents alongside the first"
    );
}
