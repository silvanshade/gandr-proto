//! The phase-L0 gate (`proposal-sequent-kernel.md` §9, phase L0): the static
//! focusing translation `𝓕` is **total on the corpus**.
//!
//! Every top-level item of every model and pathological corpus program is run
//! through [`gandr_core_sequent::focus_term`]. For all of them the translation
//! must (a) not panic and (b) produce a command whose typed-IL well-formedness
//! holds ([`gandr_core_sequent::wellformed`]) with **no free covariables** —
//! the `𝓕`-only-entry invariant, since `★` is the only top-level continuation
//! and every minted covariable is bound.
//!
//! The items are lowered live from the ported source corpus
//! ([`crate::corpus_sources`]). The surface tree remains excluded on purpose:
//! it is firewalled from execution and never lowers.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the gate walker readable \
                  (docs/workflow/rust.md)"
    )
)]

/// The phase-L0 totality gate.
#[cfg(test)]
mod tests
{
    use gandr_core_sequent::boundary::CorpusFileCount;
    use gandr_core_sequent::boundary::CorpusItemCount;
    use gandr_core_sequent::focus_term;
    use gandr_core_sequent::wellformed;

    use crate::corpus_sources::read_tree;

    /// `𝓕` is total on the model corpus tree.
    #[test]
    fn focusing_is_total_on_the_model_corpus()
    {
        let outcome = sweep("model");
        eprintln!(
            "model 𝓕-totality witness: {} fixtures, {} items focused",
            outcome.files, outcome.items
        );
        assert!(
            outcome.failures.is_empty(),
            "model corpus 𝓕 failures ({} fixtures, {} items):\n{}",
            outcome.files,
            outcome.items,
            outcome.failures.join("\n")
        );
        assert!(
            usize::from(outcome.items) >= 20,
            "the model sweep must cover a substantial item count, got {} (from {} fixtures)",
            outcome.items,
            outcome.files
        );
    }

    /// `𝓕` is total on the pathological corpus tree.
    #[test]
    fn focusing_is_total_on_the_pathological_corpus()
    {
        let outcome = sweep("pathological");
        eprintln!(
            "pathological 𝓕-totality witness: {} fixtures, {} items focused",
            outcome.files, outcome.items
        );
        assert!(
            outcome.failures.is_empty(),
            "pathological corpus 𝓕 failures ({} fixtures, {} items):\n{}",
            outcome.files,
            outcome.items,
            outcome.failures.join("\n")
        );
        assert!(
            usize::from(outcome.items) >= 10,
            "the pathological sweep must cover a substantial item count, got {} (from {} fixtures)",
            outcome.items,
            outcome.files
        );
    }

    /// The outcome of sweeping one corpus tree's fixtures through `𝓕`.
    struct Sweep
    {
        /// The number of fixtures walked.
        files: CorpusFileCount,
        /// The number of top-level items focused.
        items: CorpusItemCount,
        /// Per-item failures (a focusing error or a well-formedness violation).
        failures: Vec<String>,
    }

    /// Focuses every fixture item in `tree`, checking the result is well-formed
    /// with no free covariables.
    fn sweep(tree: &str) -> Sweep
    {
        let fixtures = read_tree(tree);
        assert!(
            !fixtures.is_empty(),
            "the `{tree}` fixture tree must contain at least one fixture"
        );
        let mut outcome = Sweep {
            files: fixtures.len().into(),
            items: 0_usize.into(),
            failures: Vec::new(),
        };
        for fixture in &fixtures {
            for (index, term) in fixture.items.iter().enumerate() {
                outcome.items = usize::from(outcome.items).saturating_add(1).into();
                let focused = match focus_term(term) {
                    | Ok(focused) => focused,
                    | Err(error) => {
                        outcome.failures.push(format!(
                            "{} item {index}: focusing failed: {error}",
                            fixture.source
                        ));
                        continue;
                    },
                };
                match wellformed(focused.arena(), focused.root()) {
                    | Ok(frees) => {
                        if !frees.covars.is_empty() {
                            outcome.failures.push(format!(
                                "{} item {index}: focused command has free covariables {:?}",
                                fixture.source, frees.covars
                            ));
                        }
                    },
                    | Err(error) => {
                        outcome.failures.push(format!(
                            "{} item {index}: typed-IL check failed: {error}",
                            fixture.source
                        ));
                    },
                }
            }
        }
        outcome
    }
}
