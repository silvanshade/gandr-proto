//! Parity ledger for the retired Nushell regression suite.
//!
//! The table is intentionally executable so a future cutover cannot silently
//! drop a legacy assertion: every row names either the Rust witness that now
//! owns the observable contract or the rationale for an intentional omission.

extern crate alloc;

use alloc::collections::BTreeSet;

/// One legacy Nushell assertion group and its Rust witness mapping.
struct LegacyParityRow
{
    /// Legacy regression file under `scripts/tests`.
    legacy_file: &'static str,
    /// Observable behavior asserted by the legacy test.
    behavior: &'static str,
    /// Named Rust witnesses that own the behavior.
    rust_witnesses: &'static [&'static str],
    /// Rationale when the behavior is intentionally outside this Rust suite.
    omitted: Option<&'static str>,
}

/// Legacy regression files audited for the Rust gate cutover.
const LEGACY_FILES: [&str; 9] = [
    "act-ci-stamps.test.nu",
    "check-docs-gates.test.nu",
    "check-nu-lint-drift.test.nu",
    "check-options-policy.test.nu",
    "check-page-balance.test.nu",
    "coverage-ratchet.test.nu",
    "gate-parity.test.nu",
    "mutants-vm-scheduled.test.nu",
    "scheduled-campaigns.test.nu",
];

/// Exhaustive parity table for the nine legacy Nushell regression files.
const PARITY_ROWS: &[LegacyParityRow] = &[
    LegacyParityRow {
        legacy_file: "gate-parity.test.nu",
        behavior: "CI and hook run bodies preserve a one-command mise boundary and reject shell composition for owned gates.",
        rust_witnesses: &[
            "ci_contracts::rejects_noncanonical_real_work_shapes",
            "ci_contracts::diagnostics_name_job_step_and_action",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "gate-parity.test.nu",
        behavior: "Local merge and push plans keep deterministic direct-task sequences, unique task names, and push-as-merge-prefix semantics.",
        rust_witnesses: &[
            "workflow::tests::merge_plan_order_is_exact",
            "workflow::tests::push_plan_order_is_exact",
            "workflow::tests::workflow_plan_projection_is_canonical_for_any_tier",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "gate-parity.test.nu",
        behavior: "The retained CLI command inventory and parseable command modes stay fixed at the Rust binary boundary.",
        rust_witnesses: &[
            "tooling::top_level_command_inventory_is_exact",
            "tooling::workflow_plan_selection_is_typed_without_execution",
            "tooling::fuzz_smoke_plan_inventory_is_exact",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "gate-parity.test.nu",
        behavior: "Exact mise, prek, and Worktrunk config entries remain wired to project-local commands.",
        rust_witnesses: &[],
        omitted: Some(
            "Configuration cutover is owned by the tooling/workflow config branches; this assignment is limited to Rust gate tests and does not edit tool configs.",
        ),
    },
    LegacyParityRow {
        legacy_file: "mutants-vm-scheduled.test.nu",
        behavior: "Merge, push, and scheduled mutation campaigns use three-dot diff ranges with validated lower and upper endpoints.",
        rust_witnesses: &[
            "mutants::range::tests::merge_diff_uses_main_three_dot_head",
            "mutants::range::tests::push_range_modes_render_exact_three_dot_specs",
            "mutants::range::tests::scheduled_range_renders_resolved_three_dot_diff",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "mutants-vm-scheduled.test.nu",
        behavior: "Scheduled mutation inputs reject malformed ref tokens, invalid object IDs, stale HEADs, and reversed or unrelated topology.",
        rust_witnesses: &[
            "mutants::range::tests::scheduled_ref_tokens_reject_malformed_inputs",
            "mutants::range::tests::scheduled_ref_token_ascii_whitelist_is_exact",
            "mutants::range::tests::scheduled_range_rejects_invalid_oid_and_topology",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "mutants-vm-scheduled.test.nu",
        behavior: "Non-Rust diffs produce an explicit no-op campaign report without booting the microVM.",
        rust_witnesses: &[
            "mutants::range::tests::non_rust_diff_is_noop_campaign",
            "mutants::sandbox::tests::non_rust_diff_skips_vm_and_writes_report",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "mutants-vm-scheduled.test.nu",
        behavior: "Containment flags, cache mounts, timeouts, worker counts, and sequential execution stay hard-coded in the microVM plan.",
        rust_witnesses: &[
            "mutants::sandbox::tests::sandbox_boot_plan_has_no_forbidden_host_mounts",
            "mutants::sandbox::tests::timeout_caps_and_sequential_jobs_are_planned",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "mutants-vm-scheduled.test.nu",
        behavior: "Report publication is atomic: a failed final rename rolls back to the previous report and leaves the staged report inspectable.",
        rust_witnesses: &[
            "mutants::report::tests::successful_publication_replaces_current_report",
            "mutants::report::tests::simulated_final_rename_failure_restores_prior_report",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "mutants-vm-scheduled.test.nu",
        behavior: "Cache scratch-volume cleanup failures are hard errors and do not claim cache reuse.",
        rust_witnesses: &["mutants::sandbox::tests::cache_cleanup_failure_is_hard_error"],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "scheduled-campaigns.test.nu",
        behavior: "Weekly and monthly workflow YAML keeps the expected cron triggers, dispatch trigger, concurrency group, runner labels, artifact paths, and job wiring.",
        rust_witnesses: &[],
        omitted: Some(
            "Workflow YAML configuration is not parsed by the Rust gate crate; cutover workflow config tests own this assertion.",
        ),
    },
    LegacyParityRow {
        legacy_file: "scheduled-campaigns.test.nu",
        behavior: "Weekly and monthly maintenance ranges prefer explicit base refs, then watermarks, then age-based first-parent selection.",
        rust_witnesses: &[
            "maintenance::tests::precedence_prefers_explicit_then_watermark_then_auto",
            "maintenance::tests::merge_topology_uses_first_parent_cutoff",
            "maintenance::tests::no_old_base_fails_closed",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "scheduled-campaigns.test.nu",
        behavior: "Maintenance watermarks reject missing, malformed, stale, or non-ancestor inputs and advance atomically only after success.",
        rust_witnesses: &[
            "maintenance::tests::missing_and_invalid_watermarks_fail_closed",
            "maintenance::tests::invalid_oid_and_timestamp_are_rejected",
            "maintenance::tests::non_ancestor_ranges_fail_closed",
            "maintenance::tests::current_head_expectation_rejects_stale_upper",
            "maintenance::tests::atomic_advancement_writes_schema_one_upper",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "scheduled-campaigns.test.nu",
        behavior: "Scheduled campaign run bodies do not call cargo fuzz, cargo mutants, mutants-vm.nu, or mutants-guest.nu directly.",
        rust_witnesses: &[],
        omitted: Some(
            "This is a workflow-source wiring assertion, not a Rust-gate behavior; config cutover tests own the YAML body shape.",
        ),
    },
    LegacyParityRow {
        legacy_file: "act-ci-stamps.test.nu",
        behavior: "Native merge and push workflow execution runs end to end against a temporary local Git remote without touching real remotes.",
        rust_witnesses: &[
            "workflow::tests::merge_and_push_workflows_run_inside_local_git_remote_fixture",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "act-ci-stamps.test.nu",
        behavior: "Workflow execution is fail-fast and reports the failed task with stable human-facing detail.",
        rust_witnesses: &[
            "workflow::tests::execution_stops_after_first_nonzero_task_and_reports_context",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "act-ci-stamps.test.nu",
        behavior: "Legacy act-ci stamp caches, lock directories, pruning, linked-worktree stamp sharing, and stale-lock recovery keep their exact file-level behavior.",
        rust_witnesses: &[],
        omitted: Some(
            "The Rust replacement does not retain the Act stamp cache wrapper; future typed cache plans will cover native caching without reintroducing this retired surface.",
        ),
    },
    LegacyParityRow {
        legacy_file: "act-ci-stamps.test.nu",
        behavior: "Legacy Docker, user .actrc, submodule-worktree, and untracked-commit safeguards remain in the Nushell act wrapper.",
        rust_witnesses: &[],
        omitted: Some(
            "Those assertions protect the retired Act wrapper and are intentionally not ported to the Rust local merge/push workflow tests.",
        ),
    },
    LegacyParityRow {
        legacy_file: "check-docs-gates.test.nu",
        behavior: "Documentation manifest drift reports clean corpora, changed nodes, missing nodes, unregistered Markdown, wrong hashes, vacuous manifests, and malformed edge records deterministically.",
        rust_witnesses: &[
            "docs::manifest::tests::clean_manifest_has_no_drift_findings",
            "docs::manifest::tests::drift_missing_and_unregistered_docs_are_reported",
            "docs::manifest::tests::empty_manifest_nodes_fail_loudly",
            "docs::manifest::tests::malformed_manifest_inputs_fail_closed",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "check-docs-gates.test.nu",
        behavior: "Documentation reference integrity reports dangling edge anchors, missing ADR records, unresolved section references, and zero-padded ADR filenames.",
        rust_witnesses: &[
            "docs::references::tests::dangling_edge_adr_and_section_refs_are_reported",
            "docs::references::tests::adr_filename_numbers_resolve_references",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "check-docs-gates.test.nu",
        behavior: "Rumdl planning fails before process launch when Markdown files contain conflict markers and preserves caller argument order when clean.",
        rust_witnesses: &[
            "docs::commands::tests::conflict_markers_block_rumdl_planning",
            "docs::commands::tests::clean_markdown_files_preserve_argument_order",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "check-docs-gates.test.nu",
        behavior: "Soundness oracle witnesses reject missing companions, substring tags, and tag-precedence mistakes while tolerating cfg/ignore attributes.",
        rust_witnesses: &[
            "source_policy::tests::soundness_cfg_and_ignore_attributes_do_not_hide_test_marker",
            "source_policy::tests::soundness_missing_witnesses_are_reported_deterministically",
            "source_policy::tests::soundness_tags_must_be_exact_doc_items",
            "source_policy::tests::soundness_tag_precedence_makes_witness_tag_free",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "check-docs-gates.test.nu",
        behavior: "The Nushell gate-message helper itself emits PASS/FAIL runner summaries.",
        rust_witnesses: &[],
        omitted: Some(
            "This is a legacy test harness behavior, not an observable Rust gate contract.",
        ),
    },
    LegacyParityRow {
        legacy_file: "check-nu-lint-drift.test.nu",
        behavior: "The nu-lint task is enabled, pinned in mise.lock, and validates the default-empty-string lint across legacy Nushell scripts.",
        rust_witnesses: &[],
        omitted: Some(
            "The Rust gate crate has no Nushell lint surface, and this assignment must not edit tool configs or delete the legacy Nushell files.",
        ),
    },
    LegacyParityRow {
        legacy_file: "check-options-policy.test.nu",
        behavior: "OPTIONS scanning reports per-flag violations, respects per-flag exemptions, and fails on vacuous scan roots.",
        rust_witnesses: &[
            "source_policy::tests::options_truth_table_property",
            "source_policy::tests::options_exemptions_are_per_flag",
            "source_policy::tests::options_vacuous_roots_fail_per_flag",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "check-page-balance.test.nu",
        behavior: "Typst page-balance probes flag only rows strictly inside the bottom band and ignore exact-boundary or empty-band rows.",
        rust_witnesses: &[
            "docs::commands::tests::strict_page_band_boundary_excludes_exact_threshold",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "coverage-ratchet.test.nu",
        behavior: "Coverage summary paths normalize to repo-relative production Rust keys while absolute paths, test roots, and non-Rust paths fail closed.",
        rust_witnesses: &[
            "coverage::model::tests::production_floor_keys_are_strict",
            "coverage::model::tests::production_floor_key_strategy_accepts_only_canonical_source_paths",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "coverage-ratchet.test.nu",
        behavior: "Percent parsing floors exact counts, rejects hidden precision, and round-trips only canonical two-decimal policy values.",
        rust_witnesses: &[
            "coverage::model::tests::percent_floors_down_without_float_math",
            "coverage::model::tests::percent_parser_rejects_hidden_precision",
            "coverage::model::tests::percent_from_counts_matches_integer_flooring",
            "coverage::model::tests::percent_parser_round_trips_canonical_policy_values",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "coverage-ratchet.test.nu",
        behavior: "Coverage policy reports missing floors, regressions, floor decreases, target-clamp violations, stale floors, and exact new-file exemptions.",
        rust_witnesses: &[
            "coverage::policy::tests::check_reports_policy_failures",
            "coverage::policy::tests::explicit_new_file_exemptions_use_measured_baselines",
            "coverage::policy::tests::base_history_fails_closed",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "coverage-ratchet.test.nu",
        behavior: "Coverage ratcheting never lowers floors, caps increases at the target, retains stale floors, emits deterministic TOML, and keeps line-count metrics authoritative.",
        rust_witnesses: &[
            "coverage::policy::tests::ratchet_report_caps_increases_and_keeps_stale_floors",
            "coverage::render::tests::render_floors_is_sorted_and_stable",
            "coverage::policy::tests::line_count_schema_failures_are_operational",
        ],
        omitted: None,
    },
    LegacyParityRow {
        legacy_file: "coverage-ratchet.test.nu",
        behavior: "Malformed JSON, malformed TOML, base/current floor key mismatches, and policy target-shape errors fail closed with named inputs.",
        rust_witnesses: &[
            "coverage::policy::tests::malformed_inputs_fail_closed",
            "coverage::policy::tests::base_current_floor_union_is_enforced",
            "coverage::policy::tests::floor_policy_shape_is_exact",
        ],
        omitted: None,
    },
];

/// The parity table covers every legacy file and every row is actionable.
#[test]
fn legacy_nushell_regression_parity_table_is_exhaustive()
{
    let expected = LEGACY_FILES.iter().copied().collect::<BTreeSet<_>>();
    let seen = PARITY_ROWS
        .iter()
        .map(|row| row.legacy_file)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        seen, expected,
        "all nine legacy regression files must be mapped"
    );
    for row in PARITY_ROWS {
        assert!(
            !row.behavior.trim().is_empty(),
            "{} contains an empty behavior row",
            row.legacy_file
        );
        assert!(
            !row.rust_witnesses.is_empty() || row.omitted.is_some(),
            "{} behavior `{}` needs a Rust witness or omission rationale",
            row.legacy_file,
            row.behavior
        );
        for witness in row.rust_witnesses {
            assert!(
                witness.contains("::"),
                "witness `{witness}` should be a named Rust module/test path"
            );
        }
        if let Some(rationale) = row.omitted {
            assert!(
                !rationale.trim().is_empty(),
                "{} omission for `{}` must explain why it is intentional",
                row.legacy_file,
                row.behavior
            );
        }
    }
}
