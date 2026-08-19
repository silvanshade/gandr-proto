//! What a module declaration does when its body could not be read.
//!
//! A module whose subtree needed repair, with no member carrying that repair,
//! was not read: the member walk reports the members it recognized rather than
//! the members the author wrote. The failure this suite exists for is what that
//! used to produce — a declaration binding as the **empty record**, cleanly,
//! with no diagnostic and no goal, and every declaration the unread region
//! covered simply gone from the program.
//!
//! Two facts separate the cases, and the suite asserts both directions: whether
//! the subtree needed repair at all, and whether a member carries it. A module
//! the melder read completely is untouched; a module whose unread region is a
//! member keeps that member's own, more precise report; only a module whose
//! unread region is invisible to the member walk is refused at the declaration.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Executable module-body readability tests.
#[cfg(test)]
mod tests
{
    use alloc::string::String;
    use alloc::vec::Vec;

    use gandr_surface_engine::session::ItemOutcome;
    use gandr_surface_engine::session::Session;

    use crate::common::TestText;

    /// One reported goal's span and note, at the test-helper boundary.
    #[derive(Debug)]
    struct GoalSpan
    {
        /// The goal's start byte.
        start: usize,
        /// The goal's end byte.
        end: usize,
        /// The goal's rendered note, when it carries one.
        note: Option<String>,
    }

    /// **The silent case, and the one this check exists for.** A signature
    /// component the grammar has no production for takes the module's whole
    /// body with it, so no member surfaces at all — and a module with no
    /// members is a perfectly good empty record.
    ///
    /// What made it dangerous is that nothing about the result said so. The
    /// declaration bound, its type was `#{}`, no diagnostic was raised, no goal
    /// was reported, and the declaration written after it was gone from the
    /// program. A reader had no signal of any kind.
    #[test]
    fn an_unread_module_body_is_refused_not_emptied()
    {
        let source = "module M : #{\n  val x : Integer\n} {\n  def x = 1;\n}\n\ndef after = 1;\n";
        let outcomes = outcomes(TestText(source));
        assert!(
            matches!(outcomes.as_slice(), [ItemOutcome::Holey]),
            "an unread body is refused rather than read as empty, got {outcomes:?}"
        );
        let spans = goal_spans(TestText(source));
        assert!(
            spans.iter().any(|span| span.start == 0
                && span.end >= 50
                && span
                    .note
                    .as_deref()
                    .is_some_and(|note| note.contains("module_declaration"))),
            "and the refusal reports the declaration it could not read, got {spans:?}"
        );
    }

    /// A module the melder read completely is untouched, members and all — and
    /// so is the declaration after it.
    #[test]
    fn a_readable_module_keeps_its_members_and_its_successor()
    {
        let source = "module M {\n  def y = 2;\n}\n\ndef after = 1;\n";
        let outcomes = outcomes(TestText(source));
        assert_eq!(
            2,
            outcomes.len(),
            "both declarations survive, got {outcomes:?}"
        );
        assert!(
            outcomes
                .iter()
                .all(|outcome| matches!(*outcome, ItemOutcome::Definition { .. })),
            "and both are ordinary definitions, got {outcomes:?}"
        );
    }

    /// **The separating case.** A module with no members at all is a legitimate
    /// program, so the refusal must not key on the absence of members: an empty
    /// module still binds, and the declaration after it still lands.
    #[test]
    fn an_empty_module_is_not_an_unread_one()
    {
        let source = "module M {}\n\ndef after = 1;\n";
        let outcomes = outcomes(TestText(source));
        assert_eq!(
            2,
            outcomes.len(),
            "an empty module is read, not refused, got {outcomes:?}"
        );
    }

    /// Where the unread region **is** a member, the member walk reaches it and
    /// declines it at its own span, which is the more precise report of the
    /// same fact. The declaration-level refusal deliberately stays out of the
    /// way.
    #[test]
    fn an_unread_member_keeps_its_own_report()
    {
        let source = "module M {\n  type Hom = Type;\n}\n\ndef after = 1;\n";
        let spans = goal_spans(TestText(source));
        assert!(
            spans.iter().any(|span| span.start > 0
                && span.end > span.start
                && span
                    .note
                    .as_deref()
                    .is_some_and(|note| !note.contains("module_declaration"))),
            "the report is the member's span rather than the whole declaration's, got {spans:?}"
        );
    }

    // --- Helpers --------------------------------------------------------------

    /// Every item outcome one submission produced.
    fn outcomes(source: TestText<'_>) -> Vec<ItemOutcome>
    {
        let mut session = Session::new();
        let submission = session.submit(source.0).expect("lowering must not fail");
        submission.outcomes
    }

    /// Every goal span one submission reported, with its note.
    fn goal_spans(source: TestText<'_>) -> Vec<GoalSpan>
    {
        let mut session = Session::new();
        let submission = session.submit(source.0).expect("lowering must not fail");
        submission
            .report
            .goals
            .iter()
            .map(|goal| GoalSpan {
                start: goal.span.start,
                end: goal.span.end,
                note: goal.note.clone(),
            })
            .collect()
    }
}
