//! The **matcher seam** in production: circuit rule bodies read as wiring
//! diagrams, matched by the embedding matcher, over real declared source.
//!
//! `gandr-theory-circuit-algebras` owns embedding-based matching and
//! `gandr-theory-computads` owns the cell engine; neither names the other, and
//! the seam that lets one consume the other is supplied here, above both. These
//! tests drive that seam from written `sign` blocks rather than from
//! hand-built diagrams, so what they exercise is the route a source takes.

/// Circuit matcher-seam tests.
#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_surface_engine::circuit::embed::CircuitEmbedError;
    use gandr_surface_engine::circuit::embed::CircuitWiringError;
    use gandr_surface_engine::circuit::embed::circuit_wiring;
    use gandr_surface_engine::circuit::embed::embed_circuit_rule;
    use gandr_surface_engine::desc_cells::elaborate_desc_cells;
    use gandr_surface_engine::desc_elab::elaborate_data_descs;
    use gandr_theory_circuit_algebras::interface::EdgeCount;
    use gandr_theory_circuit_algebras::interface::WireCount;
    use gandr_theory_circuit_algebras::matching::MatchBudget;
    use gandr_theory_computads::CmdPat;
    use gandr_theory_computads::ConsPat;
    use gandr_theory_computads::ProdPat;
    use gandr_theory_computads::Tracelet;
    use gandr_theory_computads::enumerate_overlaps;
    use gandr_theory_computads::normalize;
    use gandr_theory_levitation::CircuitBody;
    use gandr_theory_levitation::CircuitFrame;
    use gandr_theory_levitation::CircuitNode;
    use gandr_theory_levitation::CircuitRedex;
    use gandr_theory_levitation::FrameHead;
    use gandr_theory_levitation::FreeTerm;

    use crate::common::TestText;

    extern crate alloc;

    /// A `sign` block declaring two circuit rules, the second of which contains
    /// the first's shape: `cong1` is one redex under one frame, and `cong2`
    /// puts a second frame on top of the same shape.
    const NESTED_CONGRUENCES: &str = "data Nat : Type {\n  Zero : Nat;\n  Succ : (n : Nat) --> \
                                      Nat;\n}\n\nsign Nat {\n  sort Nat : Type;\n  oper add : \
                                      (Nat, Nat) --> Nat;\n\n  rule cong1 : (\n    rule p : Nat \
                                      ==> Nat,\n    data x : Nat,\n    data y : Nat\n  ) ==> (z : \
                                      Nat) {\n    node : p(x) ==> (x\u{2032});\n    node : \
                                      add(x\u{2032}, y) --> (z);\n  };\n\n  rule cong2 : (\n    \
                                      rule p : Nat ==> Nat,\n    data x : Nat,\n    data y : Nat,\n    \
                                      data w : Nat\n  ) ==> (v : Nat) {\n    node : p(x) ==> \
                                      (x\u{2032});\n    node : add(x\u{2032}, y) --> (z);\n    \
                                      node : add(z, w) --> (v);\n  };\n}";

    #[test]
    fn a_two_line_body_reads_as_a_diagram_with_one_internal_wire()
    {
        // The reading, checked against the picture the block draws. `cong1`
        // takes `x` and `y` in and sends `z` out; `x′` is written in no port
        // list, so it is an internal wire — produced by the redex line and read
        // by the frame line, and therefore neither a boundary input nor a
        // boundary output.
        let body = cong1_body();
        let wiring = circuit_wiring(&body).expect("the body reads as a diagram");
        assert_eq!(
            WireCount(4_usize),
            wiring.wire_count(),
            "four ports: x, x′, y, z"
        );
        assert_eq!(
            EdgeCount(2_usize),
            wiring.edge_count(),
            "two lines, two generators"
        );
        assert_eq!(
            2_usize,
            wiring.boundary().inputs.len(),
            "x and y come in, and x′ does not"
        );
        assert_eq!(
            1_usize,
            wiring.boundary().outputs.len(),
            "z goes out, and x′ does not"
        );
    }

    #[test]
    fn a_ground_argument_names_no_wire()
    {
        // The reading is literal rather than helpful: a ground argument is data
        // the diagram layer has no vertex for, and minting one would put a
        // shape in the diagram the body never stated.
        let body = CircuitBody::new(
            [CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Op("add".into()),
                [FreeTerm::ctor("Zero", []), FreeTerm::var("y")],
                "z",
            ))],
            "z",
        );
        let error = circuit_wiring(&body).expect_err("a ground argument names no port");
        assert!(
            matches!(error, CircuitWiringError::ArgumentIsNotAPort(_)),
            "and the refusal says which argument"
        );
    }

    #[test]
    fn a_body_binding_one_port_twice_is_not_a_wiring()
    {
        // Diagram well-formedness is the diagram's own check, not a second one
        // here: two lines binding one port is a fan-in, and the assembly
        // refuses it naming the wire.
        let line = |head: &str| {
            CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Op(head.into()),
                [FreeTerm::var("x")],
                "z",
            ))
        };
        let body = CircuitBody::new([line("f"), line("g")], "z");
        let error = circuit_wiring(&body).expect_err("two lines cannot bind one port");
        assert!(
            matches!(error, CircuitWiringError::NotAWiring(_)),
            "and the refusal is the diagram's, carrying its obstruction"
        );
    }

    #[test]
    fn a_rule_body_embeds_in_a_body_that_contains_it()
    {
        // The seam's answer, on real source. `cong2` is `cong1` with one more
        // frame stacked on its output, so `cong1`'s diagram sits inside it.
        let descs = declared_descs(TestText(NESTED_CONGRUENCES));
        let desc = signature_with_circuits(&descs);
        let (cong1, cong2) = two_circuits(desc);
        let matching = embed_circuit_rule(&cong1.body, &cong2.body, MatchBudget(4_096_usize))
            .expect("both bodies are diagrams and the search completes");
        assert!(
            matching.admitted_count().0 > 0_usize,
            "the smaller rule's diagram occurs in the larger one"
        );
        // Every admitted embedding is checked against the two diagrams it
        // claims to relate rather than counted.
        let pattern = circuit_wiring(&cong1.body).expect("pattern reads");
        let target = circuit_wiring(&cong2.body).expect("target reads");
        for embedding in matching.admitted() {
            assert!(
                embedding.check(&pattern, &target).is_ok(),
                "the matcher's own verifier re-derives every conjunct of the certificate"
            );
        }
    }

    #[test]
    fn a_body_that_does_not_contain_the_pattern_admits_no_embedding()
    {
        // The other direction of the same pair: the larger diagram does not
        // occur in the smaller one.
        let descs = declared_descs(TestText(NESTED_CONGRUENCES));
        let desc = signature_with_circuits(&descs);
        let (cong1, cong2) = two_circuits(desc);
        let matching = embed_circuit_rule(&cong2.body, &cong1.body, MatchBudget(4_096_usize))
            .expect("both bodies are diagrams and the search completes");
        assert_eq!(
            0_usize,
            matching.admitted_count().0,
            "three lines do not fit inside two"
        );
    }

    #[test]
    fn an_exhausted_budget_declines_rather_than_reporting_no_match()
    {
        // A truncated enumeration presented as a complete one is the error the
        // budget exists to avoid, so exhaustion is a decline and never an empty
        // admitted set.
        let descs = declared_descs(TestText(NESTED_CONGRUENCES));
        let desc = signature_with_circuits(&descs);
        let (cong1, cong2) = two_circuits(desc);
        let error = embed_circuit_rule(&cong1.body, &cong2.body, MatchBudget(0_usize))
            .expect_err("a zero budget cannot complete a search");
        assert!(
            matches!(error, CircuitEmbedError::Matching(_)),
            "and it declines on the search rather than on the reading"
        );
    }

    #[test]
    fn the_description_route_records_where_one_rule_occurs_in_another()
    {
        // The seam in production: the description route runs the matcher over
        // the circuit rules a source declares and keeps the redex-occurrence
        // records beside the cells the engine admitted. Both crates are
        // consumed on one source, and neither names the other.
        let descs = declared_descs(TestText(NESTED_CONGRUENCES));
        let cells = elaborate_desc_cells(&descs);
        assert!(
            !cells.circuit_sites.is_empty(),
            "the route recorded occurrences rather than skipping the matcher"
        );
        let found = cells
            .circuit_sites
            .iter()
            .find(|site| site.pattern.as_ref() == "cong1" && site.target.as_ref() == "cong2")
            .expect("the pair that stands in a containment has a record");
        assert!(
            found.admitted.0 > 0_usize,
            "and the record says the smaller rule occurs in the larger one"
        );
        let absent = cells
            .circuit_sites
            .iter()
            .find(|site| site.pattern.as_ref() == "cong2" && site.target.as_ref() == "cong1")
            .expect("the reverse pair has a record too");
        assert_eq!(
            0_usize, absent.admitted.0,
            "which says the larger rule does not occur in the smaller one"
        );
        // And the cells the engine admitted below the seam still replay. The
        // seam supplies matching; it does not take the engine's own work over,
        // and this is the half of that claim that runs.
        let store = cells
            .stores
            .iter()
            .find(|store| store.len() > gandr_theory_computads::CellCount::from(0_usize))
            .expect("the source's declarations put cells in a store");
        let peak = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let stepped = normalize(store, &peak, 64_usize.into());
        assert!(
            !stepped.path.is_empty(),
            "the declared constructors' frame cells fire on a ground configuration"
        );
        let mut overlap = enumerate_overlaps(store)
            .into_iter()
            .next()
            .expect("the store enumerates an overlap to carry a peak");
        overlap.peak = peak;
        let certificate = Tracelet {
            overlap,
            path_a: stepped.path.clone(),
            path_b: stepped.path,
            joins_at: stepped.normal,
        };
        assert!(
            bool::from(certificate.replay(store)),
            "and a certificate recording that derivation replays"
        );
    }

    /// `cong1`'s body, built directly: one redex line feeding one frame line.
    fn cong1_body() -> CircuitBody
    {
        CircuitBody::new(
            [
                CircuitNode::Redex(CircuitRedex::new(
                    "p",
                    FreeTerm::var("x"),
                    FreeTerm::var("x\u{2032}"),
                    "x\u{2032}",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("add".into()),
                    [FreeTerm::var("x\u{2032}"), FreeTerm::var("y")],
                    "z",
                )),
            ],
            "z",
        )
    }

    /// Elaborate a source's declarations, asserting the source is well formed.
    fn declared_descs(source: TestText<'_>) -> alloc::vec::Vec<gandr_theory_levitation::SignDesc>
    {
        let elaborated = elaborate_data_descs(source.0);
        assert!(
            elaborated.diagnostics.is_empty(),
            "the source declares cleanly: {:?}",
            elaborated.diagnostics
        );
        elaborated.descs
    }

    /// The one description of `descs` that declares circuit rules — the source
    /// carries a `data` block beside its `sign` block, and only the second has
    /// any.
    fn signature_with_circuits(
        descs: &[gandr_theory_levitation::SignDesc]
    ) -> &gandr_theory_levitation::SignDesc
    {
        descs
            .iter()
            .find(|desc| !desc.circuits.is_empty())
            .expect("the sign block declares circuit rules")
    }

    /// The two circuit rules of a description whose block declares exactly two.
    fn two_circuits(
        desc: &gandr_theory_levitation::SignDesc
    ) -> (
        &gandr_theory_levitation::CircuitRule,
        &gandr_theory_levitation::CircuitRule,
    )
    {
        let mut circuits = desc.circuits.iter();
        let first = circuits.next().expect("the block declares a first rule");
        let second = circuits.next().expect("the block declares a second rule");
        (first, second)
    }
}
