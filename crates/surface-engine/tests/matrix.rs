//! The pattern-matrix compiler and the join point: what an arm set the tag
//! walk cannot place now compiles to, and what it still declines.
//!
//! Three shapes reach this compiler, and all three are one problem — an arm
//! body reached from more than one branch. A top-level catch-all, an
//! or-pattern with distinguishable alternatives, and two arms at one
//! constructor head each need a branch to jump somewhere another branch also
//! jumps, and the core has no join-point former because a join point is a
//! compilation device rather than a term anyone writes. The compiler binds the
//! shared body as a thunk and each branch forces it.
//!
//! What the suite pins is behaviour rather than shape: which scrutinee reaches
//! which body, that an unfinished test still halts the arms it shadows, and
//! that a column outside the switched set is still declined by name.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Executable pattern-matrix tests.
#[cfg(test)]
mod tests
{
    use gandr_core_term::outcome::Blame;
    use gandr_core_term::outcome::Eval;
    use gandr_core_term::syntax::Value;
    use gandr_runtime_effects::ShellOutcome;
    use gandr_surface_engine::run::RunError;
    use gandr_surface_engine::run::run_source;

    use crate::common::TestInteger;
    use crate::common::TestText;

    /// A three-constructor enumeration: enough tags that a catch-all covers
    /// more than one, which is what forces a join point.
    const COLOR: &str = "data Color : Type { Red : Color; Green : Color; Blue : Color; }";

    /// A payload-bearing pair of datatypes: the fixture for two arms at one
    /// head, which are told apart only by their arguments.
    const NESTED: &str = "data Inner : Type { No : Inner; Yes : (n : Integer) --> Inner; } \
                          data Outer : Type { None : Outer; Some : (i : Inner) --> Outer; }";

    // --- The catch-all --------------------------------------------------------

    /// A catch-all takes every tag no earlier arm named, and the earlier arm
    /// still wins its own tag.
    #[test]
    fn a_catch_all_reaches_every_tag_no_arm_named()
    {
        assert_eq!(
            Some(TestInteger(0)),
            returned_int(catch_all(TestText("Red"))),
            "the named arm keeps its tag"
        );
        assert_eq!(
            Some(TestInteger(1)),
            returned_int(catch_all(TestText("Green"))),
            "a tag no arm named reaches the catch-all"
        );
        assert_eq!(
            Some(TestInteger(1)),
            returned_int(catch_all(TestText("Blue"))),
            "and so does the other one — one body, two branches"
        );
    }

    /// A catch-all may bind, and the binder names the scrutinee.
    #[test]
    fn a_catch_all_binder_names_the_scrutinee()
    {
        let program = alloc::format!(
            "{COLOR} def which(c : Color) -> F Color {{ case c {{ Red => ret Green, x => ret x }} \
             }} which(Blue)"
        );
        let outcome = run_source(program.as_str()).expect("the program must run");
        assert!(
            matches!(returned_constructor(&outcome).as_deref(), Some("Blue")),
            "the catch-all's binder is the value the match was given"
        );
    }

    /// A catch-all beside no declared constructor is **not** the matrix
    /// compiler's, and is still declined.
    ///
    /// An arm set naming no constructor family reveals none to be matched
    /// against, and reading its lowercase arms as binders would silently
    /// accept a program another eliminator must refuse — `case p { here => … }`
    /// on an identity type is the witness, and it is a K derivation.
    #[test]
    fn a_catch_all_with_no_declared_constructor_stays_declined()
    {
        let program = "def twice(n : Integer) -> F Integer { case n { x => ret (x + x) } } \
                       twice(21)";
        assert!(
            matches!(run_source(program), Err(RunError::Lower(_))),
            "no arm names a declared constructor, so the declared-data eliminator is not selected"
        );
    }

    // --- The or-pattern -------------------------------------------------------

    /// An or-pattern of constructor heads settles every tag it names, through
    /// one body.
    #[test]
    fn an_or_pattern_of_heads_settles_both_tags()
    {
        assert_eq!(
            Some(TestInteger(7)),
            returned_int(or_heads(TestText("Red"))),
            "the first alternative reaches the shared body"
        );
        assert_eq!(
            Some(TestInteger(7)),
            returned_int(or_heads(TestText("Green"))),
            "and so does the second"
        );
        assert_eq!(
            Some(TestInteger(9)),
            returned_int(or_heads(TestText("Blue"))),
            "the tag no alternative names keeps its own arm"
        );
    }

    // --- Two arms at one head -------------------------------------------------

    /// Two arms sharing a constructor head are told apart by their arguments,
    /// which is a test one branch deep rather than a second branch.
    #[test]
    fn two_arms_at_one_head_are_told_apart_by_their_arguments()
    {
        assert_eq!(
            Some(TestInteger(6)),
            returned_int(nested(
                TestText("def i = (Yes(5) : Inner); def m = (Some(i) : Outer);"),
                TestText("m")
            )),
            "the first arm at `Some` matches the nested `Yes`"
        );
        assert_eq!(
            Some(TestInteger(8)),
            returned_int(nested(
                TestText("def m = (Some(No) : Outer);"),
                TestText("m")
            )),
            "the second arm at `Some` is now reachable"
        );
        assert_eq!(
            Some(TestInteger(1)),
            returned_int(nested(TestText(""), TestText("None"))),
            "and the other tag is unaffected"
        );
    }

    // --- What is still declined, and what still halts --------------------------

    /// A column whose head domain the eliminator does not switch on is a
    /// missing *test*, not a missing join, and stays declined by name.
    #[test]
    fn a_literal_column_is_still_declined_by_name()
    {
        let program = "def pick(n : Integer) -> F Integer { case n { 0 => ret 1, _ => ret 2 } } \
                       pick(0)";
        assert!(
            matches!(run_source(program), Err(RunError::Lower(_))),
            "an integer column needs an equality test this eliminator does not have"
        );
    }

    /// An unfinished test still halts every constructor it shadows, catch-all
    /// or no catch-all.
    #[test]
    fn a_hole_still_shadows_the_catch_all_written_after_it()
    {
        let program = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, ? => ret 1, _ => \
             ret 2 }} }} pick(Green)"
        );
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(program.as_str())),
            "filling the hole could take the match, so the catch-all is not reached"
        );
        let settled = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, ? => ret 1, _ => \
             ret 2 }} }} pick(Red)"
        );
        assert_eq!(
            Some(TestInteger(0)),
            returned_int(run_source(settled.as_str())),
            "an arm before the hole is settled and unaffected"
        );
    }

    /// A tag no arm reaches is still a hole rather than a fall-through.
    #[test]
    fn a_tag_no_arm_reaches_is_still_a_hole()
    {
        let program = alloc::format!(
            "{NESTED} def pick(m : Outer) -> F Integer {{ case m {{ Some(Yes(k)) => ret (k + 1), \
             Some(No) => ret 8 }} }} pick(None)"
        );
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(program.as_str())),
            "`None` has no arm and reaches no body"
        );
    }

    // --- Helpers --------------------------------------------------------------

    /// The catch-all fixture, run against `scrutinee`.
    fn catch_all(scrutinee: TestText<'_>) -> Result<ShellOutcome, RunError>
    {
        let scrutinee = scrutinee.0;
        let program = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, _ => ret 1 }} }} \
             pick({scrutinee})"
        );
        run_source(program.as_str())
    }

    /// The or-pattern fixture, run against `scrutinee`.
    fn or_heads(scrutinee: TestText<'_>) -> Result<ShellOutcome, RunError>
    {
        let scrutinee = scrutinee.0;
        let program = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red | Green => ret 7, Blue => \
             ret 9 }} }} pick({scrutinee})"
        );
        run_source(program.as_str())
    }

    /// The two-arms-at-one-head fixture, run against `scrutinee`.
    fn nested(
        prelude: TestText<'_>,
        scrutinee: TestText<'_>,
    ) -> Result<ShellOutcome, RunError>
    {
        let prelude = prelude.0;
        let scrutinee = scrutinee.0;
        let program = alloc::format!(
            "{NESTED} {prelude} def pick(m : Outer) -> F Integer {{ case m {{ Some(Yes(k)) => ret \
             (k + 1), Some(No) => ret 8, None => ret 1 }} }} pick({scrutinee})"
        );
        run_source(program.as_str())
    }

    /// The integer a run returned, or `None` for any other outcome.
    fn returned_int(outcome: Result<ShellOutcome, RunError>) -> Option<TestInteger>
    {
        let outcome = outcome.ok()?;
        let returned = outcome.returned()?;
        match *returned {
            | Value::Int(value) => Some(TestInteger::from(value)),
            | _ => None,
        }
    }

    /// The constructor a run returned, by tag position in its datatype.
    fn returned_constructor(outcome: &ShellOutcome) -> Option<alloc::string::String>
    {
        let returned = outcome.returned()?;
        match *returned {
            | Value::Ctor { tag, .. } => Some(match tag {
                | 0 => "Red".into(),
                | 1 => "Green".into(),
                | _ => "Blue".into(),
            }),
            | _ => None,
        }
    }

    /// The blame a run reached, or `None` for any other outcome.
    fn blamed(outcome: Result<ShellOutcome, RunError>) -> Option<Blame>
    {
        let outcome = outcome.ok()?;
        match outcome {
            | ShellOutcome::Completed(Eval::Blame(blame)) => Some(blame),
            | ShellOutcome::Completed(_)
            | ShellOutcome::Exited { .. }
            | ShellOutcome::HostFailed(_) => None,
        }
    }
}
