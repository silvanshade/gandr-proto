//! Entity-attribute MVP acceptance tests (proposal-attributes.md §§2–5).
//!
//! Drives the `@[…]` marker end-to-end through the pipeline
//! ([`lower_source_total`] → [`attributes::run`] / [`report`]): the registry
//! resolution, the payload checker path, the four attribute diagnostics, the
//! inert side table, hash-neutrality, and the `Report.attributes` projection.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Attribute-layer acceptance tests.
#[cfg(test)]
mod tests
{
    use gandr_core_checker::term::syntax::Value;
    use gandr_surface_engine::attributes::AttrFinding;
    use gandr_surface_engine::attributes::AttrPass;
    use gandr_surface_engine::attributes::AttrTier;
    use gandr_surface_engine::attributes::run;
    use gandr_surface_engine::diag::AttributeProblem;
    use gandr_surface_engine::diag::DiagnosticDetail;
    use gandr_surface_engine::diag::report;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::prelude_ctx;

    use crate::common::TestText;
    #[test]
    fn clean_doc_and_deprecated_resolve_as_inert()
    {
        let source = "@[doc(\"squares\")]\n@[deprecated(#{ since = \"0.2\", note = \"use \
                      square_checked\" })]\ndef square(x: Integer) -> F Integer { ret x }\n";
        let outcome = pass(source);
        assert!(
            outcome.findings.is_empty(),
            "a well-formed pair reports no findings: {:?}",
            outcome.findings
        );
        assert_eq!(2, outcome.resolved.len(), "both attributes resolve");
        for attr in &outcome.resolved {
            assert_eq!(0, attr.node, "both annotate item 0");
            assert_eq!(AttrTier::Inert, attr.tier, "the MVP is inert-only");
        }
        assert_eq!("doc", outcome.resolved[0].schema);
        assert!(matches!(outcome.resolved[0].payload, Value::Str(ref text) if text == "squares"));
        assert_eq!("deprecated", outcome.resolved[1].schema);
        assert!(matches!(outcome.resolved[1].payload, Value::Record(_)));
    }
    #[test]
    fn unknown_attribute_suggests_a_registry_name()
    {
        let outcome = pass("@[dco(\"x\")]\ndef f = 42;\n");
        assert!(
            outcome.resolved.is_empty(),
            "an unknown attribute is not projected"
        );
        assert_eq!(1, outcome.findings.len());
        let AttrFinding::Unknown {
            ref name,
            ref suggestion,
            ..
        } = outcome.findings[0]
        else {
            panic!("expected an Unknown finding, got {:?}", outcome.findings);
        };
        assert_eq!("dco", name);
        assert_eq!(
            Some("doc"),
            suggestion.as_deref(),
            "the did-you-mean over the registry"
        );
    }
    #[test]
    fn ill_typed_payload_is_the_ordinary_type_error_but_still_projected()
    {
        // `42` checks against `doc`'s `String` schema and fails: the ordinary
        // record/scalar type error, surfaced at the payload (§3.1).
        let outcome = pass("@[doc(42)]\ndef f = 42;\n");
        assert_eq!(1, outcome.findings.len());
        assert!(matches!(
            outcome.findings[0],
            AttrFinding::IllTypedPayload { ref name, .. } if name == "doc"
        ));
        assert_eq!(
            1,
            outcome.resolved.len(),
            "the attachment is still projected (§5)"
        );
    }
    #[test]
    fn single_valued_duplicate_is_reported_once()
    {
        let outcome = pass("@[doc(\"a\")]\n@[doc(\"b\")]\ndef f = 42;\n");
        assert_eq!(
            1,
            outcome.resolved.len(),
            "the first occurrence is projected"
        );
        assert!(matches!(outcome.resolved[0].payload, Value::Str(ref text) if text == "a"));
        assert_eq!(1, outcome.findings.len());
        assert!(matches!(
            outcome.findings[0],
            AttrFinding::Duplicate { ref name, .. } if name == "doc"
        ));
    }
    #[test]
    fn bare_marker_missing_its_payload_is_reported()
    {
        let outcome = pass("@[doc]\ndef f = 42;\n");
        assert!(outcome.resolved.is_empty());
        assert!(matches!(
            outcome.findings.as_slice(),
            [AttrFinding::MissingPayload { name, .. }] if name == "doc"
        ));
    }
    #[test]
    fn computation_payload_is_rejected_as_non_value()
    {
        // `1 + 2` lowers to a computation (a forced operator application), not a
        // value — attribute purity is locality (§3.3).
        let outcome = pass("@[doc(1 + 2)]\ndef f = 42;\n");
        assert!(outcome.resolved.is_empty());
        assert!(matches!(
            outcome.findings.as_slice(),
            [AttrFinding::NonValuePayload { name, .. }] if name == "doc"
        ));
    }
    #[test]
    fn manifest_schemas_resolve_as_inert_coordinates()
    {
        // Manifest fields checked on a top-level `def`, the unit-root stand-in
        // until the module root lands. A repeatable `dependency` plus
        // single-valued `package` /
        // `toolchain` / `license` / `authors` (proposal-packages.md §7.3, §7.6
        // MVP column).
        let source = "@[package(#{ name = \"acme/parser\", version = \"1.4.0\" })]\n\
                      @[license(\"MIT\")]\n\
                      @[authors([\"ada\", \"grace\"])]\n\
                      @[dependency(#{ name = \"acme/lexer\", alias = \"lexer\", constraint = \
                      \"^2.1\" })]\n\
                      @[dependency(#{ name = \"acme/ast\", alias = \"ast\", constraint = \"^1.0\" \
                      })]\n\
                      @[toolchain(#{ gandr = \">=0.9\" })]\n\
                      def parser_root = ();\n";
        let outcome = pass(source);
        assert!(
            outcome.findings.is_empty(),
            "the manifest is well-formed: {:?}",
            outcome.findings
        );
        // Six blocks resolve; `dependency` is repeatable, so both are kept.
        assert_eq!(6, outcome.resolved.len());
        let dependencies = outcome
            .resolved
            .iter()
            .filter(|attr| attr.schema == "dependency")
            .count();
        assert_eq!(
            2, dependencies,
            "a repeatable schema keeps every occurrence"
        );
        for attr in &outcome.resolved {
            assert_eq!(AttrTier::Inert, attr.tier, "every manifest field is inert");
        }
    }
    #[test]
    fn manifest_single_valued_package_is_not_repeatable()
    {
        let outcome = pass(
            "@[package(#{ name = \"a\", version = \"1\" })]\n@[package(#{ name = \"b\", version = \
             \"2\" })]\ndef root = ();\n",
        );
        assert_eq!(1, outcome.resolved.len(), "package is single-valued");
        assert!(matches!(
            outcome.findings.as_slice(),
            [AttrFinding::Duplicate { name, .. }] if name == "package"
        ));
    }

    /// Runs the attribute pass over `source` against the prelude.
    fn pass<'text>(source: impl Into<TestText<'text>>) -> AttrPass
    {
        let source = source.into().0;
        let lowered = lower_source_total(source.into()).expect("total lowering never fails");
        run(&lowered, &prelude_ctx())
    }

    #[test]
    fn inert_attributes_are_hash_neutral()
    {
        // The inert default (§4.2): an inert attribute never
        // perturbs the entity's core-IR term. The item term is the
        // hash-neutrality proxy — the content-address is computed over it.
        let plain = lower_source_total("def f = 42;\n".into()).unwrap();
        let annotated = lower_source_total("@[doc(\"x\")]\ndef f = 42;\n".into()).unwrap();
        assert_eq!(
            plain.items[0].term, annotated.items[0].term,
            "an inert attribute leaves the item's core-IR term identical"
        );
        assert!(plain.attributes.is_empty());
        assert_eq!(
            1,
            annotated.attributes.len(),
            "the attribute lives only in the side table"
        );
    }

    #[test]
    fn report_projects_attributes_and_folds_diagnostics()
    {
        let source = "@[doc(\"ok\")]\n@[bogus]\ndef f = 42;\n";
        let lowered = lower_source_total(source.into()).unwrap();
        let built = report(&lowered, &prelude_ctx());
        // The well-formed `doc` is projected into `Report.attributes`.
        assert_eq!(1, built.attributes.len());
        assert_eq!(0, built.attributes[0].node);
        assert_eq!("doc", built.attributes[0].schema);
        assert_eq!(AttrTier::Inert, built.attributes[0].tier);
        // The unknown `bogus` folds into the ordinary diagnostics stream.
        let unknown = built.diagnostics.iter().find(|diagnostic| {
            matches!(
                diagnostic.detail,
                DiagnosticDetail::Attribute {
                    ref name,
                    problem: AttributeProblem::Unknown { .. },
                } if name == "bogus"
            )
        });
        assert!(
            unknown.is_some(),
            "the unknown attribute is a diagnostic: {:?}",
            built.diagnostics
        );
    }

    #[cfg(feature = "codecs")]
    #[test]
    fn report_json_is_deterministic_for_inspected_attributes()
    {
        let source = "@[doc(\"a\")]\n@[license(\"MIT\")]\ndef f = 42;\n";
        let lowered = lower_source_total(source.into()).unwrap();
        let first = report(&lowered, &prelude_ctx())
            .to_json()
            .expect("first report serializes");
        let second = report(&lowered, &prelude_ctx())
            .to_json()
            .expect("second report serializes");
        assert_eq!(first, second);
        let parsed: serde_json::Value = serde_json::from_str(&first).expect("valid report JSON");
        assert_eq!(
            parsed["attributes"],
            serde_json::json!([
                {
                    "node": 0_usize,
                    "schema": "doc",
                    "payload": "Str(\"a\")",
                    "tier": "inert",
                    "span": {"start": 0_usize, "end": 11_usize}
                },
                {
                    "node": 0_usize,
                    "schema": "license",
                    "payload": "Str(\"MIT\")",
                    "tier": "inert",
                    "span": {"start": 12_usize, "end": 29_usize}
                }
            ])
        );
    }
}
