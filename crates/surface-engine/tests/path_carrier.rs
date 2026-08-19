//! What an identity type does when its carrier is a bare prefix former.
//!
//! `Path(A, x, y)` reads its three arguments positionally, so anything that
//! changes how many arguments the reader sees changes which term is taken for
//! the carrier and which for the endpoints. A **prefix type former written
//! without brackets inside a comma-separated argument list does exactly that**:
//! it does not take its operand, and the operand becomes the next argument.
//!
//! The failure is under-grouping rather than swallowing, and the difference
//! matters for where the repair goes. `U Integer` in a bracketed position is
//! one node; the same spelling in a bare argument list is two sibling
//! arguments, so a three-argument identity type arrives with four and is
//! refused. Nothing is silently mis-read — the refusal is by name — but the
//! spelling the author wrote is unwritable, and the working spelling differs
//! only by a pair of parentheses.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Executable identity-carrier grouping tests.
#[cfg(test)]
mod tests
{
    use alloc::string::String;

    use gandr_surface_engine::lower::lower_source;

    /// Two declared functions, so an endpoint can be a call of either arity.
    const PRELUDE: &str = "def two(x: Integer, y: Integer) -> F Integer { ret x } \
                           def one(x: Integer) -> F Integer { ret x } ";

    /// **The defect.** A bare prefix former as the carrier is refused, because
    /// the reader sees four arguments where an identity type takes three.
    #[test]
    fn a_bare_prefix_carrier_is_refused()
    {
        assert!(
            lowers(TestType("Path(U (Integer -> F Integer), one, one)")).is_err(),
            "`U` does not take its operand in a bare argument list, so the carrier and the first \
             endpoint arrive as separate arguments"
        );
    }

    /// **The working spelling, and it differs by one pair of parentheses.**
    /// Bracketing the whole former restores the three-argument reading, which
    /// is what identifies the defect as grouping rather than as a missing
    /// fragment: the same carrier is expressible, just not as written.
    #[test]
    fn a_bracketed_prefix_carrier_is_read()
    {
        let bracketed = lowers(TestType("Path((U (Integer -> F Integer)), one, one)"));
        assert!(
            bracketed.is_ok(),
            "parenthesising the former groups it with its operand: {bracketed:?}"
        );
        // The two spellings differ by exactly one pair of parentheses, which is
        // what makes the defect a grouping one rather than a fragment one.
    }

    /// **The separating case, which refutes the obvious diagnosis.** Nested
    /// multi-argument endpoints are not the trigger: with a carrier that
    /// groups, an endpoint whose own argument is a two-argument call lowers
    /// to a real identity type.
    ///
    /// This is worth pinning because the endpoint reading is where the defect
    /// was first looked for, and the endpoint reading is correct.
    #[test]
    fn a_nested_multi_argument_endpoint_is_not_the_trigger()
    {
        assert!(
            lowers(TestType(
                "Path(Integer, two(1, two(3, 4)), two(1, two(3, 4)))"
            ))
            .is_ok(),
            "an atom carrier with nested multi-argument endpoints lowers"
        );
        assert!(
            lowers(TestType(
                "Path((U (Integer -> F Integer)), two(1, two(3, 4)), two(1, two(3, 4)))"
            ))
            .is_ok(),
            "and so does a bracketed compound carrier with the same endpoints"
        );
    }

    // --- Helpers --------------------------------------------------------------

    /// A borrowed identity-type spelling at the test-helper boundary.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug)]
    struct TestType<'source>(&'source str);

    /// Whether a definition returning `ty` lowers.
    fn lowers(ty: TestType<'_>) -> Result<(), String>
    {
        let source = alloc::format!(
            "{PRELUDE} def law(p: Integer) -> F({ty}) {{ ret here(p) }}",
            ty = ty.0
        );
        lower_source(source.as_str().into())
            .map(|_lowered| ())
            .map_err(|error| alloc::format!("{error}"))
    }
}
