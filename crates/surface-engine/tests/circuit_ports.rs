//! The circuit port discipline: the name-set fold, the internal wires it binds,
//! and its single failure mode.
//!
//! The ruling puts the whole discipline on names — "each wire name is produced
//! exactly once and consumed exactly once; internal wires are implicit, the
//! head declares the interface and the body names not in it are internal,
//! computed and checked by the name-set fold". Two things are therefore under
//! test: what the fold *binds* when a body is well-formed, and that the only
//! thing it *refuses* is a port name two distinct ports arrived under.

use gandr_surface_engine::circuit::CircuitSurface;
use gandr_surface_engine::circuit::InternalWire;
use gandr_surface_engine::circuit::WireEnd;
use gandr_surface_engine::circuit::check_circuit_surface;

use crate::common::TestText;

/// The congruence cell, in the named-port normal form the fold reads: two
/// disjoint redexes whose outputs meet in one frame.
const CONG2: TestText<'static> = TestText(
    "\
sign Cong {
  sort Nat : Type;
  oper add : (l : Nat, r : Nat) --> (sum : Nat);

  rule cong2 : (
    rule p : Nat ==> Nat,
    rule q : Nat ==> Nat,
    data x : Nat,
    data y : Nat
  ) ==> (z : Nat) {
    node : p(x) ==> (x\u{2032});
    node : q(y) ==> (y\u{2032});
    node : add(x\u{2032}, y\u{2032}) --> (z);
  };
}
",
);

/// Every message the check produced, in span order.
fn messages(source: TestText<'_>) -> Vec<String>
{
    check_circuit_surface(source.0)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

/// The internal wires the fold bound.
fn wires(source: TestText<'_>) -> Vec<InternalWire>
{
    check_circuit_surface(source.0).internal_wires
}

/// One committed surface-tree witness, read from the corpus.
fn witness(name: TestText<'_>) -> String
{
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../surface-corpus/examples/surface")
        .join(name.0);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("the committed witness `{}` is readable: {error}", name.0))
}

/// Assert that exactly one diagnostic fired, and that it names every fragment.
fn assert_sole_diagnostic(
    source: TestText<'_>,
    fragments: &[TestText<'_>],
)
{
    let found = messages(source);
    assert_eq!(
        1,
        found.len(),
        "{:?} earns exactly one diagnostic; got {found:?}",
        source.0
    );
    let Some(message) = found.first()
    else {
        panic!("a diagnostic was counted but not readable");
    };
    for fragment in fragments {
        assert!(
            message.contains(fragment.0),
            "the diagnostic must name {:?}; got {message:?}",
            fragment.0
        );
    }
}

/// The wire names the fold bound, in binding order.
fn wire_names(source: TestText<'_>) -> Vec<String>
{
    wires(source)
        .into_iter()
        .map(|wire| wire.name)
        .collect::<Vec<_>>()
}

#[test]
fn a_well_formed_block_binds_its_internal_wires()
{
    // The Done-when clause, exactly: `x′` flows from a redex output to a frame
    // input. Neither `x′` nor `y′` is written in any port list — the head
    // declares `x`, `y`, and `z`, and the fold computes the rest — so this
    // record IS the binding the ruled form calls implicit.
    let bound = wires(CONG2);
    assert_eq!(
        Vec::<String>::new(),
        messages(CONG2),
        "the congruence cell is well-formed and the fold refuses nothing"
    );
    assert_eq!(
        vec!["x\u{2032}".to_owned(), "y\u{2032}".to_owned()],
        bound
            .iter()
            .map(|wire| wire.name.clone())
            .collect::<Vec<_>>(),
        "exactly the two body names outside the interface are internal"
    );
    let Some(first) = bound.first()
    else {
        panic!("the first internal wire was counted but not readable");
    };
    assert_eq!("cong2", first.declaration, "the wire names its declaration");
    assert_eq!(
        WireEnd::Redex {
            head: "p".to_owned(),
            label: None,
        },
        first.produced_by,
        "`x\u{2032}` is produced by the redex line applying `p`"
    );
    assert_eq!(
        WireEnd::Frame {
            head: "add".to_owned(),
            label: None,
        },
        first.consumed_by,
        "`x\u{2032}` is consumed by the frame line applying `add`"
    );
}

#[test]
fn an_occurrence_label_names_the_face_a_wire_runs_between()
{
    // The label slot the ruling keeps open "for diagnostics" is what makes a
    // wire's two ends addressable when the same head is applied more than once.
    let bound = wires(TestText(
        "sign S {
  sort Nat : Type;
  oper step : (i : Nat) --> (o : Nat);
  rule twice : (rule p : x ==> x\u{2032}, data x : Nat) ==> (o : Nat) {
    node w1 : p(x) ==> (m);
    node w2 : step(m) --> (o);
  };
}",
    ));
    assert_eq!(
        vec![InternalWire {
            declaration: "twice".to_owned(),
            name: "m".to_owned(),
            produced_by: WireEnd::Redex {
                head: "p".to_owned(),
                label: Some("w1".to_owned()),
            },
            consumed_by: WireEnd::Frame {
                head: "step".to_owned(),
                label: Some("w2".to_owned()),
            },
        }],
        bound
    );
}

#[test]
fn a_feed_line_closes_the_wheel_over_two_internal_wires()
{
    // `feed` is the only cycle-forming statement, and both wires it closes over
    // are internal: neither `next` nor `state` appears in the interface. An
    // unresolved head is classified as such rather than guessed at.
    let bound = wires(TestText(
        "oper accumulate : (stream : Stream) --> (out : Stream) {
  node : zip(stream, state) --> (next, out);
  feed : (next) --> (state);
}",
    ));
    assert_eq!(
        vec![
            InternalWire {
                declaration: "accumulate".to_owned(),
                name: "next".to_owned(),
                produced_by: WireEnd::Unresolved {
                    head: "zip".to_owned(),
                    label: None,
                },
                consumed_by: WireEnd::Feed { label: None },
            },
            InternalWire {
                declaration: "accumulate".to_owned(),
                name: "state".to_owned(),
                produced_by: WireEnd::Feed { label: None },
                consumed_by: WireEnd::Unresolved {
                    head: "zip".to_owned(),
                    label: None,
                },
            },
        ],
        bound
    );
}

#[test]
fn a_shared_port_name_is_refused_naming_the_name()
{
    // Monogamy admits no implicit duplication: feeding one name into both inputs
    // of a frame shares the port name, and the diagnostic says which name.
    assert_sole_diagnostic(
        TestText(
            "sign S {
  sort Nat : Type;
  oper add : (l : Nat, r : Nat) --> (sum : Nat);
  rule doubled : (data x : Nat) ==> (z : Nat) {
    node : add(x, x) --> (z);
  };
}",
        ),
        &[
            TestText("the port name `x`"),
            TestText("consumed twice"),
            TestText("the frame line applying `add`"),
            TestText("produced exactly once and consumed exactly once"),
        ],
    );
    // And the other polarity: two frame lines cannot both produce `m`, because
    // the wire has one producing endpoint and the first line took it.
    assert_sole_diagnostic(
        TestText(
            "sign S {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  oper g : (a : Nat) --> (b : Nat);
  oper collide : (i : Nat, j : Nat) --> (o : Nat) {
    node : f(i) --> (m);
    node : g(j) --> (m);
  };
}",
        ),
        &[
            TestText("the port name `m`"),
            TestText("is produced by the frame line applying `f`"),
            TestText("again by the frame line applying `g`"),
        ],
    );
}

#[test]
fn two_redexes_sharing_a_port_name_are_not_disjoint()
{
    // The horizontal-composition fence, read at the parser: two redexes in one
    // body are disjoint exactly when they share no port name. The sharing is
    // refused by the same merge as any other, and the diagnostic states the
    // fence because both sites are redex lines.
    assert_sole_diagnostic(
        TestText(
            "sign S {
  sort Nat : Type;
  oper add : (l : Nat, r : Nat) --> (sum : Nat);
  rule overlap : (
    rule p : Nat ==> Nat,
    rule q : Nat ==> Nat,
    data x : Nat
  ) ==> (z : Nat) {
    node : p(x) ==> (m);
    node : q(x) ==> (n);
    node : add(m, n) --> (z);
  };
}",
        ),
        &[
            TestText("the port name `x`"),
            TestText("the redex line applying `p`"),
            TestText("the redex line applying `q`"),
            TestText("two redexes in one body are disjoint exactly when they share no port name"),
        ],
    );
    // The fence clause is the redex reading and only that: the same collision
    // between two frame lines is refused without it.
    let frames = messages(TestText(
        "sign S {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  oper g : (a : Nat) --> (b : Nat);
  oper both : (i : Nat) --> (o : Nat) {
    node : f(i) --> (m);
    node : g(i) --> (n);
  };
}",
    ));
    let Some(message) = frames.first()
    else {
        panic!("two frame lines sharing `i` earn a diagnostic; got {frames:?}");
    };
    assert!(
        message.contains("the port name `i`") && !message.contains("two redexes"),
        "a frame/frame sharing is refused without the redex fence; got {message:?}"
    );
}

#[test]
fn a_ports_uses_must_agree_in_sort()
{
    // The attachment discipline's second side condition. One spelling declared
    // at two sorts is two ports, not one wire, so it is the same failure as any
    // other sharing — and the diagnostic names the name and both sorts.
    assert_sole_diagnostic(
        TestText(
            "sign S { sort Nat : Type; sort Bit : Type; oper narrow : (v : Nat) --> (v : Bit); }",
        ),
        &[
            TestText("the port name `v`"),
            TestText("declared at `Nat` by the head's input list"),
            TestText("at `Bit` by the head's output list"),
            TestText("a port's uses must agree in sort"),
        ],
    );
    // Agreement is on the sort's spelling with whitespace elided, so the same
    // written type reached through different layout still agrees: a pass-through
    // wire is well-formed and is an interface port, never an internal one.
    let through = TestText("sign S { sort Nat : Type; oper id : (v : Nat) --> (v : Nat ); }");
    assert_eq!(Vec::<String>::new(), messages(through));
    assert_eq!(Vec::<String>::new(), wire_names(through));
}

#[test]
fn the_folds_only_failure_mode_is_non_disjointness()
{
    // The bead's third clause, and the reason the fold is shaped as a merge: no
    // other rejection path exists in it. Everything below is a body the full
    // discipline eventually has something to say about, and the fold says
    // nothing — because none of it is two ports under one spelling.
    for (reason, source) in [
        // A dangling producer and a dangling consumer — the misspelling case.
        // Half of "exactly once" is disjointness and half is totality; only the
        // first half is the fold's.
        (
            "a misspelled wire leaves two unpaired names",
            "sign S {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  oper g : (a : Nat) --> (b : Nat);
  oper typo : (i : Nat) --> (o : Nat) {
    node : f(i) --> (m);
    node : g(mm) --> (o);
  };
}",
        ),
        // An interface input nothing consumes and an interface output nothing
        // produces.
        (
            "an interface port the body never wires",
            "sign S {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  oper stranded : (i : Nat) --> (o : Nat) {
    node : f(u) --> (w);
  };
}",
        ),
        // A delay-free cycle written with `node` lines only. The ruling records
        // it as a checking obligation whose diagnostic is a spelling correction
        // — close loops with `feed` — and assigns it to the back-edge rung, not
        // to this fold.
        (
            "a node-only cycle that owes a feed",
            "sign S {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  oper g : (a : Nat) --> (b : Nat);
  oper wheel : () --> () {
    node : f(a) --> (b);
    node : g(b) --> (a);
  };
}",
        ),
        // A head no declaration in scope names.
        (
            "an unresolved head",
            "oper acc : (s : Stream) --> (o : Stream) {
  node : zip(s) --> (o);
}",
        ),
    ] {
        assert_eq!(
            Vec::<String>::new(),
            messages(TestText(source)),
            "the fold refuses only non-disjointness, and {reason} is not that"
        );
    }
    // The node-only cycle still *binds*: the fold classifies where it does not
    // refuse, so the wires are there for the sweep that owes the diagnostic.
    // Wires come in the order their producers are written, which is why `b` —
    // produced by the first line and consumed by the second — leads.
    assert_eq!(
        vec!["b".to_owned(), "a".to_owned()],
        wire_names(TestText(
            "sign S {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  oper g : (a : Nat) --> (b : Nat);
  oper wheel : () --> () {
    node : f(a) --> (b);
    node : g(b) --> (a);
  };
}"
        ))
    );
    // And an unpaired name is classified out rather than bound: `m` and `mm`
    // each have one endpoint, so neither is an internal wire.
    assert_eq!(
        Vec::<String>::new(),
        wire_names(TestText(
            "sign S {
  sort Nat : Type;
  oper f : (a : Nat) --> (b : Nat);
  oper g : (a : Nat) --> (b : Nat);
  oper typo : (i : Nat) --> (o : Nat) {
    node : f(i) --> (m);
    node : g(mm) --> (o);
  };
}"
        ))
    );
}

#[test]
fn an_unnamed_port_mints_a_fresh_wire_and_collides_with_nothing()
{
    // The sugar ladder's fresh-name rung: "unnamed tuple inputs mint fresh names
    // in order". A fold that keyed on the spelling `_` would find four shared
    // ports here; a freshly minted name shares nothing, so none of the four is a
    // use at all.
    let fresh = TestText(
        "oper split : (b : Bit) --> (_ : Bit, _ : Bit) {
  node : tee(b) --> (_, _);
}",
    );
    assert_eq!(Vec::<String>::new(), messages(fresh));
    assert_eq!(Vec::<String>::new(), wire_names(fresh));
    // The bare-sort rungs of the same ladder are unnamed the other way, and are
    // likewise no use of any name.
    assert_eq!(
        Vec::<String>::new(),
        messages(TestText(
            "sign S { sort Nat : Type; oper add : (Nat, Nat) --> Nat; }"
        ))
    );
}

#[test]
fn a_boundary_without_a_filler_wires_nothing_but_still_has_ports()
{
    // A declaration that fills no boundary has no body to wire, so it binds no
    // internal wire — while its interface is still a port list, and two entries
    // of it sharing a spelling is the same failure it would be anywhere else.
    let boundary =
        TestText("sign S { sort Nat : Type; oper add : (l : Nat, r : Nat) --> (s : Nat); }");
    assert_eq!(Vec::<String>::new(), messages(boundary));
    assert_eq!(Vec::<String>::new(), wire_names(boundary));
    assert_sole_diagnostic(
        TestText("sign S { sort Nat : Type; oper add : (l : Nat, l : Nat) --> (s : Nat); }"),
        &[
            TestText("the port name `l`"),
            TestText("produced twice by the head's input list"),
        ],
    );
}

#[test]
fn a_non_circuit_source_folds_nothing()
{
    let verdict = check_circuit_surface(
        "def greeting = \"hi\";\ndata Bool : Type { True : Bool; False : Bool; }\n",
    );
    assert_eq!(CircuitSurface::default(), verdict);
}

#[test]
fn the_committed_internal_wire_witness_binds_and_refuses_nothing()
{
    // The corpus's parse-gated witness for the binder. Every block in it is
    // well-formed, so the fold refuses nothing and binds exactly the six body
    // names that sit outside their heads' interfaces.
    let source = witness(TestText("circuit-internal-wires.gandr"));
    let source = TestText(source.as_str());
    assert_eq!(
        Vec::<String>::new(),
        messages(source),
        "the committed internal-wire witness is well-formed throughout"
    );
    assert_eq!(
        vec![
            "x\u{2032}".to_owned(),
            "y\u{2032}".to_owned(),
            "m1".to_owned(),
            "m2".to_owned(),
            "next".to_owned(),
            "state".to_owned(),
        ],
        wire_names(source)
    );
}

#[test]
fn the_committed_shared_port_witness_names_every_shared_name()
{
    // The corpus's refutation witness: every block parses clean and every block
    // is refused, each diagnostic naming the port name two ports arrived under.
    let source = witness(TestText("circuit-shared-ports.gandr"));
    let found = messages(TestText(source.as_str()));
    assert_eq!(
        4,
        found.len(),
        "each refutation earns exactly one diagnostic; got {found:?}"
    );
    for (index, name) in ["`x`", "`m`", "`x`", "`v`"].into_iter().enumerate() {
        let Some(message) = found.get(index)
        else {
            panic!("refutation {index} was counted but not readable");
        };
        assert!(
            message.contains("the port name ") && message.contains(name),
            "refutation {index} names its shared port {name}; got {message:?}"
        );
    }
}

#[test]
fn the_ruled_worked_examples_bind_their_internal_wires()
{
    // The witness the block form's own rung committed is a fold witness too: its
    // only decline stays the reserved `<->` arrow, and every implicit wire in it
    // binds.
    let source = witness(TestText("circuit-cells.gandr"));
    let source = TestText(source.as_str());
    let found = messages(source);
    assert_eq!(
        1,
        found.len(),
        "the ruled witness earns no port diagnostic; got {found:?}"
    );
    assert_eq!(
        vec![
            "x\u{2032}".to_owned(),
            "y\u{2032}".to_owned(),
            "next".to_owned(),
            "state".to_owned(),
            "m".to_owned(),
        ],
        wire_names(source)
    );
}
