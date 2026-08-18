//! A `case` arm whose pattern is a typed hole: what it compiles to, what it
//! runs to, and what filling it does.
//!
//! The suite is organized around the one property the arm reader had to be
//! rebuilt to state: **an unfinished test is neither satisfied nor refuted, so
//! the arms it shadows stop being reachable.** Every test below either pins a
//! branch the hole stops, a branch it leaves alone, or the fact that filling
//! it produces exactly the program the filled pattern would have produced on
//! its own.
//!
//! Two outcomes look alike and are not: an unfilled hole and a missing arm
//! both reach `Blame::Hole`, because both are the same runtime event — a
//! gradual hole met an elimination. What separates them is the note the hole
//! carries and, far more usefully, **which scrutinees reach it**: a missing
//! arm is reached only by the constructor nobody wrote a branch for, while a
//! hole is reached by every constructor the arms after it would have taken.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Executable pattern-hole tests.
#[cfg(test)]
mod tests
{
    use alloc::string::String;
    use alloc::vec::Vec;

    use gandr_core_term::outcome::Blame;
    use gandr_core_term::outcome::Eval;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Value;
    use gandr_runtime_effects::ShellOutcome;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::origin::TermRef;
    use gandr_surface_engine::origin::resolve;
    use gandr_surface_engine::run::RunError;
    use gandr_surface_engine::run::run_source;
    use gandr_surface_engine::session::ItemOutcome;
    use gandr_surface_engine::session::Session;

    use crate::common::TestInteger;
    use crate::common::TestText;

    /// A three-constructor enumeration: the fixture for "which constructors
    /// does the hole shadow".
    const COLOR: &str = "data Color : Type { Red : Color; Green : Color; Blue : Color; }";

    /// A payload-bearing pair of datatypes, the fixture for questions that have
    /// to look inside a constructor.
    const NESTED: &str = "data Inner : Type { No : Inner; Yes : (n : Integer) --> Inner; } \
                          data Outer : Type { None : Outer; Some : (i : Inner) --> Outer; }";

    // --- What the hole stops --------------------------------------------------

    /// The constructor an arm *after* the hole names no longer reaches that
    /// arm.
    ///
    /// This is the whole bead in one assertion. Before the arm reader compiled
    /// patterns, the hole arm failed its shape check and was dropped, so `Blue`
    /// fell through to the `Blue` arm and the match ran as though the
    /// programmer had written nothing at all. It now stops there, because
    /// whether the hole matches `Blue` is exactly what has not been decided.
    #[test]
    fn a_hole_shadows_the_arms_written_after_it()
    {
        assert_eq!(
            Some(Blame::Hole),
            blamed(color_pick(TestText("Blue"))),
            "the arm after the hole must not be reached: filling the hole could take the match"
        );
    }

    /// A constructor no arm mentions is stopped too, and for the same reason.
    #[test]
    fn a_hole_shadows_the_constructor_no_arm_names()
    {
        assert_eq!(
            Some(Blame::Hole),
            blamed(color_pick(TestText("Green"))),
            "`Green` has no arm of its own, and the hole is what stands between it and one"
        );
    }

    /// An arm written *before* the hole is unaffected: it is settled, so
    /// nothing about the hole bears on it.
    ///
    /// The precedence rule is what makes the hole safe to compile at all. If an
    /// indeterminate arm stopped the arms before it as well, adding a `?` to
    /// the end of a finished match would break it.
    #[test]
    fn an_arm_before_the_hole_still_wins()
    {
        assert_eq!(
            Some(TestInteger(0)),
            returned_int(color_pick(TestText("Red"))),
            "`Red` is settled by its own arm, which precedes the hole"
        );
    }

    /// **The hole never guesses a branch and never widens into a catch-all.**
    ///
    /// The arm's own body is `ret 1`; if an unfilled hole were read as a
    /// wildcard, `Green` and `Blue` would both produce `1`. Neither does.
    #[test]
    fn an_unfilled_hole_never_runs_its_own_arm()
    {
        for scrutinee in ["Green", "Blue"] {
            assert_eq!(
                None,
                returned_int(color_pick(TestText(scrutinee))),
                "an unfilled hole must produce no value at all, and `{scrutinee}` produced one"
            );
        }
    }

    // --- What filling it does -------------------------------------------------

    /// **Filling the hole changes exactly the branches it shadowed, and
    /// nothing else.**
    ///
    /// The differential is between the unfilled program and the filled one at
    /// every constructor: `Red` is settled before the hole and answers the
    /// same either way, while `Green` and `Blue` move from stopped to
    /// answered. A test comparing the filled program against itself would pass
    /// on an implementation that never compiled the hole at all, which is why
    /// the unfilled side is the one carrying the claim.
    #[test]
    fn filling_the_hole_changes_exactly_the_branches_it_shadowed()
    {
        assert_eq!(
            Some(TestInteger(0)),
            returned_int(color_pick(TestText("Red"))),
            "`Red` is settled before the hole"
        );
        assert_eq!(
            Some(TestInteger(0)),
            returned_int(run_color(TestText("Green"), TestText("Red"))),
            "and the fill leaves it exactly where it was"
        );
        for (scrutinee, expected) in [("Green", 1), ("Blue", 2)] {
            assert_eq!(
                Some(Blame::Hole),
                blamed(color_pick(TestText(scrutinee))),
                "`{scrutinee}` is stopped by the unfilled hole"
            );
            assert_eq!(
                Some(TestInteger(expected)),
                returned_int(run_color(TestText("Green"), TestText(scrutinee))),
                "and the fill settles it"
            );
        }
    }

    /// Filling the hole with a pattern that does **not** cover a constructor
    /// leaves that constructor where it was: uncovered, not stuck.
    ///
    /// The two ways a match can fail to produce a value stay distinguishable
    /// through the fill, which is what lets an author read the result as
    /// progress rather than as the same failure under a new name.
    #[test]
    fn filling_the_hole_moves_a_stuck_branch_to_uncovered_not_to_taken()
    {
        assert_eq!(
            Some(TestInteger(1)),
            returned_int(run_color(TestText("Green"), TestText("Green"))),
            "the filled pattern settles the constructor it names"
        );
        let source = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, Green => ret 1 }} \
             }}"
        );
        assert!(
            notes(TestText(source.as_str()))
                .iter()
                .any(|note| note.contains("MissingCaseArm")),
            "and `Blue`, which the filled pattern does not name, is a missing arm rather than a \
             stuck one"
        );
    }

    // --- Filling through a live session --------------------------------------

    /// **The fill is an edit to a session that already holds checkpoints, and
    /// what it produces is what a source carrying the pattern from the start
    /// produces.**
    ///
    /// This is the acceptance the whole fill-and-resume half rests on, and it
    /// has to be a differential against a *separately built* session, not
    /// against the same text twice: an implementation that recompiled the file
    /// from scratch on every submission would pass a self-comparison and fail
    /// this one only if the two disagree, so the assertion that carries weight
    /// is the before/after change inside the edited session.
    ///
    /// `Session::submit` types every append through the incremental
    /// checkpoint engine, so the second definition arrives at a session whose
    /// earlier items are already checkpointed. Which items that engine re-types
    /// and which adopt is asserted against the engine directly in the
    /// `incremental` suite's `hole_fill` module; what is asserted here is the
    /// half that suite cannot see, which is what the resumed program then
    /// evaluates to.
    #[test]
    fn filling_a_hole_in_a_live_session_evaluates_like_a_fresh_source()
    {
        let mut edited = Session::new();
        let holed = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, ? => ret 1, Blue \
             => ret 2 }} }}"
        );
        let opening = edited
            .submit(holed.as_str())
            .expect("lowering must not fail");
        assert!(
            matches!(
                opening.outcomes.last(),
                Some(&ItemOutcome::Definition { bound: true, .. })
            ),
            "an unfinished arm does not stop the definition entering scope: {:?}",
            opening.outcomes
        );

        // Before the fill, the shadowed constructor stops.
        assert_eq!(
            Some(Eval::Blame(Blame::Hole)),
            evaluated(&mut edited, TestText("pick(Blue)")),
            "the unfilled hole stops `Blue` inside the session too"
        );

        // The fill is an ordinary next submission against the retained state.
        let fill = "def pick(c : Color) -> F Integer { case c { Red => ret 0, Green => ret 1, Blue \
                    => ret 2 } }";
        let filled = edited.submit(fill).expect("lowering must not fail");
        assert!(
            matches!(
                filled.outcomes.last(),
                Some(&ItemOutcome::Definition { bound: true, .. })
            ),
            "the fill rebinds the definition: {:?}",
            filled.outcomes
        );

        // A session that carried the finished pattern from its first line.
        let mut fresh = Session::new();
        let whole = alloc::format!("{COLOR} {fill}");
        drop(
            fresh
                .submit(whole.as_str())
                .expect("lowering must not fail"),
        );

        for scrutinee in ["Red", "Green", "Blue"] {
            let call = alloc::format!("pick({scrutinee})");
            let after = evaluated(&mut edited, TestText(call.as_str()));
            assert_eq!(
                evaluated(&mut fresh, TestText(call.as_str())),
                after,
                "the filled session must agree with the fresh one at `{scrutinee}`"
            );
            assert!(
                matches!(after, Some(Eval::Value(_))),
                "and `{scrutinee}` must now answer rather than stop: {after:?}"
            );
        }
    }

    /// **A fill that covers one constructor turns another from unfinished to
    /// uncovered**, which is the transition an author is working toward.
    ///
    /// Before the fill the match reports one unfinished test and no missing
    /// arm: `Green` and `Blue` are both held by the hole rather than uncovered.
    /// After it, `Green` is settled and `Blue` is a coverage gap — a different
    /// obligation, discharged by writing an arm rather than by finishing a
    /// pattern. Both halts look identical at run time, so this is the assertion
    /// that keeps them distinguishable across the edit.
    #[test]
    fn a_fill_turns_a_held_constructor_into_an_uncovered_one()
    {
        let holed = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, ? => ret 1 }} }}"
        );
        let before = notes(TestText(holed.as_str()));
        assert!(
            before
                .iter()
                .any(|note| note.contains("IndeterminatePattern")),
            "before the fill the match reports an unfinished test: {before:?}"
        );
        assert!(
            !before.iter().any(|note| note.contains("MissingCaseArm")),
            "and reports no missing arm, because the hole stands in front of both: {before:?}"
        );

        let filled = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, Green => ret 1 }} \
             }}"
        );
        let after = notes(TestText(filled.as_str()));
        assert!(
            after.iter().any(|note| note.contains("MissingCaseArm")),
            "after it `Blue` is an uncovered constructor: {after:?}"
        );
        assert!(
            !after
                .iter()
                .any(|note| note.contains("IndeterminatePattern")),
            "and no unfinished test remains: {after:?}"
        );
    }

    // --- Where the hole sits --------------------------------------------------

    /// A hole inside a constructor pattern stops only the tag that pattern
    /// names.
    ///
    /// `Just(?)` cannot decide a `Just`, but it refutes a `Nothing` outright —
    /// the head is written, and only the payload is unfinished. Reading the
    /// whole arm as indeterminate would stop a branch the programmer had
    /// already settled.
    #[test]
    fn a_hole_in_a_payload_stops_only_its_own_constructor()
    {
        let source = "data Maybe : Type { Nothing : Maybe; Just : (x : Integer) --> Maybe; } \
                      def pick(m : Maybe) -> F Integer { case m { Just(?) => ret 0, Nothing => ret \
                      1 } }";
        assert_eq!(
            Some(TestInteger(1)),
            returned_int(run_source(
                alloc::format!("{source} pick(Nothing)").as_str()
            )),
            "`Nothing` is refuted by the written head, so its own arm runs"
        );
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(
                alloc::format!("{source} pick(Just(3))").as_str()
            )),
            "`Just` reaches the unfinished payload test and stops"
        );
    }

    /// A hole nested two constructors deep is reached only when both written
    /// tests above it pass.
    ///
    /// The determinate tests run first and can still refute, which is the
    /// ordering the compiler has to get right: a hole beside a test that
    /// refutes must not report indeterminacy, because the arm is refuted
    /// whatever the hole comes to be.
    #[test]
    fn a_doubly_nested_hole_is_reached_only_through_its_written_tests()
    {
        let program = alloc::format!(
            "{NESTED} def pick(m : Outer) -> F Integer {{ case m {{ Some(Yes(?inner)) => ret 0, \
             None => ret 1 }} }}"
        );
        assert_eq!(
            Some(TestInteger(1)),
            returned_int(run_source(alloc::format!("{program} pick(None)").as_str())),
            "the outer head refutes, so the written arm runs"
        );
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(
                alloc::format!("{program} def v = (Some((Yes(3) : Inner)) : Outer); pick(v)")
                    .as_str()
            )),
            "both written tests pass, and the unfinished one stops the match"
        );
    }

    /// **The nesting the hole forced is real machinery, not a hole-shaped
    /// exception**: a nested constructor pattern with no hole in it compiles
    /// and runs.
    ///
    /// Before this seam a nested pattern failed the arm reader's shape check
    /// exactly as a hole did. The reader that stopped requiring the shape is
    /// the reader that had to place sub-patterns, so the two arrive together.
    #[test]
    fn a_nested_constructor_pattern_binds_and_runs()
    {
        let program = alloc::format!(
            "{NESTED} def pick(m : Outer) -> F Integer {{ case m {{ Some(Yes(k)) => ret (k + 1), \
             None => ret 1 }} }} def v = (Some((Yes(41) : Inner)) : Outer); pick(v)"
        );
        assert_eq!(
            Some(TestInteger(42)),
            returned_int(run_source(program.as_str())),
            "the nested binder reaches the arm body bound to the payload it names"
        );
    }

    /// An as-binder over a hole names a value the arm never reaches, and the
    /// binder does not make the arm decidable.
    #[test]
    fn an_as_binder_over_a_hole_does_not_settle_it()
    {
        let program = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 7, ? as whole => ret \
             1 }} }} pick(Green)"
        );
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(program.as_str())),
            "naming the scrutinee decides nothing about whether the pattern tests it"
        );
    }

    /// An or-pattern is indeterminate exactly when every alternative is; one
    /// decidable alternative is a test the scrutinee can settle.
    #[test]
    fn an_or_pattern_of_holes_is_indeterminate()
    {
        let program = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 7, ? | ?other => ret \
             1 }} }} pick(Blue)"
        );
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(program.as_str())),
            "neither alternative can be settled, so the arm settles nothing"
        );
    }

    // --- Where the hole leaves no eliminator to choose ------------------------

    /// An arm set whose first arm settles nothing settles nothing for any
    /// scrutinee, so the whole `case` is one hole — and no constructor family
    /// has to be read out of arms that name none.
    #[test]
    fn a_case_whose_first_arm_is_indeterminate_is_one_hole()
    {
        let program = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ ? => ret 0 }} }} pick(Red)"
        );
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(program.as_str())),
            "an all-hole arm set reveals no eliminator and decides nothing"
        );
        let named = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ ?arm => ret 0 }} }} pick(Red)"
        );
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(named.as_str())),
            "and naming the hole changes nothing about what it decides"
        );
    }

    /// The sum eliminator takes the same rule, through the same helper: an arm
    /// that settles nothing leaves every later slot stuck.
    #[test]
    fn the_sum_eliminator_stops_at_an_indeterminate_arm()
    {
        let taken = "def pick() -> F Integer { case (Inl(41) : Integer + String) { Inl(x) => ret \
                     (x + 1), ? => ret 0 } } pick()";
        assert_eq!(
            Some(TestInteger(42)),
            returned_int(run_source(taken)),
            "the settled injection takes its own arm"
        );
        let stopped = "def pick() -> F Integer { case (Inr(\"s\") : Integer + String) { Inl(x) => \
                       ret (x + 1), ? => ret 0 } } pick()";
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(stopped)),
            "the other injection reaches the unfinished arm and stops"
        );
    }

    /// The list eliminator likewise.
    #[test]
    fn the_list_eliminator_stops_at_an_indeterminate_arm()
    {
        let taken = "def ys = ([] : List(Integer)); def pick() -> F Integer { case ys { Nil => ret \
                     0, ? => ret 9 } } pick()";
        assert_eq!(
            Some(TestInteger(0)),
            returned_int(run_source(taken)),
            "the empty list takes its own arm"
        );
        let stopped = "def xs = ([1, 2] : List(Integer)); def pick() -> F Integer { case xs { Nil \
                       => ret 0, ? => ret 9 } } pick()";
        assert_eq!(
            Some(Blame::Hole),
            blamed(run_source(stopped)),
            "a cons reaches the unfinished arm and stops"
        );
    }

    // --- The failure boundaries, named ---------------------------------------

    /// **A pattern hole is a legitimate term in strict mode, and an uncompiled
    /// pattern form is not.** The two must not be confused: one is the
    /// programmer's unfinished work and the other is this compiler's boundary.
    #[test]
    fn an_uncompiled_arm_is_declined_and_a_hole_is_not()
    {
        let declined = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, _ => ret 1 }} }} \
             pick(Red)"
        );
        assert!(
            matches!(run_source(declined.as_str()), Err(RunError::Lower(_))),
            "a top-level catch-all needs an arm body two branches can reach, and is declined by \
             name rather than dropped"
        );
        let hole = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, ? => ret 1 }} }} \
             pick(Red)"
        );
        assert!(
            run_source(hole.as_str()).is_ok(),
            "a hole the user wrote lowers in strict mode too — it is a term, not a recovery \
             artifact"
        );
    }

    /// Two arms sharing one constructor head are declined by name rather than
    /// compiled into a branch that silently never runs.
    #[test]
    fn two_arms_at_one_head_are_declined_by_name()
    {
        let program = alloc::format!(
            "{NESTED} def pick(m : Outer) -> F Integer {{ case m {{ Some(Yes(k)) => ret (k + 1), \
             Some(No) => ret 8, None => ret 1 }} }} pick(None)"
        );
        assert!(
            matches!(run_source(program.as_str()), Err(RunError::Lower(_))),
            "the second arm at `Some` is reachable only through a join point the core has no \
             former for"
        );
    }

    // --- What a reader is told ------------------------------------------------

    /// The stuck hole and the missing arm carry different notes, which is the
    /// only place the two stay apart once both have reached `Blame::Hole`.
    #[test]
    fn a_stuck_arm_and_a_missing_arm_carry_different_notes()
    {
        let stuck = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, ? => ret 1 }} }}"
        );
        assert!(
            notes(TestText(stuck.as_str()))
                .iter()
                .any(|note| note.contains("IndeterminatePattern")),
            "an arm the programmer left unfinished says so: {:?}",
            notes(TestText(stuck.as_str()))
        );

        let missing = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0 }} }}"
        );
        let missing_notes = notes(TestText(missing.as_str()));
        assert!(
            missing_notes
                .iter()
                .any(|note| note.contains("MissingCaseArm")),
            "an arm the programmer never wrote says that instead"
        );
        assert!(
            !missing_notes
                .iter()
                .any(|note| note.contains("IndeterminatePattern")),
            "and a match with no hole in it reports no unfinished test"
        );
    }

    /// A named hole's name reaches the note, so a consumer can address the
    /// unfinished test the programmer named.
    #[test]
    fn a_named_pattern_hole_carries_its_name()
    {
        let source = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, ?branch => ret 1 \
             }} }}"
        );
        assert!(
            notes(TestText(source.as_str()))
                .iter()
                .any(|note| note.contains("IndeterminatePattern") && note.contains("branch")),
            "the `?name` identifier is what addresses the hole in the goal stream"
        );
    }

    // --- The path into a compiled arm ----------------------------------------

    /// Every arm body of a declared-data case is addressable by an origin path,
    /// and an index past the last constructor addresses nothing.
    ///
    /// The stepper carried no arm for this eliminator at all, so nothing inside
    /// one could be addressed — which is why the notes above reached no
    /// consumer until it did. The out-of-range half is the same claim from the
    /// other side: the walk stops at the constructor count rather than reading
    /// past it.
    #[test]
    fn every_arm_of_a_declared_data_case_is_addressable()
    {
        let source = alloc::format!(
            "{COLOR} def c = (Red : Color); case c {{ Red => ret 0, Green => ret 1, Blue => ret 2 \
             }}"
        );
        let lowered = lower_source_total(source.as_str().into())
            .expect("total lowering fails only when the parser is unavailable");
        let Some(item) = lowered.items.last()
        else {
            panic!("expected the case item, got none");
        };
        let term = &item.term;
        for tag in 0 .. 3_u32 {
            let arm = tag.checked_add(1).expect("three arms fit");
            let path = alloc::vec![arm];
            assert!(
                matches!(
                    resolve(term, path.as_slice()),
                    Some(TermRef::Comp(&Comp::Ret(_)))
                ),
                "arm at tag {tag} must be addressable as child {arm}"
            );
        }
        let scrutinee = alloc::vec![0_u32];
        assert!(
            matches!(resolve(term, scrutinee.as_slice()), Some(TermRef::Value(_))),
            "the scrutinee is the value child"
        );
        let past_end = alloc::vec![4_u32];
        assert!(
            resolve(term, past_end.as_slice()).is_none(),
            "an index past the last constructor addresses nothing"
        );
        // A node with no child at the asked index addresses nothing either,
        // and the two answers come from different places: the case walks its
        // own arm vector, every other former falls through to the stepper's
        // catch-all. Both are pinned because the case arm was added between
        // them and could have swallowed the second.
        let inner = alloc::vec![1_u32, 1_u32];
        assert!(
            resolve(term, inner.as_slice()).is_none(),
            "the first arm's `ret` has no child at index one"
        );
    }

    // --- Helpers --------------------------------------------------------------

    /// The three-constructor match with a hole in the middle arm, run against
    /// `scrutinee`.
    fn color_pick(scrutinee: TestText<'_>) -> Result<ShellOutcome, RunError>
    {
        let scrutinee = scrutinee.0;
        let program = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, ? => ret 1, Blue \
             => ret 2 }} }} pick({scrutinee})"
        );
        run_source(program.as_str())
    }

    /// The same match with `pattern` written where the hole was.
    fn run_color(
        pattern: TestText<'_>,
        scrutinee: TestText<'_>,
    ) -> Result<ShellOutcome, RunError>
    {
        let pattern = pattern.0;
        let scrutinee = scrutinee.0;
        let program = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0, {pattern} => ret \
             1, Blue => ret 2 }} }} pick({scrutinee})"
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

    /// Submits one expression to `session` and returns its evaluation.
    ///
    /// The session is the resumed path: every append types through the
    /// incremental checkpoint engine, so an expression submitted after a fill
    /// is evaluated against the state that resume validated.
    fn evaluated(
        session: &mut Session,
        source: TestText<'_>,
    ) -> Option<Eval>
    {
        let submission = session.submit(source.0).ok()?;
        match *submission.outcomes.last()? {
            | ItemOutcome::Expression { ref value, .. } => Some(value.clone()),
            | ItemOutcome::Definition { .. }
            | ItemOutcome::TypeError { .. }
            | ItemOutcome::Holey => None,
        }
    }

    /// Every rendered hole note one submission's goal stream carries.
    ///
    /// The goal stream is where a consumer meets a hole, so it is where the
    /// note has to be legible — reading the origin map directly would prove
    /// the note exists somewhere no user surface reaches.
    fn notes(source: TestText<'_>) -> Vec<String>
    {
        let mut session = Session::new();
        let submission = session.submit(source.0).expect("lowering must not fail");
        submission
            .report
            .goals
            .iter()
            .filter_map(|goal| goal.note.clone())
            .collect()
    }
}
