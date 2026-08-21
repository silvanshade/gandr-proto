//! Integration witnesses for budgeted completion.
//!
//! These are integration tests rather than unit tests because two of them read
//! the certificate normal form, which sits one crate above this one: an inline
//! test build is a distinct crate instance, so a dev-dependency cycle cannot
//! unify the types across it while an integration target links the library and
//! does.

#[cfg(test)]
mod tests
{
    extern crate alloc;

    use alloc::vec::Vec;

    use gandr_core_sequent::il::Polarity;
    use gandr_theory_cell_complexes::alphabet::CellAlphabet as _;
    use gandr_theory_cell_complexes::cell::Cell;
    use gandr_theory_cell_complexes::cell::CellId;
    use gandr_theory_cell_complexes::cell::CellStore;
    use gandr_theory_cell_complexes::pattern::CmdPat;
    use gandr_theory_cell_complexes::pattern::ConsPat;
    use gandr_theory_cell_complexes::pattern::ProdPat;
    use gandr_theory_cell_complexes::pattern::Sym;
    use gandr_theory_cell_complexes::sequent::CellProvenance;
    use gandr_theory_cell_complexes::sequent::Orientation;
    use gandr_theory_cell_complexes::sequent::SequentAlphabet;
    use gandr_theory_cell_complexes::subst::Subst;
    use gandr_theory_coherent_resolutions::completion::*;
    use gandr_theory_coherent_resolutions::overlap::Overlap;
    use gandr_theory_coherent_resolutions::overlap::OverlapKind;

    #[test]
    fn completion_processes_within_budget()
    {
        let outcome = complete(
            overlapping_rules(),
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        assert!(
            bool::from(outcome.is_completed()),
            "the small system completes within budget"
        );
    }
    #[test]
    fn supplied_overlap_validation_returns_typed_declines()
    {
        let store = overlapping_rules();
        let valid = scheduled_confluence_batches(&store)
            .into_iter()
            .flatten()
            .next()
            .expect("the fixture supplies one confluence overlap");
        let mut unknown = valid.clone();
        unknown.left = CellId(usize::MAX);
        let expected_unknown = unknown.clone();
        let unknown_outcome = complete_with_overlap_source(
            store.clone(),
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
            move |_| alloc::vec![alloc::vec![unknown]],
        );
        let CompletionOutcome::Declined {
            reason:
                DeclineReason::InvalidSuppliedOverlap(SuppliedOverlapError::UnknownLeftCell {
                    batch,
                    overlap,
                    cell,
                }),
            pending,
            ..
        } = unknown_outcome
        else {
            panic!("a stale supplied left id is a typed decline")
        };
        assert_eq!(0_usize, batch);
        assert_eq!(0_usize, overlap);
        assert_eq!(CellId(usize::MAX), cell);
        assert_eq!(1_usize, pending.len());
        assert_eq!(expected_unknown, pending[0][0]);

        let mut unknown_right = valid.clone();
        unknown_right.right = CellId(usize::MAX);
        let expected_unknown_right = unknown_right.clone();
        let unknown_right_outcome = complete_with_overlap_source(
            store.clone(),
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
            move |_| alloc::vec![alloc::vec![unknown_right]],
        );
        let CompletionOutcome::Declined {
            reason:
                DeclineReason::InvalidSuppliedOverlap(SuppliedOverlapError::UnknownRightCell {
                    batch,
                    overlap,
                    cell,
                }),
            pending,
            ..
        } = unknown_right_outcome
        else {
            panic!("a stale supplied right id is a typed decline")
        };
        assert_eq!(0_usize, batch);
        assert_eq!(0_usize, overlap);
        assert_eq!(CellId(usize::MAX), cell);
        assert_eq!(1_usize, pending.len());
        assert_eq!(expected_unknown_right, pending[0][0]);

        let mut non_confluence = valid;
        non_confluence.kind = OverlapKind::Composition;
        let non_confluence_outcome = complete_with_overlap_source(
            store,
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
            move |_| alloc::vec![alloc::vec![non_confluence]],
        );
        let CompletionOutcome::Declined {
            reason:
                DeclineReason::InvalidSuppliedOverlap(SuppliedOverlapError::NonConfluence {
                    batch,
                    overlap,
                }),
            ..
        } = non_confluence_outcome
        else {
            panic!("a supplied composition is a typed decline")
        };
        assert_eq!(0_usize, batch);
        assert_eq!(0_usize, overlap);
    }
    // These are separate witnesses because adding this error variant widens
    // the invalid-supplied domain: initial validation, terminal resume, and
    // budget-decline revalidation each need their own path.

    #[test]
    fn supplied_non_unifying_decline_is_typed()
    {
        let store = overlapping_rules();
        let mut non_unifying = scheduled_confluence_batches(&store)
            .into_iter()
            .flatten()
            .next()
            .expect("the fixture supplies one confluence overlap");
        non_unifying.unifier = Subst::new();
        let expected = non_unifying.clone();
        let outcome = complete_with_overlap_source(
            store,
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
            move |_| alloc::vec![alloc::vec![non_unifying]],
        );
        let CompletionOutcome::Declined {
            reason:
                DeclineReason::InvalidSuppliedOverlap(SuppliedOverlapError::NonUnifyingSubstitution {
                    batch,
                    overlap,
                }),
            derived,
            certificates,
            pending,
            ..
        } = outcome
        else {
            panic!("a supplied non-unifying substitution is a typed decline")
        };
        assert!(
            derived.is_empty(),
            "validation declines before derived work"
        );
        assert!(
            certificates.is_empty(),
            "validation declines before certificates are emitted"
        );
        assert_eq!(0_usize, batch);
        assert_eq!(0_usize, overlap);
        assert_eq!(expected, pending[0][0]);
    }

    #[test]
    fn non_unifying_supplied_decline_is_terminal_on_resume()
    {
        let store = overlapping_rules();
        let mut non_unifying = scheduled_confluence_batches(&store)
            .into_iter()
            .flatten()
            .next()
            .expect("the fixture supplies one confluence overlap");
        non_unifying.unifier = Subst::new();
        let outcome = complete_with_overlap_source(
            store,
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
            move |_| alloc::vec![alloc::vec![non_unifying]],
        );
        let resumed = outcome.clone().resume(CompletionBudget::new(
            4_096_usize.into(),
            4_096_usize.into(),
            4_096_usize.into(),
        ));
        assert_eq!(
            outcome, resumed,
            "a non-unifying supplied decline remains a typed terminal refusal"
        );
    }

    #[test]
    fn budget_decline_revalidates_non_unifying_pending_overlap()
    {
        let store = overlapping_rules();
        let mut non_unifying = scheduled_confluence_batches(&store)
            .into_iter()
            .flatten()
            .next()
            .expect("the fixture supplies one confluence overlap");
        non_unifying.unifier = Subst::new();
        let expected = non_unifying.clone();
        let outcome = CompletionOutcome::Declined {
            store,
            derived: Vec::new(),
            certificates: Vec::new(),
            pending: alloc::vec![alloc::vec![non_unifying]],
            reason: DeclineReason::StepBudget,
        };
        let resumed = outcome.resume(CompletionBudget::new(
            4_096_usize.into(),
            4_096_usize.into(),
            4_096_usize.into(),
        ));
        let CompletionOutcome::Declined {
            reason:
                DeclineReason::InvalidSuppliedOverlap(SuppliedOverlapError::NonUnifyingSubstitution {
                    batch,
                    overlap,
                }),
            pending,
            ..
        } = resumed
        else {
            panic!("resume revalidates malformed pending work before completion")
        };
        assert_eq!(0_usize, batch);
        assert_eq!(0_usize, overlap);
        assert_eq!(expected, pending[0][0]);
    }

    #[test]
    fn invalid_supplied_decline_is_terminal_on_resume()
    {
        let store = overlapping_rules();
        let mut unknown = scheduled_confluence_batches(&store)
            .into_iter()
            .flatten()
            .next()
            .expect("the fixture supplies one confluence overlap");
        unknown.right = CellId(usize::MAX);
        let outcome = complete_with_overlap_source(
            store,
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
            move |_| alloc::vec![alloc::vec![unknown]],
        );
        let resumed = outcome.clone().resume(CompletionBudget::new(
            4_096_usize.into(),
            4_096_usize.into(),
            4_096_usize.into(),
        ));
        assert_eq!(
            outcome, resumed,
            "invalid supplied input remains a typed terminal refusal"
        );
    }

    #[test]
    fn every_generated_certificate_matches_its_replay_plan()
    {
        let outcome = complete(
            overlapping_rules(),
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        let CompletionOutcome::Completed {
            store,
            certificates,
            ..
        } = outcome
        else {
            panic!("the generated fixture completes within budget");
        };
        assert!(
            !certificates.is_empty(),
            "the completion fixture emits a generated certificate family"
        );
        assert_eq!(
            certificates.len(),
            1,
            "the generated fixture emits its exact one-certificate family"
        );
        for certificate in certificates {
            let witness_a = gandr_theory_deep_inference::normal_form::normalize_certified(
                &store,
                &certificate.overlap.peak,
                &certificate.joins_at,
                &certificate.path_a,
            )
            .expect("every generated certificate path_a replays");
            let witness_b = gandr_theory_deep_inference::normal_form::normalize_certified(
                &store,
                &certificate.overlap.peak,
                &certificate.joins_at,
                &certificate.path_b,
            )
            .expect("every generated certificate path_b replays");
            assert_eq!(
                witness_a.normal_form().joins_at,
                witness_b.normal_form().joins_at,
                "both generated certificate paths reach the same join"
            );
            let plan_a = witness_a.replay_plan();
            let plan_b = witness_b.replay_plan();
            // The two legs of a confluence certificate are different
            // derivations of one boundary — one fires the left cell and
            // whatever normalizes its reduct, the other the right cell and
            // whatever normalizes its own — so their plans schedule different
            // cells. The shared invariant is the join they replay to.
            assert_ne!(
                plan_a.levels(),
                plan_b.levels(),
                "the two certificate legs are different derivations of one boundary"
            );
            let planned_a = plan_a
                .replay_with_fuel(&store, plan_a.critical_path())
                .expect("planned path_a replay does not obstruct")
                .expect("critical-path fuel completes path_a");
            let planned_b = plan_b
                .replay_with_fuel(&store, plan_b.critical_path())
                .expect("planned path_b replay does not obstruct")
                .expect("critical-path fuel completes path_b");
            assert_eq!(
                planned_a, planned_b,
                "planned replay agrees for both generated certificate paths"
            );
            // A plan replays the *skolemized* peak, so it lands on the
            // skolemized join: the certified join still carries the critical
            // pair's metavariables.
            let join = SequentAlphabet::skolemize(&certificate.joins_at);
            assert_eq!(
                join, planned_a,
                "planned replay matches path_a for every generated certificate"
            );
            assert_eq!(
                join, planned_b,
                "planned replay matches path_b for every generated certificate"
            );
        }
    }

    #[test]
    fn cell_budget_decline_preserves_pending_work()
    {
        // The cell ceiling and the step ceiling are reached at the same point
        // in this fixture — the leading batch's second overlap, the first one
        // that diverges — so the two declines must carry the same structural
        // residue and differ only in the reason they carry.
        let scheduled = scheduled_confluence_batches(&independent_rule_clusters());
        let outcome = complete(
            independent_rule_clusters(),
            CompletionBudget::new(64_usize.into(), 6_usize.into(), 64_usize.into()),
        );
        match outcome {
            | CompletionOutcome::Declined {
                reason,
                ref pending,
                ref derived,
                ref certificates,
                ..
            } => {
                assert_eq!(
                    DeclineReason::CellBudget,
                    reason,
                    "the cell ceiling, not the step ceiling, is what stopped this run"
                );
                assert!(
                    derived.is_empty(),
                    "the ceiling is reached before the divergence is oriented, so nothing was \
                     derived"
                );
                assert_eq!(
                    1_usize,
                    certificates.len(),
                    "the joinable leading pair was certified before the ceiling was reached"
                );
                assert_eq!(
                    residue_after_leading_step(&scheduled),
                    *pending,
                    "the cell ceiling preserves the remainder of the leading batch and every \
                     later batch unchanged"
                );
            },
            | CompletionOutcome::Completed { .. } => {
                panic!("the cell ceiling must decline before inserting a derived rule")
            },
        }
    }

    #[test]
    fn decline_resume_matches_uninterrupted_completion()
    {
        let scheduled = scheduled_confluence_batches(&independent_rule_clusters());
        assert_eq!(
            2_usize,
            scheduled.len(),
            "the fixture schedules its six critical pairs into two independent batches"
        );
        assert!(
            scheduled
                .first()
                .is_some_and(|batch| batch.len() == 3_usize),
            "the leading batch holds three independent overlaps, so a one-step budget stops \
             inside it rather than between batches"
        );
        let uninterrupted = complete(
            independent_rule_clusters(),
            CompletionBudget::new(64_usize.into(), 64_usize.into(), 64_usize.into()),
        );
        let declined = complete(
            independent_rule_clusters(),
            CompletionBudget::new(1_usize.into(), 64_usize.into(), 64_usize.into()),
        );
        match declined {
            | CompletionOutcome::Declined {
                reason,
                ref pending,
                ref derived,
                ..
            } => {
                assert_eq!(
                    DeclineReason::StepBudget,
                    reason,
                    "the step ceiling is what stopped this run"
                );
                assert!(
                    derived.is_empty(),
                    "the one step taken was the joinable leading pair, which derives nothing"
                );
                // Exactly one step was taken, so the residue is the leading
                // batch minus its first member, then every later batch
                // untouched. A partition that drops the overlap the ceiling
                // interrupted, or that drops the unfinished batch wholesale,
                // fails here.
                assert_eq!(
                    residue_after_leading_step(&scheduled),
                    *pending,
                    "the decline carries the unprocessed remainder of the leading batch and \
                     every later batch unchanged"
                );
            },
            | CompletionOutcome::Completed { .. } => {
                panic!("a one-step budget must decline inside the leading batch")
            },
        }
        let resumed = declined.resume(CompletionBudget::new(
            64_usize.into(),
            64_usize.into(),
            64_usize.into(),
        ));
        assert!(
            bool::from(resumed.is_completed()),
            "resuming from the carried batches finishes the whole worklist"
        );
        assert_eq!(
            uninterrupted, resumed,
            "and reaches the uninterrupted outcome exactly: same store, same derived cells, same \
             certificates"
        );
    }

    #[test]
    fn a_starved_budget_declines_with_pending()
    {
        let initial = overlapping_rules();
        let expected = scheduled_confluence_batches(&initial);
        let outcome = complete(
            initial,
            CompletionBudget::new(0_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        match outcome {
            | CompletionOutcome::Declined {
                reason, pending, ..
            } => {
                assert_eq!(
                    DeclineReason::StepBudget,
                    reason,
                    "the step ceiling was zero"
                );
                assert_eq!(
                    expected, pending,
                    "budget decline preserves batch and first-appearance order"
                );
            },
            | CompletionOutcome::Completed { .. } => {
                panic!("a zero step budget must decline")
            },
        }
    }

    /// Two rules that overlap on `⟨Zero | f(α)⟩` with divergent right-hand
    /// sides — a genuine critical pair completion must resolve.
    fn overlapping_rules() -> CellStore
    {
        // r1: ⟨Zero | f(α)⟩ ~> ⟨Zero | α⟩
        let r1 = Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("f", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        );
        // r2: ⟨x | f(α)⟩ ~> ⟨x | g(α)⟩ (a broader rule overlapping r1 at x=Zero)
        let r2 = Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::op("f", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::op("g", [], ConsPat::meta("alpha")),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        );
        let mut store = CellStore::new();
        store.insert(r1);
        store.insert(r2);
        store
    }
    /// A rule `⟨K | op(α)⟩ ~> ⟨K | rhs⟩` over a nullary constructor `K`.
    fn ground_rule(
        ctor: &Sym,
        op: &Sym,
        rhs: ConsPat,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor(ctor.as_ref(), []),
                ConsPat::op(op.as_ref(), [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::ctor(ctor.as_ref(), []), rhs),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// A rule `⟨binder | op(α)⟩ ~> ⟨binder | rhs⟩` over a producer
    /// metavariable.
    fn schematic_rule(
        binder: &Sym,
        op: &Sym,
        rhs: ConsPat,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(binder.as_ref()),
                ConsPat::op(op.as_ref(), [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::meta(binder.as_ref()), rhs),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// Three two-rule clusters over the disjoint operations `f`, `g` and `h`.
    ///
    /// Each cluster overlaps on its own operation and on nothing else, so the
    /// six critical pairs schedule into two batches of three. The leading
    /// cluster's pair joins outright (both rules reduce to `p`); the other two
    /// diverge by size and orient into a derived cell each — so a budget that
    /// stops after one step stops inside the leading batch, before anything is
    /// derived.
    fn independent_rule_clusters() -> CellStore
    {
        let reduced = ConsPat::op("p", [], ConsPat::meta("alpha"));
        let wrapped = ConsPat::op("q", [], ConsPat::op("p", [], ConsPat::meta("alpha")));
        let (f, g, h) = (Sym::new("f"), Sym::new("g"), Sym::new("h"));
        let mut store = CellStore::new();
        store.insert(ground_rule(&Sym::new("Zero"), &f, reduced.clone()));
        store.insert(schematic_rule(&Sym::new("x"), &f, reduced.clone()));
        store.insert(ground_rule(&Sym::new("Nil"), &g, reduced.clone()));
        store.insert(schematic_rule(&Sym::new("y"), &g, wrapped.clone()));
        store.insert(ground_rule(&Sym::new("Unit"), &h, reduced));
        store.insert(schematic_rule(&Sym::new("z"), &h, wrapped));
        store
    }

    /// The pending residue a decline one step into the leading batch must
    /// carry: that batch's unprocessed remainder, then every later batch
    /// unchanged.
    fn residue_after_leading_step(scheduled: &[Vec<Overlap>]) -> Vec<Vec<Overlap>>
    {
        let mut residue = Vec::with_capacity(scheduled.len());
        residue.push(
            scheduled
                .first()
                .expect("the schedule has a leading batch")
                .get(1 ..)
                .expect("the leading batch has a processed head")
                .to_vec(),
        );
        residue.extend(scheduled.iter().skip(1).cloned());
        residue
    }
}
