---
name: reference-project-convention-drift
description: "Use when checking or aligning a project's conventions against its declared reference project or the vendored agentic-dev core, including guidance docs, gate configs, coding-convention docs, and work tracking. Reads the consumer's .agents/conventions.toml manifest."
---

# Reference Project Convention Drift Check

Use when asked to align this project's conventions (workflow, coding, gate configs) with its reference project or the shared core, or when drift between them may matter.

## Parameterization — read the manifest first

1. Read `.agents/conventions.toml` in the current project root.
   It declares:
   + `[project]` — this project's name and languages;
   + `[core]` — where the agentic-dev core is vendored (default `.agents/core`);
   + `[reference]` — the reference project and its sibling-checkout root (may be absent: then the core is the only reference);
   + `[surfaces]` — the guidance docs, gate configs, and JSON provisioning fragments maintained as copies or deltas;
   + `[gates]` — the project's documented verification commands;
   + `[tracker]` — the beads issue prefix.
2. If the manifest is missing, say so, propose one from `fragments/conventions.example.toml` in the core, and proceed with what the request and repo context give you.
3. Treat memory as a hint, not current truth: read the reference's _current_ convention sources before comparing.

## Procedure

1. Identify the drift axes in scope: the vendored core (pin currency, fragment copies) and/or the reference project (convention docs, configs, code style).
2. For the core axis: run the core-pin gate (`nu <core>/scripts/check-core-pin.nu`), then diff the project's copy-basis configs (prek, wt, tool configs) against the core's `fragments/` — copies drift silently; the fragments are the canonical shapes.
   Commitlint is not copy-basis: check instead that the consumer's config imports the core's `makeConfig` and keeps only the scope vocabulary local.
   `.claude/settings.json` is a provisioning fragment, checked by **subset**, not equality: the consumer's copy may carry more, but must not be _missing_ what `fragments/claude-settings.base.json` provisions — the beads context hooks (PreCompact `bd prime`; SessionStart pull-then-prime, the H2 read-side mechanization), the worktrunk `extraKnownMarketplaces` / `enabledPlugins`, and the `wt` `statusLine`.
   Flag any of these absent from the consumer's copy: a consumer born before the fragment landed silently misses what it provisions, and the gap stays invisible until noticed by hand.
3. For the reference axis: read the reference project's current convention sources first (its workflow/coding docs and representative source files/tests), then map the local surface with targeted searches, not file-by-file browsing — the `[surfaces]` lists say where to look.
4. Classify each divergence: **intentional delta** (documented in this project's AGENTS.md or ADR — leave, but ensure it is documented), **stale copy** (align to reference/core), or **local improvement** (upstream-first: file it as an upstreaming candidate through the consumer→core feedback channel — a bead in the core's tracker, `<core>/core/FEEDBACK.md` — per the read-only discipline; keep a consumer-local copy only as a documented exception).
5. Track substantial alignment work in the project's durable tracker (the `[tracker]` prefix).
   Prefer an epic with child tasks when the work spans sessions or many files.
6. Delegate parallel slices when the change spans many files.
   Subagents edit only; the orchestrator runs formatters and gates once over the combined change.
7. Verify with the project's documented gates (`[gates].check`), narrowest first.
8. Update changelog/docs-manifest artifacts when the project maintains them and visible docs or behavior changed.
9. If the repo workflow expects commits: commit a clean coherent slice with the required canonical trailer, update tracker items with verification notes, sync tracker state, and report tool caveats.

## Language-specific notes (apply when the project's languages include them)

* **Rust** (design-by-contract convention, where the reference uses it): clauses in order `- requires:`, `- ensures:`, `- provides:`, `- fails:`, `- panics:`; keep `# Errors` for `Result` functions; remove low-value contract blocks on constants, trivial accessors, thin wrappers, and obvious data holders.
  For dependencies, use the `find-best-rust-crates` skill; `proptest` conventionally keeps `default-features = true` (test runtime lives behind defaults), dev-only.
* **TypeScript**: use the `find-best-typescript-packages` skill for dependency decisions; keep dependency scopes narrow.

## Known caveats

* Periodic drift checks are a core principle (`core/PRINCIPLES.md` §"Drift checks are periodic, not incidental"): suggest one after long gaps, before large refactors, or when a convention question has no local answer — do not wait to be asked.
* Avoid broad reference-project lint-posture imports unless specifically scoped; they can create unrelated churn.
* An intentional, documented delta is not drift — do not "fix" it; an undocumented delta is drift even when it is better (document it or land it upstream).
