//! Type-lowering coverage (the `lower::types` module): every covered type
//! former, each sort-directed position, and the out-of-fragment / malformed
//! error table, driven through the public `lower_source` (strict) and
//! `lower_source_total` (total) entries.
//!
//! The vehicle is the `def name : T; def name = ret 1;` signature: the
//! signature type lowers in sort-free position and is recorded verbatim as the
//! item's `ascription`, so an exact `Ty` assertion pins the lowering. Sort-
//! directed positions (value-sorted parameters, computation-sorted `F`/`->`
//! members) drive the `lower_value_ty` / `lower_comp_ty` mismatch arms.

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
    use gandr_core_checker::boundary::GradeBound;
    use gandr_core_checker::grade::Grade;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::types::CompType;
    use gandr_core_checker::types::Ty;
    use gandr_core_checker::types::ValueType;
    use gandr_surface_engine::lower::LowerError;
    use gandr_surface_engine::lower::Lowered;
    use gandr_surface_engine::lower::lower_source;
    use gandr_surface_engine::lower::lower_source_total;

    use crate::common::TestText;

    fn atom<'text>(name: impl Into<TestText<'text>>) -> ValueType
    {
        let name = name.into().0;
        ValueType::atom(name)
    }
    #[test]
    fn out_of_fragment_types_are_unsupported_strictly_and_holed_totally()
    {
        // Each of these is out of the covered fragment: strict rejects with a
        // structured error, total resolves the signature to the sort-free
        // `Value(Unknown)` fallback.
        let unsupported: &[&str] = &[
            "Integer | Unit",        // union
            "Integer /\\ Unit",      // intersection
            "List(Integer, Unit)",   // `List` is unary — wrong arity
            "Path(Integer, x)",      // `Path` is ternary — wrong arity
            "Path(Integer, (x), y)", // compound endpoint is not rung-1
            "Pair(Integer, Unit)",   // non-`List`/`Path` type constructor
        ];
        for input in unsupported {
            assert!(
                matches!(
                    strict_signature_ascription(input),
                    Err(LowerError::Unsupported { .. })
                ),
                "strict lowering of `{input}` must be Unsupported, got {:?}",
                strict_signature_ascription(input)
            );
            assert_eq!(
                Ty::Value(ValueType::Unknown),
                total_signature_ascription(input),
                "total lowering of `{input}` must hole to the gradual top"
            );
        }
    }
    #[test]
    fn grade_annotations_parse_or_fail_by_shape()
    {
        // A non-`u64` numeral (float) and a grade *variable* (identifier) are both
        // invalid grades; strict surfaces `InvalidGrade`, total holes the thunk.
        for bad in ["U[1.5] (F Unit)", "U[r] (F Unit)"] {
            assert!(
                matches!(
                    strict_signature_ascription(bad),
                    Err(LowerError::InvalidGrade { .. })
                ),
                "`{bad}` must be an InvalidGrade in strict mode, got {:?}",
                strict_signature_ascription(bad)
            );
            assert_eq!(
                Ty::Value(ValueType::Unknown),
                total_signature_ascription(bad),
                "`{bad}` holes to the gradual top in total mode"
            );
        }

        // A valid finite grade is preserved verbatim.
        assert_eq!(
            strict_signature_ascription("U[0] (F Unit)"),
            Ok(Ty::Value(ValueType::thunk(
                Grade::fin(GradeBound::from(0)),
                CompType::returner(ValueType::Unit),
            ))),
            "a `0` grade is the finite zero grade"
        );
    }
    #[test]
    fn computation_position_rejects_a_value_type()
    {
        // `F`'s argument is value-sorted, so `F (F Unit)` is a value/computation
        // mismatch; the `->` result and `&` members are computation-sorted, so a
        // value there is the dual mismatch.
        for (input, expected_sort) in [
            ("F (F Unit)", "a value type"),
            ("Unit -> Integer", "a computation type"),
            ("Unit & Unit", "a computation type"),
        ] {
            let result = strict_signature_ascription(input);
            assert!(
                matches!(
                    result,
                    Err(LowerError::TypeSortMismatch { expected, .. }) if expected == expected_sort
                ),
                "`{input}` must be a `{expected_sort}` sort mismatch, got {result:?}"
            );
            // Total mode never errs; each stays computation-sorted with its
            // offending member holed to the position's `Unknown`.
            assert!(
                matches!(total_signature_ascription(input), Ty::Comp(_)),
                "`{input}` stays computation-sorted in total mode"
            );
        }
    }

    /// Lowers `def f : {ty}; def f = ret 1;` strictly, returning the recorded
    /// ascription (or the first `LowerError`).
    fn strict_signature_ascription<'text>(ty: impl Into<TestText<'text>>)
    -> Result<Ty, LowerError>
    {
        let ty = ty.into().0;
        let source = format!("def f : {ty}; def f = ret 1;");
        let lowered = lower_source((&source).into())?;
        Ok(lowered
            .items
            .into_iter()
            .find_map(|item| {
                (item.name.as_deref() == Some("f"))
                    .then_some(item.ascription)
                    .flatten()
            })
            .expect("the matched `def f` item must carry the signature ascription"))
    }
    #[test]
    fn every_covered_type_former_lowers_to_its_core_type()
    {
        let cases: Vec<(&str, Ty)> = vec![
            // Primitives, including the three structurally-meaningful keywords.
            ("Integer", Ty::Value(atom("Integer"))),
            ("Unit", Ty::Value(ValueType::Unit)),
            ("Boolean", Ty::Value(boolean())),
            ("Unknown", Ty::Value(ValueType::Unknown)),
            // A `type_identifier` (non-primitive) is an opaque atom.
            ("Widget", Ty::Value(atom("Widget"))),
            // Parenthesized type re-enters the dispatch unchanged.
            ("(Integer)", Ty::Value(atom("Integer"))),
            // The returner `F A` and thunk `U[r] B` computation/value formers.
            ("F Integer", Ty::Comp(CompType::returner(atom("Integer")))),
            // Until the surface gains a standalone computation-sort hole,
            // `F Unknown` is the explicit gradual computation ascription. A
            // known payload still lowers to the pure empty-row returner above.
            ("F Unknown", Ty::Comp(CompType::Unknown)),
            (
                "U[1] (F Unit)",
                Ty::Value(ValueType::thunk(
                    Grade::fin(GradeBound::from(1)),
                    CompType::returner(ValueType::Unit),
                )),
            ),
            // Absent grade defaults to `ω`; `ω` spelled explicitly agrees.
            (
                "U (F Unit)",
                Ty::Value(ValueType::thunk(
                    Grade::OMEGA,
                    CompType::returner(ValueType::Unit),
                )),
            ),
            (
                "U[ω] (F Unit)",
                Ty::Value(ValueType::thunk(
                    Grade::OMEGA,
                    CompType::returner(ValueType::Unit),
                )),
            ),
            // The function type is computation-sorted, value argument.
            (
                "Integer -> F Unit",
                Ty::Comp(CompType::arrow(
                    atom("Integer"),
                    CompType::returner(ValueType::Unit),
                )),
            ),
            // Eager product / tagged sum / lazy product chains parse cleanly
            // without explicit grouping. Their lowered core shape follows the
            // lowerer's right-nesting contract, and the mixed-band cases below
            // pin the existing precedence ladder across product, sum, set, and arrow.
            (
                "Integer * Unit",
                Ty::Value(ValueType::prod(atom("Integer"), ValueType::Unit)),
            ),
            (
                "Integer * Unit * Boolean",
                Ty::Value(ValueType::prod(
                    atom("Integer"),
                    ValueType::prod(ValueType::Unit, boolean()),
                )),
            ),
            (
                "Integer * Unit * Boolean * String",
                Ty::Value(ValueType::prod(
                    atom("Integer"),
                    ValueType::prod(ValueType::Unit, ValueType::prod(boolean(), atom("String"))),
                )),
            ),
            (
                "Integer + Unit",
                Ty::Value(ValueType::sum(atom("Integer"), ValueType::Unit)),
            ),
            (
                "Integer + Unit + Boolean",
                Ty::Value(ValueType::sum(
                    atom("Integer"),
                    ValueType::sum(ValueType::Unit, boolean()),
                )),
            ),
            (
                "Integer + Unit + Boolean + String",
                Ty::Value(ValueType::sum(
                    atom("Integer"),
                    ValueType::sum(ValueType::Unit, ValueType::sum(boolean(), atom("String"))),
                )),
            ),
            (
                "Integer * Unit + Boolean * String",
                Ty::Value(ValueType::sum(
                    ValueType::prod(atom("Integer"), ValueType::Unit),
                    ValueType::prod(boolean(), atom("String")),
                )),
            ),
            // Lazy product `&` is computation-sorted; members must be computations.
            (
                "F Integer & F Unit",
                Ty::Comp(CompType::with(
                    CompType::returner(atom("Integer")),
                    CompType::returner(ValueType::Unit),
                )),
            ),
            (
                "F Integer & F Unit & F Boolean",
                Ty::Comp(CompType::with(
                    CompType::returner(atom("Integer")),
                    CompType::with(
                        CompType::returner(ValueType::Unit),
                        CompType::returner(boolean()),
                    ),
                )),
            ),
            (
                "F Integer & F Unit & F Boolean & F String",
                Ty::Comp(CompType::with(
                    CompType::returner(atom("Integer")),
                    CompType::with(
                        CompType::returner(ValueType::Unit),
                        CompType::with(
                            CompType::returner(boolean()),
                            CompType::returner(atom("String")),
                        ),
                    ),
                )),
            ),
            (
                "Unit -> F Integer & F Boolean",
                Ty::Comp(CompType::arrow(
                    ValueType::Unit,
                    CompType::with(
                        CompType::returner(atom("Integer")),
                        CompType::returner(boolean()),
                    ),
                )),
            ),
            (
                "F Integer & F Unit & F Boolean & F String",
                Ty::Comp(CompType::with(
                    CompType::returner(atom("Integer")),
                    CompType::with(
                        CompType::returner(ValueType::Unit),
                        CompType::with(
                            CompType::returner(boolean()),
                            CompType::returner(atom("String")),
                        ),
                    ),
                )),
            ),
            (
                "Unit -> F Integer & F Boolean",
                Ty::Comp(CompType::arrow(
                    ValueType::Unit,
                    CompType::with(
                        CompType::returner(atom("Integer")),
                        CompType::returner(boolean()),
                    ),
                )),
            ),
            // The one Stage-1 type constructor: `List(A)`.
            ("List(Integer)", Ty::Value(ValueType::list(atom("Integer")))),
            // The rung-1 identity former admits literal and variable endpoints
            // as terms in type position.
            (
                "Path(Integer, 1, x)",
                Ty::Value(ValueType::path(
                    atom("Integer"),
                    Value::int(1),
                    Value::var("x"),
                )),
            ),
            // Record type into a canonical (label-sorted) map.
            (
                "#{ y : Unit, x : Integer }",
                Ty::Value(ValueType::record([
                    ("x".to_owned(), atom("Integer")),
                    ("y".to_owned(), ValueType::Unit),
                ])),
            ),
            // The empty record type is the top of the record order.
            (
                "#{}",
                Ty::Value(ValueType::record(core::iter::empty::<(String, ValueType)>())),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(
                strict_signature_ascription(input),
                Ok(expected.clone()),
                "strict lowering of `{input}`"
            );
            // Every well-formed type lowers identically in total mode.
            assert_eq!(
                total_signature_ascription(input),
                expected,
                "total lowering of `{input}`"
            );
        }
    }

    fn boolean() -> ValueType
    {
        ValueType::sum(ValueType::Unit, ValueType::Unit)
    }
    #[test]
    fn path_endpoint_overflow_is_an_integer_literal_error()
    {
        // Path endpoints are terms in type position. A numeral endpoint must be
        // an `i64` source integer, so an overflowing literal is not silently
        // reinterpreted as a variable or holed before strict mode reports the
        // exact literal error.
        let too_large = "Path(Integer, 9223372036854775808, x)";
        let strict = strict_signature_ascription(too_large);
        assert!(
            matches!(
                strict,
                Err(LowerError::InvalidIntegerLiteral { ref text, .. })
                    if text == "9223372036854775808"
            ),
            "`{too_large}` must surface the overflowing endpoint literal, got {strict:?}"
        );
        assert_eq!(
            Ty::Value(ValueType::Unknown),
            total_signature_ascription(too_large),
            "total mode holes the bad endpoint to the signature fallback"
        );
    }
    #[test]
    fn duplicate_record_label_is_rejected_strictly_and_kept_totally()
    {
        // A duplicate label is a strict error; total keeps the last field type.
        assert!(
            matches!(
                strict_signature_ascription("#{ x : Integer, x : Unit }"),
                Err(LowerError::Unsupported { .. })
            ),
            "a duplicate record label must be Unsupported in strict mode"
        );
        assert_eq!(
            total_signature_ascription("#{ x : Integer, x : Unit }"),
            Ty::Value(ValueType::record([("x".to_owned(), ValueType::Unit)])),
            "total mode keeps the last field type for a duplicate label"
        );
    }

    /// Lowers the same signature totally, returning the recorded ascription.
    fn total_signature_ascription<'text>(ty: impl Into<TestText<'text>>) -> Ty
    {
        let ty = ty.into().0;
        let source = format!("def f : {ty}; def f = ret 1;");
        let lowered: Lowered =
            lower_source_total((&source).into()).expect("total lowering never errs");
        lowered
            .items
            .into_iter()
            .find_map(|item| {
                (item.name.as_deref() == Some("f"))
                    .then_some(item.ascription)
                    .flatten()
            })
            .expect("the matched `def f` item must carry the signature ascription")
    }

    #[test]
    fn value_position_rejects_a_computation_type()
    {
        // A function *parameter* is a value-sorted position; a computation type
        // there is a sort mismatch (strict) or a `Value(Unknown)` hole (total).
        let strict = lower_source("def g(x: F Integer) { ret x }".into());
        assert!(
            matches!(
                strict,
                Err(LowerError::TypeSortMismatch {
                    expected: "a value type",
                    ..
                })
            ),
            "a computation-typed parameter must be a value/computation sort mismatch, got {strict:?}"
        );
        // Total mode lowers the whole program without error (the bad parameter
        // annotation holes to `Unknown`).
        let total = lower_source_total("def g(x: F Integer) { ret x }".into());
        assert!(
            total.is_ok(),
            "total lowering absorbs the parameter sort mismatch, got {total:?}"
        );
    }

    #[test]
    fn a_syntactically_broken_type_is_a_syntax_error_strictly()
    {
        // A malformed signature type (an `ERROR`/`MISSING` type region) is a
        // `Syntax` error in strict mode; total lowers it without failing.
        let strict = lower_source("def f : U[] (F Unit); def f = ret 1;".into());
        assert!(
            matches!(
                strict,
                Err(LowerError::Syntax { .. } | LowerError::MalformedNode { .. })
            ),
            "a broken grade region must be a Syntax/MalformedNode error, got {strict:?}"
        );
        assert!(
            lower_source_total("def f : U[] (F Unit); def f = ret 1;".into()).is_ok(),
            "total lowering absorbs the broken type region"
        );
    }
}
