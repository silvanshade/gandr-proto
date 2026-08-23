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
    use alloc::format;
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
        let source = r#"module M : #{
  val x : Integer
} {
  def x = 1;
}

def after = 1;
"#;
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
        let source = r#"module M {
  def y = 2;
}

def after = 1;
"#;
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
        let source = r#"module M {}

def after = 1;
"#;
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
        let source = r#"module M {
  type Hom = Type;
}

def after = 1;
"#;
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

    /// **The separating witness.** Module declarations reserve the
    /// uppercase-initial name stratum. Single-word and multi-word uppercase
    /// names lower with the same signature; their lowercase counterparts
    /// decline at the declaration span with the attempted name in the goal.
    #[test]
    fn module_name_case_boundary_covers_single_and_multi_names()
    {
        for (name, readable) in [
            ("M", true),
            ("N", true),
            ("Nat", true),
            ("NatAdd", true),
            ("m", false),
            ("natAdd", false),
            ("intadd", false),
            ("monoid", false),
        ] {
            let source = format!(
                "module {name} : #{{ type M = Integer, zero : M, add : U[ω] (M -> M -> F M) }} {{ def zero = 0; def add(x: Integer, y: Integer) -> F Integer {{ ret x + y }} }}"
            );
            let mut session = Session::new();
            let submission = session
                .submit(source.as_str())
                .expect("module case witness must lower in total mode");
            if readable {
                assert!(
                    matches!(
                        submission.outcomes.as_slice(),
                        [ItemOutcome::Definition {
                            name: bound_name,
                            bound: true,
                            ..
                        }] if bound_name == name
                    ),
                    "uppercase module `{name}` must bind one definition, got {:?}",
                    submission.outcomes
                );
                let rendered_type = format!(
                    "{:?}",
                    submission
                        .outcomes
                        .first()
                        .expect("the uppercase outcome assertion must pass")
                );
                assert!(
                    rendered_type.contains("\"add\"") && rendered_type.contains("\"zero\""),
                    "uppercase module `{name}` must preserve both signature members, got {rendered_type}"
                );
                assert!(
                    submission.report.goals.is_empty(),
                    "uppercase module `{name}` must not mint a refusal goal, got {:?}",
                    submission.report.goals
                );
            }
            else {
                assert!(
                    matches!(submission.outcomes.as_slice(), [ItemOutcome::Holey]),
                    "lowercase module `{name}` must decline rather than bind empty, got {:?}",
                    submission.outcomes
                );
                let located_note = submission
                    .report
                    .goals
                    .iter()
                    .find(|goal| goal.span.start == 0 && goal.span.end == source.len())
                    .and_then(|goal| goal.note.as_deref());
                assert!(
                    located_note.is_some_and(|note| {
                        note.contains("LowercaseModuleName") && note.contains(name)
                    }),
                    "lowercase module `{name}` must report its declaration span and name, got {:?}",
                    submission.report.goals
                );
            }
        }
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

    /// A nested module declares under either case spelling (`gandr-nl7i`).
    ///
    /// The nested form's name tile admitted only the lowercase spelling, so
    /// `module Limits` inside `module Config` went to parser repair and
    /// surfaced as holes and goals — malformed input, not a name the reader
    /// could be told about. Both spellings now parse as the modules they are;
    /// the top level keeps its uppercase-only tile and its named decline.
    #[test]
    fn a_nested_module_declares_under_either_case_spelling()
    {
        let uppercase = r#"module Config {
  module Limits { def hard = 1; }
  def soft = 2;
}
"#;
        let observed = outcomes(TestText(uppercase));
        assert!(
            !matches!(
                observed.as_slice(),
                [ItemOutcome::Holey] | [.., ItemOutcome::Holey]
            ),
            "an uppercase nested module is a module, not repair: {observed:?}"
        );
        assert!(
            goal_spans(TestText(uppercase)).is_empty(),
            "no goal may stand in for the declaration"
        );
        // And the lowercase spelling that always parsed still does.
        let lowercase = r#"module Config {
  module limits { def hard = 1; }
}
"#;
        let observed = outcomes(TestText(lowercase));
        assert!(
            !matches!(
                observed.as_slice(),
                [ItemOutcome::Holey] | [.., ItemOutcome::Holey]
            ),
            "the lowercase spelling keeps its parse: {observed:?}"
        );
    }
}
