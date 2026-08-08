//! The circuit surface check: arrow-kind confirmation and the reserved
//! reversible glyph's decline.
//!
//! The ruled block form's redundancy is the thing under test. A declaration's
//! kind keyword, its signature arrow, and its body lines' arrows spell the
//! block's dimension three independent ways, so each of these tests writes one
//! of those three down wrong and asserts the check names it.

use gandr_surface_engine::circuit::check_circuit_surface;

use crate::common::TestText;

/// The congruence cell, verbatim from the ruling
/// (`docs/gandr/spec/surface-language/circuit-cells.md` §"The block form,
/// ruled"): a `sign` block presents sorts, operations, and rules — the
/// item-level `data` member is retired, so no constructors stand here.
const CONG2: TestText<'static> = TestText(
    "\
sign Nat {
  sort Nat : Type;
  oper add : (Nat, Nat) --> Nat;

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

/// The first stateful wheel, verbatim from the ruling.
const ACCUMULATE: TestText<'static> = TestText(
    "\
oper accumulate : (stream : Stream(Nat)) --> (out2 : Stream(Nat)) {
  node : zip(stream, state) --> (next, out2);
  feed : (next) --> (state);
}
",
);

/// Every message the check produced, in source order.
fn messages(source: TestText<'_>) -> Vec<String>
{
    check_circuit_surface(source.0)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
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

#[test]
fn the_ruled_worked_examples_confirm_every_arrow()
{
    // Both worked examples agree with themselves at every arrow: the `oper`
    // members carry `-->`, the `rule` member and its rewrite binders carry
    // `==>`, the redex lines carry `==>` because `p` and `q` are rules, the
    // frame line carries `-->` because `add` is an oper, and the `feed` wire
    // carries `-->`.
    for source in [CONG2, ACCUMULATE] {
        assert_eq!(
            Vec::<String>::new(),
            messages(source),
            "the ruled worked example confirms clean"
        );
    }
}

#[test]
fn a_non_circuit_source_is_not_read_at_all()
{
    // The pass reads circuit declarations and nothing else.
    assert_eq!(
        Vec::<String>::new(),
        messages(TestText(
            "def greeting = \"hi\";\ndata Bool : Type { True : Bool; False : Bool; }\n"
        ))
    );
}

#[test]
fn a_declaration_arrow_disagreeing_with_its_kind_is_named()
{
    // An `oper` is a circuit 1-cell former, so a rewrite face on its signature
    // is a disagreement — and the message says which name, which glyph, and
    // what to write instead.
    assert_sole_diagnostic(
        TestText("sign S { oper add : (a : Nat, b : Nat) ==> (c : Nat); }"),
        &[
            TestText("`oper add`"),
            TestText("a circuit 1-cell former"),
            TestText("`==>`"),
            TestText("a rewrite face"),
            TestText("write `-->`"),
        ],
    );
    // And symmetrically: a `rule` takes a face, so a circuit former on its
    // signature is the same disagreement read the other way.
    assert_sole_diagnostic(TestText("rule step : (x : Nat) --> (y : Nat)"), &[
        TestText("`rule step`"),
        TestText("a rewrite face"),
        TestText("`-->`"),
        TestText("a circuit 1-cell former"),
        TestText("write `==>`"),
    ]);
    // A `data` constructor is a circuit 1-cell former too, and the bare-sort
    // sugar rung is checked exactly like the named-port one.
    assert_sole_diagnostic(TestText("sign S { data Succ : Nat ==> Nat; }"), &[
        TestText("`data Succ`"),
        TestText("write `-->`"),
    ]);
}

#[test]
fn a_rewrite_binder_arrow_disagreeing_with_its_kind_is_named()
{
    // A rewrite-sorted binder in a parameter telescope is a rule wherever it
    // sits, so its endpoints are joined by a face. The pinned-endpoint form is
    // the same binder with variable endpoints and is checked identically.
    assert_sole_diagnostic(
        TestText("sign S { rule cong : (rule p : Nat --> Nat, data x : Nat) ==> (z : Nat); }"),
        &[
            TestText("the binder `rule p`"),
            TestText("`-->`"),
            TestText("write `==>`"),
        ],
    );
}

#[test]
fn a_body_line_arrow_disagreeing_with_its_head_is_named()
{
    // The redex/frame distinction, made locally visible and then written down
    // wrong: `add` is an oper, so its node line is a frame line and takes
    // `-->`. The two arguments are distinct wires because the port discipline
    // admits no implicit duplication — `add(x, x)` would earn a second,
    // unrelated diagnostic from the name-set fold.
    assert_sole_diagnostic(
        TestText(
            "sign S {
  oper add : (Nat, Nat) --> Nat;
  rule r : (data x : Nat, data y : Nat) ==> (z : Nat) {
    node : add(x, y) ==> (z);
  };
}",
        ),
        &[
            TestText("the node line applying `add`"),
            TestText("declared `oper`"),
            TestText("`==>`"),
            TestText("write `-->`"),
        ],
    );
    // And the redex line the other way: `p` is a rule binder, so its line is a
    // redex line and takes `==>`.
    assert_sole_diagnostic(
        TestText(
            "sign S {
  rule r : (rule p : Nat ==> Nat, data x : Nat) ==> (z : Nat) {
    node : p(x) --> (z);
  };
}",
        ),
        &[
            TestText("the node line applying `p`"),
            TestText("declared `rule`"),
            TestText("`-->`"),
            TestText("write `==>`"),
        ],
    );
    // A `feed` line applies nothing: it is a wire, so it takes the circuit
    // former whatever the enclosing block's dimension is.
    assert_sole_diagnostic(
        TestText(
            "oper acc : (s : Stream) --> (o : Stream) {
  feed : (n) ==> (t);
}",
        ),
        &[
            TestText("the feed line's back-edge wire"),
            TestText("`==>`"),
            TestText("write `-->`"),
        ],
    );
}

#[test]
fn an_unresolved_head_is_skipped_rather_than_guessed()
{
    // `zip` is not declared anywhere in scope, so the pass has no kind to
    // confirm against. That is a name-resolution question — the sibling
    // port/name discipline's — and answering it here would report an error the
    // program does not have. Both spellings therefore stay silent.
    for arrow in ["-->", "==>"] {
        let source = format!(
            "oper acc : (s : Stream) --> (o : Stream) {{\n  node : zip(s) {arrow} (o);\n}}"
        );
        assert_eq!(
            Vec::<String>::new(),
            messages(TestText(source.as_str())),
            "an unresolved head is skipped, not guessed ({arrow})"
        );
    }
}

#[test]
fn the_reserved_reversible_glyph_declines_naming_its_lane()
{
    // `<->` is in the circuit row, so it agrees with an `oper` — and still
    // declines, because a reversible oper is a semantic claim the reversible
    // lane owes a checking discipline for.
    assert_sole_diagnostic(
        TestText("sign S { oper negate : (b : Bit) <-> (c : Bit); }"),
        &[
            TestText("`oper negate`"),
            TestText("reserved arrow `<->`"),
            TestText("data-level iso"),
            TestText("reversible-oper lane"),
        ],
    );
    // The invertible *face* `<=>` is not reserved: on a rule it is the
    // certificate algebra's unconditional-composition licence, and it is
    // accepted.
    assert_eq!(
        Vec::<String>::new(),
        messages(TestText(
            "sign S { rule involutive : (b : Bit) <=> (c : Bit); }"
        )),
        "the invertible face is a licence, not a reservation"
    );
    // A reserved glyph in the wrong row reports the row, not the reservation:
    // `<->` on a rule is in the wrong row before it is reserved, and one arrow
    // earns one diagnostic.
    assert_sole_diagnostic(TestText("sign S { rule r : (b : Bit) <-> (c : Bit); }"), &[
        TestText("`rule r`"),
        TestText("a rewrite face"),
        TestText("`<->`"),
        TestText("write `==>`"),
    ]);
    // The reservation reaches a body line too, through the same confirmation.
    assert_sole_diagnostic(
        TestText(
            "sign S {
  oper flip : (Bit) --> (Bit);
  oper twice : (b : Bit) --> (d : Bit) {
    node : flip(b) <-> (d);
  };
}",
        ),
        &[
            TestText("the node line applying `flip`"),
            TestText("reserved arrow `<->`"),
        ],
    );
}

#[test]
fn the_committed_surface_witness_confirms_clean()
{
    // The corpus's parse-gated witness is a well-formed program at every rung
    // of the ruled form except its one deliberate reservation: the `<->` oper
    // it records as reserved. Exactly that one arrow declines, and nothing
    // else in the file does.
    let witness = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../surface-corpus/examples/surface/circuit-cells.gandr"),
    )
    .expect("the committed surface witness is readable");
    let found = messages(TestText(witness.as_str()));
    assert_eq!(
        1,
        found.len(),
        "the witness declines exactly its reserved arrow; got {found:?}"
    );
    let Some(message) = found.first()
    else {
        panic!("a diagnostic was counted but not readable");
    };
    assert!(
        message.contains("`oper negate`") && message.contains("reversible-oper lane"),
        "the witness's decline is the reserved reversible glyph; got {message:?}"
    );
}
