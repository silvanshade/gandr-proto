//! The phase-L1 corpus differential (`proposal-sequent-kernel.md` §9, phase L1
//! gate): `L-run ∘ 𝓕 ≡ run` over every lowered corpus program.
//!
//! Every top-level item of every model and pathological corpus program is run
//! on BOTH the CEK oracle (`gandr_core_checker::eval::run_comp`) and the L
//! machine (`gandr_core_sequent::machine::run_comp`), under the same empty
//! prelude. Wherever the L machine realizes the program (it does not report the
//! defined `UnsupportedByReference` sentinel it uses for the not-yet-built
//! surface — the prelude resolution, the higher-order combinators, and the
//! ADR-76 identity formers), the two must **agree** on the canonicalized
//! outcome; a disagreement is a genuine machine defect, never tolerated. The
//! effect / control surface realizes fully: every corpus `perform` (shell / ffi
//! / host lowering) is unhandled under the empty prelude, so both machines
//! agree on the `PerformNoHandler` blame; `shift` / `reset` are faithful on the
//! L machine, though the corpus carries no delimited-control syntax to exercise
//! them (the property differential does — `tests/differential.rs`).
//!
//! The items are read from the pre-lowered corpus fixtures
//! ([`crate::corpus_fixtures`]) rather than lowered live: the front-end that
//! lowers `.gandr` sources is outside the B1 machine-port scope, so its output
//! was captured once into the checked-in fixtures.
//!
//! # Empty-prelude reading (ADR-71 external oracle)
//!
//! Both machines run with the empty prelude — the differential compares the two
//! realizations of the SAME term, not the operator semantics (that needs the
//! prelude table, whose plumbing is a later checkpoint). So an operator program
//! whose `force` misses the prelude halts at `ForcedNonThunk` on BOTH machines
//! (a genuine agreement that the L machine handles the operator-force seam
//! identically), while a pure-data / codata program agrees on its real value.
//! The harness reports the coverage split so the growth across checkpoints is
//! visible; the assertion is that the realized subset agrees exactly.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the corpus differential readable \
                  (docs/workflow/rust.md)"
    )
)]

#[cfg(test)]
mod tests
{
    use gandr_core_checker::eval::Eval;
    use gandr_core_checker::eval::StuckReason;
    use gandr_core_checker::eval::run_comp;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Term;
    use gandr_core_sequent::boundary::CorpusItemCount;
    use gandr_core_sequent::boundary::UnsupportedFormerStatus;
    use gandr_core_sequent::differential::agree;
    use gandr_core_sequent::differential::canonical;
    use gandr_core_sequent::machine;

    use crate::corpus_fixtures::read_tree;

    /// `L-run ∘ 𝓕 ≡ run` on the realized subset of the model corpus.
    #[test]
    fn l_machine_agrees_with_the_oracle_on_the_model_corpus()
    {
        let outcome = sweep("model");
        eprintln!(
            "model corpus differential: {} of {} items realized on the L machine",
            outcome.realized, outcome.items
        );
        assert!(
            outcome.disagreements.is_empty(),
            "model corpus L≡run disagreements ({} of {} items realized):\n{}",
            outcome.realized,
            outcome.items,
            outcome.disagreements.join("\n")
        );
        // Realization ratchet: 69 (pure spine) → 81 (faithful `perform`) → 84 at
        // this effects / control (`shift`) checkpoint — the measured realization
        // over the current corpus with zero disagreements. The corpus expresses
        // no `shift` / `reset` (the surface carries no delimited-control syntax),
        // so faithful `shift` is gated by the property differential
        // (`tests/differential.rs`), not the corpus; this ratchet tightens the
        // floor to the actual realized count. The remaining declines are the
        // ADR-76 identity formers (whole-program), the prelude resolution, and
        // the higher-order combinators. Raise (never lower) as those land.
        assert!(
            usize::from(outcome.realized) >= 84,
            "the L machine's model-corpus realization regressed below the pinned \
             checkpoint floor (84), got {} of {}",
            outcome.realized,
            outcome.items
        );
    }

    /// `L-run ∘ 𝓕 ≡ run` on the realized subset of the pathological corpus.
    #[test]
    fn l_machine_agrees_with_the_oracle_on_the_pathological_corpus()
    {
        let outcome = sweep("pathological");
        eprintln!(
            "pathological corpus differential: {} of {} items realized on the L machine",
            outcome.realized, outcome.items
        );
        assert!(
            outcome.disagreements.is_empty(),
            "pathological corpus L≡run disagreements ({} of {} items realized):\n{}",
            outcome.realized,
            outcome.items,
            outcome.disagreements.join("\n")
        );
        // Realization ratchet, as for the model corpus: 32 → 33 (faithful
        // `perform`) → 34, the full pathological corpus (34 of 34) now realizing
        // with zero disagreements. The ADR-76 K-rejection witness's rejected
        // `case` item realizes — both machines agree on the holed arms; its
        // identity ANNOTATIONS are type-side only. Raise (never lower) as later
        // checkpoints land.
        assert!(
            usize::from(outcome.realized) >= 34,
            "the L machine's pathological-corpus realization regressed below the \
             pinned checkpoint floor (34), got {} of {}",
            outcome.realized,
            outcome.items
        );
    }

    /// Runs each fixture item on both machines, asserting agreement on the
    /// realized subset.
    fn sweep(tree: &str) -> Sweep
    {
        let fixtures = read_tree(tree);
        assert!(
            !fixtures.is_empty(),
            "the `{tree}` fixture tree must contain at least one fixture"
        );
        let mut outcome = Sweep::default();
        for fixture in &fixtures {
            for (index, term) in fixture.items.iter().enumerate() {
                outcome.items = usize::from(outcome.items).saturating_add(1).into();
                let comp = as_comp(term);
                let oracle = run_comp(comp.clone());
                let machine = machine::run_comp(&comp);
                if bool::from(is_unsupported(&machine)) {
                    // The L machine's not-yet-built surface (the prelude
                    // resolution, the higher-order combinators, and the ADR-76
                    // identity formers) declines here; that is not a
                    // disagreement.
                    continue;
                }
                outcome.realized = usize::from(outcome.realized).saturating_add(1).into();
                if !bool::from(agree(&oracle, &machine)) {
                    outcome.disagreements.push(format!(
                        "{} item {index}: oracle {:?} vs L {:?}",
                        fixture.source,
                        canonical(&oracle),
                        canonical(&machine)
                    ));
                }
            }
        }
        outcome
    }

    /// The outcome of sweeping one corpus tree through the differential.
    #[derive(Default)]
    struct Sweep
    {
        /// The number of top-level items compared.
        items: CorpusItemCount,
        /// The number the L machine realizes (does not decline).
        realized: CorpusItemCount,
        /// Per-item disagreements (a genuine defect).
        disagreements: Vec<String>,
    }

    /// A top-level term as a computation (a value item runs as `ret v`,
    /// matching how a bare value's terminal reads back).
    fn as_comp(term: &Term) -> Comp
    {
        match *term {
            | Term::Comp(ref comp) => comp.clone(),
            | Term::Value(ref value) => Comp::ret(value.clone()),
            // A future top-level sort runs as an opaque hole (both machines
            // agree on the blame).
            | _ => Comp::hole(0),
        }
    }

    /// Whether an outcome is the L machine's "not yet built" sentinel — the one
    /// defined stuck the CEK's `run` never produces (it is a reference-only
    /// `eval_comp` outcome), so it unambiguously marks a declined program.
    fn is_unsupported(eval: &Eval) -> UnsupportedFormerStatus
    {
        matches!(*eval, Eval::Stuck(StuckReason::UnsupportedByReference)).into()
    }
}
