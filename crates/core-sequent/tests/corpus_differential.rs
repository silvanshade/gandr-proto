//! The phase-L1 corpus **outcome-snapshot** sweep (`proposal-sequent-kernel.md`
//! §9, phase L1 gate; coordinator decision D3): `canonical(L-run ∘ 𝓕)` over
//! every lowered corpus program must match a checked-in expected-outcome
//! record.
//!
//! Through stages C–E this sweep was an L-vs-CEK **agreement** differential: it
//! ran every top-level item of every model and pathological corpus program on
//! both the CEK oracle (`gandr_core_checker::eval::run_comp`) and the L machine
//! (`gandr_core_sequent::machine::run_comp`) and asserted the two
//! [`canonical`]ized outcomes were equal. With the CEK retiring (B1 phase-3
//! stage F), the differential's guarantee is preserved by **freezing** the
//! oracle-agreeing outcome of each item into a checked-in snapshot: the L
//! machine must keep reproducing exactly that outcome. The frozen snapshot *is*
//! the retired oracle.
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
//! Each corpus `.sexp` fixture has a sibling `.outcome` record under
//! `tests/fixtures/corpus/{model,pathological}/`, carrying a provenance header
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
//! # Transitional oracle cross-check
//!
//! While the CEK still exists (through stage F), the sweep additionally proves
//! the frozen snapshot still mirrors the retiring oracle — every item's CEK
//! outcome must also render to the recorded snapshot — so this commit shows
//! both anchors (L and CEK) agreeing with the fixture before the CEK is
//! removed. That cross-check ([`tests::assert_oracle_matches_snapshot`])
//! retires with the CEK.
//!
//! The items are read from the pre-lowered corpus fixtures
//! ([`crate::corpus_fixtures`]) rather than lowered live: the front-end that
//! lowers `.gandr` sources is outside the B1 machine-port scope, so its output
//! was captured once into the checked-in fixtures.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the corpus snapshot sweep readable \
                  (docs/workflow/rust.md)"
    )
)]

#[cfg(test)]
mod tests
{
    use std::fs;
    use std::path::PathBuf;

    // TRANSITIONAL: the retiring CEK oracle, used only by the cross-check that
    // proves the frozen snapshots still mirror it. Removed with the CEK at
    // stage F, leaving the pure L-vs-snapshot sweep.
    use gandr_core_checker::eval::Eval;
    use gandr_core_checker::eval::StuckReason;
    use gandr_core_checker::eval::run_comp as cek_run_comp;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Term;
    use gandr_core_sequent::boundary::CorpusItemCount;
    use gandr_core_sequent::boundary::UnsupportedFormerStatus;
    use gandr_core_sequent::differential::canonical;
    use gandr_core_sequent::machine;

    use crate::corpus_fixtures::Fixture;
    use crate::corpus_fixtures::read_tree;

    /// The environment variable that switches [`bless_corpus_outcomes`] from a
    /// no-op into the snapshot regenerator.
    const BLESS_ENV: &str = "GANDR_BLESS_CORPUS_OUTCOMES";

    /// `canonical(L-run ∘ 𝓕)` matches the recorded snapshot on the model
    /// corpus.
    #[test]
    fn l_machine_matches_the_outcome_snapshots_on_the_model_corpus()
    {
        let outcome = sweep("model");
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
        // data are realized, so no corpus item declines. Raise (never lower) as
        // later work lands.
        assert!(
            usize::from(outcome.realized) >= 106,
            "the L machine's model-corpus realization regressed below the pinned \
             checkpoint floor (106), got {} of {}",
            outcome.realized,
            outcome.items
        );
    }

    /// `canonical(L-run ∘ 𝓕)` matches the recorded snapshot on the pathological
    /// corpus.
    #[test]
    fn l_machine_matches_the_outcome_snapshots_on_the_pathological_corpus()
    {
        let outcome = sweep("pathological");
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

    /// Runs each fixture item on the L machine, asserting it realizes and that
    /// its canonical outcome matches the recorded snapshot; also cross-checks
    /// the retiring CEK oracle against the snapshot (transitional).
    fn sweep(tree: &str) -> Sweep
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
                if realized != *snapshot {
                    outcome.mismatches.push(format!(
                        "{} item {index}: L {realized} vs snapshot {snapshot}",
                        fixture.source
                    ));
                }
                assert_oracle_matches_snapshot(&comp, snapshot, &fixture.source, index);
            }
        }
        outcome
    }

    /// TRANSITIONAL (retires with the CEK at stage F): the retiring CEK
    /// oracle's canonical outcome must also render to the recorded
    /// snapshot, proving the frozen fixture still mirrors the oracle it was
    /// blessed from.
    fn assert_oracle_matches_snapshot(
        comp: &Comp,
        expected: &str,
        source: &str,
        index: usize,
    )
    {
        let oracle = cek_run_comp(comp.clone());
        assert!(
            !bool::from(is_unsupported(&oracle)),
            "{source} item {index}: the retiring CEK oracle declined a corpus item"
        );
        assert_eq!(
            format!("{:?}", canonical(&oracle)),
            expected,
            "{source} item {index}: the retiring CEK oracle disagrees with the recorded \
             snapshot; the snapshot provenance is broken"
        );
    }

    /// Regenerates every `.outcome` snapshot from the current L machine when
    /// `GANDR_BLESS_CORPUS_OUTCOMES` is set; a no-op otherwise so it is inert
    /// under the ordinary gate.
    #[test]
    fn bless_corpus_outcomes()
    {
        if std::env::var_os(BLESS_ENV).is_none() {
            return;
        }
        for tree in ["model", "pathological"] {
            for fixture in &read_tree(tree) {
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
    fn outcome_path(sexp: &std::path::Path) -> PathBuf
    {
        sexp.with_extension("outcome")
    }

    /// The lowercase BLAKE3 digest of a `.sexp` fixture's bytes, the snapshot's
    /// provenance anchor.
    fn sexp_digest(sexp: &std::path::Path) -> String
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
        let recorded = header_field(&text, "sexp-b3sum").unwrap_or_else(|| {
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

    /// The value of a `; <name>: <value>` provenance header line, trimmed.
    fn header_field(
        text: &str,
        name: &str,
    ) -> Option<String>
    {
        let needle = format!("; {name}:");
        text.lines()
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
            // A future top-level sort runs as an opaque hole.
            | _ => Comp::hole(0),
        }
    }

    /// Whether an outcome is the L machine's "not yet built" sentinel — the
    /// defined stuck it reserves for a declined program.
    fn is_unsupported(eval: &Eval) -> UnsupportedFormerStatus
    {
        matches!(*eval, Eval::Stuck(StuckReason::UnsupportedByReference)).into()
    }
}
