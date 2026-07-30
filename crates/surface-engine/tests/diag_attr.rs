//! Attribute-diagnostic coverage (`src/diag.rs`): each `AttrFinding` maps to
//! its source-ranged `Diagnostic` through `diag::report` (the attribute pass is
//! folded into the report's diagnostics, proposal-attributes.md §3.2).

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

#[cfg(test)]
mod tests
{
    use gandr_surface_engine::diag::AttributeProblem;
    use gandr_surface_engine::diag::Diagnostic;
    use gandr_surface_engine::diag::DiagnosticDetail;
    use gandr_surface_engine::diag::report;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::prelude_ctx;

    use crate::common::TestDecision;
    use crate::common::TestText;
    #[test]
    fn a_bare_marker_missing_its_payload_is_a_diagnostic()
    {
        let source = "@[doc]\ndef f = 42;\n";
        assert!(
            bool::from(has_attribute_problem(
                source,
                "doc",
                &AttributeProblem::MissingPayload
            )),
            "a payload-requiring attribute used bare is a MissingPayload diagnostic"
        );
    }
    #[test]
    fn a_computation_payload_is_rejected_as_non_value()
    {
        let source = "@[doc(1 + 2)]\ndef f = 42;\n";
        assert!(
            bool::from(has_attribute_problem(
                source,
                "doc",
                &AttributeProblem::NonValuePayload
            )),
            "a computation-valued payload is a NonValuePayload diagnostic"
        );
    }

    /// Whether any diagnostic carries the given attribute problem for `name`.
    fn has_attribute_problem<'source, 'name>(
        source: impl Into<TestText<'source>>,
        attr_name: impl Into<TestText<'name>>,
        problem: &AttributeProblem,
    ) -> TestDecision
    {
        let source = source.into().0;
        let attr_name = attr_name.into().0;
        diagnostics(source)
            .iter()
            .any(|diagnostic| {
                matches!(
                    diagnostic.detail,
                    DiagnosticDetail::Attribute { ref name, problem: ref found }
                        if name == attr_name && found == problem
                )
            })
            .into()
    }
    #[test]
    fn duplicate_single_valued_attribute_is_a_diagnostic()
    {
        let source = "@[doc(\"a\")]\n@[doc(\"b\")]\ndef f = 42;\n";
        assert!(
            bool::from(has_attribute_problem(
                source,
                "doc",
                &AttributeProblem::Duplicate
            )),
            "a repeated single-valued attribute is a Duplicate diagnostic"
        );
        assert!(
            diagnostics(source)
                .iter()
                .any(|diagnostic| diagnostic.message.contains("duplicate attribute `doc`")),
            "the duplicate message names the attribute"
        );
    }
    #[test]
    fn an_ill_typed_payload_becomes_an_ordinary_type_error()
    {
        // `doc` expects a `String`; a `42` payload is the ordinary type-error
        // diagnostic (not an `Attribute` detail).
        let diags = diagnostics("@[doc(42)]\ndef f = 42;\n");
        assert!(
            diags
                .iter()
                .any(|diagnostic| !matches!(diagnostic.detail, DiagnosticDetail::Attribute { .. })),
            "an ill-typed payload surfaces as a plain type-error diagnostic: {diags:?}"
        );
    }
    #[test]
    fn an_unknown_attribute_reports_with_and_without_a_suggestion()
    {
        // A near-miss of a registry name gets a did-you-mean.
        let near = diagnostics("@[dco(\"x\")]\ndef f = 42;\n");
        assert!(
            near.iter()
                .any(|diagnostic| diagnostic.message.contains("did you mean `doc`?")),
            "a close unknown attribute suggests the registry name: {near:?}"
        );
        // A far-off name gets no suggestion.
        let far = diagnostics("@[zzzzzzzz]\ndef f = 42;\n");
        assert!(
            far.iter().any(|diagnostic| {
                diagnostic.message.contains("unknown attribute `zzzzzzzz`")
                    && !diagnostic.message.contains("did you mean")
            }),
            "a distant unknown attribute has no suggestion: {far:?}"
        );
        // Both are the `Unknown` attribute problem.
        assert!(
            far.iter().any(
                |diagnostic| matches!(diagnostic.detail, DiagnosticDetail::Attribute {
                    problem: AttributeProblem::Unknown { .. },
                    ..
                })
            ),
            "an unknown attribute carries the Unknown problem"
        );
    }

    /// The report diagnostics of a totally-lowered source.
    fn diagnostics<'text>(source: impl Into<TestText<'text>>) -> Vec<Diagnostic>
    {
        let source = source.into().0;
        let lowered = lower_source_total(source.into()).expect("total lowering never errs");
        report(&lowered, &prelude_ctx()).diagnostics
    }
}
