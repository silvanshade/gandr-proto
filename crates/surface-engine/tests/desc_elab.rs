//! End-to-end levitation stage-0 elaboration tests (the predecessor design
//! record).
//!
//! These drive the whole path — parse a `data` / `codata` block, elaborate it
//! to a `gandr_theory_levitation::DataDesc`, then run the generic consumers
//! over the result — on the real surface corpus fixtures, plus the pathological
//! declines.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// End-to-end stage-0 elaboration tests.
#[cfg(test)]
mod tests
{
    use gandr_core_checker::boundary::ConstructorTag;
    use gandr_surface_engine::desc_elab::elaborate_data_descs;
    use gandr_theory_levitation::DeclPolarity;
    use gandr_theory_levitation::DescValue;
    use gandr_theory_levitation::Payload;
    use gandr_theory_levitation::generic_eq;
    use gandr_theory_levitation::serialize_desc;

    /// The local `data` engine fixture.
    const DATA_DECLARATIONS: &str = include_str!("fixtures/surface/data-declarations.gandr");
    /// The local `data` operation/rule-member engine fixture.
    const OPERATION_MEMBERS: &str = include_str!("fixtures/surface/data-operation-members.gandr");
    /// The local `codata` engine fixture.
    const CODATA_DECLARATIONS: &str = include_str!("fixtures/surface/codata-declarations.gandr");
    /// The local declined-declaration engine fixture.
    const DECLINED_DECLARATION: &str =
        include_str!("fixtures/surface/desc-declined-metadata.gandr");

    #[test]
    fn the_data_corpus_fixture_elaborates_to_its_descriptions()
    {
        let elab = elaborate_data_descs(DATA_DECLARATIONS);
        assert!(
            elab.diagnostics.is_empty(),
            "the model `data` fixture elaborates cleanly: {:?}",
            elab.diagnostics
        );
        let rendered: Vec<String> = elab
            .descs
            .iter()
            .map(|desc| serialize_desc(desc).to_string())
            .collect();
        assert_eq!(
            rendered,
            vec![
                "data Color { Red = 1, Green = 1, Blue = 1 }".to_owned(),
                "data Maybe(a) { None = 1, Some = a }".to_owned(),
                "data Tree(a) { Leaf = 1, Node = (var × (a × var)) }".to_owned(),
                "data Empty {}".to_owned(),
            ],
            "each declared datatype elaborates to its tagged description"
        );
    }

    #[test]
    fn a_declared_datatype_drives_the_generic_consumer_end_to_end()
    {
        // Parse → elaborate → consume, on a real corpus declaration.
        let elab = elaborate_data_descs("data Maybe(a) { None, Some(x: a) }");
        assert_eq!(1, elab.descs.len(), "one datatype");
        let maybe = &elab.descs[0];
        assert_eq!(
            "data Maybe(a) { None = 1, Some = a }",
            serialize_desc(maybe).as_ref(),
            "the description inspects as expected"
        );

        // The generic structural equality (driven by the elaborated
        // description) agrees on `Some` payloads and separates constructors.
        let some_a = DescValue::new(ConstructorTag::from(1), Payload::Leaf(Box::from(&b"a"[..])));
        let some_a2 = DescValue::new(ConstructorTag::from(1), Payload::Leaf(Box::from(&b"a"[..])));
        let some_b = DescValue::new(ConstructorTag::from(1), Payload::Leaf(Box::from(&b"b"[..])));
        let none = DescValue::new(ConstructorTag::from(0), Payload::Unit);
        assert!(
            bool::from(generic_eq(maybe, &some_a, &some_a2)),
            "Some(a) == Some(a)"
        );
        assert!(
            !bool::from(generic_eq(maybe, &some_a, &some_b)),
            "Some(a) ≠ Some(b)"
        );
        assert!(
            !bool::from(generic_eq(maybe, &some_a, &none)),
            "Some(a) ≠ None"
        );
    }
    #[test]
    fn description_wrapper_boundaries_stay_observable_to_pipeline_consumers()
    {
        let elab = elaborate_data_descs("data Maybe(a) { None, Some(x: a) }");
        let maybe = &elab.descs[0];
        let rendered = serialize_desc(maybe);
        let none = DescValue::new(ConstructorTag::from(0), Payload::Unit);

        assert_eq!(
            "Maybe",
            maybe.id.name.as_ref(),
            "nominal ids expose the declared name"
        );
        assert_eq!(
            "data Maybe(a) { None = 1, Some = a }",
            rendered.as_ref(),
            "serialized descriptions remain comparable without unwrapping carriers"
        );
        assert!(
            bool::from(generic_eq(maybe, &none, &none)),
            "generic equality exposes its named boolean result to consumers"
        );
    }

    #[test]
    fn operation_and_rule_members_elaborate_with_arities_and_faces()
    {
        let elab = elaborate_data_descs(OPERATION_MEMBERS);
        // `NatOp` carries an `op add`, `NatRule` a `rule`, `NatDiv` a multi-out
        // `op divmod`.
        let nat_op = elab
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatOp")
            .expect("NatOp elaborated");
        assert_eq!(1, nat_op.ops.len(), "NatOp has one operation");
        assert_eq!("add", nat_op.ops[0].name.as_ref(), "the operation is `add`");
        assert_eq!(2, nat_op.ops[0].arity.inputs.len(), "add reads two inputs");

        let nat_rule = elab
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatRule")
            .expect("NatRule elaborated");
        assert_eq!(1, nat_rule.cells.len(), "NatRule has one 2-cell face");

        let nat_div = elab
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatDiv")
            .expect("NatDiv elaborated");
        assert_eq!(
            2,
            nat_div.ops[0].arity.outputs.len(),
            "divmod has a two-port result tuple (the Π-layer)"
        );

        // Well-formedness runs over the real fixture: `NatRule`'s `rule id(Zero)
        // ==> Zero` mentions `id`, which the datatype's signature does not
        // declare — the host check surfaces exactly that decline (proposal §8
        // `desc-illformed-cell`).
        assert!(
            elab.diagnostics
                .iter()
                .any(|diag| diag.message.contains("`id`")
                    && diag.message.contains("not in the datatype's signature")),
            "the out-of-signature rule symbol is declined: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn the_ruled_face_arrow_elaborates_and_the_retired_one_declines_with_its_respelling()
    {
        // The block-form ruling makes `==>` the rewrite-face former at every
        // position and retires `~>`. The two spellings are exercised over one
        // otherwise-identical declaration, so what separates them is the arrow
        // and nothing else.
        let ruled = elaborate_data_descs(
            "data NatFace { Zero, op id(x: NatFace) -> NatFace, rule id(Zero) ==> Zero }",
        );
        let face = ruled
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatFace")
            .expect("NatFace elaborated");
        assert_eq!(1, face.cells.len(), "the ruled `==>` face becomes a cell");
        assert!(
            ruled.diagnostics.is_empty(),
            "the ruled face earns no decline: {:?}",
            ruled.diagnostics
        );

        // A stale `~>` is not a parse failure — the grammar still admits it in
        // the arrow slot precisely so the decline can be an elaboration
        // diagnostic that names the respelling rather than a repair that names
        // a token.
        let retired = elaborate_data_descs(
            "data NatFace { Zero, op id(x: NatFace) -> NatFace, rule id(Zero) ~> Zero }",
        );
        let stale = retired
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatFace")
            .expect("NatFace still elaborates around the declined member");
        assert!(
            stale.cells.is_empty(),
            "the retired face is declined rather than admitted as a silent synonym"
        );
        assert!(
            retired.diagnostics.iter().any(|diagnostic| {
                diagnostic.message.contains("`~>`")
                    && diagnostic.message.contains("retired")
                    && diagnostic.message.contains("`==>`")
            }),
            "the decline points at the respelling: {:?}",
            retired.diagnostics
        );
        // The diagnostic is located at the arrow, not at the whole member: the
        // respelling is a one-token edit and the span is what an editor acts on.
        let located = retired
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message.contains("`~>`"))
            .expect("the migration decline is located");
        let source = "data NatFace { Zero, op id(x: NatFace) -> NatFace, rule id(Zero) ~> Zero }";
        let start = usize::from(located.span.start);
        let end = usize::from(located.span.end);
        assert_eq!(
            Some("~>"),
            source.get(start .. end),
            "the decline's span covers the retired arrow"
        );
    }

    #[test]
    fn codata_declarations_elaborate_with_codata_polarity()
    {
        let elab = elaborate_data_descs(CODATA_DECLARATIONS);
        let stream = elab
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "Stream")
            .expect("Stream elaborated");
        assert_eq!(
            DeclPolarity::Codata,
            stream.polarity,
            "a codata block elaborates with Codata polarity"
        );
        // `head: a` is a leaf observation; `tail: Stream(a)` is the recursive
        // one.
        assert_eq!(
            "codata Stream(a) { head = a, tail = var }",
            serialize_desc(stream).as_ref(),
            "observations elaborate as constructor-shaped entries; `tail` recurses"
        );
    }

    #[test]
    fn a_higher_order_field_is_declined_at_elaboration()
    {
        // A function-typed field is outside the first-order fragment (proposal
        // §8's `desc-higher-order-field`, pinning V2).
        let elab = elaborate_data_descs("data Cell(a) { Mk(get: a -> a) }");
        assert!(
            elab.diagnostics
                .iter()
                .any(|diag| diag.message.contains("first-order code fragment")),
            "a function-typed field is declined: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn the_declined_metadata_golden_is_declined()
    {
        // The VDC declined-declaration golden: a datatype that declares the
        // reserved derived `variance` metadata as an attribute is declined.
        let elab = elaborate_data_descs(DECLINED_DECLARATION);
        assert!(
            elab.diagnostics
                .iter()
                .any(|diag| diag.message.contains("derived metadata")
                    && diag.message.contains("variance")),
            "declaring derived variance metadata is declined: {:?}",
            elab.diagnostics
        );
    }
}
