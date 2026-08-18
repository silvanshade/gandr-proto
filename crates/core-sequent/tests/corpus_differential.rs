//! The phase-L1 corpus **outcome-snapshot** sweep (`proposal-sequent-kernel.md`
//! §9, phase L1 gate; coordinator decision D3): `canonical(L-run ∘ 𝓕)` over
//! every lowered corpus program must match a checked-in expected-outcome
//! record.
//!
//! The snapshot carries the guarantee a second implementation used to carry.
//! This sweep was once an agreement differential against a CEK evaluator that
//! ran beside the L machine and shared no step code with it; that evaluator is
//! retired and removed (B1 phase-3 stage F). What replaced it is the outcome
//! the two agreed on, frozen into a checked-in record the L machine must keep
//! reproducing exactly. **The frozen snapshot is the oracle**, so a diff here
//! is a change in observable behaviour and never a disagreement between two
//! live implementations.
//!
//! # What is asserted
//!
//! For every corpus item the sweep asserts:
//!
//! - **realization** — the L machine realizes the item (it does not report the
//!   defined `UnsupportedByReference` sentinel it reserves for a not-yet-built
//!   surface). With the ADR-76 identity formers and ADR-80 declared data
//!   realized, the full corpus (106 model, 54 pathological) realizes.
//! - **no drift** — `canonical(machine::run_comp(item))` renders identically to
//!   the item's recorded snapshot outcome, through the same [`canonical`] the
//!   agreement differential used (first-order values exact; codata / native
//!   terminals structural through the un-focusing readback `𝓕⁻¹`; a reified
//!   stack in value position stays coarse — the §7a k-in-value residual).
//!
//! The full-realization counts (106 / 54) stay asserted as floors.
//!
//! # Snapshots and their provenance
//!
//! Each corpus `.sexp` fixture has a sibling `.outcome` record under the
//! corpus-fixture model/pathological directories, carrying a provenance header
//! (the `.gandr` source path, the BLAKE3 digest of the `.sexp` fixture bytes,
//! the generator identity, and the item count) and one line per item — the
//! `Debug` rendering of that item's [`canonical`] outcome. The header's
//! `sexp-b3sum` is checked against the live fixture digest on every run, so a
//! changed fixture forces a re-bless rather than silently comparing against a
//! stale record.
//!
//! **Regeneration** is reproducible and documented: run the ignored-by-default
//! [`tests::bless_corpus_outcomes`] generator with
//! `GANDR_BLESS_CORPUS_OUTCOMES` set (e.g. `GANDR_BLESS_CORPUS_OUTCOMES=1 cargo
//! nextest run -p gandr-core-sequent bless_corpus_outcomes`). It rewrites every
//! `.outcome` record from the current L machine, mirroring how the pre-lowered
//! `.sexp` fixtures document their own capture. The records here were blessed
//! from the final oracle-agreeing run.
//!
//! When these snapshots were first frozen (B1 stage F) the sweep additionally
//! cross-checked that each item's retiring-CEK-oracle outcome rendered to the
//! same snapshot, so both anchors (L and the oracle) agreed with the fixture
//! before the CEK was removed. That cross-check retired with the CEK; the sweep
//! is now L-vs-snapshot only.
//!
//! The items are lowered live from the ported source corpus
//! ([`crate::common`]); the checked-in `.sexp` files remain only as
//! immutable byte/provenance anchors for the outcome records and kernel
//! manifests.

#[cfg(test)]
mod tests
{
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    use gandr_core_sequent::boundary::CorpusItemCount;
    use gandr_core_sequent::boundary::UnsupportedFormerStatus;
    use gandr_core_sequent::differential::canonical;
    use gandr_core_sequent::machine;
    use gandr_core_term::outcome::Eval;
    use gandr_core_term::outcome::StuckReason;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Term;

    use crate::common::CorpusTree;
    use crate::common::Fixture;
    use crate::common::read_tree;

    /// The environment variable that switches [`bless_corpus_outcomes`] from a
    /// no-op into the snapshot regenerator.
    const BLESS_ENV: &str = "GANDR_BLESS_CORPUS_OUTCOMES";

    /// `canonical(L-run ∘ 𝓕)` matches the recorded snapshot on the model
    /// corpus.
    #[test]
    fn l_machine_matches_the_outcome_snapshots_on_the_model_corpus()
    {
        let outcome = sweep(CorpusTree::MODEL);
        eprintln!(
            "model corpus outcome snapshot: {} of {} items realized on the L machine",
            outcome.realized, outcome.items
        );
        assert!(
            outcome.mismatches.is_empty(),
            "model corpus outcome-snapshot mismatches ({} of {} items realized):\n{}",
            outcome.realized,
            outcome.items,
            outcome.mismatches.join("\n")
        );
        // Realization ratchet: 69 (pure spine) → 81 (faithful `perform`) → 84
        // (effects / control `shift`) → 106, the full model corpus (106 of 106)
        // now realizing. The B1 phase-3 seams landed the remainder: prelude
        // free-name resolution (ADR-42) leaves the empty-prelude force miss at
        // `ForcedNonThunk`, and the ADR-76 identity formers and ADR-80 declared
        // data are realized, so no corpus item declines. The corpus itself
        // shrank by one when the `14-agda-deps-walkthrough` example left with
        // the Agda metatheory extraction (2026-08-15), moving the full-corpus
        // count to 105 of 105. Raise (never lower) as later work lands.
        assert!(
            usize::from(outcome.realized) >= 105,
            "the L machine's model-corpus realization regressed below the pinned \
             checkpoint floor (105), got {} of {}",
            outcome.realized,
            outcome.items
        );
    }

    /// `canonical(L-run ∘ 𝓕)` matches the recorded snapshot on the pathological
    /// corpus.
    #[test]
    fn l_machine_matches_the_outcome_snapshots_on_the_pathological_corpus()
    {
        let outcome = sweep(CorpusTree::PATHOLOGICAL);
        eprintln!(
            "pathological corpus outcome snapshot: {} of {} items realized on the L machine",
            outcome.realized, outcome.items
        );
        assert!(
            outcome.mismatches.is_empty(),
            "pathological corpus outcome-snapshot mismatches ({} of {} items realized):\n{}",
            outcome.realized,
            outcome.items,
            outcome.mismatches.join("\n")
        );
        // Realization ratchet, as for the model corpus: 32 → 33 (faithful
        // `perform`) → 54, the full pathological corpus (54 of 54) now
        // realizing once the B1 phase-3 seams land the ADR-76 identity formers
        // and ADR-80 declared data. Raise (never lower) as later checkpoints
        // land.
        assert!(
            usize::from(outcome.realized) >= 54,
            "the L machine's pathological-corpus realization regressed below the \
             pinned checkpoint floor (54), got {} of {}",
            outcome.realized,
            outcome.items
        );
    }

    /// Runs each fixture item on the L machine and asserts it realizes. For
    /// non-staged fixtures, its canonical outcome must also match the recorded
    /// snapshot; F4 O6 keeps the FFI-capability and regex records frozen.
    fn sweep(tree: CorpusTree) -> Sweep
    {
        let fixtures = read_tree(tree);
        assert!(
            !fixtures.is_empty(),
            "the `{tree}` fixture tree must contain at least one fixture"
        );
        let mut outcome = Sweep::default();
        for fixture in &fixtures {
            let expected = read_snapshot(fixture);
            assert_eq!(
                expected.len(),
                fixture.items.len(),
                "outcome snapshot `{}` records {} outcomes for {} items; regenerate with \
                 {BLESS_ENV}=1",
                outcome_path(&fixture.path).display(),
                expected.len(),
                fixture.items.len()
            );
            for (index, (term, snapshot)) in fixture.items.iter().zip(expected.iter()).enumerate() {
                outcome.items = usize::from(outcome.items).saturating_add(1).into();
                let comp = as_comp(term);
                let machine = machine::run_comp(&comp);
                // Every corpus item must realize on the L machine — the
                // not-yet-built `UnsupportedByReference` sentinel is a defect
                // now that the full corpus realizes.
                assert!(
                    !bool::from(is_unsupported(&machine)),
                    "{} item {index}: the L machine declined a corpus item \
                     (UnsupportedByReference); the full corpus must realize",
                    fixture.source
                );
                outcome.realized = usize::from(outcome.realized).saturating_add(1).into();
                let realized = format!("{:?}", canonical(&machine));
                if !fixture.snapshot_is_feature_frozen && realized != *snapshot {
                    outcome.mismatches.push(format!(
                        "{} item {index}: L {realized} vs snapshot {snapshot}",
                        fixture.source
                    ));
                }
            }
        }
        outcome
    }

    /// Regenerates every `.outcome` snapshot from the current L machine when
    /// `GANDR_BLESS_CORPUS_OUTCOMES` is set; a no-op otherwise so it is inert
    /// under the ordinary gate.
    #[test]
    #[cfg_attr(
        dylint_lib = "non_thread_safe_call_in_test",
        allow(
            unknown_lints,
            non_thread_safe_call_in_test,
            reason = "the bless-only snapshot regeneration runs single-threaded behind the bless environment gate and writes only checked-in fixture paths"
        )
    )]
    fn bless_corpus_outcomes()
    {
        if std::env::var_os(BLESS_ENV).is_none() {
            return;
        }
        for tree in [CorpusTree::MODEL, CorpusTree::PATHOLOGICAL] {
            for fixture in &read_tree(tree) {
                if fixture.snapshot_is_feature_frozen {
                    continue;
                }
                write_snapshot(fixture);
            }
        }
    }

    /// The outcome of sweeping one corpus tree against its snapshots.
    #[derive(Default)]
    struct Sweep
    {
        /// The number of top-level items compared.
        items: CorpusItemCount,
        /// The number the L machine realizes (does not decline).
        realized: CorpusItemCount,
        /// Per-item snapshot mismatches (a genuine defect).
        mismatches: Vec<String>,
    }

    /// The sibling `.outcome` snapshot path for a `.sexp` fixture.
    fn outcome_path(sexp: &Path) -> PathBuf
    {
        sexp.with_extension("outcome")
    }

    /// The lowercase BLAKE3 digest of a `.sexp` fixture's bytes, the snapshot's
    /// provenance anchor.
    fn sexp_digest(sexp: &Path) -> String
    {
        let bytes = fs::read(sexp)
            .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", sexp.display()));
        String::from(blake3::hash(&bytes).to_hex().as_str())
    }

    /// Reads a fixture's sibling `.outcome` record, verifying the recorded
    /// `sexp-b3sum` still matches the live fixture bytes, and returns the
    /// per-item expected canonical-outcome renderings in file order.
    fn read_snapshot(fixture: &Fixture) -> Vec<String>
    {
        let path = outcome_path(&fixture.path);
        let text = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "cannot read outcome snapshot `{}` ({error}); regenerate with {BLESS_ENV}=1",
                path.display()
            )
        });
        let recorded = header_field(SnapshotHeader {
            text: &text,
            name: "sexp-b3sum",
        })
        .unwrap_or_else(|| {
            panic!(
                "outcome snapshot `{}` is missing its `sexp-b3sum` provenance header",
                path.display()
            )
        });
        let actual = sexp_digest(&fixture.path);
        assert_eq!(
            recorded,
            actual,
            "outcome snapshot `{}` is stale (recorded fixture b3sum {recorded} != live \
             {actual}); regenerate with {BLESS_ENV}=1",
            path.display()
        );
        text.lines()
            .filter(|line| !line.starts_with(';'))
            .map(str::to_owned)
            .collect()
    }

    /// Writes one fixture's `.outcome` snapshot from the current L machine.
    fn write_snapshot(fixture: &Fixture)
    {
        let mut lines = vec![
            "; gandr corpus outcome snapshot (B1 exit gate; L-machine oracle)".to_owned(),
            format!("; source: {}", fixture.source),
            format!("; sexp-b3sum: {}", sexp_digest(&fixture.path)),
            format!(
                "; generator: gandr-core-sequent corpus_differential::bless_corpus_outcomes \
                 ({BLESS_ENV}=1)"
            ),
            format!("; items: {}", fixture.items.len()),
        ];
        for term in &fixture.items {
            let comp = as_comp(term);
            lines.push(format!("{:?}", canonical(&machine::run_comp(&comp))));
        }
        let mut out = lines.join("\n");
        out.push('\n');
        let path = outcome_path(&fixture.path);
        fs::write(&path, out)
            .unwrap_or_else(|error| panic!("cannot write `{}`: {error}", path.display()));
    }

    /// One provenance-header lookup.
    #[derive(Clone, Copy)]
    struct SnapshotHeader<'text>
    {
        /// Whole snapshot text.
        text: &'text str,
        /// Header field name.
        name: &'text str,
    }

    /// The value of a `; <name>: <value>` provenance header line, trimmed.
    fn header_field(header: SnapshotHeader<'_>) -> Option<String>
    {
        let needle = format!("; {}:", header.name);
        header
            .text
            .lines()
            .find_map(|line| line.strip_prefix(&needle))
            .map(|rest| rest.trim().to_owned())
    }

    /// A top-level term as a computation (a value item runs as `ret v`,
    /// matching how a bare value's terminal reads back).
    fn as_comp(term: &Term) -> Comp
    {
        match *term {
            | Term::Comp(ref comp) => comp.clone(),
            | Term::Value(ref value) => Comp::ret(value.clone()),
        }
    }

    /// Whether an outcome is the L machine's "not yet built" sentinel — the
    /// defined stuck it reserves for a declined program.
    fn is_unsupported(eval: &Eval) -> UnsupportedFormerStatus
    {
        matches!(*eval, Eval::Stuck(StuckReason::UnsupportedByReference)).into()
    }
}
