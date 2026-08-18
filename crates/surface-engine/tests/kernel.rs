//! Kernel admission tests: the session's crossing from checked core into the
//! certified kernel — which surface definitions the bridge lowers, which the
//! choke point admits, how a later definition reaches an earlier one, and the
//! fact that a definition the kernel turns away changes nothing else about the
//! session that submitted it.
//!
//! The environment a session accumulates is exercised through the export
//! pipeline as well as through the verdicts, because a declaration that admits
//! but does not survive `write` → `read` would leave the crossing wired and
//! useless.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Kernel admission tests.
#[cfg(test)]
mod tests
{
    use gandr_core_checker::kernel_bridge::BridgeRejection;
    use gandr_core_term::syntax::Term;
    use gandr_core_term::syntax::Value;
    use gandr_core_term::types::Ty;
    use gandr_core_term::types::ValueType;
    use gandr_kernel_core::read;
    use gandr_kernel_core::write;
    use gandr_surface_engine::boundary::DefinitionName;
    use gandr_surface_engine::kernel::AdmittedCount;
    use gandr_surface_engine::kernel::DefinitionOffer;
    use gandr_surface_engine::kernel::KernelAdmissions;
    use gandr_surface_engine::kernel::KernelVerdict;
    use gandr_surface_engine::kernel::WithheldReason;
    use gandr_surface_engine::session::Session;
    use gandr_surface_engine::session::Submission;

    use crate::common::TestCount;
    use crate::common::TestText;

    /// A value definition of S1-eligible shape crosses the choke point and
    /// enters the environment at the first admission index.
    #[test]
    fn a_value_definition_is_admitted()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def x = 3;");
        assert_positions(&submission, &[TestCount(0)]);
        assert_eq!(
            AdmittedCount::from(1),
            session.kernel().admitted(),
            "one definition admitted, so the ledger holds one declaration"
        );
    }

    /// A function definition — a thunked lambda, which is how the surface
    /// spells a function — is S1-eligible and admits.
    #[test]
    fn a_function_definition_is_admitted()
    {
        let mut session = Session::new();
        let submission = submit(
            &mut session,
            "def id(x: Integer) -> F Integer {\n  ret x\n}",
        );
        assert_positions(&submission, &[TestCount(0)]);
    }

    /// A later definition naming an earlier one lowers to a kernel constant
    /// through the bridge's naming environment, so both admit and the second
    /// rests on the first.
    ///
    /// This is the path a single-item harness cannot reach: its naming
    /// environment is always empty, so every free name is unbound there.
    #[test]
    fn a_later_definition_refers_to_an_earlier_one()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def a = 3;\ndef b = a;");
        assert_positions(&submission, &[TestCount(0), TestCount(1)]);
        assert_eq!(
            AdmittedCount::from(2),
            session.kernel().admitted(),
            "both definitions admitted, the second through the constant map"
        );
    }

    /// The naming environment persists across submissions, so a definition
    /// entered on a later line still resolves a name bound on an earlier one.
    #[test]
    fn the_naming_environment_survives_across_submissions()
    {
        let mut session = Session::new();
        let first = submit(&mut session, "def a = 3;");
        assert_positions(&first, &[TestCount(0)]);
        let second = submit(&mut session, "def b = a;");
        assert_positions(&second, &[TestCount(1)]);
    }

    /// A definition whose type is outside the S1 base stock is reported as
    /// having no S1 image, and nothing enters the environment.
    #[test]
    fn an_out_of_s1_definition_is_reported_outside_s1()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def x = 1u32;");
        let verdict = only_verdict(&submission);
        assert!(
            matches!(*verdict, KernelVerdict::OutsideS1 { .. }),
            "a machine-numeric atom is outside the S1 stock, got {verdict:?}"
        );
        assert_eq!(
            AdmittedCount::default(),
            session.kernel().admitted(),
            "a definition with no S1 image admits nothing"
        );
    }

    /// The unknown type has no S1 image on either sort, and the bridge
    /// reports each sort's rejection by name (gandr-89k): a `?` atom in a
    /// computation position (the computation top) rejects as
    /// `UnknownComputationType`, while the bare `?` ascription — the value
    /// unknown, as is the legacy `Unknown` keyword it normalizes with —
    /// rejects as `UnknownValueType`.
    #[test]
    fn unknown_ascriptions_reject_by_sort()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def g(x: Integer) -> ? { ret x }");
        assert!(
            matches!(only_verdict(&submission), KernelVerdict::OutsideS1 {
                rejection: BridgeRejection::UnknownComputationType,
            }),
            "a computation-top result has no S1 image, got {:?}",
            submission.kernel
        );

        let mut session = Session::new();
        let submission = submit(&mut session, "def h : ?; def h = 1;");
        let last = submission.kernel.last().expect("the pair carries verdicts");
        assert!(
            matches!(last, KernelVerdict::OutsideS1 {
                rejection: BridgeRejection::UnknownValueType,
            }),
            "the value unknown has no S1 image, got {:?}",
            submission.kernel
        );
    }

    /// An expression item declares nothing, so it is withheld rather than
    /// reported as a kernel failure.
    #[test]
    fn an_expression_is_withheld()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "ret 7");
        assert_withheld(&submission, WithheldReason::Expression);
    }

    /// An item that fails to type has no declared type to admit against.
    #[test]
    fn an_untyped_item_is_withheld()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def x = undefined_name;");
        assert_withheld(&submission, WithheldReason::Untyped);
    }

    /// An item carrying a hole is declined before a type exists.
    #[test]
    fn a_holey_item_is_withheld()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def q = ?;");
        assert_withheld(&submission, WithheldReason::Holey);
    }

    /// A computation definition of returner type is reported at its payload's
    /// value type, which does not determine the returner's effect row, so the
    /// item is withheld rather than offered under a guessed row.
    #[test]
    fn a_returner_definition_is_withheld()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def total = 1 + 2 * 3;");
        assert_withheld(&submission, WithheldReason::IndeterminateDeclaredType);
    }

    /// The kernel's naming environment is strictly narrower than the session's
    /// typing context, and a body can reach the gap between them.
    ///
    /// A withheld definition still binds its name for the session — the item
    /// typed, so the session records it — while the kernel adds a constant only
    /// for a definition that *admitted*. A later body naming it therefore types
    /// against a bound name and reaches the bridge against a free one, which is
    /// the only way a session drives the bridge's unresolved-name rejection: a
    /// name no prior declaration binds at all fails typing first and never
    /// reaches [`KernelAdmissions::offer`].
    #[test]
    fn a_body_naming_a_withheld_definition_has_no_s1_image()
    {
        let mut session = Session::new();
        let withheld = submit(&mut session, "def total = 1 + 2 * 3;");
        assert_withheld(&withheld, WithheldReason::IndeterminateDeclaredType);

        let submission = submit(&mut session, "def again = total;");
        let verdict = only_verdict(&submission);
        assert!(
            matches!(*verdict, KernelVerdict::OutsideS1 { .. }),
            "the session binds `total` and the kernel does not, so the bridge \
             finds the name free, got {verdict:?}"
        );
        assert_eq!(
            AdmittedCount::default(),
            session.kernel().admitted(),
            "neither definition admitted, so the environment is still empty"
        );
    }

    /// The choke point grants the bridge no credence: a definition that lowers
    /// cleanly but whose body does not inhabit its declared type is refused,
    /// and the environment is left as it was.
    #[test]
    fn the_choke_point_refuses_a_mistyped_definition()
    {
        let mut ledger = KernelAdmissions::new();
        let term = Term::Value(Value::Int(3));
        let ty = Ty::Value(ValueType::Atom(String::from("String")));
        let verdict = ledger.offer(DefinitionOffer {
            name: DefinitionName::from("mistyped"),
            term: &term,
            ty: &ty,
        });
        assert!(
            matches!(verdict, KernelVerdict::Refused { .. }),
            "an integer does not inhabit `String`, so the kernel refuses it, got {verdict:?}"
        );
        assert_eq!(
            AdmittedCount::default(),
            ledger.admitted(),
            "a refused declaration admits nothing"
        );
    }

    /// A definition the kernel turns away leaves the environment untouched, so
    /// the next admitted definition still takes the first admission index.
    #[test]
    fn a_rejected_definition_leaves_the_environment_unchanged()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def wide = 1u32;\ndef narrow = 3;");
        let verdicts = &submission.kernel;
        let Some((first, rest)) = verdicts.split_first()
        else {
            panic!("expected two verdicts, got {verdicts:?}");
        };
        assert!(
            matches!(*first, KernelVerdict::OutsideS1 { .. }),
            "the machine-numeric definition has no S1 image, got {first:?}"
        );
        assert_eq!(
            &[KernelVerdict::Admitted {
                position: 0_usize.into(),
            }][..],
            rest,
            "the admitted definition takes position zero, so the rejected one \
             left nothing behind"
        );
    }

    /// A rejection that surfaces after the lowering has already minted nodes —
    /// the pair type's `Integer` half is minted before the machine-numeric
    /// half is rejected — leaves the environment byte-identical to one that
    /// never saw the rejected definition: the staged builder's rollback
    /// reclaims the partial content the choke point never reached.
    #[test]
    fn a_partially_lowered_rejection_leaves_no_arena_content()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def wide = (3, 1u32);\ndef narrow = 3;");
        let verdicts = &submission.kernel;
        let Some((first, rest)) = verdicts.split_first()
        else {
            panic!("expected two verdicts, got {verdicts:?}");
        };
        assert!(
            matches!(*first, KernelVerdict::OutsideS1 { .. }),
            "the pair's machine-numeric half has no S1 image, got {first:?}"
        );
        assert_eq!(
            &[KernelVerdict::Admitted {
                position: 0_usize.into(),
            }][..],
            rest,
            "the admitted definition takes position zero, so the partial lowering \
             left nothing behind"
        );

        let mut clean = Session::new();
        let clean_submission = submit(&mut clean, "def narrow = 3;");
        assert_eq!(
            &[KernelVerdict::Admitted {
                position: 0_usize.into(),
            }][..],
            clean_submission.kernel,
            "the clean session admits the same definition at position zero"
        );
        assert_eq!(
            write(session.kernel().environment()),
            write(clean.kernel().environment()),
            "the environment is byte-identical to one that never saw the rejected \
             definition"
        );
    }

    /// A session's own definitions produce a kernel artifact that round-trips
    /// byte-identically through the export format — the crossing reaching the
    /// persisted form rather than stopping at the choke point.
    #[test]
    fn a_session_environment_round_trips_through_the_export_format()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def a = 3;\ndef b = a;\ndef c = (a, b);");
        assert_positions(&submission, &[TestCount(0), TestCount(1), TestCount(2)]);
        let bytes = write(session.kernel().environment());
        let reread = read(bytes.as_ref().into()).expect("an admitted environment replays");
        assert_eq!(
            bytes,
            write(&reread),
            "a session's kernel artifact round-trips byte-identically"
        );
    }

    /// Every lowered item gets exactly one verdict, in the same source order as
    /// its outcome — the alignment a consumer reads the two by.
    #[test]
    fn every_item_carries_exactly_one_verdict()
    {
        let mut session = Session::new();
        let submission = submit(&mut session, "def a = 3;\nret a\ndef q = ?;");
        assert_eq!(
            submission.outcomes.len(),
            submission.kernel.len(),
            "one verdict per outcome, got {:?} against {:?}",
            submission.kernel,
            submission.outcomes
        );
        assert_eq!(
            3,
            submission.kernel.len(),
            "three items submitted, got {:?}",
            submission.kernel
        );
    }

    /// Submits one source and returns the submission, failing the test on an
    /// infrastructure lowering failure.
    fn submit<'source, S>(
        session: &mut Session,
        source: S,
    ) -> Submission
    where
        S: Into<TestText<'source>>,
    {
        session
            .submit(source.into().0)
            .expect("lowering must not fail")
    }

    /// The submission's single verdict, or a test failure naming what arrived.
    fn only_verdict(submission: &Submission) -> &KernelVerdict
    {
        let Some((verdict, rest)) = submission.kernel.split_first()
        else {
            panic!("expected one verdict, got []");
        };
        assert!(
            rest.is_empty(),
            "expected exactly one verdict, got {:?}",
            submission.kernel
        );
        verdict
    }

    /// Asserts the submission admitted exactly the given admission positions,
    /// in order.
    fn assert_positions(
        submission: &Submission,
        positions: &[TestCount],
    )
    {
        let admitted: Vec<KernelVerdict> = positions
            .iter()
            .map(|position| KernelVerdict::Admitted {
                position: position.0.into(),
            })
            .collect();
        assert_eq!(
            admitted, submission.kernel,
            "expected every item to admit at its own position"
        );
    }

    /// Asserts the submission's single item was withheld for `reason`.
    fn assert_withheld(
        submission: &Submission,
        reason: WithheldReason,
    )
    {
        assert_eq!(
            &KernelVerdict::Withheld { reason },
            only_verdict(submission),
            "expected the item to be withheld as {reason:?}"
        );
    }
}
