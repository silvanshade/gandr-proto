//! The **matcher seam** in production: circuit rule bodies read as wiring
//! diagrams, matched by the embedding matcher, over real declared source.
//!
//! `gandr-theory-circuit-algebras` owns embedding-based matching and
//! `gandr-theory-computads` owns the cell engine; generic overlaps, completion
//! and certificates belong to `gandr-theory-coherent-resolutions`. None of
//! those lower crates names another; the seam that lets one consume the other
//! is supplied here, above both. These tests drive that seam from written
//! `sign` blocks rather than from hand-built diagrams, so source-authored
//! occurrences stay observable. The refusal arms and the hand-built
//! multi-root/reconvergent witness keep matcher rejection and certificate
//! replay observable as well.

/// Circuit matcher-seam tests.
#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_surface_engine::boundary::MatchDecision;
    use gandr_surface_engine::circuit::embed::CircuitAdapterBudget;
    use gandr_surface_engine::circuit::embed::CircuitEmbedError;
    use gandr_surface_engine::circuit::embed::CircuitRuleCell;
    use gandr_surface_engine::circuit::embed::CircuitWiringError;
    use gandr_surface_engine::circuit::embed::circuit_wiring;
    use gandr_surface_engine::circuit::embed::complete_circuit_rules;
    use gandr_surface_engine::circuit::embed::embed_circuit_rule;
    use gandr_surface_engine::desc_cells::elaborate_desc_cells;
    use gandr_surface_engine::desc_cells::elaborate_desc_cells_with_completion_budget;
    use gandr_surface_engine::desc_elab::elaborate_data_descs;
    use gandr_theory_cell_complexes::Cell;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes::CmdPat;
    use gandr_theory_cell_complexes::ConsPat;
    use gandr_theory_cell_complexes::ProdPat;
    use gandr_theory_cell_complexes::sequent::CellProvenance;
    use gandr_theory_cell_complexes::sequent::Orientation;
    use gandr_theory_circuit_algebras::interface::EdgeCount;
    use gandr_theory_circuit_algebras::interface::Wire;
    use gandr_theory_circuit_algebras::interface::WireCount;
    use gandr_theory_circuit_algebras::matching::MatchBudget;
    use gandr_theory_coherent_resolutions::CompletionBudget;
    use gandr_theory_coherent_resolutions::CompletionOutcome;
    use gandr_theory_coherent_resolutions::DeclineReason;
    use gandr_theory_coherent_resolutions::SuppliedOverlapError;
    use gandr_theory_coherent_resolutions::Tracelet;
    use gandr_theory_coherent_resolutions::enumerate_overlaps;
    use gandr_theory_coherent_resolutions::normalize;
    use gandr_theory_coherent_resolutions::overlaps_between;
    use gandr_theory_levitation::CircuitBody;
    use gandr_theory_levitation::CircuitFrame;
    use gandr_theory_levitation::CircuitNode;
    use gandr_theory_levitation::FrameHead;
    use gandr_theory_levitation::FreeTerm;

    use crate::common::TestText;

    extern crate alloc;

    /// A `sign` block declaring two circuit rules, the second of which contains
    /// the first's shape: `cong1` is one redex under one frame, and `cong2`
    /// puts a second frame on top of the same shape.
    const NESTED_CONGRUENCES: &str = r#"data Nat : Type {
  Zero : Nat;
  Succ : (n : Nat) --> Nat;
}

sign Nat {
  sort Nat : Type;
  oper add : (Nat, Nat) --> Nat;
  oper neg : (Nat) --> Nat;

  rule cong1 : (
    rule p : Nat ==> Nat,
    data x : Nat,
    data y : Nat
  ) ==> (z : Nat) {
    node : p(x) ==> (x′);
    node : add(x′, y) --> (z);
  };

  rule cong2 : (
    rule p : Nat ==> Nat,
    data x : Nat,
    data y : Nat,
    data w : Nat
  ) ==> (v : Nat) {
    node : p(x) ==> (x′);
    node : add(x′, y) --> (z);
    node : add(z, w) --> (v);
  };

  rule viaOper : (
    data x : Nat
  ) ==> (z : Nat) {
    node : neg(x) --> (z);
  };

  rule viaRewrite : (
    rule neg : Nat ==> Nat,
    data x : Nat
  ) ==> (z : Nat) {
    node : neg(x) ==> (z);
  };
}"#;
    /// A real description whose two admitted circuit cells have different
    /// spheres while the target body contains the pattern body.
    const ADMITTED_COMPLETION_ROUTE: &str = r#"data Nat : Type {
  Zero : Nat;
}

sign Nat {
  sort Nat : Type;
  oper add : (Nat, Nat) --> Nat;

  rule first : (
    rule p : Nat ==> Nat,
    data x : Nat,
    data y : Nat
  ) ==> (z : Nat) {
    node : p(x) ==> (x′);
    node : add(x′, y) --> (z);
  };

  rule second : (
    rule p : Nat ==> Nat,
    data a : Nat,
    data b : Nat
  ) ==> (z : Nat) {
    node : p(a) ==> (a′);
    node : add(a′, b) --> (z);
  };
}"#;

    #[test]
    fn a_two_line_body_reads_as_a_diagram_with_one_internal_wire()
    {
        // The reading, checked against the picture the block draws. `cong1`
        // takes `x` and `y` in and sends `z` out; `x′` is written in no port
        // list, so it is an internal wire — produced by the redex line and read
        // by the frame line, and therefore neither a boundary input nor a
        // boundary output.
        let descs = declared_descs(TestText(NESTED_CONGRUENCES));
        let desc = signature_with_circuits(&descs);
        let cong1 = named_circuit(desc, RuleName("cong1"));
        let wiring = circuit_wiring(&cong1.body).expect("the body reads as a diagram");
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
        let cong1 = named_circuit(desc, RuleName("cong1"));
        let cong2 = named_circuit(desc, RuleName("cong2"));
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
        let cong1 = named_circuit(desc, RuleName("cong1"));
        let cong2 = named_circuit(desc, RuleName("cong2"));
        let matching = embed_circuit_rule(&cong2.body, &cong1.body, MatchBudget(4_096_usize))
            .expect("both bodies are diagrams and the search completes");
        assert_eq!(
            0_usize,
            matching.admitted_count().0,
            "three lines do not fit inside two"
        );
    }

    #[test]
    fn one_spelling_in_two_roles_is_two_boxes()
    {
        // Why a generator's label carries its role beside its name. `viaOper`
        // applies the declared operation `neg`; `viaRewrite` applies a rewrite
        // parameter that is also spelled `neg`. Same name, same arity, same
        // wiring — one input, one output, one line — and they are two different
        // boxes. A reading that kept only the name would match either into the
        // other, which is the collision the sort vocabulary exists to stop, and
        // it is exactly the collision that a fired rewrite having no role of
        // its own would have reintroduced.
        let descs = declared_descs(TestText(NESTED_CONGRUENCES));
        let desc = signature_with_circuits(&descs);
        let via_oper = named_circuit(desc, RuleName("viaOper"));
        let via_rewrite = named_circuit(desc, RuleName("viaRewrite"));

        // The hypothesis: the two bodies really are the same shape.
        let operation = circuit_wiring(&via_oper.body).expect("the operation body reads");
        let rewrite = circuit_wiring(&via_rewrite.body).expect("the rewrite body reads");
        assert_eq!(
            operation.wire_count(),
            rewrite.wire_count(),
            "the two bodies name the same number of ports"
        );
        assert_eq!(
            operation.edge_count(),
            rewrite.edge_count(),
            "and each is one line"
        );

        for (label, pattern, target) in [
            (
                "a rewrite into an operation",
                &via_rewrite.body,
                &via_oper.body,
            ),
            (
                "an operation into a rewrite",
                &via_oper.body,
                &via_rewrite.body,
            ),
        ] {
            let matching = embed_circuit_rule(pattern, target, MatchBudget(4_096_usize))
                .expect("both bodies are diagrams and the search completes");
            assert_eq!(
                0_usize,
                matching.admitted_count().0,
                "{label}: one spelling worn in two roles is two boxes, not one"
            );
        }

        // And each does embed in itself, so the refusal above is the role and
        // not a shape the matcher cannot see at all.
        for (label, body) in [
            ("the operation body", &via_oper.body),
            ("the rewrite body", &via_rewrite.body),
        ] {
            let matching = embed_circuit_rule(body, body, MatchBudget(4_096_usize))
                .expect("the search completes");
            assert!(
                matching.admitted_count().0 > 0_usize,
                "{label} embeds in itself"
            );
        }
    }

    #[test]
    fn an_exhausted_budget_declines_rather_than_reporting_no_match()
    {
        // A truncated enumeration presented as a complete one is the error the
        // budget exists to avoid, so exhaustion is a decline and never an empty
        // admitted set.
        let descs = declared_descs(TestText(NESTED_CONGRUENCES));
        let desc = signature_with_circuits(&descs);
        let cong1 = named_circuit(desc, RuleName("cong1"));
        let cong2 = named_circuit(desc, RuleName("cong2"));
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
        assert_eq!(descs.len(), cells.circuit_completions.len());
        let (completion_index, completion) = cells
            .circuit_completions
            .iter()
            .enumerate()
            .find(|&(_, completion)| {
                completion.matches.iter().any(|matched| {
                    matched.pattern.as_ref() == "cong1" && matched.target.as_ref() == "cong2"
                })
            })
            .expect("the real route retains a structured completion result");
        assert_eq!(
            cells.circuit_completions[completion_index].outcome.store(),
            completion.outcome.store(),
            "the route's store is the outcome projection, not its replacement"
        );
        let found = completion
            .matches
            .iter()
            .find(|matched| {
                matched.pattern.as_ref() == "cong1" && matched.target.as_ref() == "cong2"
            })
            .expect("the pair that stands in a containment has a record");
        assert!(
            found.admitted.0 > 0_usize,
            "and the record says the smaller rule occurs in the larger one"
        );
        let absent = completion
            .matches
            .iter()
            .find(|matched| {
                matched.pattern.as_ref() == "cong2" && matched.target.as_ref() == "cong1"
            })
            .expect("the reverse pair has a record too");
        assert_eq!(
            0_usize, absent.admitted.0,
            "which says the larger rule does not occur in the smaller one"
        );
        // And the cells the engine admitted below the seam still replay. The
        // seam supplies matching; it does not take the engine's own work over,
        // and this is the half of that claim that runs.
        let store = cells
            .circuit_completions
            .iter()
            .map(|completion| completion.outcome.store())
            .find(|store| store.len() > gandr_theory_cell_complexes::CellCount::from(0_usize))
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

    #[test]
    fn a_constructor_line_is_a_value_generator_and_every_refusal_renders()
    {
        // The two arms the declared fixtures do not reach. A body line may
        // apply a constructor rather than an operation or a rewrite, which is
        // the third role a label can be worn in; and each refusal has a reading
        // a caller may show a user, which is worth having exercised rather than
        // merely written.
        let ctor_body = CircuitBody::new(
            [CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Ctor("Succ".into()),
                [FreeTerm::var("x")],
                "z",
            ))],
            "z",
        );
        let wiring = circuit_wiring(&ctor_body).expect("a constructor line reads as a diagram");
        assert_eq!(
            EdgeCount(1_usize),
            wiring.edge_count(),
            "one line, one generator"
        );
        let generator = wiring
            .generators()
            .first()
            .expect("the diagram holds the line's generator");
        assert_eq!(
            gandr_theory_circuit_algebras::interface::GeneratorSort::Value,
            generator.label.sort,
            "a constructor head is worn in the value role"
        );

        let ground = CircuitBody::new(
            [CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Op("add".into()),
                [FreeTerm::ctor("Zero", []), FreeTerm::var("y")],
                "z",
            ))],
            "z",
        );
        let not_a_port = circuit_wiring(&ground).expect_err("a ground argument names no port");
        assert!(
            alloc::format!("{not_a_port}").contains("Zero(…)"),
            "the refusal names the argument it could not read"
        );

        let line = |head: &str| {
            CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Op(head.into()),
                [FreeTerm::var("x")],
                "z",
            ))
        };
        let fan_in = CircuitBody::new([line("f"), line("g")], "z");
        let not_a_wiring = circuit_wiring(&fan_in).expect_err("two lines cannot bind one port");
        assert!(
            alloc::format!("{not_a_wiring}").contains("not a wiring diagram"),
            "the refusal says the diagram is the thing that refused it"
        );
        assert!(
            alloc::format!("{}", CircuitEmbedError::Wiring(not_a_wiring))
                .contains("not a wiring diagram"),
            "and the embedding error passes a reading refusal through unchanged"
        );

        let descs = declared_descs(TestText(NESTED_CONGRUENCES));
        let desc = signature_with_circuits(&descs);
        let cong1 = named_circuit(desc, RuleName("cong1"));
        let cong2 = named_circuit(desc, RuleName("cong2"));
        let exhausted = embed_circuit_rule(&cong1.body, &cong2.body, MatchBudget(0_usize))
            .expect_err("a zero budget cannot complete a search");
        assert!(
            alloc::format!("{exhausted}").contains("larger budget"),
            "and a search decline says what to do about it"
        );
    }

    #[test]
    fn the_multi_root_and_reconvergent_case_stays_matcher_only()
    {
        let multi_root = CircuitBody::new(
            [
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("left".into()),
                    [FreeTerm::var("x")],
                    "y",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("right".into()),
                    [FreeTerm::var("z")],
                    "w",
                )),
            ],
            "w",
        );
        let reconvergent = CircuitBody::new(
            [
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("left".into()),
                    [FreeTerm::var("x")],
                    "y",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("right".into()),
                    [FreeTerm::var("z")],
                    "w",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("branch-left".into()),
                    [FreeTerm::var("a")],
                    "b",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("branch-right".into()),
                    [FreeTerm::var("c")],
                    "d",
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op("join".into()),
                    [FreeTerm::var("b"), FreeTerm::var("d")],
                    "e",
                )),
            ],
            "e",
        );
        let forward = embed_circuit_rule(&multi_root, &reconvergent, MatchBudget(4_096_usize))
            .expect("the matcher handles the multi-root/reconvergent pair");
        assert!(
            forward.admitted_count().0 > 0_usize,
            "the matcher admits the multi-root embedding"
        );
        let reverse = embed_circuit_rule(&reconvergent, &multi_root, MatchBudget(4_096_usize))
            .expect("the reverse matcher search completes");
        assert_eq!(
            0_usize,
            reverse.admitted_count().0,
            "the reconvergent target does not fit in the multi-root pattern"
        );
    }
    #[test]
    fn w1_two_embeddings_supply_two_critical_pairs()
    {
        let pattern_body = unary_body("left", "x", "y");
        let target_body = parallel_unary_body("left", ("a", "b"), ("c", "d"));
        let mut store = CellStore::new();
        let pattern_cell = fixed_cell(
            &mut store,
            Polarity::Positive,
            "pattern",
            "pattern-rhs",
            CellProvenance::SurfaceRule,
        );
        let target_cell = fixed_cell(
            &mut store,
            Polarity::Positive,
            "pattern",
            "pattern-rhs",
            CellProvenance::MuMuTilde,
        );
        let pattern_name: gandr_theory_levitation::Name = "w1-pattern".into();
        let target_name: gandr_theory_levitation::Name = "w1-target".into();
        let rules = [
            CircuitRuleCell {
                name: &pattern_name,
                body: &pattern_body,
                cell: Some(pattern_cell),
            },
            CircuitRuleCell {
                name: &target_name,
                body: &target_body,
                cell: Some(target_cell),
            },
        ];
        let completion = complete_circuit_rules(
            store,
            &rules,
            MatchBudget(4_096_usize),
            CompletionBudget::new(0_usize.into(), 64_usize.into(), 64_usize.into()),
            CircuitAdapterBudget(64_usize),
        );
        let forward = completion
            .matches
            .iter()
            .find(|matched| {
                matched.pattern.as_ref() == "w1-pattern" && matched.target.as_ref() == "w1-target"
            })
            .expect("the ordered pair has a completion record");
        assert_eq!(
            2_usize, forward.admitted.0,
            "the one-line pattern has two admitted placements"
        );
        assert_eq!(2_usize, forward.embeddings.len());
        assert!(
            forward.embeddings.iter().all(|embedding| {
                embedding.overlap_count == 1_usize && embedding.decline.is_none()
            }),
            "each embedding supplies one ordinary overlap"
        );
        let mut seam_pairs = forward
            .embeddings
            .iter()
            .map(|embedding| {
                let input = embedding
                    .origin
                    .seam
                    .inputs
                    .image_of(Wire(0))
                    .expect("the full origin seam keeps the pattern input pair");
                let output = embedding
                    .origin
                    .seam
                    .outputs
                    .image_of(Wire(1))
                    .expect("the full origin seam keeps the pattern output pair");
                (input, output)
            })
            .collect::<alloc::vec::Vec<_>>();
        seam_pairs.sort_unstable_by_key(|&(input, output)| (input.0, output.0));
        assert_eq!(
            alloc::vec![(Wire(0), Wire(1)), (Wire(2), Wire(3))],
            seam_pairs,
            "the public origin retains every input/output seam pair"
        );
        assert_eq!(2_usize, forward.overlap_count);
        let CompletionOutcome::Declined {
            ref pending,
            reason,
            ..
        } = completion.outcome
        else {
            panic!("the zero completion budget preserves supplied work")
        };
        assert_eq!(DeclineReason::StepBudget, reason);
        let supplied = pending
            .iter()
            .flat_map(|batch| batch.iter())
            .filter(|overlap| overlap.left == pattern_cell && overlap.right == target_cell)
            .count();
        assert_eq!(
            1_usize, supplied,
            "structurally identical ordinary work is supplied once"
        );
        let supplied_origins = completion
            .replay_evidence
            .iter()
            .filter(|evidence| {
                evidence.origin.pattern.name.as_ref() == "w1-pattern"
                    && evidence.origin.target.name.as_ref() == "w1-target"
            })
            .count();
        assert_eq!(
            2_usize, supplied_origins,
            "both circuit placements remain attached to replay evidence"
        );
    }

    #[test]
    fn w2_embedding_supplies_a_non_unifying_sequent_pair()
    {
        let pattern_body = unary_body("left", "x", "y");
        let target_body = unary_body("left", "a", "b");
        let mut store = CellStore::new();
        let pattern_cell = fixed_cell(
            &mut store,
            Polarity::Positive,
            "pattern",
            "pattern-rhs",
            CellProvenance::SurfaceRule,
        );
        let target_cell = fixed_cell(
            &mut store,
            Polarity::Negative,
            "pattern",
            "pattern-rhs",
            CellProvenance::MuMuTilde,
        );
        assert!(
            overlaps_between(
                (
                    pattern_cell,
                    store.get(pattern_cell).expect("pattern cell is stored"),
                ),
                (
                    target_cell,
                    store.get(target_cell).expect("target cell is stored"),
                ),
            )
            .is_empty(),
            "the ordinary sequent matcher rejects the opposite-polarity pair"
        );
        let pattern_name: gandr_theory_levitation::Name = "w2-pattern".into();
        let target_name: gandr_theory_levitation::Name = "w2-target".into();
        let rules = [
            CircuitRuleCell {
                name: &pattern_name,
                body: &pattern_body,
                cell: Some(pattern_cell),
            },
            CircuitRuleCell {
                name: &target_name,
                body: &target_body,
                cell: Some(target_cell),
            },
        ];
        let completion = complete_circuit_rules(
            store,
            &rules,
            MatchBudget(4_096_usize),
            CompletionBudget::new(0_usize.into(), 64_usize.into(), 64_usize.into()),
            CircuitAdapterBudget(64_usize),
        );
        let supplied = completion
            .matches
            .iter()
            .find(|matched| {
                matched.pattern.as_ref() == "w2-pattern" && matched.target.as_ref() == "w2-target"
            })
            .expect("the ordered pair has a completion record");
        assert_eq!(1_usize, supplied.admitted.0);
        assert_eq!(
            1_usize, supplied.overlap_count,
            "the circuit seam supplies an overlap despite empty generic enumeration"
        );
        assert!(supplied.embeddings[0].decline.is_none());
        let &CompletionOutcome::Declined { ref reason, .. } = &completion.outcome
        else {
            panic!("the malformed supplied overlap must decline");
        };
        assert!(
            matches!(
                reason,
                DeclineReason::InvalidSuppliedOverlap(
                    SuppliedOverlapError::NonUnifyingSubstitution { .. }
                )
            ),
            "completion decline: {reason:?}"
        );
    }

    #[test]
    fn w3_deduplicated_declarations_keep_replay_attribution()
    {
        let body = unary_body("left", "x", "y");
        let mut store = CellStore::new();
        let cell = fixed_cell(
            &mut store,
            Polarity::Positive,
            "shared",
            "shared-rhs",
            CellProvenance::SurfaceRule,
        );
        let first_name: gandr_theory_levitation::Name = "w3-first".into();
        let second_name: gandr_theory_levitation::Name = "w3-second".into();
        let rules = [
            CircuitRuleCell {
                name: &first_name,
                body: &body,
                cell: Some(cell),
            },
            CircuitRuleCell {
                name: &second_name,
                body: &body,
                cell: Some(cell),
            },
        ];
        let completion = complete_circuit_rules(
            store,
            &rules,
            MatchBudget(4_096_usize),
            CompletionBudget::new(64_usize.into(), 64_usize.into(), 64_usize.into()),
            CircuitAdapterBudget(64_usize),
        );
        let origins = completion
            .cell_origins
            .get(&cell)
            .expect("the shared cell has a side-map entry");
        assert_eq!(2_usize, origins.len());
        assert_eq!("w3-first", origins[0].name.as_ref());
        assert_eq!("w3-second", origins[1].name.as_ref());
        let pair = completion
            .matches
            .iter()
            .find(|matched| {
                matched.pattern.as_ref() == "w3-first" && matched.target.as_ref() == "w3-second"
            })
            .expect("the cross-declaration pair remains observable");
        assert_eq!(1_usize, pair.admitted.0);
        assert!(
            pair.embeddings[0].decline.is_none(),
            "the supplied overlap is faithful for the shared declaration cell"
        );
        assert_eq!(1_usize, pair.overlap_count);
        let replay = completion
            .replay_evidence
            .iter()
            .find(|evidence| {
                evidence.origin.pattern.name.as_ref() == "w3-first"
                    && evidence.origin.target.name.as_ref() == "w3-second"
            })
            .expect("the shared overlap retains the cross-declaration origin");
        assert!(
            !replay.certificate_indices.is_empty(),
            "a nonzero completion budget makes replay attribution observable"
        );
        assert!(
            pair.certificates_replayed > 0_usize,
            "the completion match records its replayed certificates"
        );
    }

    #[test]
    fn w4_distinct_wire_renderings_keep_origin_seams()
    {
        let pattern_body = two_component_body(MatchDecision::from(false));
        let target_body = two_component_body(MatchDecision::from(true));
        let mut store = CellStore::new();
        let pattern_cell = fixed_cell(
            &mut store,
            Polarity::Positive,
            "pattern",
            "pattern-rhs",
            CellProvenance::SurfaceRule,
        );
        let target_cell = fixed_cell(
            &mut store,
            Polarity::Positive,
            "pattern",
            "pattern-rhs",
            CellProvenance::MuMuTilde,
        );
        let pattern_name: gandr_theory_levitation::Name = "w4-pattern".into();
        let target_name: gandr_theory_levitation::Name = "w4-target".into();
        let rules = [
            CircuitRuleCell {
                name: &pattern_name,
                body: &pattern_body,
                cell: Some(pattern_cell),
            },
            CircuitRuleCell {
                name: &target_name,
                body: &target_body,
                cell: Some(target_cell),
            },
        ];
        let completion = complete_circuit_rules(
            store,
            &rules,
            MatchBudget(4_096_usize),
            CompletionBudget::new(0_usize.into(), 64_usize.into(), 64_usize.into()),
            CircuitAdapterBudget(64_usize),
        );
        let pair = completion
            .matches
            .iter()
            .find(|matched| {
                matched.pattern.as_ref() == "w4-pattern" && matched.target.as_ref() == "w4-target"
            })
            .expect("the ordered pair has a completion record");
        assert_eq!(2_usize, pair.admitted.0);
        assert_eq!(2_usize, pair.overlap_count);
        assert!(
            pair.embeddings
                .iter()
                .all(|embedding| embedding.decline.is_none()),
            "both placements render without collapsing their circuit origins"
        );
        let mut seam_pairs = pair
            .embeddings
            .iter()
            .map(|embedding| {
                let input = embedding
                    .origin
                    .seam
                    .inputs
                    .image_of(Wire(2))
                    .expect("the origin retains the second component input");
                let output = embedding
                    .origin
                    .seam
                    .outputs
                    .image_of(Wire(3))
                    .expect("the origin retains the second component output");
                (input, output)
            })
            .collect::<alloc::vec::Vec<_>>();
        seam_pairs.sort_unstable_by_key(|&(input, output)| (input.0, output.0));
        assert_eq!(
            alloc::vec![(Wire(2), Wire(3)), (Wire(4), Wire(5))],
            seam_pairs,
            "the full seam distinguishes the two target placements"
        );
    }

    #[test]
    fn the_description_route_preserves_a_bounded_completion_decline()
    {
        let descs = declared_descs(TestText(ADMITTED_COMPLETION_ROUTE));
        let cells = elaborate_desc_cells_with_completion_budget(
            &descs,
            MatchBudget(4_096_usize),
            CompletionBudget::new(0_usize.into(), 64_usize.into(), 64_usize.into()),
        );
        assert_eq!(descs.len(), cells.circuit_completions.len());
        let (index, completion) = cells
            .circuit_completions
            .iter()
            .enumerate()
            .find(|&(_, completion)| !completion.matches.is_empty())
            .expect("the sign description has structured matcher evidence");
        let CompletionOutcome::Declined {
            reason, pending, ..
        } = completion.outcome.clone()
        else {
            panic!("the zero budget is observable as a structured decline")
        };
        assert_eq!(DeclineReason::StepBudget, reason);
        assert!(!pending.is_empty(), "the supplied overlap remains pending");
        assert_eq!(
            cells.circuit_completions[index].outcome.store(),
            completion.outcome.store()
        );
    }

    fn unary_body<N>(
        head: N,
        input: N,
        output: N,
    ) -> CircuitBody
    where
        N: Into<gandr_theory_levitation::Name>,
    {
        let head = head.into();
        let input = input.into();
        let output = output.into();
        CircuitBody::new(
            [CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Op(head),
                [FreeTerm::var(input)],
                output.clone(),
            ))],
            output,
        )
    }

    fn parallel_unary_body<N>(
        head: N,
        first: (N, N),
        second: (N, N),
    ) -> CircuitBody
    where
        N: Into<gandr_theory_levitation::Name>,
    {
        let head = head.into();
        let first_input = first.0.into();
        let first_output = first.1.into();
        let second_input = second.0.into();
        let second_output = second.1.into();
        CircuitBody::new(
            [
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op(head.clone()),
                    [FreeTerm::var(first_input)],
                    first_output,
                )),
                CircuitNode::Frame(CircuitFrame::new(
                    FrameHead::Op(head),
                    [FreeTerm::var(second_input)],
                    second_output.clone(),
                )),
            ],
            second_output,
        )
    }

    fn two_component_body(extra: MatchDecision) -> CircuitBody
    {
        let mut nodes = alloc::vec![
            CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Op("left".into()),
                [FreeTerm::var("x")],
                "y",
            )),
            CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Op("right".into()),
                [FreeTerm::var("z")],
                "w",
            )),
        ];
        if bool::from(extra) {
            nodes.push(CircuitNode::Frame(CircuitFrame::new(
                FrameHead::Op("right".into()),
                [FreeTerm::var("q")],
                "r",
            )));
        }
        CircuitBody::new(nodes, "w")
    }

    fn fixed_cell<N>(
        store: &mut CellStore,
        polarity: Polarity,
        lhs: N,
        rhs: N,
        provenance: CellProvenance,
    ) -> gandr_theory_cell_complexes::CellId
    where
        N: Into<Box<str>>,
    {
        store.insert(Cell::new(
            CmdPat::cut(polarity, ProdPat::ctor(lhs, []), ConsPat::Top),
            CmdPat::cut(polarity, ProdPat::ctor(rhs, []), ConsPat::Top),
            Orientation::PolarityDerived,
            provenance,
        ))
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

    /// A circuit rule of a description, by the name its block wrote.
    fn named_circuit<'desc>(
        desc: &'desc gandr_theory_levitation::SignDesc,
        name: RuleName<'_>,
    ) -> &'desc gandr_theory_levitation::CircuitRule
    {
        desc.circuits
            .iter()
            .find(|rule| rule.name.as_ref() == name.0)
            .unwrap_or_else(|| panic!("the block declares `{}`", name.0))
    }

    /// A circuit rule's declared name, as a fixture names it.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct RuleName<'fixture>(&'fixture str);
}
