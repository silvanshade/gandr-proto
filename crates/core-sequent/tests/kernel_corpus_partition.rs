//! The **S1-eligible corpus partition** (gandr-wvd.2, B2.3 deliverable 3).
//!
//! The pre-lowered corpus ([`crate::corpus_fixtures`]) is the checked-in output
//! of the elaborator's lowering, so its items are exactly the checked core CBPV
//! terms the B2.3 kernel bridge ([`gandr_core_checker::kernel_bridge`]) lowers
//! FROM. This sweep classifies the corpus **per item** (never per file — the
//! files-vs-items unit pitfall is a recorded hazard): an item is
//! **S1-eligible** iff its lowered core form uses only the S1 stock, it types
//! in the empty context, and it re-admits through the kernel choke point. Every
//! eligible item then **exports and round-trips byte-identically**; every
//! ineligible item is tagged with the exclusion class that rejected it.
//!
//! The partition is recorded durably in the checked-in manifest
//! `tests/fixtures/kernel_partition.manifest`, one line per item with its class
//! tag; this sweep re-derives the classification and asserts it matches the
//! manifest (regenerate with `GANDR_BLESS_KERNEL_PARTITION=1`). The
//! exact-variant structural rejections of every exclusion class are pinned by
//! the bridge's own unit witnesses
//! (`gandr_core_checker::kernel_bridge::tests`); this sweep pins
//! the corpus-wide partition and the eligible round-trip.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::pattern_type_mismatch,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the partition sweep readable \
                  (docs/workflow/rust.md)"
    )
)]

#[cfg(test)]
mod tests
{
    extern crate alloc;

    use std::fs;
    use std::path::PathBuf;

    use gandr_core_checker::checker::infer_comp;
    use gandr_core_checker::checker::infer_value;
    use gandr_core_checker::ctx::Ctx;
    use gandr_core_checker::kernel_bridge::BridgeContext;
    use gandr_core_checker::kernel_bridge::lower_comp;
    use gandr_core_checker::kernel_bridge::lower_computation_definition;
    use gandr_core_checker::kernel_bridge::lower_value;
    use gandr_core_checker::kernel_bridge::lower_value_definition;
    use gandr_core_checker::syntax::Term;
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::TermArena;
    use gandr_kernel_core::read;
    use gandr_kernel_core::write;

    use self::alloc::collections::BTreeMap;
    use crate::corpus_fixtures::read_tree;

    /// The environment variable that switches the sweep from verify to bless.
    const BLESS_ENV: &str = "GANDR_BLESS_KERNEL_PARTITION";

    /// The eligible-class tag.
    const ELIGIBLE: &str = "eligible";

    /// One classified corpus item.
    struct Classified
    {
        /// The source-relative fixture path.
        source: String,
        /// The item's index within its fixture, in file order.
        index: usize,
        /// The class tag (`eligible` or an exclusion class).
        class: String,
    }

    /// Classify every item of every corpus tree, in a deterministic order, and
    /// assert every eligible item round-trips byte-identically.
    fn sweep() -> Vec<Classified>
    {
        let mut rows = Vec::new();
        for tree in ["model", "pathological"] {
            for fixture in &read_tree(tree) {
                for (index, term) in fixture.items.iter().enumerate() {
                    let class = match classify(term) {
                        | Ok(environment) => {
                            assert_round_trips(&environment, &fixture.source, index);
                            String::from(ELIGIBLE)
                        },
                        | Err(tag) => tag,
                    };
                    rows.push(Classified {
                        source: fixture.source.clone(),
                        index,
                        class,
                    });
                }
            }
        }
        rows
    }

    /// Attempt the full bridge → infer → admit pipeline for one item, returning
    /// the admitted environment when it is S1-eligible or the exclusion-class
    /// tag otherwise.
    ///
    /// The term's bridgeability (structural stock and closedness) is probed
    /// first, so an out-of-S1 node or a free name wins the classification over
    /// a later typing failure — the S1-eligibility criterion is exactly
    /// "the lowered core form uses only S1 stock".
    fn classify(term: &Term) -> Result<Environment, String>
    {
        let context = BridgeContext::new();
        // 1. Structural / free-name verdict from lowering the term alone.
        match term {
            | Term::Value(value) => {
                let mut scratch = TermArena::new();
                if let Err(rejection) = lower_value(&context, &mut scratch, value) {
                    return Err(String::from(rejection.exclusion_class()));
                }
            },
            | Term::Comp(comp) => {
                let mut scratch = TermArena::new();
                if let Err(rejection) = lower_comp(&context, &mut scratch, comp) {
                    return Err(String::from(rejection.exclusion_class()));
                }
            },
            | _ => return Err(String::from("non-term-item")),
        }
        // 2. Type inference (empty context) + admission (value-polarity Def).
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let ids = match term {
            | Term::Value(value) => {
                let Ok(core_type) = infer_value(Ctx::new(), value.clone())
                else {
                    return Err(String::from("not-typeable"));
                };
                match lower_value_definition(&context, builder.arena(), value, &core_type) {
                    | Ok(ids) => ids,
                    | Err(rejection) => return Err(String::from(rejection.exclusion_class())),
                }
            },
            | Term::Comp(comp) => {
                let Ok(core_type) = infer_comp(Ctx::new(), comp.clone())
                else {
                    return Err(String::from("not-typeable"));
                };
                match lower_computation_definition(&context, builder.arena(), comp, &core_type) {
                    | Ok(ids) => ids,
                    | Err(rejection) => return Err(String::from(rejection.exclusion_class())),
                }
            },
            | _ => return Err(String::from("non-term-item")),
        };
        let (declared_id, body_id) = ids;
        let declaration = builder.def(LevelSignature::monomorphic(), declared_id, body_id);
        if environment.add_decl(declaration).is_err() {
            return Err(String::from("kernel-rejected"));
        }
        Ok(environment)
    }

    /// Assert `write ∘ read ∘ write` is byte-identical on an admitted item's
    /// environment.
    fn assert_round_trips(
        environment: &Environment,
        source: &str,
        index: usize,
    )
    {
        let bytes = write(environment);
        let reread = read(&bytes).unwrap_or_else(|error| {
            panic!("{source} item {index}: an eligible item failed to re-read: {error}")
        });
        assert_eq!(
            bytes,
            write(&reread),
            "{source} item {index}: an eligible item did not round-trip byte-identically"
        );
    }

    /// The checked-in manifest path.
    fn manifest_path() -> PathBuf
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kernel_partition.manifest")
    }

    /// Render the manifest data lines (`<source>\t<index>\t<class>`) for the
    /// classified rows, in sweep order.
    fn render_data_lines(rows: &[Classified]) -> Vec<String>
    {
        rows.iter()
            .map(|row| format!("{}\t{}\t{}", row.source, row.index, row.class))
            .collect()
    }

    /// The data lines of a manifest text (the non-`;` lines).
    fn data_lines(text: &str) -> Vec<String>
    {
        text.lines()
            .filter(|line| !line.starts_with(';') && !line.trim().is_empty())
            .map(str::to_owned)
            .collect()
    }

    /// The per-class cardinalities, ascending by class.
    fn cardinalities(rows: &[Classified]) -> BTreeMap<String, usize>
    {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for row in rows {
            *counts.entry(row.class.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// The live classification matches the checked-in manifest, and every
    /// eligible item round-trips (asserted inside [`sweep`]).
    #[test]
    fn corpus_partition_matches_the_manifest()
    {
        let rows = sweep();
        let live = render_data_lines(&rows);
        let counts = cardinalities(&rows);
        let eligible = counts.get(ELIGIBLE).copied().unwrap_or(0);
        eprintln!("kernel corpus partition: {} items", rows.len());
        for (class, count) in &counts {
            eprintln!("  {class}: {count}");
        }

        if std::env::var_os(BLESS_ENV).is_some() {
            write_manifest(&rows, &counts);
            return;
        }

        let path = manifest_path();
        let text = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "cannot read the partition manifest `{}` ({error}); regenerate with {BLESS_ENV}=1",
                path.display()
            )
        });
        let expected = data_lines(&text);
        assert_eq!(
            live,
            expected,
            "the live corpus partition drifted from `{}`; regenerate with {BLESS_ENV}=1",
            path.display()
        );

        // Floor chosen deliberately (the files-vs-items unit pitfall): most of
        // the 160-item corpus exceeds S1 (139 ineligible at bless time, chiefly
        // the 62 open free-name programs), but a substantive eligible partition
        // (21 at bless) bridges, admits, and round-trips. The floor is set below
        // that with margin so a minor corpus edit does not require lowering it;
        // the manifest exact-match above is the precise guard.
        assert!(
            eligible >= 15,
            "the S1-eligible partition regressed below its deliberate floor (15), got {eligible}"
        );
    }

    /// Rewrite the checked-in manifest from the live classification.
    fn write_manifest(
        rows: &[Classified],
        counts: &BTreeMap<String, usize>,
    )
    {
        let mut lines = vec![
            String::from("; gandr S1-eligible corpus partition (B2.3 deliverable 3)"),
            String::from(
                "; generator: gandr-core-sequent kernel_corpus_partition \
                 (GANDR_BLESS_KERNEL_PARTITION=1)",
            ),
            format!("; items: {}", rows.len()),
        ];
        for (class, count) in counts {
            lines.push(format!("; class {class}: {count}"));
        }
        lines.push(String::from("; columns: <source>\\t<item-index>\\t<class>"));
        lines.extend(render_data_lines(rows));
        let mut out = lines.join("\n");
        out.push('\n');
        let path = manifest_path();
        fs::write(&path, out)
            .unwrap_or_else(|error| panic!("cannot write `{}`: {error}", path.display()));
    }

    /// The corpus exercises the bridge's structural rejection across many
    /// exclusion classes (a coarse coverage floor over the live partition; the
    /// exact-variant witnesses live in the bridge's own unit tests).
    #[test]
    fn corpus_exercises_multiple_exclusion_classes()
    {
        let rows = sweep();
        let counts = cardinalities(&rows);
        let observed: usize = counts
            .keys()
            .filter(|class| class.as_str() != ELIGIBLE)
            .count();
        assert!(
            observed >= 4,
            "the corpus should exercise several exclusion classes (got {observed}): {:?}",
            counts.keys().collect::<Vec<_>>()
        );
    }
}
