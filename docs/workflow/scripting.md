# Workflow: project tooling and diagnostics

> Read when: changing the project-local gate driver, a shared-core script call, a tooling regression surface, or compiler/linter diagnostic handling.
> Base discipline: `.agents/core/core/WORKFLOW.md` §"Scripting".

## Keep the ownership boundary explicit

Project gate policy and landing orchestration live in the Rust crate `crates/workflow-gates` (package `gandr-workflow-gates`).
Its stable human, hook, and (future) CI entrypoints are the task names in `mise.toml`; task bodies route into `cargo run --quiet -p gandr-workflow-gates -- <subcommand>`.
Use a stable `mise run <task>` at callsites so command spelling, arguments, and prerequisites have one owner; invoke the CLI directly only when implementing or diagnosing that boundary.

Small and pipeline-shaped work that is **not** gate policy stays in typed **Nushell** under `scripts/` (project policy: typed only — Nushell for small/pipeline scripts, TypeScript for larger; no untyped `bash`/`sh`).
Each is reached through its named `mise` task or `prek` hook, never inlined:

* `scripts/check-conflict-markers.nu` — the `docs:conflict-markers` gate and its pre-commit hook;
* `scripts/check-machine-local-paths.nu` — the `no-machine-local-paths` pre-commit hook (publishable-history backstop);
* `scripts/commitlint-range.nu` and `scripts/check-signed-commits.nu` — the pre-push range checks;
* `scripts/lib/git.nu`, `scripts/lib/push-range.nu` — shared helpers;
* `scripts/refs-yml/*.nu` — the `docs/spec/refs.yml` generator ([docs.md](docs.md)).

The vendored agentic-dev core is a separate ownership domain that is **not yet vendored in the reboot**: `.agents/core` does not exist, so its shared-core calls (`core:check`, `core-init`, the Worktrunk ADR guard) are parked and re-grow with the core.
Until then, the checks that would delegate to the core run as the project-local Nushell above.
Do not revive retired project gate scripts, and do not fold a typed helper into the Rust crate ad hoc — graduate one into `gandr-workflow-gates` only when it becomes gate policy.

## One typed CLI, domain-owned modules

`src/main.rs` owns only the typed command grammar, dispatch, stable exit classes, and process boundary.
Library modules own the work: contracts and CI policy, documentation gates, graph and source policy, project dependency policy, coverage, maintenance ranges, contained mutation, workflow planning, and shared process/filesystem support.
This keeps every operation callable through one binary without concentrating domain logic in a stringly dispatcher.

Prefer the narrow stable task that proves the change:

* `mise run docs:manifest-drift` and `mise run docs:reference-integrity` for corpus documentation;
* `mise run test:options-policy`, `mise run test:soundness-oracles`, `mise run test:graph-gates`, and `mise run test:dep-graph` for policy surfaces;
* `mise run coverage:check` and `mise run coverage:ratchet` for per-file coverage policy;
* the fixed landing tiers — `mise run gate:merge` for the merge wall, and the `cargo run --quiet -p gandr-workflow-gates -- workflow push` plan (parked during the reboot, [ci.md](ci.md)) — plus the `mise run mutants:*` family for mutation modes.

Semantic violations are stable Rust `Finding` values; malformed input, I/O, subprocess, and containment failures are typed errors.
Both paths fail closed at the CLI boundary.

## Tests, benchmarks, and fuzzing

The crate's regression surface is Rust:

* `cargo nextest run -p gandr-workflow-gates` runs its composed integration suite; the stable `mise run test:*` tasks add live fixtures where the policy requires them.
* `cargo bench -p gandr-workflow-gates --bench commands` measures representative command surfaces without turning performance numbers into gate semantics.
* The feature-gated parser facade feeds the independent AFL++ `gates` target.
  `mise run fuzz:gates` runs that campaign, while `mise run fuzz:rust-smoke` deterministically replays every committed seed across all five Rust targets.

Keep pure parsing/planning separate from side effects so tests, the benchmark, and the fuzz target exercise the same decisions as the CLI.
Every fix needs a regression witness for its observable contract; do not recreate standalone Nushell test runners beside the Rust suite.

## Git and subprocesses are sanitized by construction

Hook runners can export `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE`, and object/ref overrides that redirect a child Git process away from its requested working directory.
Project Rust domains therefore pass exact argv vectors through the shared support boundary and request Git sanitization; repository-control variables are removed before spawn.
The project-local Nushell helpers use `scripts/lib/git.nu` for the same reason.

Do not add shell-string interpolation or an ad hoc `Command` wrapper.
Use the shared status path when output is purely live, and the shared output path when a domain must parse stdout: stdout is streamed while a bounded semantic copy is retained, stderr remains live, and over-limit output is drained before returning a typed failure.
Extend that seam and its child-process fixtures when a new process contract is required.

## Diagnostics go through aifix

aifix is the diagnostic adapter, not the pass/fail gate.
For a project-wide Rust sweep, use the MCP batch surface with profile `auto`; discover profiles before selecting a narrower one, and use `aifix_pipeline` for diagnostics already captured in a structured stream.
It normalizes, deduplicates, and groups findings; replay applies only explicitly recorded project-local fixes.

Reserve `mise run cargo:clippy` (`-D warnings`) for the binary gate.
The `agda:check` task owns its `aifix batch agda` invocations and then calls the Rust OPTIONS-policy sweep.
Best-effort posture stands ([docs.md](docs.md)): aifix improves triage but never licenses degrading an artifact to silence a finding.
