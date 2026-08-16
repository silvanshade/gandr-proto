//! End-to-end levitation stage-0 elaboration tests (the levitation design's
//! description layer).
//!
//! These drive the whole path — parse a `data` / `codata` block, elaborate it
//! to a `gandr_theory_levitation::SignDesc`, then run the generic consumers
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
    use gandr_theory_levitation::Code;
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
                "sign Color { sort Color : Type; }".to_owned(),
                "sign Maybe(a) { sort Maybe : Type; }".to_owned(),
                "sign Tree(a) { sort Tree : Type; }".to_owned(),
                "sign Vec(a) { sort Vec : Type; }".to_owned(),
                "sign Empty { sort Empty : Type; }".to_owned(),
            ],
            "each declared datatype elaborates, read back in the ruled sign normal form — \
             sorts, operations, and rules; constructors have no item-level member spelling"
        );
        // The constructors the render no longer spells live on the
        // descriptions themselves, in declaration (tag) order.
        let ctor_names: Vec<Vec<&str>> = elab
            .descs
            .iter()
            .map(|desc| desc.ctors.iter().map(|ctor| ctor.name.as_ref()).collect())
            .collect();
        assert_eq!(
            ctor_names,
            vec![
                vec!["Red", "Green", "Blue"],
                vec!["None", "Some"],
                vec!["Leaf", "Node"],
                vec!["Nil", "Cons"],
                vec![],
            ],
            "generator members elaborate to constructor descriptors in tag order"
        );
        // The indexed family: its parameter is bound once at the head, and the
        // recursive field reads as a `var` occurrence of the family sort.
        let vec = &elab.descs[3];
        assert_eq!(1, vec.params.len(), "the head binds `a` once");
        assert!(
            bool::from(vec.is_recursive()),
            "`Cons`'s `xs : Vec(a, n)` field is a recursive occurrence"
        );
    }

    #[test]
    fn the_generator_ladder_elaborates_bare_sides_and_telescopes()
    {
        // The three side rungs of one family: the bare result (`Zero : Nat`
        // declares no fields), the bare single-field sort (`Succ : Nat -->
        // Nat`), and the parenthesized binder telescope.
        let elab = elaborate_data_descs(
            "data Nat : Type { Zero : Nat; Succ : Nat --> Nat; Pred : (n : Nat) --> Nat; }",
        );
        assert!(
            elab.diagnostics.is_empty(),
            "the ladder elaborates cleanly: {:?}",
            elab.diagnostics
        );
        let nat = &elab.descs[0];
        assert_eq!(3, nat.ctors.len(), "one constructor per generator");
        assert!(
            matches!(nat.ctors[0].code, Code::Unit),
            "a bare result declares no fields"
        );
        assert!(
            matches!(nat.ctors[1].code, Code::Var(_)),
            "a bare single-field side at the family sort is a recursive occurrence"
        );
        assert!(
            matches!(nat.ctors[2].code, Code::Var(_)),
            "a telescope binder at the family sort is a recursive occurrence"
        );
    }

    #[test]
    fn a_declared_datatype_drives_the_generic_consumer_end_to_end()
    {
        // Parse → elaborate → consume, on a real corpus declaration.
        let elab = elaborate_data_descs(
            "data Maybe(a : Type) : Type { None : Maybe(a); Some : (x : a) --> Maybe(a); }",
        );
        assert!(
            elab.diagnostics.is_empty(),
            "the nested generator block elaborates cleanly: {:?}",
            elab.diagnostics
        );
        assert_eq!(1, elab.descs.len(), "one datatype");
        let maybe = &elab.descs[0];
        assert_eq!(
            "sign Maybe(a) { sort Maybe : Type; }",
            serialize_desc(maybe).as_ref(),
            "the description inspects as its sign normal form"
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
        let elab = elaborate_data_descs(
            "data Maybe(a : Type) : Type { None : Maybe(a); Some : (x : a) --> Maybe(a); }",
        );
        let maybe = &elab.descs[0];
        let rendered = serialize_desc(maybe);
        let none = DescValue::new(ConstructorTag::from(0), Payload::Unit);

        assert_eq!(
            "Maybe",
            maybe.id.name.as_ref(),
            "nominal ids expose the declared name"
        );
        assert_eq!(
            "sign Maybe(a) { sort Maybe : Type; }",
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
        assert_eq!(1, nat_op.opers.len(), "NatOp has one operation");
        assert_eq!(
            "add",
            nat_op.opers[0].name.as_ref(),
            "the operation is `add`"
        );
        assert_eq!(
            2,
            nat_op.opers[0].arity.inputs.len(),
            "add reads two inputs"
        );

        let nat_rule = elab
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatRule")
            .expect("NatRule elaborated");
        assert_eq!(1, nat_rule.rules.len(), "NatRule has one 2-cell face");

        let nat_div = elab
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatDiv")
            .expect("NatDiv elaborated");
        assert_eq!(
            2,
            nat_div.opers[0].arity.outputs.len(),
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
            "data NatFace : Type { Zero : NatFace; oper id(x : NatFace) -> NatFace; rule \
             id(Zero) ==> Zero; }",
        );
        let face = ruled
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatFace")
            .expect("NatFace elaborated");
        assert_eq!(1, face.rules.len(), "the ruled `==>` face becomes a cell");
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
            "data NatFace : Type { Zero : NatFace; oper id(x : NatFace) -> NatFace; rule \
             id(Zero) ~> Zero; }",
        );
        let stale = retired
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatFace")
            .expect("NatFace still elaborates around the declined member");
        assert!(
            stale.rules.is_empty(),
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
        let source = "data NatFace : Type { Zero : NatFace; oper id(x : NatFace) -> NatFace; \
                      rule id(Zero) ~> Zero; }";
        let start = usize::from(located.span.start);
        let end = usize::from(located.span.end);
        assert_eq!(
            Some("~>"),
            source.get(start .. end),
            "the decline's span covers the retired arrow"
        );
    }

    #[test]
    fn the_retired_op_member_lead_declines_with_its_respelling()
    {
        // The signature unification respells the operation member as `oper`
        // (`op` is the operator-fixity declaration only). The retired lead
        // still parses — the retired-`~>` precedent — so a stale program is
        // told what to write rather than silently accepted.
        let retired = elaborate_data_descs(
            "data NatRetired : Type { Zero : NatRetired; op stale(x : NatRetired) -> NatRetired; \
             }",
        );
        let desc = retired
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "NatRetired")
            .expect("NatRetired still elaborates around the declined member");
        assert!(
            desc.opers.is_empty(),
            "the retired member is declined rather than admitted as a silent synonym"
        );
        assert!(
            retired.diagnostics.iter().any(|diagnostic| {
                diagnostic.message.contains("`op`")
                    && diagnostic.message.contains("retired")
                    && diagnostic.message.contains("`oper`")
            }),
            "the decline points at the respelling: {:?}",
            retired.diagnostics
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
        // `head : a` is a leaf observation; `tail : Stream(a)` is the
        // recursive one. Both elaborate as constructor-shaped entries; the
        // sign normal form gives them no member spelling.
        assert_eq!(
            "sign Stream(a) { sort Stream : Type; }",
            serialize_desc(stream).as_ref(),
            "the sign normal form carries the sort set only"
        );
        assert_eq!(
            2,
            stream.ctors.len(),
            "observations elaborate as constructor-shaped entries"
        );
        assert!(
            bool::from(stream.is_recursive()),
            "`tail : Stream(a)` recurses"
        );
    }

    #[test]
    fn a_higher_order_field_is_declined_at_elaboration()
    {
        // A function-typed field is outside the first-order fragment (proposal
        // §8's `desc-higher-order-field`, pinning V2).
        let elab =
            elaborate_data_descs("data Cell(a : Type) : Type { Mk : (get : a -> a) --> Cell(a); }");
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

    #[test]
    fn a_bare_parameter_head_declines_the_whole_declaration_with_its_respelling()
    {
        // The retired `data Maybe(a)` head: the grammar keeps the bare
        // parameter admissible so this decline can name the respelling.
        let elab = elaborate_data_descs("data Maybe(a) { None : Maybe(a); }");
        assert!(
            elab.descs.is_empty(),
            "a retired head elaborates no description: {:?}",
            elab.descs
        );
        assert!(
            elab.diagnostics.iter().any(|diag| {
                diag.message.contains("carries no type")
                    && diag.message.contains("Maybe(a : Type, …) : Type")
            }),
            "the decline names the typed-binder respelling: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn a_missing_head_annotation_declines_the_whole_declaration_with_its_respelling()
    {
        // The retired unannotated head: the index arity is unreadable without
        // the annotation, so no member can be checked and the whole
        // declaration declines.
        let elab = elaborate_data_descs("data Color { Red : Color; }");
        assert!(
            elab.descs.is_empty(),
            "an unannotated head elaborates no description: {:?}",
            elab.descs
        );
        assert!(
            elab.diagnostics.iter().any(|diag| {
                diag.message.contains("no index-arity annotation")
                    && diag.message.contains("`Color(…) : Type { … }`")
            }),
            "the decline names the annotation respelling: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn the_retired_field_tuple_member_declines_with_the_generator_respelling()
    {
        // A valid nested head over retired members: the members decline
        // individually, and the block elaborates around them.
        let elab = elaborate_data_descs("data Maybe(a : Type) : Type { None, Some(x : a) }");
        let maybe = elab
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "Maybe")
            .expect("Maybe still elaborates around the declined members");
        assert!(
            maybe.ctors.is_empty(),
            "retired members are declined rather than admitted"
        );
        let declines = elab
            .diagnostics
            .iter()
            .filter(|diag| {
                diag.message.contains("is retired") && diag.message.contains("--> Result")
            })
            .count();
        assert_eq!(
            2, declines,
            "both retired members decline with the generator respelling: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn an_instantiated_result_head_declines_as_uninferable()
    {
        // Head uniformity, enforced executably: `Bad(Integer)` instantiates
        // the parameter, which no eliminator schema can consume.
        let elab = elaborate_data_descs("data Bad(a : Type) : Type { Mk : Bad(Integer); }");
        let bad = elab
            .descs
            .iter()
            .find(|desc| desc.id.name.as_ref() == "Bad")
            .expect("Bad elaborates around the declined member");
        assert!(
            bad.ctors.is_empty(),
            "the violating generator contributes no constructor"
        );
        assert!(
            elab.diagnostics.iter().any(|diag| {
                diag.message.contains("instantiates parameter `a`")
                    && diag.message.contains("instantiation is uninferable")
            }),
            "an instantiated head declines: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn a_result_head_of_the_wrong_arity_declines()
    {
        let elab = elaborate_data_descs("data Bad : Type { Mk : Bad(0); }");
        assert!(
            elab.diagnostics.iter().any(|diag| {
                diag.message.contains("takes 1 argument(s)")
                    && diag.message.contains("`Bad` takes 0")
            }),
            "a wrong-arity head declines with both counts: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn a_bare_result_head_over_a_parameterized_family_declines()
    {
        let elab = elaborate_data_descs("data Bad(a : Type) : Type { Mk : Bad; }");
        assert!(
            elab.diagnostics
                .iter()
                .any(|diag| diag.message.contains("bare result head takes no arguments")),
            "a bare head over a parameterized family declines: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn a_foreign_result_head_declines()
    {
        let elab = elaborate_data_descs("data Bad : Type { Mk : Other; }");
        assert!(
            elab.diagnostics.iter().any(|diag| {
                diag.message.contains("result head is `Other`")
                    && diag.message.contains("not the family `Bad`")
            }),
            "a foreign head declines: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn a_constructor_led_member_of_a_codata_block_declines()
    {
        let elab = elaborate_data_descs("codata Boxed : Type { Mk : Boxed; }");
        assert!(
            elab.diagnostics
                .iter()
                .any(|diag| diag.message.contains("has no place in a `codata` block")),
            "a constructor-led member declines in a codata block: {:?}",
            elab.diagnostics
        );
    }

    #[test]
    fn a_codata_oper_declines_without_swallowing_observation_and_rule_siblings()
    {
        let source = "codata S : Type { head : Nat; oper tail(s : S) -> S; rule head ==> head; }";
        let elab = elaborate_data_descs(source);
        let desc = elab.descs.first().expect("the codata block elaborated");
        assert_eq!(1, desc.ctors.len(), "the observation sibling remains");
        assert_eq!(1, desc.rules.len(), "the rule sibling remains");
        assert!(
            desc.opers.is_empty(),
            "a codata `oper` never reaches the description"
        );
        assert_eq!(
            1,
            elab.diagnostics.len(),
            "the malformed member region earns exactly one primary diagnostic: {:?}",
            elab.diagnostics
        );
        let decline = elab.diagnostics.first().expect("one localized decline");
        assert!(
            decline.message.contains("`codata` block")
                && decline.message.contains("observations and `rule` members")
                && decline
                    .message
                    .contains("`oper` is `data` / `sign` vocabulary")
                && decline.message.contains("`tail(s : S) : S`"),
            "the decline names the block law and recoverable observation spelling: {}",
            decline.message
        );
        let start = usize::from(decline.span.start);
        let end = usize::from(decline.span.end);
        assert_eq!(
            "oper tail(s : S) -> S",
            source
                .get(start .. end)
                .expect("the decline span is in source"),
            "the decline covers only the offending member"
        );
    }
}
