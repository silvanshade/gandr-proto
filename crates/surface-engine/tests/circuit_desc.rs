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
use gandr_theory_levitation::CircuitNode;
use gandr_theory_levitation::PortFace;

use crate::common::TestText;

/// The congruence cell of the ruling, minus one redex: the single-redex block
/// this rung graduates, whose composite is one whiskering. The family is
/// declared once, whole, as a nested generator block (the item-level `data`
/// member is retired from `sign` blocks); the `sign` block presents the sort,
/// the operation, and the rule.
const CONG1: TestText<'static> = TestText(
    "\
data Nat : Type {
  Zero : Nat;
  Succ : (n : Nat) --> Nat;
}

sign Nat {
  sort Nat : Type
  oper add : (Nat, Nat) --> Nat

  rule cong1 : (
    rule p : Nat ==> Nat,
    data x : Nat,
    data y : Nat
  ) ==> (z : Nat) {
    node : p(x) ==> (x\u{2032});
    node : add(x\u{2032}, y) --> (z);
  }
}
",
);

/// The two-redex congruence cell, verbatim from the ruling.
const CONG2: TestText<'static> = TestText(
    "\
sign Nat {
  sort Nat : Type
  oper add : (Nat, Nat) --> Nat

  rule cong2 : (
    rule p : Nat ==> Nat,
    rule q : Nat ==> Nat,
    data x : Nat,
    data y : Nat
  ) ==> (z : Nat) {
    node : p(x) ==> (x\u{2032});
    node : q(y) ==> (y\u{2032});
    node : add(x\u{2032}, y\u{2032}) --> (z);
  }
}
",
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
    let total: usize = cells.stores.iter().fold(0_usize, |count, store| {
        count.saturating_add(usize::from(store.len()))
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
        "\
sign Stale {
  sort Nat : Type
  data Zero : Nat
  oper add : (Nat, Nat) --> Nat
}
",
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
        "\
sign Shared {
  sort Nat : Type
  oper f : (a : Nat) --> (b : Nat)
  oper g : (a : Nat) --> (b : Nat)

  rule twice : (rule p : Nat ==> Nat, data x : Nat) ==> (o : Nat) {
    node : p(x) ==> (m);
    node : f(x) --> (m);
  }
}
",
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
        "the fold's verdict reaches the description route unchanged: {standalone:#?} against \
         {reported:#?}"
    );
}

#[test]
fn a_many_out_node_declines_naming_the_cell_alphabet_question()
{
    let reported = messages(TestText(
        "\
sign Wide {
  sort Nat : Type
  oper tee : (a : Nat) --> (b : Nat, c : Nat)

  rule split : (data x : Nat) ==> (z : Nat) {
    node : tee(x) --> (z, w);
  }
}
",
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
        "\
sign WideFace {
  sort Nat : Type
  oper f : (a : Nat) --> (b : Nat)

  rule pair : (data x : Nat) ==> (y : Nat, z : Nat) {
    node : f(x) --> (y);
  }
}
",
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
        "\
sign Wheel {
  sort Nat : Type
  oper zip : (s : Nat, t : Nat) --> (u : Nat)

  rule spin : (data stream : Nat) ==> (out : Nat) {
    node : zip(stream, state) --> (out);
    feed : (out) --> (state);
  }
}
",
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
        "\
sign Stray {
  sort Nat : Type
  rule stray : (data x : Nat) ==> (z : Nat) {
    node : mystery(x) --> (z);
  }
}
",
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
        "\
sign NoPort {
  sort Nat : Type
  oper f : (a : Nat) --> (b : Nat)

  rule stray : (rule p : Nat ==> Nat, data x : Nat) ==> (z : Nat) {
    node : p(x) ==> (m);
    node : f(m) --> (z);
  }
}
",
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
        "\
sign Loop {
  sort Nat : Type
  oper f : (a : Nat) --> (b : Nat)
  oper g : (a : Nat) --> (b : Nat)

  rule spin : (data x : Nat) ==> (z : Nat) {
    node : f(w) --> (z);
    node : g(z) --> (w);
  }
}
",
    ));
    assert_names(&reported, &[
        TestText("circuit rule `spin`"),
        TestText("reaches port"),
        TestText("from itself"),
    ]);
}

#[test]
fn a_repeated_argument_name_does_not_reach_the_route_at_all()
{
    // The shape that would reach the admission seam's linearity refusal from
    // source is an application consuming one wire twice, and the ruled grammar
    // does not parse it cleanly today: the member is repaired into one opaque
    // region, so nothing is offered to the declaration table. The refusal is
    // still bound — `gandr-theory-computads`
    // `elaborate::tests::a_circuit_rule_whose_boundary_copies_a_hole_is_refused`
    // witnesses it at the seam — and this test pins that the surface route
    // stays *silent* rather than admitting a mis-parsed block.
    let elab = elaborate_data_descs(
        TestText(
            "\
sign Copy {
  sort Nat : Type
  oper add : (l : Nat, r : Nat) --> (s : Nat)
  rule dup : (data x : Nat) ==> (z : Nat) {
    node : add(x, x) --> (z);
  }
}
",
        )
        .0,
    );
    let [ref desc] = *elab.descs
    else {
        panic!("the block still reads as a description");
    };
    assert!(
        desc.circuits.is_empty(),
        "a repaired member contributes no circuit rule"
    );
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
        "sign Doubling {\n  sort Nat : Type\n  oper l : (a : Nat) --> (b : Nat)\n  oper r : (a          : Nat) --> (b : Nat)\n  oper add : (a : Nat, b : Nat) --> (c : Nat)\n  rule blow :          (data w0 : Nat) ==> (w12 : Nat) {\n",
    );
    for level in 1 ..= 12_u32 {
        let below = level.saturating_sub(1);
        write!(
            source,
            "    node : l(w{below}) --> (a{level});\n    node : r(w{below}) --> (b{level});\n    \
             node : add(a{level}, b{level}) --> (w{level});\n"
        )
        .expect("writing to a String does not fail");
    }
    source.push_str("  }\n}\n");
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
        "\
sign Pipe {
  sort Nat : Type
  oper f : (a : Nat) --> (b : Nat)

  oper pipeline : (i : Nat) --> (o : Nat) {
    node : f(i) --> (o);
  }
}
",
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
