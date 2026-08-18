//! The **`GANDR_BLESS_KERNEL_EXPORT` exit-gate harness** (gandr-wvd.2, B2.3
//! deliverable 1; ADR-84 internal-before-surface — this harness *is* the B2.3
//! landing evidence).
//!
//! It drives the whole export pipeline end to end — bridge → `add_decl` →
//! export v1 `write` → `decode` → `read` (replay) — over the S1-eligible corpus
//! partition (the 23 items the B2.3 bridge lowers and the choke point admits)
//! **plus** the kernel-native C5 goldens (universe/lift), and pins each item's
//! outcome against a checked-in per-item record. `GANDR_BLESS_KERNEL_EXPORT=1`
//! re-blesses the records; the gate is otherwise inert (a pure verify).
//!
//! # What each record pins (D3 telemetry floors — gandr-3ln)
//!
//! Every record is a line of **deterministic** metrics — each a function of the
//! canonical bytes alone, so an S2 description-code inflation (the levitation
//! carry-note, wvd.2 20:50) changes a number the day it appears:
//!
//! * `size-bytes` — the export artifact byte size (`write(env).len()`).
//! * `table-entries` — the decoded subterm-table entry count
//!   ([`gandr_kernel_core::DecodeMetrics::table_entries`]).
//! * `expanded-work` — the maximum per-declaration-root expanded (tree) size
//!   ([`MAX_EXPANDED_TERM_WORK`](gandr_kernel_core::MAX_EXPANDED_TERM_WORK)
//!   caps this).
//! * `artifact-work` — the artifact-total expanded (tree) size
//!   ([`MAX_ARTIFACT_EXPANDED_WORK`](gandr_kernel_core::MAX_ARTIFACT_EXPANDED_WORK)
//!   caps this; gandr-4p3).
//! * `artifact-identity` — the outer [`ArtifactIdentity`] (massive-term design
//!   §6, the b3sum-provenance successor): `BLAKE3` of the manifest binding the
//!   chunker commitment, record count, root node hash, and inner format
//!   version. This is the artifact-level provenance; the source-byte BLAKE3
//!   digest (`corpus-fixtures-b3sum`) is the input-level provenance — two
//!   layers, two planes.
//!
//! **Wall-clock decode time is advisory only** — emitted to stderr per item,
//! never persisted and never asserted (it is CI-flaky; pinning it would make
//! the gate flap). The persisted record carries only deterministic quantities.
//!
//! # Framing (delegated decision, recorded)
//!
//! W-A landed equivalent *coverage* split across three named tests: the
//! `kernel_corpus_partition` sweep (classification: which items are
//! S1-eligible, and the exclusion partition) and `kernel-core`'s `goldens.rs`
//! (the C5 round-trip witnesses). This harness **complements** them rather than
//! subsuming or wrapping: the partition owns *classification* and the goldens
//! own *round-trip fixed-point*, both distinct, already-serving concerns; this
//! gate owns *outcome + telemetry* (the D3 size/work floors and the artifact
//! identity). The three tests stay; this adds the telemetry layer, and adds the
//! missing b3sum fixture-provenance guard (W-A H3) to both single-manifest
//! sweeps via
//! [`corpus_fixtures_b3sum`](crate::common::corpus_fixtures_b3sum).
//! The C5 goldens are reconstructed here (via kernel-core's public arena API)
//! so one manifest carries the full driven corpus with the artifact identity
//! the `kernel-core` test cannot mint (the TCB wall forbids kernel-core
//! depending on the outer `storage-artifact` layer).
//!
//! # Cardinality
//!
//! A [`Gate`] Drop-guard (the `differential.rs` mechanism) asserts, even on an
//! early return, that the sweep processed exactly the pinned cardinality: 21
//! S1-eligible corpus items (an independent re-derivation of the partition's
//! eligible count) and 6 kernel-native goldens, and that every recorded outcome
//! line was consumed.

#[cfg(test)]
mod tests
{
    use std::fs;
    use std::path::PathBuf;
    use std::time::Instant;

    use gandr_core_checker::judgements::checker::infer_comp;
    use gandr_core_checker::judgements::checker::infer_value;
    use gandr_core_checker::kernel_bridge::BridgeContext;
    use gandr_core_checker::kernel_bridge::lower_computation_definition;
    use gandr_core_checker::kernel_bridge::lower_value_definition;
    use gandr_core_term::ctx::Ctx;
    use gandr_core_term::syntax::Term;
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::LevelParamCount;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::decode;
    use gandr_kernel_core::read;
    use gandr_kernel_core::write;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;
    use gandr_storage_artifact::build_from_environment;
    use gandr_storage_prolly_trees::InMemoryBlockStore;
    use gandr_storage_prolly_trees::TreeParams;

    use crate::common::CorpusTree;
    use crate::common::corpus_fixtures_b3sum;
    use crate::common::read_tree;

    /// The environment variable that switches the gate from verify to bless.
    const BLESS_ENV: &str = "GANDR_BLESS_KERNEL_EXPORT";

    /// The corpus trees the gate reads (also the b3sum-provenance scope).
    const TREES: [CorpusTree; 2] = [CorpusTree::MODEL, CorpusTree::PATHOLOGICAL];

    /// The pinned count of S1-eligible corpus items (an independent
    /// re-derivation of the `kernel_corpus_partition` eligible count).
    const ELIGIBLE_CARDINALITY: usize = 23;

    /// The pinned count of reconstructed kernel-native C5 goldens.
    const GOLDEN_CARDINALITY: usize = 6;

    /// One item's deterministic export outcome — every field a function of the
    /// canonical bytes, so it is safe to pin (advisory decode time is not
    /// here).
    struct Outcome
    {
        /// The export artifact byte size.
        size_bytes: usize,
        /// The decoded subterm-table entry count.
        table_entries: usize,
        /// The maximum per-declaration-root expanded (tree) size.
        expanded_work: u64,
        /// The artifact-total expanded (tree) size (gandr-4p3).
        artifact_work: u64,
        /// The outer artifact identity (BLAKE3 of the manifest), lowercase hex.
        identity: String,
    }

    /// Corpus identity of one exported item.
    #[derive(Clone, Copy)]
    struct ExportLocation<'source>
    {
        /// Corpus-relative source path.
        source: &'source str,
        /// Item ordinal within the source.
        index: usize,
    }

    impl Outcome
    {
        /// Renders the pinned record line: `source \t index \t size \t entries
        /// \t expanded \t artifact \t identity`.
        fn render(
            &self,
            location: ExportLocation<'_>,
        ) -> String
        {
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                location.source,
                location.index,
                self.size_bytes,
                self.table_entries,
                self.expanded_work,
                self.artifact_work,
                self.identity
            )
        }
    }

    /// The exit gate over the driven corpus: bless-or-verify state, plus the
    /// cardinality counters the Drop-guard checks.
    struct Gate
    {
        /// Whether `GANDR_BLESS_KERNEL_EXPORT` is set (re-bless mode).
        blessing: bool,
        /// The recorded outcome lines (verify mode's expectation).
        expected: Vec<String>,
        /// The produced outcome lines (bless mode's output).
        recorded: Vec<String>,
        /// The number of records verified so far (verify mode).
        cursor: usize,
        /// The number of S1-eligible corpus items processed.
        eligible: usize,
        /// The number of kernel-native goldens processed.
        goldens: usize,
    }

    impl Gate
    {
        /// Opens the gate, reading the pinned records unless blessing.
        fn new() -> Self
        {
            let blessing = std::env::var_os(BLESS_ENV).is_some();
            let expected = if blessing { Vec::new() } else { read_records() };
            Self {
                blessing,
                expected,
                recorded: Vec::new(),
                cursor: 0,
                eligible: 0,
                goldens: 0,
            }
        }

        /// Records (bless) or verifies (gate) one item's outcome line.
        fn record(
            &mut self,
            line: String,
        )
        {
            if self.blessing {
                self.recorded.push(line);
                return;
            }
            let expected = self.expected.get(self.cursor).unwrap_or_else(|| {
                panic!(
                    "export exit-gate manifest has no record for item {} ({line}); regenerate with \
                     {BLESS_ENV}=1",
                    self.cursor
                )
            });
            assert_eq!(
                &line, expected,
                "export exit-gate outcome drift at record {} (regenerate with {BLESS_ENV}=1 if the \
                 change is intended)",
                self.cursor
            );
            self.cursor = self.cursor.saturating_add(1);
        }
    }

    impl Drop for Gate
    {
        fn drop(&mut self)
        {
            // Never assert while unwinding a case failure — a double panic would
            // abort and hide the real assertion (the `differential.rs` rule).
            if std::thread::panicking() {
                return;
            }
            assert_eq!(
                ELIGIBLE_CARDINALITY, self.eligible,
                "the export exit gate drove {} S1-eligible corpus items but pinned {ELIGIBLE_CARDINALITY} \
                 (the partition changed); regenerate with {BLESS_ENV}=1",
                self.eligible
            );
            assert_eq!(
                GOLDEN_CARDINALITY, self.goldens,
                "the export exit gate drove {} kernel-native goldens but pinned {GOLDEN_CARDINALITY}",
                self.goldens
            );
            if self.blessing {
                write_records(&self.recorded);
                return;
            }
            assert_eq!(
                self.cursor,
                self.expected.len(),
                "the export exit-gate manifest pins {} records but the gate consumed {}; regenerate \
                 with {BLESS_ENV}=1",
                self.expected.len(),
                self.cursor
            );
        }
    }

    /// The exit gate holds: every driven item's export outcome matches its
    /// pinned record, and the driven cardinality is exactly what is pinned.
    #[test]
    fn the_kernel_export_exit_gate_holds()
    {
        let mut gate = Gate::new();

        // (1) The S1-eligible corpus partition, driven per item (never per file
        // — the files-vs-items unit pitfall, hazard-3).
        for tree in TREES {
            for fixture in &read_tree(tree) {
                for (index, term) in fixture.items.iter().enumerate() {
                    let Some(environment) = admit_eligible(term)
                    else {
                        continue;
                    };
                    gate.eligible = gate.eligible.saturating_add(1);
                    let label = format!("{} item {index}", fixture.source);
                    let outcome = measure(&environment, MeasurementLabel(&label));
                    gate.record(outcome.render(ExportLocation {
                        source: fixture.source.as_str(),
                        index,
                    }));
                }
            }
        }

        // (2) The kernel-native C5 goldens (universe/lift): no bridge counterpart,
        // reconstructed via kernel-core's public arena API. `goldens.rs` owns the
        // round-trip witness; this owns their telemetry + identity.
        for golden in c5_goldens() {
            gate.goldens = gate.goldens.saturating_add(1);
            let label = format!("{} item 0", golden.source);
            let outcome = measure(&golden.environment, MeasurementLabel(&label));
            gate.record(outcome.render(ExportLocation {
                source: golden.source,
                index: 0,
            }));
        }
    }

    /// Borrowed label for one measured export item.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct MeasurementLabel<'label>(&'label str);

    impl core::fmt::Display for MeasurementLabel<'_>
    {
        fn fmt(
            &self,
            f: &mut core::fmt::Formatter<'_>,
        ) -> core::fmt::Result
        {
            f.write_str(self.0)
        }
    }

    /// Runs one admitted environment through the full export pipeline — `write`
    /// → outer `ArtifactIdentity` → `decode` (metrics + advisory decode time) →
    /// `read` (replay) — asserting the byte-identical round-trip and returning
    /// the deterministic outcome.
    fn measure(
        environment: &Environment,
        label: MeasurementLabel<'_>,
    ) -> Outcome
    {
        let bytes = write(environment);
        let identity = artifact_identity(environment);

        let start = Instant::now();
        let decoded = decode(bytes.as_ref().into()).unwrap_or_else(|error| {
            panic!(
                "{}: an exit-gate artifact failed to decode: {error}",
                label.0
            )
        });
        let decode_micros = start.elapsed().as_micros();
        let metrics = decoded.metrics();

        // Replay: `read` re-admits through the choke point (K2/E3) and the whole
        // artifact round-trips byte-identically (the frozen fixed point).
        let reread = read(bytes.as_ref().into()).unwrap_or_else(|error| {
            panic!("{label}: an exit-gate artifact failed to replay: {error}")
        });
        assert_eq!(
            bytes,
            write(&reread),
            "{label}: an exit-gate artifact did not round-trip byte-identically"
        );

        // Advisory decode-time telemetry: emitted, never pinned (CI flakiness).
        eprintln!(
            "export exit-gate {label}: {} bytes, {} entries, expanded {}, artifact {}, decode ~{decode_micros}us (advisory)",
            bytes.len(),
            metrics.table_entries(),
            metrics.max_declaration_expanded_work(),
            metrics.artifact_expanded_work()
        );

        Outcome {
            size_bytes: bytes.len(),
            table_entries: usize::from(metrics.table_entries()),
            expanded_work: u64::from(metrics.max_declaration_expanded_work()),
            artifact_work: u64::from(metrics.artifact_expanded_work()),
            identity,
        }
    }

    /// The outer artifact identity (massive-term design §6) of an environment —
    /// `BLAKE3` of the manifest the `storage-artifact` layer mints, lowercase
    /// hex. This exercises the outer content-addressed layer end to end in the
    /// gate; the integrity wall never substitutes the K2/E3 replay wall above.
    fn artifact_identity(environment: &Environment) -> String
    {
        let mut store = InMemoryBlockStore::new();
        let built = build_from_environment(environment, TreeParams::default(), &mut store)
            .unwrap_or_else(|error| {
                panic!("an exit-gate artifact failed to build its identity: {error}")
            });
        format!("{}", built.identity())
    }

    /// Attempts the full bridge → infer → admit pipeline for one corpus item,
    /// returning the admitted environment when it is S1-eligible or `None`
    /// otherwise (an out-of-S1 node, a free name, a typing failure, or a choke-
    /// point rejection). The happy path of `kernel_corpus_partition::classify`,
    /// specialized to the gate's needs. Every failure abandons the staged
    /// builder, whose rollback truncates whatever the partial lowering minted
    /// — the discarded environment holds no orphan content.
    fn admit_eligible(term: &Term) -> Option<Environment>
    {
        let context = BridgeContext::new();
        let mut environment = Environment::new();
        let mut builder = environment.stage();
        let ids = match *term {
            | Term::Value(ref value) => {
                let core_type = infer_value(Ctx::new(), value.clone()).ok()?;
                lower_value_definition(&context, builder.arena(), value, &core_type).ok()?
            },
            | Term::Comp(ref comp) => {
                let core_type = infer_comp(Ctx::new(), comp.clone()).ok()?;
                lower_computation_definition(&context, builder.arena(), comp, &core_type).ok()?
            },
        };
        let (declared_id, body_id) = ids;
        let declaration = builder.def(LevelSignature::monomorphic(), declared_id, body_id);
        environment.add_decl(declaration).ok()?;
        Some(environment)
    }

    // ----- The kernel-native C5 goldens (reconstructed; `goldens.rs` is the
    // round-trip authority). -----

    /// Constant universe level fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct ConstantLevel(LevelConstant);

    impl From<u64> for ConstantLevel
    {
        fn from(value: u64) -> Self
        {
            Self(LevelConstant::from(value))
        }
    }

    /// The constant level `value`.
    fn constant(value: ConstantLevel) -> Level
    {
        Level::constant(value.0)
    }

    /// Prenex level-variable fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct VariableLevel(LevelVarIndex);

    impl From<u32> for VariableLevel
    {
        fn from(index: u32) -> Self
        {
            Self(LevelVarIndex::from(index))
        }
    }

    /// The level of the prenex variable `index`.
    fn level_var(index: VariableLevel) -> Level
    {
        Level::var(LevelVar::new(index.0))
    }

    /// A monomorphic level signature.
    fn mono() -> LevelSignature
    {
        LevelSignature::monomorphic()
    }

    /// One named kernel-native golden environment.
    struct GoldenEnvironment
    {
        /// Stable record source.
        source: &'static str,
        /// Kernel environment exported by the gate.
        environment: Environment,
    }

    /// The six C5 goldens, each a `(record-source, environment)` pair, in a
    /// fixed order — mirroring `gandr_kernel_core`'s `tests/goldens.rs`.
    fn c5_goldens() -> Vec<GoldenEnvironment>
    {
        vec![
            GoldenEnvironment {
                source: "golden/universe-constant-axiom",
                environment: golden_universe_constant_axiom(),
            },
            GoldenEnvironment {
                source: "golden/universe-variable-axiom",
                environment: golden_universe_variable_axiom(),
            },
            GoldenEnvironment {
                source: "golden/lift-type-axiom",
                environment: golden_lift_type_axiom(),
            },
            GoldenEnvironment {
                source: "golden/lift-value-definition",
                environment: golden_lift_value_definition(),
            },
            GoldenEnvironment {
                source: "golden/universe-in-arrow-domain",
                environment: golden_universe_in_arrow_domain(),
            },
            GoldenEnvironment {
                source: "golden/universe-and-lift-environment",
                environment: golden_universe_and_lift_environment(),
            },
        ]
    }

    /// `Axiom : U_0`.
    fn golden_universe_constant_axiom() -> Environment
    {
        let mut environment = Environment::new();
        let declaration = {
            let mut builder = environment.stage();
            let universe = builder.arena().value_type_universe(constant((0).into()));
            builder.axiom(mono(), universe)
        };
        environment
            .add_decl(declaration)
            .expect("U_0 is a well-formed declared type");
        environment
    }

    /// `Axiom : U_{x0}` over one prenex level parameter.
    fn golden_universe_variable_axiom() -> Environment
    {
        let mut environment = Environment::new();
        let levels = LevelSignature::new(LevelParamCount::from(1_u32), Vec::new());
        let declaration = {
            let mut builder = environment.stage();
            let universe = builder.arena().value_type_universe(level_var((0).into()));
            builder.axiom(levels, universe)
        };
        environment
            .add_decl(declaration)
            .expect("U_{x0} is well-formed under one prenex parameter");
        environment
    }

    /// `Axiom : Lift Unit 1`.
    fn golden_lift_type_axiom() -> Environment
    {
        let mut environment = Environment::new();
        let declaration = {
            let mut builder = environment.stage();
            let lifted = {
                let arena = builder.arena();
                let unit = arena.value_type_unit();
                arena.value_type_lift(unit, constant((1).into()))
            };
            builder.axiom(mono(), lifted)
        };
        environment
            .add_decl(declaration)
            .expect("Lift Unit 1 is a well-formed declared type (0 < 1)");
        environment
    }

    /// `Def (Lift Unit 1) = lift_1 unit`.
    fn golden_lift_value_definition() -> Environment
    {
        let mut environment = Environment::new();
        let declaration = {
            let mut builder = environment.stage();
            let (declared, body) = {
                let arena = builder.arena();
                let unit_type = arena.value_type_unit();
                let declared = arena.value_type_lift(unit_type, constant((1).into()));
                let unit = arena.value_unit();
                let body = arena.value_lift(constant((1).into()), unit);
                (declared, body)
            };
            builder.def(mono(), declared, body)
        };
        environment
            .add_decl(declaration)
            .expect("lift_1 unit inhabits Lift Unit 1");
        environment
    }

    /// `Axiom : U (U_0 -> F Unit)` — a universe nested as an arrow domain.
    fn golden_universe_in_arrow_domain() -> Environment
    {
        let mut environment = Environment::new();
        let declaration = {
            let mut builder = environment.stage();
            let declared = {
                let arena = builder.arena();
                let unit = arena.value_type_unit();
                let returner = arena.comp_type_returner(unit);
                let universe = arena.value_type_universe(constant((0).into()));
                let arrow = arena.comp_type_arrow(universe, returner);
                arena.value_type_thunk(arrow)
            };
            builder.axiom(mono(), declared)
        };
        environment
            .add_decl(declaration)
            .expect("U (U_0 -> F Unit) is a well-formed declared type");
        environment
    }

    /// A whole environment of C5 goldens — a universe axiom, a lift axiom, and
    /// a lift definition — the admission-ordered multi-declaration
    /// artifact.
    fn golden_universe_and_lift_environment() -> Environment
    {
        let mut environment = Environment::new();

        let universe = {
            let mut builder = environment.stage();
            let universe = builder.arena().value_type_universe(constant((0).into()));
            builder.axiom(mono(), universe)
        };
        environment.add_decl(universe).expect("U_0 admits");

        let lift_type = {
            let mut builder = environment.stage();
            let lifted = {
                let arena = builder.arena();
                let unit = arena.value_type_unit();
                arena.value_type_lift(unit, constant((2).into()))
            };
            builder.axiom(mono(), lifted)
        };
        environment.add_decl(lift_type).expect("Lift Unit 2 admits");

        let lift_value = {
            let mut builder = environment.stage();
            let (declared, body) = {
                let arena = builder.arena();
                let unit_type = arena.value_type_unit();
                let declared = arena.value_type_lift(unit_type, constant((1).into()));
                let unit = arena.value_unit();
                let body = arena.value_lift(constant((1).into()), unit);
                (declared, body)
            };
            builder.def(mono(), declared, body)
        };
        environment
            .add_decl(lift_value)
            .expect("lift_1 unit admits");

        environment
    }

    // ----- The manifest (records + b3sum fixture provenance). -----

    /// The checked-in exit-gate manifest path.
    fn manifest_path() -> PathBuf
    {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/kernel_export_gate.manifest"
        ))
    }

    /// Reads the pinned record lines, verifying the `corpus-fixtures-b3sum`
    /// provenance header still matches the live fixtures (the W-A H3 guard),
    /// then returns the data lines in file order.
    fn read_records() -> Vec<String>
    {
        let path = manifest_path();
        let text = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "cannot read the export exit-gate manifest `{}` ({error}); regenerate with \
                 {BLESS_ENV}=1",
                path.display()
            )
        });
        let recorded = header_field(HeaderQuery {
            text: &text,
            name: "corpus-fixtures-b3sum",
        })
        .unwrap_or_else(|| {
            panic!(
                "export exit-gate manifest `{}` is missing its `corpus-fixtures-b3sum` provenance \
                 header",
                path.display()
            )
        });
        let live = corpus_fixtures_b3sum(&TREES);
        assert_eq!(
            recorded,
            live,
            "export exit-gate manifest `{}` is stale (recorded corpus b3sum != live); regenerate \
             with {BLESS_ENV}=1",
            path.display()
        );
        text.lines()
            .filter(|line| !line.starts_with(';'))
            .map(str::to_owned)
            .collect()
    }

    /// Writes the exit-gate manifest from the produced records (bless path).
    fn write_records(records: &[String])
    {
        let mut lines = vec![
            String::from(
                "; gandr kernel export exit-gate outcomes (B2.3 deliverable 1; \
                 GANDR_BLESS_KERNEL_EXPORT)",
            ),
            String::from(
                "; generator: gandr-core-sequent kernel_export_gate (GANDR_BLESS_KERNEL_EXPORT=1)",
            ),
            format!(
                "; items: {} ({ELIGIBLE_CARDINALITY} S1-eligible corpus + {GOLDEN_CARDINALITY} \
                 kernel-native C5 goldens)",
                records.len()
            ),
            format!("; corpus-fixtures-b3sum: {}", corpus_fixtures_b3sum(&TREES)),
            String::from(
                "; columns: <source>\\t<item-index>\\t<size-bytes>\\t<table-entries>\
                 \\t<expanded-work>\\t<artifact-work>\\t<artifact-identity>",
            ),
        ];
        lines.extend(records.iter().cloned());
        let mut out = lines.join("\n");
        out.push('\n');
        let path = manifest_path();
        fs::write(&path, out)
            .unwrap_or_else(|error| panic!("cannot write `{}`: {error}", path.display()));
    }

    /// One export-manifest provenance-header lookup.
    #[derive(Clone, Copy)]
    struct HeaderQuery<'header>
    {
        /// Whole manifest text.
        text: &'header str,
        /// Header field name.
        name: &'header str,
    }

    /// The value of a `; <name>: <value>` provenance header line, trimmed.
    fn header_field(query: HeaderQuery<'_>) -> Option<String>
    {
        let needle = format!("; {}:", query.name);
        query
            .text
            .lines()
            .find_map(|line| line.strip_prefix(&needle))
            .map(|rest| rest.trim().to_owned())
    }
}
