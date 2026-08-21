//! How a bare prefix type former groups inside a comma-separated argument list.
//!
//! A prefix type former written without brackets, `U A` or `F A`, must take
//! `A` as its operand wherever it is written. **Inside a comma-separated
//! type-argument list it does not**, and the same text that groups correctly
//! in a plain type position takes a different parse one layer in.
//!
//! The measurements below are what identify the failure as a grouping one
//! rather than a fragment one. Every failing spelling has a working spelling
//! one pair of parentheses away, and the two must lower to the same type.
//!
//! # The two signatures of one failure
//!
//! The failure surfaces differently at the two consumers, and the difference
//! is the argument sorts each consumer declares.
//!
//! A type application parses every argument at the type sort, so a thunked
//! list declines strict as an unsupported `type_application` and degrades in
//! total mode to the gradual `Unknown`.
//!
//! `path_type` is `Path ( type , expression , expression )`. Its second and
//! third slots are expression-sorted, so a carrier that fails to take its
//! operand shifts every argument by one and the shifted arguments land in
//! expression position. That surfaces as a malformed `call_expression`.
//!
//! Two consumers, two error kinds, one grouping failure.
//!
//! # Where the repair goes
//!
//! The grammar or the mold, never lowering. The argument reader faithfully
//! reports what it is given, and re-grouping a childless prefix node beside
//! its operand in the consumer would repair a parse defect at every consumer
//! of a comma-separated argument list.
//!
//! # The gradual unknown is not an acceptable degradation
//!
//! `buildout-standing-02` constructs `Unknown` at the raw-decode boundary only
//! and never at lowering. A declared type that degrades to `Unknown` is
//! consistent with everything, so a law field would be accepted against a type
//! its own source does not state. The no-unknown assertions below are that
//! rule applied at this former.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Executable prefix-former grouping tests.
#[cfg(test)]
mod tests
{
    use alloc::string::String;

    use gandr_core_term::types::CompType;
    use gandr_core_term::types::Ty;
    use gandr_core_term::types::ValueType;
    use gandr_surface_engine::lower::lower_source;
    use gandr_surface_engine::lower::lower_source_total;

    /// Two declared functions, so an endpoint can be a call of either arity.
    const PRELUDE: &str = "def two(x: Integer, y: Integer) -> F Integer { ret x } \
                           def one(x: Integer) -> F Integer { ret x } ";

    // --- gandr-gd4r: the thunked list ----------------------------------------

    /// **The gandr-gd4r acceptance.** A bare thunk former takes its operand
    /// inside a type-argument list, so a thunked list is writable as written.
    ///
    /// The claim is stated as an agreement rather than as a type literal: the
    /// double-parenthesised spelling already lowers to the real thunked list,
    /// so requiring the two spellings to lower to the same type says the bare
    /// one is correct without restating what correct is in a second place.
    #[test]
    fn the_single_and_double_parenthesised_list_spellings_agree()
    {
        let bare = declared(TestType("List(U(F Integer))"));
        let bracketed = declared(TestType("List((U(F Integer)))"));
        assert_eq!(
            bare, bracketed,
            "the two spellings differ by one pair of parentheses and must lower to one type"
        );
    }

    /// **The degradation half of gandr-gd4r.** The bare spelling carries no
    /// gradual `Unknown` in total mode.
    ///
    /// Stated separately from the agreement above because the two can fail
    /// independently: a strict decline that total mode turns into `Unknown` is
    /// the silent failure, and a reader who only sees the agreement test go
    /// green learns nothing about what total mode did.
    #[test]
    fn a_thunked_list_carries_no_gradual_unknown()
    {
        let total = declared_total(TestType("List(U(F Integer))"))
            .expect("total mode lowers every parseable input");
        assert!(
            !bool::from(total.mentions_unknown()),
            "the declared type degraded to the gradual unknown: {total:?}"
        );
    }

    /// **The separating measurement, and it is what shows context decides.**
    /// The same text groups correctly when it is the whole type rather than an
    /// argument in a list.
    ///
    /// This one passes on the unrepaired tree. It is pinned because it is the
    /// fact that rules out the spelling being at fault, which is where the
    /// investigation would otherwise go next.
    #[test]
    fn a_bare_thunk_former_outside_an_argument_list_already_groups()
    {
        assert_eq!(
            declared(TestType("U(F Integer)")),
            declared(TestType("(U(F Integer))")),
            "outside an argument list the bare and bracketed spellings already agree"
        );
    }

    // --- gandr-ly42: the identity type's carrier ------------------------------

    /// **The gandr-ly42 acceptance.** A `Path` whose carrier is written as a
    /// bare prefix former lowers to a real identity type, and its carrier is
    /// the type the source states.
    ///
    /// The expected carrier is derived from a spelling known to lower rather
    /// than written as a literal, so the claim cannot drift away from what the
    /// thunked arrow actually is.
    ///
    /// This test replaces one asserting that the bare carrier is refused. That
    /// assertion pinned the defect as the expected behaviour, so it had to be
    /// replaced rather than re-run.
    #[test]
    fn a_bare_prefix_carrier_lowers_to_the_identity_type()
    {
        let expected_carrier = declared(TestType("(U (Integer -> F Integer))"))
            .expect("a bracketed thunked arrow lowers");
        let path = declared(TestType("Path(U (Integer -> F Integer), one, one)"))
            .expect("a bare prefix carrier lowers to an identity type");
        let Ty::Value(ValueType::Path { ref ty, .. }) = path
        else {
            panic!("the declared type is not an identity type: {path:?}");
        };
        assert_eq!(
            Ty::Value(ValueType::clone(ty)),
            expected_carrier,
            "the carrier is not the thunked arrow the source states"
        );
    }

    /// **The endpoints are held fixed while the carrier varies**, which is what
    /// makes the carrier the measured trigger rather than the assumed one.
    ///
    /// An atom carrier and a bare prefix carrier lower to identity types over
    /// the same two endpoints, so nothing about the endpoints can explain a
    /// difference between them.
    #[test]
    fn only_the_carrier_varies_between_two_identity_types()
    {
        let atom = declared(TestType(
            "Path(Integer, two(1, two(3, 4)), two(1, two(3, 4)))",
        ));
        let bare = declared(TestType(
            "Path(U (Integer -> F Integer), two(1, two(3, 4)), two(1, two(3, 4)))",
        ));
        assert!(atom.is_ok(), "an atom carrier lowers: {atom:?}");
        assert!(bare.is_ok(), "and so must a bare prefix carrier: {bare:?}");
    }

    /// **The working spelling, and it differs by one pair of parentheses.**
    /// Bracketing the whole former restores the reading, which is what
    /// identifies the defect as grouping rather than as a missing fragment.
    #[test]
    fn a_bracketed_prefix_carrier_is_read()
    {
        let bracketed = declared(TestType("Path((U (Integer -> F Integer)), one, one)"));
        assert!(
            bracketed.is_ok(),
            "parenthesising the former groups it with its operand: {bracketed:?}"
        );
    }

    /// **The separating case that refutes the endpoint diagnosis.** Nested
    /// multi-argument endpoints are not the trigger: with a carrier that
    /// groups, an endpoint whose own argument is a two-argument call lowers to
    /// a real identity type.
    ///
    /// Pinned because the endpoint reading is where the defect was first
    /// looked for, and the endpoint reading is correct.
    #[test]
    fn a_nested_multi_argument_endpoint_is_not_the_trigger()
    {
        assert!(
            declared(TestType(
                "Path(Integer, two(1, two(3, 4)), two(1, two(3, 4)))"
            ))
            .is_ok(),
            "an atom carrier with nested multi-argument endpoints lowers"
        );
        assert!(
            declared(TestType(
                "Path((U (Integer -> F Integer)), two(1, two(3, 4)), two(1, two(3, 4)))"
            ))
            .is_ok(),
            "and so does a bracketed compound carrier with the same endpoints"
        );
    }

    // --- Helpers --------------------------------------------------------------

    /// A borrowed type spelling at the test-helper boundary.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct TestType<'source>(&'source str);

    /// Which lowering mode a helper call exercises.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Mode
    {
        /// Fail-fast over the whole file.
        Strict,
        /// Total on parseable input, with per-region holes.
        Total,
    }

    /// The type a definition declares, lowered strictly.
    fn declared(ty: TestType<'_>) -> Result<Ty, String>
    {
        return declared_with(ty, Mode::Strict);
    }

    /// The type a definition declares, lowered in total mode.
    fn declared_total(ty: TestType<'_>) -> Result<Ty, String>
    {
        return declared_with(ty, Mode::Total);
    }

    /// Lower `def law(p: Integer) -> F(<ty>) { ret here(p) }` and recover the
    /// declared payload of the returner.
    ///
    /// The payload rather than the whole ascription, because the surrounding
    /// thunked arrow is the definition sugar and says nothing about the type
    /// under test.
    fn declared_with(
        ty: TestType<'_>,
        mode: Mode,
    ) -> Result<Ty, String>
    {
        let source = alloc::format!(
            "{PRELUDE} def law(p: Integer) -> F({ty}) {{ ret here(p) }}",
            ty = ty.0
        );
        let lowered = match mode {
            | Mode::Strict => lower_source(source.as_str().into()),
            | Mode::Total => lower_source_total(source.as_str().into()),
        }
        .map_err(|error| alloc::format!("{error}"))?;
        let item = lowered
            .items
            .iter()
            .find(|item| item.name.as_deref() == Some("law"))
            .ok_or_else(|| String::from("the law definition did not lower to an item"))?;
        let ascription = item
            .ascription
            .as_ref()
            .ok_or_else(|| String::from("the law definition lowered without an ascription"))?;
        return returner_payload(ascription)
            .ok_or_else(|| alloc::format!("unexpected law ascription shape: {ascription:?}"));
    }

    /// The value type a `def`'s ascription declares as its returner's payload.
    ///
    /// `def f(p: A) -> F(T) { .. }` ascribes `U(A -> F T)`, so the type under
    /// test is three constructors in. Recovering it here keeps every assertion
    /// above about `T` rather than about the definition sugar wrapped round it.
    fn returner_payload(ascription: &Ty) -> Option<Ty>
    {
        let Ty::Value(ValueType::Thunk(_, ref arrow)) = *ascription
        else {
            return None;
        };
        let CompType::Arrow { ref res, .. } = **arrow
        else {
            return None;
        };
        let CompType::F(ref payload, _) = **res
        else {
            return None;
        };
        return Some(Ty::Value(ValueType::clone(payload)));
    }
}
