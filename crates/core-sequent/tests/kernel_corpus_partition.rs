//! The **S1-eligible corpus partition** (gandr-wvd.2, B2.3 deliverable 3).
//!
//! The live source corpus ([`crate::common`]) is lowered through the
//! elaborator, so its items are exactly the checked core CBPV terms the B2.3
//! kernel bridge ([`gandr_core_checker::kernel_bridge`]) lowers FROM.
//! This sweep classifies the corpus **per item** (never per file — the
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

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::collections::BTreeSet;
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

    use crate::common::CorpusTree;
    use crate::common::corpus_fixtures_b3sum;
    use crate::common::read_tree;

    /// The environment variable that switches the sweep from verify to bless.
    const BLESS_ENV: &str = "GANDR_BLESS_KERNEL_PARTITION";

    /// The eligible-class tag.
    const ELIGIBLE: &str = "eligible";

    /// The corpus trees the sweep classifies (also the b3sum-provenance scope).
    const TREES: [CorpusTree; 2] = [CorpusTree::MODEL, CorpusTree::PATHOLOGICAL];

    /// One classified corpus item.
    struct Classified
    {
        /// The source-relative fixture path.
        source: String,
        /// The item's index within its fixture, in file order.
        index: usize,
        /// The class tag (`eligible` or an exclusion class).
        class: String,
        /// Whether F4 O6 freezes this row's pre-feature partition record.
        snapshot_is_feature_frozen: bool,
    }

    /// Classify every item of every corpus tree, in a deterministic order, and
    /// assert every eligible item round-trips byte-identically.
    fn sweep() -> Vec<Classified>
    {
        let mut rows = Vec::new();
        for tree in TREES {
            for fixture in &read_tree(tree) {
                for (index, term) in fixture.items.iter().enumerate() {
                    let class = match classify(term) {
                        | Ok(environment) => {
                            assert_round_trips(&environment, CorpusItemLocation {
                                source: fixture.source.as_str(),
                                index,
                            });
                            String::from(ELIGIBLE)
                        },
                        | Err(tag) => tag,
                    };
                    rows.push(Classified {
                        snapshot_is_feature_frozen: fixture.snapshot_is_feature_frozen,
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
        match *term {
            | Term::Value(ref value) => {
                let mut scratch = TermArena::new();
                if let Err(rejection) = lower_value(&context, &mut scratch, value) {
                    return Err(String::from(rejection.exclusion_class().as_ref()));
                }
            },
            | Term::Comp(ref comp) => {
                let mut scratch = TermArena::new();
                if let Err(rejection) = lower_comp(&context, &mut scratch, comp) {
                    return Err(String::from(rejection.exclusion_class().as_ref()));
                }
            },
        }
        // 2. Type inference (empty context) + admission (value-polarity Def).
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let ids = match *term {
            | Term::Value(ref value) => {
                let Ok(core_type) = infer_value(Ctx::new(), value.clone())
                else {
                    return Err(String::from("not-typeable"));
                };
                match lower_value_definition(&context, builder.arena(), value, &core_type) {
                    | Ok(ids) => ids,
                    | Err(rejection) => {
                        return Err(String::from(rejection.exclusion_class().as_ref()));
                    },
                }
            },
            | Term::Comp(ref comp) => {
                let Ok(core_type) = infer_comp(Ctx::new(), comp.clone())
                else {
                    return Err(String::from("not-typeable"));
                };
                match lower_computation_definition(&context, builder.arena(), comp, &core_type) {
                    | Ok(ids) => ids,
                    | Err(rejection) => {
                        return Err(String::from(rejection.exclusion_class().as_ref()));
                    },
                }
            },
        };
        let (declared_id, body_id) = ids;
        let declaration = builder.def(LevelSignature::monomorphic(), declared_id, body_id);
        if environment.add_decl(declaration).is_err() {
            return Err(String::from("kernel-rejected"));
        }
        Ok(environment)
    }

    /// Corpus location of one classified item.
    #[derive(Clone, Copy)]
    struct CorpusItemLocation<'source>
    {
        /// Corpus-relative source path.
        source: &'source str,
        /// Item ordinal within the source.
        index: usize,
    }

    /// Assert `write ∘ read ∘ write` is byte-identical on an admitted item's
    /// environment.
    fn assert_round_trips(
        environment: &Environment,
        location: CorpusItemLocation<'_>,
    )
    {
        let bytes = write(environment);
        let reread = read(bytes.as_ref().into()).unwrap_or_else(|error| {
            panic!(
                "{} item {}: an eligible item failed to re-read: {error}",
                location.source, location.index
            )
        });
        assert_eq!(
            bytes,
            write(&reread),
            "{} item {}: an eligible item did not round-trip byte-identically",
            location.source,
            location.index
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

    /// Borrowed partition-manifest text.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct ManifestText<'text>(&'text str);

    /// The data lines of a manifest text (the non-`;` lines).
    fn data_lines(text: ManifestText<'_>) -> Vec<String>
    {
        text.0
            .lines()
            .filter(|line| !line.starts_with(';') && !line.trim().is_empty())
            .map(str::to_owned)
            .collect()
    }

    /// One feature-freeze membership query.
    #[derive(Clone, Copy)]
    struct FeatureFreezeQuery<'line, 'set, 'source>
    {
        /// Rendered partition row.
        line: &'line str,
        /// Corpus sources still frozen at this stage.
        sources: &'set BTreeSet<&'source str>,
    }

    /// Whether a manifest row belongs to a frozen source.
    enum FeatureFreezeStatus
    {
        /// The row is frozen.
        Frozen,
        /// The row participates in live comparison.
        Live,
    }

    /// Whether a rendered row belongs to an O6 feature-frozen source.
    fn feature_freeze_status(query: FeatureFreezeQuery<'_, '_, '_>) -> FeatureFreezeStatus
    {
        if query
            .line
            .split_once('\t')
            .is_some_and(|(source, _rest)| query.sources.contains(source))
        {
            FeatureFreezeStatus::Frozen
        }
        else {
            FeatureFreezeStatus::Live
        }
    }

    /// One partition-manifest provenance-header lookup.
    #[derive(Clone, Copy)]
    struct HeaderQuery<'header>
    {
        /// Whole manifest text.
        text: &'header str,
        /// Header field name.
        name: &'header str,
    }

    /// The value of a `; <name>: <value>` provenance header line, trimmed.
    fn header_value(query: HeaderQuery<'_>) -> Option<String>
    {
        let needle = format!("; {}:", query.name);
        query
            .text
            .lines()
            .find_map(|line| line.strip_prefix(&needle))
            .map(|rest| rest.trim().to_owned())
    }

    /// Per-class partition cardinalities.
    #[repr(transparent)]
    struct ClassCardinalities(BTreeMap<String, usize>);

    /// The per-class cardinalities, ascending by class.
    fn cardinalities(rows: &[Classified]) -> ClassCardinalities
    {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for row in rows {
            let count = counts.entry(row.class.clone()).or_insert(0);
            *count = count.saturating_add(1);
        }
        ClassCardinalities(counts)
    }

    /// The live classification matches the checked-in manifest, and every
    /// eligible item round-trips (asserted inside [`sweep`]).
    #[test]
    fn corpus_partition_matches_the_manifest()
    {
        let rows = sweep();
        let feature_frozen_sources: BTreeSet<&str> = rows
            .iter()
            .filter(|row| row.snapshot_is_feature_frozen)
            .map(|row| row.source.as_str())
            .collect();
        let live: Vec<String> = render_data_lines(&rows)
            .into_iter()
            .filter(|line| {
                matches!(
                    feature_freeze_status(FeatureFreezeQuery {
                        line: line.as_str(),
                        sources: &feature_frozen_sources,
                    }),
                    FeatureFreezeStatus::Live
                )
            })
            .collect();
        let counts = cardinalities(&rows);
        let eligible = counts.0.get(ELIGIBLE).copied().unwrap_or(0);
        eprintln!("kernel corpus partition: {} items", rows.len());
        for (class, count) in &counts.0 {
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
        // The b3sum fixture-provenance guard (W-A H3): the manifest couples to
        // the exact corpus bytes it was classified from, so a fixture edit forces
        // a re-bless rather than silently comparing against a stale partition.
        let recorded_b3sum = header_value(HeaderQuery {
            text: &text,
            name: "corpus-fixtures-b3sum",
        })
        .unwrap_or_else(|| {
            panic!(
                "partition manifest `{}` is missing its `corpus-fixtures-b3sum` provenance header; \
                 regenerate with {BLESS_ENV}=1",
                path.display()
            )
        });
        assert_eq!(
            recorded_b3sum,
            corpus_fixtures_b3sum(&TREES),
            "partition manifest `{}` is stale (recorded corpus b3sum != live); regenerate with \
             {BLESS_ENV}=1",
            path.display()
        );
        let expected: Vec<String> = data_lines(ManifestText(&text))
            .into_iter()
            .filter(|line| {
                matches!(
                    feature_freeze_status(FeatureFreezeQuery {
                        line: line.as_str(),
                        sources: &feature_frozen_sources,
                    }),
                    FeatureFreezeStatus::Live
                )
            })
            .collect();
        assert_eq!(
            live,
            expected,
            "the non-staged live corpus partition drifted from `{}`; regenerate with \
             {BLESS_ENV}=1",
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
        counts: &ClassCardinalities,
    )
    {
        let mut lines = vec![
            String::from("; gandr S1-eligible corpus partition (B2.3 deliverable 3)"),
            String::from(
                "; generator: gandr-core-sequent kernel_corpus_partition \
                 (GANDR_BLESS_KERNEL_PARTITION=1)",
            ),
            format!("; items: {}", rows.len()),
            format!("; corpus-fixtures-b3sum: {}", corpus_fixtures_b3sum(&TREES)),
        ];
        for (class, count) in &counts.0 {
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
            .0
            .keys()
            .filter(|class| class.as_str() != ELIGIBLE)
            .count();
        assert!(
            observed >= 4,
            "the corpus should exercise several exclusion classes (got {observed}): {:?}",
            counts.0.keys().collect::<Vec<_>>()
        );
    }
}
