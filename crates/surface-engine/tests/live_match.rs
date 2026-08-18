//! Live pattern matching against a checkpoint scrutinee with holes: branch
//! satisfaction, exhaustiveness, redundancy, and how the verdicts reach a
//! consumer.
//!
//! The suite is organized around the one property that makes this analysis
//! worth having: **every verdict is proved in its own direction, and anything
//! a hole could decide is the middle verdict instead.** Each test below either
//! pins one of the three values or pins that the analysis changed nothing about
//! lowering, typing, or evaluation.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Live pattern matching tests.
#[cfg(test)]
mod tests
{
    use alloc::string::String;
    use alloc::vec::Vec;

    use gandr_core_checker::discipline::mark::Mark;
    use gandr_core_incremental::boundary::MatchOrdinal;
    use gandr_core_incremental::boundary::SourceItemOrdinal;
    use gandr_core_incremental::boundary::SubmissionOrdinal;
    use gandr_core_incremental::stream::BranchStatus;
    use gandr_core_incremental::stream::MatchOrigin;
    use gandr_core_incremental::stream::MatchOrigin as PublishedOrigin;
    use gandr_core_incremental::stream::SynthesisEvent;
    use gandr_core_sequent::machine::run_comp;
    use gandr_core_term::outcome::Eval;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Value;
    use gandr_core_term::types::DataId;
    use gandr_surface_engine::boundary::MatchIndex;
    use gandr_surface_engine::boundary::SourceItemId;
    use gandr_surface_engine::boundary::SourceRange;
    use gandr_surface_engine::boundary::SubmissionIndex;
    use gandr_surface_engine::live_match::BranchPattern;
    use gandr_surface_engine::live_match::ConstructorTable;
    use gandr_surface_engine::live_match::DeclaredConstructor;
    use gandr_surface_engine::live_match::LiteralConstraint;
    use gandr_surface_engine::live_match::MatchAnalysis;
    use gandr_surface_engine::live_match::MatchObligation;
    use gandr_surface_engine::live_match::MatchSite;
    use gandr_surface_engine::live_match::PatternShape;
    use gandr_surface_engine::live_match::analyze_site;
    use gandr_surface_engine::live_match::translate_pattern;
    use gandr_surface_engine::session::ItemOutcome;
    use gandr_surface_engine::session::Session;
    use gandr_surface_engine::session::Submission;
    use gandr_surface_engine::synnode::SynTree;

    use crate::common::TestCount;
    use crate::common::TestHoleId;
    use crate::common::TestText;

    /// A three-constructor enumeration, the fixture for coverage questions.
    const COLOR: &str = "data Color : Type { Red : Color; Green : Color; Blue : Color; }";

    /// A parameterised datatype with a payload-bearing constructor, the fixture
    /// for questions that must look inside a constructor.
    const MAYBE: &str =
        "data Maybe(a : Type) : Type { Nothing : Maybe(a); Just : (x : a) --> Maybe(a); }";

    // --- Branch satisfaction --------------------------------------------------

    /// A hole scrutinee decides nothing, so no branch is satisfied and none is
    /// refuted.
    ///
    /// This is the base case of live matching: the match is **stuck**, and the
    /// honest description of a stuck match is that every branch is still in
    /// play. Reporting any branch as refuted here would hide a match the
    /// finished program takes.
    #[test]
    fn a_hole_scrutinee_leaves_every_branch_possible()
    {
        let analysis = sole_match(TestText::from(
            "def pick() -> F Integer { case ? { Red => ret 0, Green => ret 1, Blue => ret 2 } }",
        ));
        assert_eq!(
            &[
                BranchStatus::Possibly,
                BranchStatus::Possibly,
                BranchStatus::Possibly
            ],
            analysis.branches.as_slice(),
            "a scrutinee that is a hole leaves every branch undecided"
        );
    }

    /// A constructor-headed scrutinee refutes the branches whose constructor it
    /// is not, satisfies the branch that only tests the constructor, and leaves
    /// undecided the branch whose verdict depends on the hole in the payload.
    ///
    /// The three verdicts appear together here on purpose: they are what
    /// distinguishes this analysis from a two-valued one. The head is known, so
    /// `Nothing` is genuinely impossible; the payload is not, so `Just(0)` is
    /// genuinely undecided; and `Just(x)` binds rather than tests, so it holds
    /// whatever the hole becomes.
    #[test]
    fn a_constructor_scrutinee_refutes_the_head_and_suspends_the_payload()
    {
        let analysis = sole_match(TestText::from(
            "def probe() -> F Integer { case (Just(?) : Maybe(Integer)) { \
             Nothing => ret 0, Just(0) => ret 1, Just(x) => ret 2 } }",
        ));
        assert_eq!(
            &[
                BranchStatus::Refuted,
                BranchStatus::Possibly,
                BranchStatus::Satisfied
            ],
            analysis.branches.as_slice(),
            "the head decides the first branch, the hole suspends the second, \
             and the binder holds regardless"
        );
    }

    /// The evaluator's β-rule on a constructor whose payload is a hole is
    /// unchanged: it selects the arm by tag and binds the hole as the payload.
    ///
    /// This slice adds no evaluator behaviour and removes none. The pin is here
    /// rather than left implicit because the analysis's whole subject is what a
    /// hole in a payload means, and the one thing it must **not** mean is that
    /// the eliminator stops firing.
    #[test]
    fn the_eliminator_still_fires_on_a_hole_payload()
    {
        let fired = run_comp(&Comp::data_case(
            Value::ctor(DataId::new(0_u64, "D"), 0_usize, Value::hole(0)),
            alloc::vec![(String::from("payload"), Comp::ret(Value::var("payload")))],
        ));
        assert_eq!(
            Eval::Value(Comp::ret(Value::hole(0))),
            fired,
            "the tag selects the arm and the hole rides through as the payload"
        );
    }

    // --- Exhaustiveness -------------------------------------------------------

    /// A constructor no branch covers is reported as uncovered, by name.
    #[test]
    fn a_missing_constructor_is_reported_by_name()
    {
        let analysis = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { Red => ret 0, Green => ret 1 } }",
        ));
        assert_eq!(
            alloc::vec![MatchObligation::Inexhaustive {
                uncovered: alloc::vec![String::from("Blue")],
            }],
            exhaustiveness_obligations(&analysis),
            "the residue is named, not merely counted"
        );
    }

    /// A pattern hole beside the branches downgrades the same residue from
    /// proved-uncovered to possibly-uncovered.
    ///
    /// The residue has not gone away — `Blue` is still unhandled — but the
    /// programmer has written something that could handle it, and reporting a
    /// proved gap over an unfinished pattern would be false.
    #[test]
    fn a_pattern_hole_downgrades_the_residue()
    {
        let analysis = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { Red => ret 0, Green => ret 1, \
             ? => ret 2 } }",
        ));
        assert_eq!(
            alloc::vec![MatchObligation::PossiblyInexhaustive {
                uncovered: alloc::vec![String::from("Blue")],
            }],
            exhaustiveness_obligations(&analysis),
            "a hole that could cover the residue makes the gap possible, not proved"
        );
    }

    /// Completing the match clears the obligation entirely.
    #[test]
    fn completing_the_match_clears_the_obligation()
    {
        let analysis = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { Red => ret 0, Green => ret 1, \
             Blue => ret 2 } }",
        ));
        assert!(
            exhaustiveness_obligations(&analysis).is_empty(),
            "a complete match raises nothing; obligations: {:?}",
            analysis.obligations
        );
    }

    /// An inexhaustive match blocks nothing: the definition beside it still
    /// lowers, types, and enters scope.
    ///
    /// This is the property the whole slice is built to preserve. The analysis
    /// is an observer — it produces obligations and changes no verdict anyone
    /// else reaches.
    #[test]
    fn an_inexhaustive_match_does_not_block_its_siblings()
    {
        let mut session = Session::new();
        let source = alloc::format!(
            "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ Red => ret 0 }} }} \
             def sibling = 41 + 1;"
        );
        let submission = session.submit(&source).expect("lowering must not fail");
        let Some(outcome) = submission.outcomes.last()
        else {
            panic!("expected the sibling definition's outcome, got []");
        };
        match *outcome {
            | ItemOutcome::Definition {
                ref name, bound, ..
            } => {
                assert_eq!("sibling", name, "the sibling definition lands");
                assert!(bound, "and enters scope");
            },
            | ref other => panic!("the sibling should be a definition, got {other:?}"),
        }
        let Some(analysis) = submission.matches.first()
        else {
            panic!("expected one match analysis, got []");
        };
        assert!(
            !exhaustiveness_obligations(analysis).is_empty(),
            "and the obligation is still raised beside it"
        );
    }

    // --- Redundancy -----------------------------------------------------------

    /// An or-pattern covers each of its alternatives, so a later branch
    /// repeating one of them is entailed by it.
    #[test]
    fn an_or_pattern_entails_a_later_repeat_of_either_alternative()
    {
        let analysis = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { Red | Green => ret 0, \
             Green => ret 1, Blue => ret 2 } }",
        ));
        assert_eq!(
            alloc::vec![MatchObligation::Redundant {
                branch: 1_usize.into(),
                entailing_prefix: 1_usize.into(),
            }],
            redundancy_obligations(&analysis),
            "the first branch alone entails the second, and the report says which prefix"
        );
    }

    /// An as-pattern tests exactly what it wraps, so its binder does not stop
    /// it entailing a later branch.
    #[test]
    fn an_as_pattern_entails_through_its_binder()
    {
        let analysis = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { Red as whole => ret 0, \
             Red => ret 1, Green => ret 2, Blue => ret 3 } }",
        ));
        assert_eq!(
            alloc::vec![MatchObligation::Redundant {
                branch: 1_usize.into(),
                entailing_prefix: 1_usize.into(),
            }],
            redundancy_obligations(&analysis),
            "naming a match does not change what it matches"
        );
    }

    /// A pattern hole in the prefix leaves a later branch's redundancy
    /// undecided rather than proved.
    ///
    /// Filled one way the hole entails the branch; filled another it does not.
    /// The obligation says exactly that, and it says which prefix the
    /// dependence runs through.
    #[test]
    fn a_hole_in_the_prefix_leaves_redundancy_unproved()
    {
        let analysis = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { ? => ret 0, Red => ret 1 } }",
        ));
        assert_eq!(
            alloc::vec![MatchObligation::PossiblyRedundant {
                branch: 1_usize.into(),
                entailing_prefix: 1_usize.into(),
            }],
            redundancy_obligations(&analysis),
            "hole dependence is reported as dependence, never as proof"
        );
    }

    /// A branch that is genuinely live is reported as neither redundant nor
    /// possibly redundant.
    #[test]
    fn a_live_branch_raises_no_redundancy_obligation()
    {
        let analysis = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { Red => ret 0, Green => ret 1, \
             Blue => ret 2 } }",
        ));
        assert!(
            redundancy_obligations(&analysis).is_empty(),
            "distinct constructors entail nothing of each other; obligations: {:?}",
            analysis.obligations
        );
    }

    // --- The pattern-position mark -------------------------------------------

    /// A pattern hole carries the pattern-position mark, and that mark is not
    /// an error mark.
    ///
    /// The position is part of the mark's identity: an unfinished *test* and an
    /// unfinished *value* license different conclusions, and collapsing them
    /// would hand this analysis an expression's reading of a pattern's hole.
    #[test]
    fn a_pattern_hole_carries_the_pattern_position_mark()
    {
        let analysis = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { Red => ret 0, ? => ret 1 } }",
        ));
        let Some(mark) = analysis.pattern_marks.first()
        else {
            panic!("expected one pattern-position mark, got []");
        };
        assert!(
            matches!(*mark, Mark::PatternHole(_)),
            "the mark names the position, got {mark:?}"
        );
        assert!(
            !bool::from(mark.is_error()),
            "an unfinished pattern leaves the program well-typed and merely incomplete"
        );
        assert_eq!(
            1,
            analysis.pattern_marks.len(),
            "one hole, one mark; marks: {:?}",
            analysis.pattern_marks
        );
    }

    // --- Translation ----------------------------------------------------------

    /// Every form of the landed pattern grammar translates into the constraint
    /// language, in one match.
    ///
    /// The point of asserting the whole grammar at once is that the decision
    /// procedures never see a CST: whatever translation does not carry across
    /// this boundary is invisible to them forever after, and would degrade
    /// silently into an unreadable region rather than fail.
    #[test]
    fn translation_covers_the_landed_pattern_grammar()
    {
        let shapes = arm_shapes(TestText::from(
            "def f = case v { \
             _ => 0, \
             x => 1, \
             ? => 2, \
             ?named => 3, \
             Red => 4, \
             Just(Just(y)) => 5, \
             7 => 6, \
             true => 7, \
             \"s\" => 8, \
             (a, b) => 9, \
             #{ l = p } => 10, \
             [h, ..t] => 11, \
             Red | Green => 12, \
             Red as whole => 13 \
             };",
        ));
        let expected = alloc::vec![
            PatternShape::Wildcard,
            PatternShape::Bind(String::from("x")),
            PatternShape::Hole {
                mark: Mark::PatternHole(hole_start(&shapes, TestCount::from(2_usize)).into()),
                name: None,
            },
            PatternShape::Hole {
                mark: Mark::PatternHole(hole_start(&shapes, TestCount::from(3_usize)).into()),
                name: Some(String::from("named")),
            },
            PatternShape::Ctor {
                name: String::from("Red"),
                arguments: Vec::new(),
            },
            PatternShape::Ctor {
                name: String::from("Just"),
                arguments: alloc::vec![PatternShape::Ctor {
                    name: String::from("Just"),
                    arguments: alloc::vec![PatternShape::Bind(String::from("y"))],
                }],
            },
            PatternShape::Literal(LiteralConstraint::Int(7)),
            PatternShape::Literal(LiteralConstraint::Bool(true)),
            PatternShape::Literal(LiteralConstraint::Str(String::from("s"))),
            PatternShape::Tuple(alloc::vec![
                PatternShape::Bind(String::from("a")),
                PatternShape::Bind(String::from("b")),
            ]),
            PatternShape::Record(alloc::vec![(
                String::from("l"),
                PatternShape::Bind(String::from("p")),
            )]),
            PatternShape::List {
                elements: alloc::vec![PatternShape::Bind(String::from("h"))],
                rest: Some(alloc::boxed::Box::new(PatternShape::Bind(String::from(
                    "t",
                )))),
            },
            PatternShape::Or(
                alloc::boxed::Box::new(PatternShape::Ctor {
                    name: String::from("Red"),
                    arguments: Vec::new(),
                }),
                alloc::boxed::Box::new(PatternShape::Ctor {
                    name: String::from("Green"),
                    arguments: Vec::new(),
                }),
            ),
            PatternShape::As {
                pattern: alloc::boxed::Box::new(PatternShape::Ctor {
                    name: String::from("Red"),
                    arguments: Vec::new(),
                }),
                binder: String::from("whole"),
            },
        ];
        assert_eq!(
            expected, shapes,
            "every pattern form reaches the constraint language"
        );
    }

    // --- The synthesis stream and the incremental seam ------------------------

    /// The per-branch statuses reach a consumer through the synthesis stream,
    /// addressed by the source origin the match was written at.
    #[test]
    fn the_stream_publishes_per_branch_status()
    {
        let mut session = Session::new();
        let source = alloc::format!(
            "{COLOR} def pick() -> F Integer {{ case ? {{ Red => ret 0, Green => ret 1, \
             Blue => ret 2 }} }}"
        );
        let _submission = session.submit(&source).expect("lowering must not fail");
        let events = session
            .latest_synthesis_stream()
            .expect("a successful submission publishes a stream")
            .collect::<Vec<SynthesisEvent>>();
        let matches = events
            .iter()
            .filter(|event| matches!(**event, SynthesisEvent::Match { .. }))
            .collect::<Vec<_>>();
        let Some(&event) = matches.first()
        else {
            panic!("expected one match event, got {events:?}");
        };
        assert_eq!(1, matches.len(), "one match, one event; events: {events:?}");
        match *event {
            | SynthesisEvent::Match {
                origin,
                ref branches,
            } => {
                assert_eq!(
                    at(0_usize.into(), 0_usize.into(), 0_usize.into()),
                    origin,
                    "the first submission, and the datatype declaration is no source item, so \
                     `pick` is source item 0 carrying that item's first match"
                );
                assert_eq!(
                    &[
                        BranchStatus::Possibly,
                        BranchStatus::Possibly,
                        BranchStatus::Possibly
                    ],
                    branches.as_slice(),
                    "the stuck match publishes its branches, not a result"
                );
            },
            | ref other => panic!("expected a match event, got {other:?}"),
        }
    }

    /// A submission's obligations and the stream's events address the same
    /// match, by the same origin.
    ///
    /// Two addressing spaces reaching one consumer is a defect waiting to
    /// happen — an obligation attributed to the wrong definition reads as
    /// perfectly coherent — so there is only one, and it is the source origin:
    /// which submission, which source item of it, which match of that item.
    #[test]
    fn the_submission_and_the_stream_agree_on_the_origin()
    {
        let mut session = Session::new();
        let prefix = alloc::format!("{COLOR} def base = 1;");
        let _first = session
            .submit(&prefix)
            .expect("prefix lowering must not fail");
        let second = session
            .submit("def pick(c : Color) -> F Integer { case c { Red => ret 0 } }")
            .expect("appended lowering must not fail");
        let Some(analysis) = second.matches.first()
        else {
            panic!("expected one match analysis, got []");
        };
        assert_eq!(
            1_usize,
            usize::from(analysis.origin.submission),
            "the appended source is the session's second submission"
        );
        assert_eq!(
            0_usize,
            usize::from(analysis.origin.source_item),
            "and `pick` is its own submission's first source item, not the program's second item"
        );
        let Some(SynthesisEvent::Match { origin, .. }) =
            match_events(&session).into_iter().find(|event| {
                matches!(*event, SynthesisEvent::Match { origin, .. }
                if usize::from(origin.submission) == 1)
            })
        else {
            panic!("expected the appended submission's match event on the stream");
        };
        assert_eq!(
            PublishedOrigin::from(analysis.origin),
            origin,
            "the obligation and the event name the same match"
        );
    }

    /// A match analyzed after an incremental resume that adopted its
    /// predecessors gets the same statuses as one analyzed from scratch.
    ///
    /// The analysis reads the retained record, never the path taken to it, so
    /// adoption cannot move a verdict. This is the property that makes the
    /// obligations safe to publish from an incremental pipeline at all: a
    /// consumer comparing two runs must see a difference only where the program
    /// differs. The origins do differ — one session wrote the match in its
    /// first submission and the other in its second, which is a true fact about
    /// each — so what is compared is the verdicts.
    #[test]
    fn adoption_does_not_move_a_verdict()
    {
        let prefix = alloc::format!("{COLOR} def base = 1;");
        let match_source = "def pick(c : Color) -> F Integer { case c { Red => ret 0, \
                            ? => ret 1 } }";

        let combined = alloc::format!("{prefix} {match_source}");
        let mut whole = Session::new();
        let _submission = whole
            .submit(&combined)
            .expect("whole-file lowering must not fail");
        let from_scratch = published_statuses(&whole);

        let mut split = Session::new();
        let _first = split
            .submit(&prefix)
            .expect("prefix lowering must not fail");
        let second = split
            .submit(match_source)
            .expect("appended lowering must not fail");
        let across_the_seam = published_statuses(&split);

        assert!(
            second
                .outcomes
                .iter()
                .any(|outcome| matches!(*outcome, ItemOutcome::Definition { .. })),
            "the appended submission produced its own item"
        );
        assert!(
            split
                .latest_synthesis_stream()
                .expect("a successful submission publishes a stream")
                .any(|event| matches!(event, SynthesisEvent::Item { adopted, .. }
                    if bool::from(adopted))),
            "and the resume genuinely adopted an earlier checkpoint, so the seam was crossed"
        );
        assert_eq!(
            from_scratch, across_the_seam,
            "identical checkpoints yield identical statuses on both sides of the seam"
        );
    }

    // --- Retained source records across submissions ---------------------------

    /// A match submitted on an earlier line is still published after a later
    /// line arrives.
    ///
    /// **This is the property a stream built from checkpoints alone cannot
    /// have.** Checkpoints keep lowered terms, and a lowered `case` has no
    /// patterns in it, so an implementation that recomputed liveness from the
    /// resume would watch every earlier match quietly stop existing the moment
    /// the next submission landed. The session retains one source record per
    /// submission precisely so the whole program keeps answering.
    #[test]
    fn a_prior_submissions_matches_survive_the_next_one()
    {
        let mut session = Session::new();
        let _first = session
            .submit(&alloc::format!(
                "{COLOR} def first(c : Color) -> F Integer {{ case c {{ Red => ret 0 }} }}"
            ))
            .expect("first lowering must not fail");
        let first_statuses = published_statuses(&session);
        let _second = session
            .submit("def second(c : Color) -> F Integer { case c { Green => ret 1 } }")
            .expect("second lowering must not fail");

        let published = published_origins(&session);
        assert_eq!(
            alloc::vec![
                at(0_usize.into(), 0_usize.into(), 0_usize.into()),
                at(1_usize.into(), 0_usize.into(), 0_usize.into()),
            ],
            published,
            "both submissions' matches are on the stream, each at its own origin"
        );
        assert_eq!(
            first_statuses.first(),
            published_statuses(&session).first(),
            "and the earlier match's verdicts are the ones it already had"
        );
    }

    /// A source item that lowered to more than one core item does not move the
    /// origins of the matches that follow it.
    ///
    /// The first submission holds one source item and lowers to **two** core
    /// items: the dangling signature becomes an item of its own. Core positions
    /// and source identities part company there and stay parted, so the second
    /// submission's match sits at core item 2 and at source item 0. A
    /// core-indexed scheme reports 2; the origin reports 0, because that is
    /// where the programmer wrote it — and it is the answer that survives
    /// anything the lowerer decides to do with core items.
    #[test]
    fn one_source_item_lowering_to_several_core_items_keeps_one_entry()
    {
        let mut session = Session::new();
        let first = session
            .submit(&alloc::format!(
                "{COLOR} def base = 1; def ghost : Integer;"
            ))
            .expect("first lowering must not fail");
        assert_eq!(
            2,
            first.outcomes.len(),
            "one source item, two core items — the divergence this test needs; outcomes: {:?}",
            first.outcomes
        );

        let second = session
            .submit("def pick(c : Color) -> F Integer { case c { Red => ret 0 } }")
            .expect("second lowering must not fail");
        let Some(analysis) = second.matches.first()
        else {
            panic!("expected one match analysis, got {:?}", second.matches);
        };
        assert_eq!(
            1,
            second.matches.len(),
            "the fixture holds exactly one match; matches: {:?}",
            second.matches
        );
        assert_eq!(
            at(1_usize.into(), 0_usize.into(), 0_usize.into()),
            PublishedOrigin::from(analysis.origin),
            "the match is the second submission's first source item's first match, whatever \
             core index it landed on"
        );
        assert_eq!(
            alloc::vec![at(1_usize.into(), 0_usize.into(), 0_usize.into())],
            published_origins(&session),
            "and the stream publishes exactly one entry for it"
        );
    }

    /// A session that stopped and resumed from its retained records reports
    /// what an uninterrupted one reports.
    ///
    /// Verdicts and obligations are compared rather than origins: the two
    /// sessions genuinely wrote the same match in different submissions, and
    /// saying so is correct. What must not differ is anything about the
    /// program.
    #[test]
    fn a_resumed_session_matches_an_uninterrupted_one()
    {
        let prefix = alloc::format!("{COLOR} def base = 1;");
        let match_source = "def pick(c : Color) -> F Integer { case c { Red => ret 0, \
                            Green => ret 1 } }";

        let mut whole = Session::new();
        let uninterrupted = whole
            .submit(&alloc::format!("{prefix} {match_source}"))
            .expect("whole-file lowering must not fail");

        let mut split = Session::new();
        let _first = split
            .submit(&prefix)
            .expect("prefix lowering must not fail");
        let resumed = split
            .submit(match_source)
            .expect("appended lowering must not fail");

        assert_eq!(
            uninterrupted
                .matches
                .iter()
                .map(|analysis| analysis.obligations.clone())
                .collect::<Vec<_>>(),
            resumed
                .matches
                .iter()
                .map(|analysis| analysis.obligations.clone())
                .collect::<Vec<_>>(),
            "the same obligations are raised on both paths"
        );
        assert_eq!(
            published_statuses(&whole),
            published_statuses(&split),
            "and the stream publishes the same verdicts"
        );
    }

    /// A session reopened on core state alone reports its earlier submission as
    /// `SourceNotRetained`, and analyzes the new one normally.
    ///
    /// **Silence would be the wrong answer, and it is the answer an
    /// implementation falls into by accident.** A checkpoint artifact carries
    /// no patterns — they were never in the core — so the earlier matches are
    /// unrecoverable. A consumer told nothing would read the reopened session
    /// as a program whose earlier matches had all become decided; told
    /// `SourceNotRetained`, it knows the difference between an answer and the
    /// absence of one.
    #[test]
    fn a_record_stripped_resume_reports_source_not_retained()
    {
        let mut original = Session::new();
        let _first = original
            .submit(&alloc::format!(
                "{COLOR} def first(c : Color) -> F Integer {{ case c {{ Red => ret 0 }} }}"
            ))
            .expect("first lowering must not fail");
        assert_eq!(
            1,
            published_origins(&original).len(),
            "the original session does publish that match"
        );

        // Only the core crosses: the program and its checkpoints, exactly what
        // a persisted artifact restores. The declaration registry does not, so
        // the resumed submission redeclares the datatype it matches on.
        let mut resumed =
            Session::resume_from_core(original.program().clone(), original.checkpoints().clone());
        let submission = resumed
            .submit(&alloc::format!(
                "{COLOR} def second(c : Color) -> F Integer {{ case c {{ Green => ret 1 }} }}"
            ))
            .expect("resumed lowering must not fail");

        let events = resumed
            .latest_synthesis_stream()
            .expect("a successful submission publishes a stream")
            .collect::<Vec<SynthesisEvent>>();
        assert!(
            events.contains(&SynthesisEvent::SourceNotRetained {
                submission: SubmissionOrdinal::from(0_usize),
            }),
            "the restored program is named as unretained, not passed over; events: {events:?}"
        );
        assert_eq!(
            alloc::vec![at(1_usize.into(), 0_usize.into(), 0_usize.into())],
            published_origins(&resumed),
            "and only the newly submitted match publishes verdicts"
        );
        assert_eq!(
            1,
            submission.matches.len(),
            "the current submission is analyzed exactly as it would have been without the \
             resume; matches: {:?}",
            submission.matches
        );
    }

    /// Two programs whose lowered items are identical share a checkpoint set
    /// and still differ in liveness.
    ///
    /// The lowerer compiles a `case` to tag-indexed arms and drops any arm
    /// whose pattern is not a constructor pattern — so the pattern hole in the
    /// second program never reaches the core, and the two programs are
    /// **equal** as far as the core is concerned. They are not equal as
    /// programs: one provably fails to cover `Blue` and the other might yet
    /// cover it. This is the whole argument for retaining source records rather
    /// than reading liveness off a checkpoint, and it is why core equality had
    /// to be left alone rather than widened to notice the difference.
    #[test]
    fn core_equal_programs_differ_in_liveness()
    {
        let mut without_hole = Session::new();
        let plain = without_hole
            .submit(&alloc::format!(
                "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ \
                 Red => ret 0, Green => ret 1 }} }}"
            ))
            .expect("lowering must not fail");

        let mut with_hole = Session::new();
        let holed = with_hole
            .submit(&alloc::format!(
                "{COLOR} def pick(c : Color) -> F Integer {{ case c {{ \
                 Red => ret 0, Green => ret 1, ? => ret 2 }} }}"
            ))
            .expect("lowering must not fail");

        assert_eq!(
            without_hole.checkpoints(),
            with_hole.checkpoints(),
            "the pattern hole never reaches the core, so the checkpoints are the same ones"
        );
        assert_eq!(
            alloc::vec![MatchObligation::Inexhaustive {
                uncovered: alloc::vec![String::from("Blue")],
            }],
            sole_analysis(&plain).obligations,
            "without the hole the gap is proved"
        );
        assert_eq!(
            alloc::vec![MatchObligation::PossiblyInexhaustive {
                uncovered: alloc::vec![String::from("Blue")],
            }],
            sole_analysis(&holed).obligations,
            "with it the same gap is only possible — a difference the core cannot see"
        );
    }

    // --- A hole and an unreadable region are different not-knowings -----------

    /// An unreadable branch beside a missing constructor leaves that
    /// constructor undecided — neither covered, nor a proved gap, nor the
    /// author's to fill.
    ///
    /// The three wrong answers are all reachable, and each is wrong in its own
    /// way. Covered would hide a real gap. `Inexhaustive` would claim a proof
    /// over a pattern nobody read. `PossiblyInexhaustive` would tell the author
    /// their unfinished work is what is holding the answer up, when the
    /// unfinished work is this analysis's.
    #[test]
    fn an_unreadable_row_leaves_a_missing_constructor_undecided()
    {
        let analysis = analyze_branches(alloc::vec![
            PatternShape::Ctor {
                name: String::from("Red"),
                arguments: Vec::new(),
            },
            PatternShape::Unreadable,
        ]);
        assert_eq!(
            alloc::vec![MatchObligation::AnalysisIncomplete {
                undecided: alloc::vec![String::from("Green"), String::from("Blue")],
            }],
            analysis.obligations,
            "the analyzer's own limit is reported as its own, and names what it could not decide"
        );
    }

    /// A readable branch standing after an unreadable one is not reported
    /// redundant, and the same branch after a readable wildcard is.
    ///
    /// An unreadable region shadows nothing: it establishes no coverage, so it
    /// cannot entail a later branch. The wildcard control is what makes this an
    /// assertion rather than an accident — an implementation that simply never
    /// reported redundancy would pass the first half and fail the second.
    #[test]
    fn a_readable_row_after_an_unreadable_twin_is_not_redundant()
    {
        let red = || PatternShape::Ctor {
            name: String::from("Red"),
            arguments: Vec::new(),
        };
        let after_unreadable = analyze_branches(alloc::vec![PatternShape::Unreadable, red()]);
        assert!(
            !after_unreadable
                .obligations
                .iter()
                .any(|obligation| matches!(
                    *obligation,
                    MatchObligation::Redundant { .. } | MatchObligation::PossiblyRedundant { .. }
                )),
            "an unreadable region entails nothing; obligations: {:?}",
            after_unreadable.obligations
        );
        let after_wildcard = analyze_branches(alloc::vec![PatternShape::Wildcard, red()]);
        assert!(
            after_wildcard.obligations.iter().any(
                |obligation| matches!(*obligation, MatchObligation::Redundant {
                    branch,
                    ..
                } if usize::from(branch) == 1)
            ),
            "but a wildcard does, so the silence above is about the unreadable region rather \
             than about redundancy reporting; obligations: {:?}",
            after_wildcard.obligations
        );
    }

    /// Branches that already cover every constructor stay exhaustive with an
    /// unreadable branch beside them.
    ///
    /// The rule is one-directional: an unreadable region stops a question being
    /// *decided*, never a decided one being *unsettled*. A question the
    /// readable branches already answered is answered.
    #[test]
    fn an_exhaustive_match_stays_exhaustive_beside_an_unreadable_row()
    {
        let analysis = analyze_branches(alloc::vec![
            PatternShape::Ctor {
                name: String::from("Red"),
                arguments: Vec::new(),
            },
            PatternShape::Ctor {
                name: String::from("Green"),
                arguments: Vec::new(),
            },
            PatternShape::Ctor {
                name: String::from("Blue"),
                arguments: Vec::new(),
            },
            PatternShape::Unreadable,
        ]);
        assert!(
            analysis.obligations.is_empty(),
            "a complete match raises nothing, and an unreadable region beside it adds nothing; \
             obligations: {:?}",
            analysis.obligations
        );
    }

    /// Filling a hole moves a `Possibly` verdict to a decided one, on both
    /// sides of the analysis.
    ///
    /// The middle verdict is a claim about an unfinished program, so the test
    /// of whether it means anything is whether finishing the program discharges
    /// it. A branch undecided by a hole in the scrutinee becomes satisfied when
    /// the hole is filled; a residue only possibly uncovered becomes covered
    /// when the pattern hole is replaced by the constructor it was standing in
    /// for.
    #[test]
    fn filling_a_hole_refines_a_possible_verdict()
    {
        let stuck = sole_match(TestText::from(
            "def probe() -> F Integer { case (Just(?) : Maybe(Integer)) { \
             Nothing => ret 0, Just(0) => ret 1, Just(x) => ret 2 } }",
        ));
        assert_eq!(
            &[
                BranchStatus::Refuted,
                BranchStatus::Possibly,
                BranchStatus::Satisfied
            ],
            stuck.branches.as_slice(),
            "the hole in the payload leaves the literal branch undecided"
        );
        let filled = sole_match(TestText::from(
            "def probe() -> F Integer { case (Just(0) : Maybe(Integer)) { \
             Nothing => ret 0, Just(0) => ret 1, Just(x) => ret 2 } }",
        ));
        assert_eq!(
            &[
                BranchStatus::Refuted,
                BranchStatus::Satisfied,
                BranchStatus::Satisfied
            ],
            filled.branches.as_slice(),
            "filling it decides that branch, and moves nothing else"
        );

        let possible = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { Red => ret 0, Green => ret 1, \
             ? => ret 2 } }",
        ));
        assert_eq!(
            alloc::vec![MatchObligation::PossiblyInexhaustive {
                uncovered: alloc::vec![String::from("Blue")],
            }],
            exhaustiveness_obligations(&possible),
            "a pattern hole makes the residue possible rather than proved"
        );
        let discharged = sole_match(TestText::from(
            "def pick(c : Color) -> F Integer { case c { Red => ret 0, Green => ret 1, \
             Blue => ret 2 } }",
        ));
        assert!(
            exhaustiveness_obligations(&discharged).is_empty(),
            "and filling it with the constructor it stood for discharges the obligation; \
             obligations: {:?}",
            discharged.obligations
        );
    }

    // --- Fixture helpers ------------------------------------------------------

    /// The sole match analysis of a source submitted after both fixture
    /// datatype declarations.
    fn sole_match(source: TestText<'_>) -> MatchAnalysis
    {
        let mut session = Session::new();
        let full = alloc::format!("{COLOR} {MAYBE} {source}", source = source.0);
        let submission = session.submit(&full).expect("lowering must not fail");
        let mut matches = submission.matches.into_iter();
        let Some(analysis) = matches.next()
        else {
            panic!("expected one match analysis, got none");
        };
        assert!(
            matches.next().is_none(),
            "the fixture holds exactly one match"
        );
        analysis
    }

    /// A match analysis's exhaustiveness obligations, in report order.
    fn exhaustiveness_obligations(analysis: &MatchAnalysis) -> Vec<MatchObligation>
    {
        analysis
            .obligations
            .iter()
            .filter(|obligation| {
                matches!(
                    **obligation,
                    MatchObligation::Inexhaustive { .. }
                        | MatchObligation::PossiblyInexhaustive { .. }
                )
            })
            .cloned()
            .collect()
    }

    /// A match analysis's redundancy obligations, in report order.
    fn redundancy_obligations(analysis: &MatchAnalysis) -> Vec<MatchObligation>
    {
        analysis
            .obligations
            .iter()
            .filter(|obligation| {
                matches!(
                    **obligation,
                    MatchObligation::Redundant { .. } | MatchObligation::PossiblyRedundant { .. }
                )
            })
            .cloned()
            .collect()
    }

    /// The translated pattern of every arm of the sole match in `source`.
    fn arm_shapes(source: TestText<'_>) -> Vec<PatternShape>
    {
        let tree = SynTree::parse(source.0).expect("the fixture parses");
        let Some(item) = tree.root().named_children().into_iter().next()
        else {
            panic!("expected one item, got none");
        };
        let Some(case) = item.child_by_field_name("value")
        else {
            panic!("expected the definition's value, got none");
        };
        case.named_children()
            .into_iter()
            .filter_map(|arm| arm.child_by_field_name("pattern"))
            .map(translate_pattern)
            .collect()
    }

    /// The hole identity the translation gave the `index`-th arm's pattern,
    /// read back so the expectation names the value rather than restating the
    /// derivation.
    fn hole_start(
        shapes: &[PatternShape],
        index: TestCount,
    ) -> TestHoleId
    {
        let index = usize::from(index);
        match shapes.get(index) {
            | Some(&PatternShape::Hole {
                mark: Mark::PatternHole(id),
                ..
            }) => TestHoleId::from(id),
            | other => panic!("arm {index} should translate to a pattern hole, got {other:?}"),
        }
    }

    /// The match events of a session's latest synthesis stream.
    fn match_events(session: &Session) -> Vec<SynthesisEvent>
    {
        session
            .latest_synthesis_stream()
            .expect("a successful submission publishes a stream")
            .filter(|event| matches!(*event, SynthesisEvent::Match { .. }))
            .collect()
    }

    /// The origins the session's latest stream publishes liveness at, in stream
    /// order.
    fn published_origins(session: &Session) -> Vec<MatchOrigin>
    {
        match_events(session)
            .into_iter()
            .filter_map(|event| match event {
                | SynthesisEvent::Match { origin, .. } => Some(origin),
                | _ => None,
            })
            .collect()
    }

    /// One origin, so a fixture states `(submission, source item, match)`
    /// directly rather than restating the wrapper types carrying them.
    fn at(
        submission: SubmissionOrdinal,
        source_item: SourceItemOrdinal,
        match_index: MatchOrdinal,
    ) -> MatchOrigin
    {
        MatchOrigin {
            submission,
            source_item,
            match_index,
        }
    }

    /// The per-branch verdicts the session's latest stream publishes, in stream
    /// order and stripped of the origins that name them.
    fn published_statuses(session: &Session) -> Vec<Vec<BranchStatus>>
    {
        match_events(session)
            .into_iter()
            .filter_map(|event| match event {
                | SynthesisEvent::Match { branches, .. } => Some(branches),
                | _ => None,
            })
            .collect()
    }

    /// The sole match analysis of a submission.
    fn sole_analysis(submission: &Submission) -> &MatchAnalysis
    {
        assert_eq!(
            1,
            submission.matches.len(),
            "the fixture holds exactly one match; matches: {:?}",
            submission.matches
        );
        let Some(analysis) = submission.matches.first()
        else {
            panic!(
                "the fixture holds exactly one match, got {:?}",
                submission.matches
            );
        };
        analysis
    }

    /// The `Color` declaration as the analysis reads it.
    ///
    /// Built directly rather than lowered, because the tests that use it are
    /// about branch shapes the surface has no spelling for — an unreadable
    /// region is what the translation produces when a parse did *not* yield a
    /// pattern, so it cannot be written down as one.
    fn color_table() -> ConstructorTable
    {
        let mut table = ConstructorTable::new();
        table.declare(
            "Color".into(),
            ["Red", "Green", "Blue"]
                .into_iter()
                .map(|name| DeclaredConstructor {
                    name: String::from(name),
                    arity: 0,
                })
                .collect(),
        );
        table
    }

    /// Analyzes a `Color` match with the given branch shapes, against a
    /// scrutinee that decides nothing.
    ///
    /// The scrutinee is a variable, so every branch is undecided against it and
    /// nothing the branch statuses do can be mistaken for a coverage verdict:
    /// what these fixtures are about is what the branches say about each other
    /// and about the declaration.
    fn analyze_branches(shapes: Vec<PatternShape>) -> MatchAnalysis
    {
        let site = MatchSite {
            source_item: SourceItemId::from(0_usize),
            match_index: MatchIndex::from(0_usize),
            byte_range: SourceRange::from(0 .. 0),
            scrutinee: Value::var("c"),
            branches: shapes
                .into_iter()
                .map(|shape| BranchPattern {
                    shape,
                    byte_range: SourceRange::from(0 .. 0),
                })
                .collect(),
        };
        analyze_site(&site, &color_table(), SubmissionIndex::from(0_usize))
    }
}
